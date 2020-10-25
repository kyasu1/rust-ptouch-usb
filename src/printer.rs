use log::{debug, error, info, warn};
use rusb::{Context, Device, DeviceDescriptor, DeviceHandle, Direction, TransferType, UsbContext};
use std::time::Duration;

use crate::{
    error::{Error, PrinterError},
    media::Media,
    model::Model,
};

#[macro_use]
use bitflags::bitflags;

#[derive(Debug, Clone, Copy)]
struct Endpoint {
    config: u8,
    iface: u8,
    setting: u8,
    address: u8,
}

pub struct Printer {
    handle: Box<DeviceHandle<Context>>,
    endpoint_out: Endpoint,
    endpoint_in: Endpoint,
}

struct PrinterInner {
    handle: Box<DeviceHandle<Context>>,
    endpoint_out: Endpoint,
    endpoint_in: Endpoint,
}

pub trait Printable: Clone + Sized {
    fn cancel() -> Result<(), Error>;
}

impl Printer {
    pub fn new(model: Model, serial: String) -> Result<Self, Error> {
        rusb::set_log_level(rusb::LogLevel::Debug);
        match Context::new() {
            Ok(mut context) => match Self::open_device(&mut context, 0x04F9, model.pid(), serial) {
                Some((mut device, device_desc, mut handle)) => {
                    handle.reset()?;

                    let endpoint_in = match Self::find_endpoint(
                        &mut device,
                        &device_desc,
                        Direction::In,
                        TransferType::Bulk,
                    ) {
                        Some(endpoint) => endpoint,
                        None => return Err(Error::MissingEndpoint),
                    };

                    let endpoint_out = match Self::find_endpoint(
                        &mut device,
                        &device_desc,
                        Direction::Out,
                        TransferType::Bulk,
                    ) {
                        Some(endpoint) => endpoint,
                        None => return Err(Error::MissingEndpoint),
                    };

                    // QL-800では`has_kernel_driver`が`true`となる
                    // QL-820NWBでは`has_kernel_driver`が`false`となる
                    // `has_kernel_driver`が`true`の場合に、カーネルドライバーをデタッチしないとエラーとなる
                    //
                    handle.set_auto_detach_kernel_driver(true)?;
                    let has_kernel_driver = match handle.kernel_driver_active(0) {
                        Ok(true) => {
                            handle.detach_kernel_driver(0).ok();
                            true
                        }
                        _ => false,
                    };
                    info!(" Kernel driver support is {}", has_kernel_driver);
                    handle.set_active_configuration(1)?;
                    handle.claim_interface(0)?;
                    handle.set_alternate_setting(0, 0)?;

                    Ok(Printer {
                        handle: Box::new(handle),
                        endpoint_out: endpoint_out,
                        endpoint_in: endpoint_in,
                    })
                }
                None => Err(Error::DeviceOffline),
            },
            Err(err) => Err(Error::UsbError(err)),
        }
    }

    fn open_device(
        context: &mut Context,
        vid: u16,
        pid: u16,
        serial: String,
    ) -> Option<(Device<Context>, DeviceDescriptor, DeviceHandle<Context>)> {
        let devices = match context.devices() {
            Ok(d) => d,
            Err(_) => return None,
        };
        for device in devices.iter() {
            let device_desc = match device.device_descriptor() {
                Ok(d) => d,
                Err(_) => continue,
            };
            if device_desc.vendor_id() == vid && device_desc.product_id() == pid {
                match device.open() {
                    Ok(handle) => {
                        let timeout = Duration::from_secs(1);
                        let languages = match handle.read_languages(timeout) {
                            Ok(l) => l,
                            Err(_) => return None,
                        };
                        if languages.len() > 0 {
                            let language = languages[0];
                            match handle.read_serial_number_string(language, &device_desc, timeout)
                            {
                                Ok(s) => {
                                    if s == serial {
                                        return Some((device, device_desc, handle));
                                    } else {
                                        continue;
                                    }
                                }
                                Err(_) => continue,
                            }
                        } else {
                            continue;
                        }
                    }
                    Err(_) => continue,
                }
            }
        }
        None
    }

    fn find_endpoint(
        device: &mut Device<Context>,
        device_desc: &DeviceDescriptor,
        direction: Direction,
        transfer_type: TransferType,
    ) -> Option<Endpoint> {
        for n in 0..device_desc.num_configurations() {
            let config_desc = match device.config_descriptor(n) {
                Ok(c) => c,
                Err(_) => continue,
            };
            for interface in config_desc.interfaces() {
                for interface_desc in interface.descriptors() {
                    for endpoint_desc in interface_desc.endpoint_descriptors() {
                        if endpoint_desc.direction() == direction
                            && endpoint_desc.transfer_type() == transfer_type
                        {
                            return Some(Endpoint {
                                config: config_desc.number(),
                                iface: interface_desc.interface_number(),
                                setting: interface_desc.setting_number(),
                                address: endpoint_desc.address(),
                            });
                        }
                    }
                }
            }
        }
        None
    }

    fn write(&self, buf: Vec<u8>) -> Result<usize, Error> {
        let timeout = Duration::from_secs(1);
        let result = self
            .handle
            .write_bulk(self.endpoint_out.address, &buf, timeout);
        match result {
            Ok(n) => {
                if n == buf.len() {
                    Ok(n)
                } else {
                    println!("write error: {:?}", result);
                    Err(Error::InvalidResponse(n))
                }
            }
            Err(e) => Err(Error::UsbError(e)),
        }
    }

    pub fn read_status(&self) -> Result<Status, Error> {
        let timeout = Duration::from_secs(1);
        let mut buf: [u8; 32] = [0x00; 32];
        let mut counter: u8 = 10;
        while counter > 0 {
            match self
                .handle
                .read_bulk(self.endpoint_in.address, &mut buf, timeout)
            {
                // TODO: Check the first 4bytes match to [0x80, 0x20, 0x42, 0x34]
                Ok(32) => {
                    println!("raw staus code");
                    println!("{:x?}", buf);
                    return Ok(Status::from_buf(buf));
                }
                Ok(_) => {
                    println!("raw staus code: someting is wrong");
                    println!("{:0x?}", buf);
                    counter = counter - 1;
                    continue;
                }
                Err(e) => return Err(Error::UsbError(e)),
            }
        }
        Err(Error::ReadStatusTimeout)
    }

    // pub print_continuous(image: Vec<u8>) -> Result<(), Error> {
    //     let mut buffer: Vec<u8>;

    //     self.initialize();
    //     self.request_status();
    //     self.set_raster_mode();
    // }
    /// Initialize printer
    ///
    pub fn initialize(&self) -> Result<(), Error> {
        self.write([0x00; 400].to_vec())?;
        self.write([0x1b, 0x40].to_vec())?;
        Ok(())
    }

    pub fn print_label(&self, image: Vec<Vec<u8>>, config: Config) -> Result<usize, Error> {
        let mut buf: Vec<u8> = Vec::new();
        buf.append(&mut [0x00; 400].to_vec());
        buf.append(&mut [0x1B, 0x40].to_vec());
        buf.append(&mut [0x1B, 0x69, 0x61, 0x01].to_vec()); // Set raster command mode
        buf.append(&mut [0x1B, 0x69, 0x21, 0x00].to_vec()); // Set auto status notificatoin mode

        // ESC i z 印刷情報司令
        // buf.append(&mut [0x1B, 0x69, 0x7A, 0x8E, 0x0A, 0x3E, 0x64].to_vec());
        buf.append(&mut [0x1B, 0x69, 0x7A, 0x86, 0x0A, 0x3E, 0x00].to_vec());
        let len = (image.len() as u32).to_le_bytes();
        buf.append(&mut len.to_vec());
        buf.append(&mut [0x00, 0x00].to_vec());

        // apply config values
        config.build(&mut buf);

        buf.append(&mut [0x1B, 0x69, 0x64, 0x23, 0x00].to_vec()); // Set margin / feed amount to 3mm
        buf.append(&mut [0x4D, 0x00].to_vec());
        for mut row in image {
            //            buf.append(&mut [0x67, 0x00, row.len() as u8].to_vec());
            buf.append(&mut [0x67, 0x00, 90].to_vec()); // 無圧縮の場合はn=90とする
            buf.append(&mut row);
        }
        buf.append(&mut [0x1A].to_vec()); // Control-Z : Print then Eject
        self.write(buf)
    }
    /// Request printer status. call this function once before printing.
    ///
    pub fn request_status(&self) -> Result<usize, Error> {
        let mut buf: Vec<u8> = Vec::new();
        buf.append(&mut [0x00; 400].to_vec());
        buf.append(&mut [0x1B, 0x40].to_vec());
        buf.append(&mut [0x1b, 0x69, 0x53].to_vec());
        self.write(buf)
    }

    /// Specify margin amount (feed amount)
    ///
    /// ESC i d {n1} {n2}
    ///
    /// - Margin amount (dots) = n1 + n2 * 256
    ///
    /// This setting is ignored for die-cut labels.
    ///
    pub fn set_margin(&self, n1: u8, n2: u8) -> Result<usize, Error> {
        self.write([0x1b, 0x69, 0x64, n1, n2].to_vec())
    }

    /// Helper function to set the margin amount in dots.
    ///
    /// This setting is ignored for die-cut labels.
    ///
    pub fn set_margin_with_dots(&self, dots: u16) -> Result<(), Error> {
        let bytes = dots.to_be_bytes();
        self.write([0x1b, 0x69, 0x64, bytes[1], bytes[0]].to_vec())?;
        Ok(())
    }

    /// Switch dynamic command mode. Fixed to the raster mode in this module.
    /// ESC i a 1
    ///
    pub fn set_raster_mode(&self) -> Result<(), Error> {
        self.write([0x1B, 0x69, 0x61, 0x01].to_vec())?;
        Ok(())
    }

    /// Switch automatic status notificaton mode.Model
    /// ESC i ! {n1}
    ///
    /// notify == true => 0: Notify (default)
    /// notify == false => 1: Do not notify
    ///
    pub fn set_nottification_mode(&self, notify: bool) -> Result<(), Error> {
        if notify {
            self.write([0x1B, 0x69, 0x21, 0x00].to_vec())?;
        } else {
            self.write([0x1B, 0x69, 0x21, 0x01].to_vec())?;
        }
        Ok(())
    }

    ///
    ///
    /// TODO: support compression mode
    pub fn transfer_raster(&self, buf: Vec<u8>) -> Result<(), Error> {
        self.write([0x67, 0x00, 0x5A].to_vec())?;
        self.write(buf)?;
        Ok(())
    }

    ///
    ///
    /// Compression mode only ?
    pub fn transfer_raster_color(&self, buf: Vec<u8>) -> Result<(), Error> {
        self.write([0x77, 0x01, buf.len() as u8].to_vec())?;
        Ok(())
    }

    /// Zero raster graphics
    ///
    /// QL-800 does not support this command
    ///
    pub fn zero_raster(&self) -> Result<(), Error> {
        self.write([0x5A].to_vec())?;
        Ok(())
    }

    /// Print command
    ///
    /// Used as a print command at the end of pages other than the last page when multiple pages are printed.
    pub fn print(&self) -> Result<(), Error> {
        self.write([0xFF].to_vec())?;
        Ok(())
    }

    /// Print command with feeding
    ///
    /// Used as a print command at the end of the last page.
    pub fn print_last(&self) -> Result<(), Error> {
        self.write([0x1A].to_vec())?;
        Ok(())
    }

    pub fn print_information(&self) -> Result<(), Error> {
        Ok(())
    }

    fn set_autocut(mut buf: Vec<u8>, enabled: bool) {
        let byte: u8 = if enabled { 0b0100_0000 } else { 0b0000_0000 };

        buf.append(&mut [0x1B, 0x69, 0x4B, byte].to_vec())
    }
}

///
/// Status received from the printer encoded to Rust friendly type.
///
#[derive(Debug)]
pub struct Status {
    model: Model,
    error: PrinterError,
    media: Option<Media>,
    mode: u8,
    status_type: StatusType,
    phase: Phase,
    notification: Notification,
}

impl Status {
    fn from_buf(buf: [u8; 32]) -> Self {
        Status {
            model: Model::from_code(buf[4]),
            error: PrinterError::from_buf(buf),
            media: Media::from_buf(buf),
            mode: buf[15],
            status_type: StatusType::from_code(buf[18]),
            phase: Phase::from_buf(buf),
            notification: Notification::from_code(buf[22]),
        }
    }
}

// StatusType

#[derive(Debug)]
enum StatusType {
    ReplyToRequest,
    Completed,
    Error,
    Offline,
    Notification,
    PhaseChange,
    Unknown,
}

impl StatusType {
    fn from_code(code: u8) -> StatusType {
        match code {
            0x00 => Self::ReplyToRequest,
            0x01 => Self::Completed,
            0x02 => Self::Error,
            0x04 => Self::Offline,
            0x05 => Self::Notification,
            0x06 => Self::PhaseChange,
            _ => Self::Unknown,
        }
    }
}
// Phase

#[derive(Debug)]
enum Phase {
    Receiving,
    Printing,
    Waiting(u16),
    // Printing(u16),
}

impl Phase {
    fn from_buf(buf: [u8; 32]) -> Self {
        match buf[19] {
            0x00 => Self::Receiving,
            0x01 => Self::Printing,
            _ => Self::Waiting(0),
        }
    }
}

// Notification

#[derive(Debug)]
enum Notification {
    NotAvailable,
    CoolingStarted,
    CoolingFinished,
}

impl Notification {
    fn from_code(code: u8) -> Self {
        match code {
            0x03 => Self::CoolingStarted,
            0x04 => Self::CoolingFinished,
            _ => Self::NotAvailable,
        }
    }
}

//

bitflags! {
    struct ExtendedMode: u8 {
        const ColorMode = 0b00000001;
        const CutAtEnd = 0b00001000;
        const Resolution = 0b01000000;
    }
}

enum AutoCut {
    Enabled(u8),
    Disabled,
}

pub struct Config {
    auto_cut: AutoCut,
    two_colors: bool,
    cut_at_end: bool,
    high_resolution: bool,
}

impl Config {
    pub fn new() -> Config {
        Config {
            auto_cut: AutoCut::Enabled(1),
            two_colors: false,
            cut_at_end: true,
            high_resolution: false,
        }
    }

    pub fn enable_auto_cut(self, size: u8) -> Self {
        Config {
            auto_cut: AutoCut::Enabled(size),
            ..self
        }
    }

    pub fn disable_auto_cut(self) -> Self {
        Config {
            auto_cut: AutoCut::Disabled,
            ..self
        }
    }

    pub fn change_resolution(self, high: bool) -> Self {
        Config {
            high_resolution: high,
            ..self
        }
    }

    fn build(self, buf: &mut std::vec::Vec<u8>) {
        // set auto cut
        let mut various_mode: u8 = 0b00000000;
        let mut auto_cut_num: u8 = 1;

        if let AutoCut::Enabled(n) = self.auto_cut {
            auto_cut_num = n;
            various_mode = various_mode | 0b01000000;
        }
        buf.append(&mut [0x1B, 0x69, 0x41, auto_cut_num].to_vec()); // ESC i A : Set auto cut number
        buf.append(&mut [0x1B, 0x69, 0x4D, various_mode].to_vec()); // ESC i M : Set various mode

        // set expanded mode
        let mut expanded_mode: u8 = 0b00000000;

        if self.two_colors == true {
            expanded_mode = expanded_mode | 0b00000001;
        }

        if self.cut_at_end == true {
            expanded_mode = expanded_mode | 0b00001000;
        };

        if self.high_resolution == true {
            expanded_mode = expanded_mode | 0b01000000;
        }

        buf.append(&mut [0x1B, 0x69, 0x4B, expanded_mode].to_vec()); // ESC i K : Set expanded mode
    }
}
