use std::fmt;

#[derive(Debug, Clone, Copy)]
pub enum Model {
    QL500,
    QL550,
    QL560,
    QL570,
    QL580N,
    QL600,
    QL650TD,
    QL700,
    QL710W,
    QL720NW, // TESTED
    QL800,   // TESTED
    QL810W,
    QL820NWB, //TESTED
    QL1050,
    QL1060N,
    QL1100,
    QL1110NWB,
    QL1115NWB,
}

impl Model {
    pub fn from_str(model: &str) -> Option<Self> {
        match model {
            "QL-600" => Some(Self::QL600),
            "QL-720NW" => Some(Self::QL720NW),
            "QL-800" => Some(Self::QL800),
            "QL-810W" => Some(Self::QL810W),
            "QL-820NWB" => Some(Self::QL820NWB),
            "QL-1100" => Some(Self::QL1100),
            "QL-1110NWB" => Some(Self::QL1110NWB),
            "QL-1115NWB" => Some(Self::QL1115NWB),
            _ => None,
        }
    }
    pub fn from_code(code: u8) -> Self {
        match code {
            0x47 => (Self::QL600),
            0x37 => (Self::QL720NW),
            0x38 => (Self::QL800),
            0x39 => (Self::QL810W),
            0x41 => (Self::QL820NWB),
            0x43 => (Self::QL1100),
            0x44 => (Self::QL1110NWB),
            0x45 => (Self::QL1115NWB),
            _ => panic!("Unknown model code {}", code),
        }
    }

    pub fn pid(&self) -> u16 {
        match self {
            Self::QL600 => 0x20C0,
            Self::QL720NW => 0x2044,
            Self::QL800 => 0x209b,
            Self::QL810W => 0x209c,
            Self::QL820NWB => 0x209d,
            Self::QL1100 => 0x20A7,
            Self::QL1110NWB => 0x20A8,
            Self::QL1115NWB => 0x20AB,
            _ => 0x0000,
        }
    }

    pub fn pins(&self) -> u32 {
        match self {
            Self::QL1050 => crate::WIDE_PRINTER_WIDTH,
            Self::QL1060N => crate::WIDE_PRINTER_WIDTH,
            Self::QL1100 => crate::WIDE_PRINTER_WIDTH,
            Self::QL1110NWB => crate::WIDE_PRINTER_WIDTH,
            Self::QL1115NWB => crate::WIDE_PRINTER_WIDTH,
            _ => crate::NORMAL_PRINTER_WIDTH,
        }
    }

    // pub fn supported_medias(&self) -> Vec<Media> {
    //     match self {
    //         Self::QL800 => vec![Media::Continuous29],
    //         Self::QL810W => vec![Media::Continuous29],
    //         Self::QL820NWB => vec![Media::Continuous29],
    //     }
    // }
}


impl fmt::Display for Model {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let model = match *self {
            Self::QL500 =>
                "QL-500",

            Self::QL550 =>
                "QL-550",
            Self::QL560 =>
                "QL-560",
            Self::QL570 =>
                "QL-570",
            Self::QL580N =>
                "QL-580N",
            Self::QL600 =>
                "QL-600",
            Self::QL650TD =>
                "QL-650TD",
            Self::QL700 =>
                "QL-700",
            Self::QL710W =>
                "QL-710W",
            Self::QL720NW =>
                "QL-720NW",
            Self::QL800 =>
                "QL-800",
            Self::QL810W =>
                "QL-810W",
            Self::QL820NWB =>
                "QL-820NWB",
            Self::QL1050 =>
                "QL-1050",
            Self::QL1060N =>
                "QL-1060N",
            Self::QL1100 =>
                "QL-1100",
            Self::QL1110NWB =>
                "QL-1110NWB",
            Self::QL1115NWB =>
                "QL-1115NWB",

            _ =>
                "UNKNOWN"
        };

        write!(f, "{}", model)
    }
}