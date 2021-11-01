//! P-Touch Printer Driver
//!
//! This crate provides a printer driver for Brother P-Touch QL series label printers.
//!
//! ```rust
//! use ptouch;
//!
//! let media
//! ```

mod error;
mod media;
mod model;
mod printer;
mod utils;

pub use crate::{
    error::{Error, PrinterError},
    media::{DieCut, Endless, Media},
    model::Model,
    printer::Printer,
    printer::PrinterProfile,
    utils::{convert, convert_fit},
};

pub type Matrix = Vec<Vec<u8>>;
pub const NORMAL_PRINTER_WIDTH: u32 = 720;
pub const WIDE_PRINTER_WIDTH: u32 = 1296;

pub enum PRINTER_WIDTH {
    NORMAL,
    WIDE,
}

impl PRINTER_WIDTH {
    fn to_int(self) -> u32 {
        match self {
            PRINTER_WIDTH::NORMAL => 720,
            PRINTER_WIDTH::WIDE => 1296,
        }
    }
}
