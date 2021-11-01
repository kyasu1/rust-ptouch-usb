use crate::{media::Media, model::Model, Error, PrinterError};
use log::{debug, info, warn};
use rusb::{Context, Device, DeviceDescriptor, DeviceHandle, Direction, TransferType, UsbContext};
use std::time::Duration;
pub struct PrinterProfile {
    connection: Connection,
}

enum Connection {
    Usb {
        endpoint_out: Endpoint,
        endpoint_in: Endpoint,
        handle: Box<DeviceHandle<Context>>,
    },
    Network {
        host: String,
        port: u16,
    },
    Serial {},
}

#[derive(Debug, Clone, Copy)]
struct Endpoint {
    config: u8,
    iface: u8,
    setting: u8,
    address: u8,
}

impl PrinterProfile {
    pub fn build_usb_profile(model: Model, serial: String) -> Result<Self, Error> {
        match Context::new() {
            Ok(mut context) => {
                match Self::open_device(&mut context, model, serial) {
                    Ok((mut device, device_desc, mut handle)) => {
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

                        Ok(PrinterProfile {
                            connection: Connection::Usb {
                                handle: Box::new(handle),
                                endpoint_out,
                                endpoint_in,
                            },
                        })
                    }
                    Err(err) => {
                        debug!("{:?}", err);
                        Err(Error::DeviceOffline)
                    }
                }
            }
            Err(err) => Err(Error::UsbError(err)),
        }
    }

    pub fn build_network_profile(host: String, port: u16) -> Result<Self, Error> {
        Ok(Self {
            connection: Connection::Network { host, port },
        })
    }

    pub fn build_serial_profile() -> Result<Self, Error> {
        unimplemented!()
    }

    fn open_device(
        context: &mut Context,
        model: Model,
        serial: String,
    ) -> Result<(Device<Context>, DeviceDescriptor, DeviceHandle<Context>), Error> {
        const VID: u16 = 0x04F9;

        let devices = context.devices()?;

        for device in devices.iter() {
            let device_desc = match device.device_descriptor() {
                Ok(d) => d,
                Err(err) => {
                    debug!("{:?}", err);
                    continue;
                }
            };
            debug!("{:?}", device_desc);

            if device_desc.vendor_id() == VID && device_desc.product_id() == model.pid() {
                match device.open() {
                    Ok(handle) => {
                        let timeout = Duration::from_secs(1);
                        let languages = handle.read_languages(timeout)?;

                        if !languages.is_empty() {
                            let language = languages[0];
                            match handle.read_serial_number_string(language, &device_desc, timeout)
                            {
                                Ok(s) => {
                                    if s == serial {
                                        return Ok((device, device_desc, handle));
                                    } else {
                                        continue;
                                    }
                                }
                                Err(err) => {
                                    debug!("Failed to read serial number string: {:?}", err);
                                    continue;
                                }
                            }
                        } else {
                            continue;
                        }
                    }
                    Err(err) => {
                        debug!("Failed to open device: {:?}", err);
                        continue;
                    }
                }
            }
        }
        debug!("No device match with this serial: {:?}", serial);
        Err(Error::DeviceOffline)
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

    pub(crate) fn write(&self, buf: Vec<u8>) -> Result<usize, Error> {
        match &self.connection {
            Connection::Usb {
                endpoint_in: _,
                endpoint_out,
                handle,
            } => {
                let timeout = Duration::from_secs(10);
                let result = handle.write_bulk(endpoint_out.address, &buf, timeout);
                match result {
                    Ok(n) => {
                        if n == buf.len() {
                            Ok(n)
                        } else {
                            warn!(
                                "write error: bytes wrote {} != bytes supplied {}, possibly timeout ?",
                                n,
                                buf.len()
                            );
                            Err(Error::InvalidResponse(n))
                        }
                    }
                    Err(e) => Err(Error::UsbError(e)),
                }
            }
            Connection::Network { host: _, port: _ } => unimplemented!(),
            Connection::Serial {} => unimplemented!(),
        }
    }

    pub(crate) fn read_status(&self) -> Result<Status, Error> {
        match &self.connection {
            Connection::Usb {
                endpoint_in,
                endpoint_out: _,
                handle,
            } => {
                let timeout = Duration::from_secs(1);
                let mut buf: [u8; 32] = [0x00; 32];
                let mut counter = 0;

                while counter < 10 {
                    match handle.read_bulk(endpoint_in.address, &mut buf, timeout) {
                        // TODO: Check the first 4bytes match to [0x80, 0x20, 0x42, 0x34]
                        // TODO: Check the error status
                        Ok(32) => {
                            let status = Status::from_buf(buf);
                            debug!("Raw status code: {:X?}", buf);
                            debug!("Parsed Status struct: {:?}", status);
                            if status.phase == Phase::Receiving {
                                return Ok(status);
                            } else {
                                std::thread::sleep(std::time::Duration::from_secs(1));
                            }
                        }
                        Ok(_) => {
                            std::thread::sleep(std::time::Duration::from_secs(1));
                        }
                        Err(e) => return Err(Error::UsbError(e)),
                    };
                    counter += 1;
                }
                Err(Error::ReadStatusTimeout)
            }

            Connection::Network { host: _, port: _ } => unimplemented!(),
            Connection::Serial {} => unimplemented!(),
        }
    }
}

/// Status read from a printer
#[derive(Debug)]
pub struct Status {
    model: Model,
    error: PrinterError,
    media: Option<Media>,
    mode: u8,
    status_type: StatusType,
    phase: Phase,
    notification: Notification,
    id: u8,
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
            id: buf[14],
        }
    }

    pub fn check_media(self, media: Media) -> Result<(), Error> {
        if let Some(m) = self.media {
            if m == media {
                Ok(())
            } else {
                Err(Error::InvalidMedia(media))
            }
        } else {
            Err(Error::InvalidMedia(media))
        }
    }
}

// StatusType
#[derive(Debug, PartialEq)]
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

/// This represents the internal printing phase.
#[derive(Debug, PartialEq)]
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

/// Notification variants
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
