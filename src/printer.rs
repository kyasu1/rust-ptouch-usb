use log::{debug, info, warn};

pub use self::printer_profile::{PrinterProfile, Status};

mod printer_profile;

use crate::{error::Error, media::Media, Matrix};

#[derive(Debug, Clone, Copy)]
struct Endpoint {
    config: u8,
    iface: u8,
    setting: u8,
    address: u8,
}

pub struct Printer {
    profile: PrinterProfile,
    media: Media,
    config: Config,
}

impl Printer {
    pub fn new(profile: PrinterProfile, media: Media) -> Self {
        Self {
            profile,
            media,
            config: Config::default(),
        }
    }

    /// Read printer status.
    ///
    /// This method is convinent for inspection when a new media is added.
    ///
    pub fn check_status(&self) -> Result<Status, Error> {
        self.request_status()?;
        self.profile.read_status()
    }

    /// Cancel printing
    pub fn cancel(&self) -> Result<(), Error> {
        let buf = self.initialize();
        self.profile.write(buf)?;
        Ok(())
    }

    /// Send multipe rastred images to a printer
    pub fn print(&self, images: impl Iterator<Item = Matrix>) -> Result<(), Error> {
        self.request_status()?;
        match self.profile.read_status() {
            Ok(status) => {
                status.check_media(self.media)?;
                self.print_label(images)?;
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    pub fn enable_auto_cut(mut self, size: u8) -> Self {
        self.config.auto_cut = AutoCut::Enabled(size);
        self
    }

    /// Diable auto cutting
    pub fn disable_auto_cut(mut self) -> Self {
        self.config.auto_cut = AutoCut::Disabled;
        self
    }

    pub fn cut_at_end(mut self, flag: bool) -> Self {
        self.config.cut_at_end = flag;
        self
    }

    pub fn high_resolution(mut self, high_resolution: bool) -> Self {
        self.config.high_resolution = high_resolution;
        self
    }

    pub fn set_feed_in_dots(mut self, feed: u16) -> Self {
        self.config.feed = Some(feed);
        self
    }

    pub fn two_colors(mut self, two_colors: bool) -> Self {
        self.config.two_colors = two_colors;
        self
    }

    // Always need to send these Hex values when issuing a new command.
    fn initialize(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();
        buf.append(&mut [0x00; 400].to_vec());
        buf.append(&mut [0x1B, 0x40].to_vec());
        buf
    }

    fn print_label(&self, images: impl Iterator<Item = Matrix>) -> Result<(), Error> {
        let mut preamble: Vec<u8> = self.initialize();
        preamble.append(&mut [0x1B, 0x69, 0x61, 0x01].to_vec()); // Set raster command mode
        preamble.append(&mut [0x1B, 0x69, 0x21, 0x00].to_vec()); // Set auto status notificatoin mode
        preamble.append(&mut [0x4D, 0x00].to_vec()); // Set to no compression mode

        // Apply config values
        match self.config.clone().build(self.media) {
            Ok(mut buf) => preamble.append(&mut buf),
            Err(err) => return Err(err),
        }

        let mut start_flag: bool = true;
        let mut color = false;

        let mut iter = images.into_iter().peekable();

        loop {
            let mut buf: Vec<u8> = Vec::new();

            match iter.next() {
                Some(image) => {
                    if start_flag {
                        buf.append(&mut preamble);
                    }

                    // ESC i z 印刷情報司令
                    buf.append(&mut [0x1B, 0x69, 0x7A].to_vec());

                    // Set media lenght and width
                    self.media.set_media(&mut buf, true);

                    // Set number of raster lines
                    let raster_lines: [u8; 4] = if self.config.two_colors {
                        ((image.len() / 2) as u32).to_le_bytes()
                    } else {
                        ((image.len()) as u32).to_le_bytes()
                    };

                    buf.append(&mut raster_lines.to_vec());
                    if start_flag {
                        buf.append(&mut [0x00, 0x00].to_vec());
                        start_flag = false;
                    } else {
                        buf.append(&mut [0x01, 0x00].to_vec());
                    }

                    // Send data by splitting to raster lines
                    if self.config.two_colors {
                        for mut row in image {
                            if color {
                                buf.append(&mut [0x77, 0x01, 90].to_vec());
                                buf.append(&mut row);
                                color = !color;
                            } else {
                                buf.append(&mut [0x77, 0x02, 90].to_vec());
                                buf.append(&mut row);
                                color = !color;
                            }
                        }
                    } else {
                        for mut row in image {
                            buf.append(&mut [0x67, 0x00, 90].to_vec());
                            buf.append(&mut row);
                        }
                    }

                    // Check if there is a next page, if not send eject command
                    if iter.peek().is_some() {
                        buf.push(0x0C); // FF : Print
                        self.profile.write(buf)?;
                        self.profile.read_status()?;
                    } else {
                        buf.push(0x1A); // Control-Z : Print then Eject
                        self.profile.write(buf)?;
                    }
                }
                None => {
                    break;
                }
            }
        }
        Ok(())
    }

    fn request_status(&self) -> Result<usize, Error> {
        let mut buf: Vec<u8> = self.initialize();
        buf.append(&mut [0x1b, 0x69, 0x53].to_vec());
        self.profile.write(buf)
    }
}

/// AutoCut settings
#[derive(Debug, Clone, Copy)]
enum AutoCut {
    Enabled(u8),
    Disabled,
}

/// PTouch configuration settings
#[derive(Debug, Clone)]
struct Config {
    auto_cut: AutoCut,
    two_colors: bool,
    cut_at_end: bool,
    high_resolution: bool,
    feed: Option<u16>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auto_cut: AutoCut::Enabled(1),
            two_colors: false,
            cut_at_end: true,
            high_resolution: false,
            feed: None,
        }
    }
}
impl Config {
    // Generate config data in binary format.
    fn build(self, media: Media) -> Result<Vec<u8>, Error> {
        let mut buf: Vec<u8> = Vec::new();

        // Set feeding values in dots
        {
            let feed = match self.feed {
                Some(feed) => feed,
                None => media.get_default_feed_dots(),
            };

            match media.check_feed_value(feed) {
                Ok(feed) => {
                    buf.append(&mut [0x1B, 0x69, 0x64].to_vec());
                    buf.append(&mut feed.to_vec());
                }
                Err(msg) => return Err(Error::InvalidConfig(msg)),
            }
        }
        // Set auto cut settings
        {
            let mut various_mode: u8 = 0b0000_0000;
            let mut auto_cut_num: u8 = 1;

            if let AutoCut::Enabled(n) = self.auto_cut {
                various_mode |= 0b0100_0000;
                auto_cut_num = n;
            }

            debug!("Various mode: {:X}", various_mode);
            debug!("Auto cut num: {:X}", auto_cut_num);

            buf.append(&mut [0x1B, 0x69, 0x4D, various_mode].to_vec()); // ESC i M : Set various mode
            buf.append(&mut [0x1B, 0x69, 0x41, auto_cut_num].to_vec()); // ESC i A : Set auto cut number
        }
        // Set expanded mode
        {
            let mut expanded_mode: u8 = 0b00000000;

            if self.two_colors {
                expanded_mode |= 0b0000_0001;
            }

            if self.cut_at_end {
                expanded_mode |= 0b0000_1000;
            };

            if self.high_resolution {
                expanded_mode |= 0b0100_0000;
            }

            debug!("Expanded mode: {:X}", expanded_mode);

            buf.append(&mut [0x1B, 0x69, 0x4B, expanded_mode].to_vec()); // ESC i K : Set expanded mode
        }
        Ok(buf)
    }
}

fn pack_bits(src: Vec<u8>) -> Vec<u8> {
    enum Mode {
        Literal,
        Fill,
    }

    let src_length = src.len();
    println!("src_length: {} ", src_length);

    let mut dst: Vec<u8> = Vec::new();

    let mut mode = Mode::Literal;
    let mut next = 1;
    let mut count = 0;

    let mut c = src[0];
    while next <= src_length {
        match mode {
            Mode::Literal => {
                let mut buf: Vec<u8> = Vec::new();
                buf.push(c);

                while next < src_length {
                    if src[next] == c {
                        buf.pop();
                        break;
                    }
                    c = src[next];
                    next += 1;
                    buf.push(c);
                }

                let len = buf.len();

                if len > 0 || next == src_length {
                    dst.push(len as u8 - 1);
                    dst.extend_from_slice(&buf[0..len]);
                }

                if len == src_length || next == src_length {
                    break;
                } else {
                    mode = Mode::Fill;
                    c = src[next];
                    next += 1;
                    count = 2;
                }
            }
            Mode::Fill => {
                while next < src_length {
                    if c != src[next] {
                        dst.push(complement(count - 1));
                        dst.push(c);
                        break;
                    } else {
                        count += 1;
                        next += 1;
                    }
                }
                if next == src_length {
                    dst.push(complement(count - 1));
                    dst.push(c);
                    break;
                } else {
                    mode = Mode::Literal;
                    c = src[next];
                    next += 1;
                }
            }
        }
    }

    dst
}

fn complement(value: u8) -> u8 {
    (value as i8 * (-1i8)) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Endless, Media};

    #[test]
    fn build_config_with_default() {
        let config = Config::default();
        let media = Media::Endless(Endless::Endless62);

        if let Ok(buf) = config.build(media) {
            assert_eq!(
                buf,
                [27, 105, 100, 35, 0, 27, 105, 77, 64, 27, 105, 65, 1, 27, 105, 75, 8]
            );
        };
    }

    #[test]
    fn test_complement() {
        assert_eq!(super::complement(89), (89i8 * (-1i8)) as u8);
    }

    #[test]
    fn pack_bits_all_zero() {
        let src: Vec<u8> = vec![0x00u8; 90];
        let dist = super::pack_bits(src);
        assert_eq!(dist, vec![super::complement(89), 0]);
    }

    #[test]
    fn pack_bits_end_filled() {
        let src = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 22, 22, 22, 23, 54, 99, 251, 32, 0, 0, 0, 0,
        ];
        let dist = super::pack_bits(src);
        assert_eq!(
            dist,
            vec![
                super::complement(9),
                0,
                super::complement(2),
                22,
                4,
                23,
                54,
                99,
                251,
                32,
                super::complement(3),
                0,
            ]
        );
    }

    #[test]
    fn pack_bits_end_literal() {
        let src = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 22, 22, 22, 23, 54, 99, 251, 32,
        ];
        let dist = super::pack_bits(src);
        assert_eq!(
            dist,
            vec![
                super::complement(9),
                0,
                super::complement(2),
                22,
                4,
                23,
                54,
                99,
                251,
                32,
            ]
        );
    }

    #[test]
    fn pack_bits_start_literal() {
        let src = vec![
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 22, 22, 22, 23, 54, 99, 251, 32, 0, 0, 0, 0,
        ];
        let dist = super::pack_bits(src);
        assert_eq!(
            dist,
            vec![
                0,
                1,
                super::complement(9),
                0,
                super::complement(2),
                22,
                4,
                23,
                54,
                99,
                251,
                32,
                super::complement(3),
                0
            ]
        );
    }

    #[test]
    fn pack_bits_all_different() {
        let src = (0..90).collect();
        let dist = super::pack_bits(src);
        assert_eq!(dist.len(), 91);
        assert_eq!(dist[0], 89);
        assert_eq!(dist[1..91], (0..90).collect::<Vec<u8>>());
    }
}
