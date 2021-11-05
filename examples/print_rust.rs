use std::ffi::OsString;

use clap::{App, Arg};
use image::{GenericImage, GenericImageView, ImageBuffer, Luma};
use ptouch::{DieCut, Endless, Matrix, Media, Model, Printer, PrinterProfile};
use qrcode::QrCode;

struct Args {
    model: String,
}

impl Args {
    fn new() -> Self {
        Self::new_from(std::env::args_os().into_iter()).unwrap_or_else(|e| e.exit())
    }
    fn new_from<I, T>(args: I) -> Result<Self, clap::Error>
    where
        I: Iterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let app = App::new("ptouch-rs")
            .version("1.0")
            .version("2.0")
            .about("P-Touch Printing Utility")
            .author("Yasuyuki Komatsubara");

        let model_option = Arg::new("model")
            .short('m')
            .long("model")
            .takes_value(true)
            .about("Please specify printer model")
            .required(true);

        let print_command = App::new("print")
            .about("Print a label of the provided IMAGE.")
            .arg(
                Arg::new("label")
                    .short('l')
                    .long("label")
                    .takes_value(true)
                    .required(true),
            );

        let app = app.arg(model_option).subcommand(print_command);

        let matches = app.try_get_matches_from(args)?;

        let model = matches
            .value_of("match")
            .expect("Please specify a printer model");

        Ok(Args {
            model: model.to_string(),
        })
    }
}
fn main() {
    env_logger::init();

    let args = Args::new();

    return;
    enum PrintOption {
        TestLabelNormalRes,
        TestLabelHighRes,
        TestLabelHighResMultiple,
        TestLabelHighResMultipleQrCode,
        TestGrayScale,
    }

    let option = PrintOption::TestGrayScale;

    let media = Media::Endless(Endless::Endless62);
    // let media = Media::DieCut(DieCut::DieCut12Dia);

    let profile =
        PrinterProfile::build_usb_profile(Model::QL800, "000G0Z714634".to_string()).unwrap();

    match option {
        PrintOption::TestLabelNormalRes => {
            let file = "examples/assets/label-720-300.png";
            let label: image::DynamicImage = image::open(file).unwrap();

            let matrix = ptouch::convert(label, Model::QL800.pins());

            let printer = Printer::new(profile, media)
                .high_resolution(false)
                .cut_at_end(true)
                .two_colors(false)
                .enable_auto_cut(1);

            let result = printer.print(vec![matrix].into_iter());
            println!("{:?}", result);
        }
        PrintOption::TestLabelHighRes => {
            let file = "examples/assets/label-720-600.png";
            let label: image::DynamicImage = image::open(file).unwrap();

            let matrix = ptouch::convert(label, Model::QL800.pins());

            let printer = Printer::new(profile, media)
                .high_resolution(true)
                .cut_at_end(true)
                .two_colors(false)
                .enable_auto_cut(1);

            let result = printer.print(vec![matrix].into_iter());
            println!("{:?}", result);
        }
        PrintOption::TestLabelHighResMultiple => {
            Printer::new(profile, media)
                .high_resolution(true)
                .disable_auto_cut()
                .print(Label { counter: 2 })
                .unwrap();
        }
        PrintOption::TestLabelHighResMultipleQrCode => Printer::new(profile, media)
            .high_resolution(true)
            .print(Label2 { counter: 2 })
            .unwrap(),
        PrintOption::TestGrayScale => {
            let file = "examples/assets/yagi.jpg";
            let label: image::DynamicImage = image::open(file).unwrap();

            let matrix = ptouch::convert_fit(label, false, Model::QL800.pins(), media);

            let printer = Printer::new(profile, media)
                .high_resolution(false)
                .cut_at_end(true)
                .two_colors(false)
                .enable_auto_cut(1);

            let result = printer.print(vec![matrix].into_iter());
            println!("{:?}", result);
        }
    };
}

struct Label {
    counter: u16,
}

impl Iterator for Label {
    type Item = Matrix;

    fn next(&mut self) -> Option<Self::Item> {
        if self.counter > 0 {
            let file = "examples/label-mini.png";
            let image: image::DynamicImage = image::open(file).unwrap();
            let (_, length) = image.dimensions();
            let image = image.grayscale();

            let mut buffer = image::DynamicImage::new_luma8(ptouch::NORMAL_PRINTER_WIDTH, length);
            buffer.invert();
            buffer.copy_from(&image, 0, 0).unwrap();
            buffer.invert();

            let bw = ptouch::convert(buffer, Model::QL800.pins());
            self.counter = self.counter - 1;
            Some(bw)
        } else {
            None
        }
    }
}

struct Label2 {
    counter: u16,
}

impl Iterator for Label2 {
    type Item = Matrix;

    fn next(&mut self) -> Option<Self::Item> {
        if self.counter > 0 {
            let length = 220;
            let qrcode = QrCode::new(format!("12345-{}", self.counter + 1)).unwrap();
            let qrcode: image::ImageBuffer<image::Rgba<u8>, std::vec::Vec<u8>> = qrcode
                .render::<image::Rgba<u8>>()
                .quiet_zone(false)
                .min_dimensions(100, 200)
                .build();

            let mut buffer = image::DynamicImage::new_luma8(ptouch::NORMAL_PRINTER_WIDTH, length);
            buffer.invert();
            buffer.copy_from(&qrcode, 0, 0).unwrap();

            let matrix = ptouch::convert(buffer, Model::QL800.pins());

            self.counter = self.counter - 1;
            Some(matrix)
        } else {
            None
        }
    }
}
