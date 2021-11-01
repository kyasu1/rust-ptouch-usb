use image::{GenericImage, GenericImageView, ImageBuffer, Luma};
use ptouch::{
    grayscale_to_matrix, step_filter_normal, DieCut, Endless, Matrix, Media, Model, Printer,
    PrinterProfile,
};
use qrcode::QrCode;

fn main() {
    env_logger::init();

    enum PrintOption {
        TestLabelNormalRes,
        TestLabelHighRes,
        TestLabelHighResMultiple,
        TestLabelHighResMultipleQrCode,
        TestGrayScale,
    }

    let option = PrintOption::TestGrayScale;

    let media = Media::Endless(Endless::Endless62);

    let profile =
        PrinterProfile::build_usb_profile(Model::QL800, "000G0Z714634".to_string()).unwrap();

    match option {
        PrintOption::TestLabelNormalRes => {
            let file = "examples/assets/label-720-300.png";
            let label: image::DynamicImage = image::open(file).unwrap();
            let (_, length) = label.dimensions();
            let bytes = label.to_bytes();
            let bw = step_filter_normal(80, length, bytes);

            let printer = Printer::new(profile, media)
                .high_resolution(false)
                .cut_at_end(true)
                .two_colors(false)
                .enable_auto_cut(1);

            printer.print(vec![bw].into_iter()).unwrap();
        }
        PrintOption::TestLabelHighRes => {
            let file = "examples/assets/label-720-600.png";
            let label: image::DynamicImage = image::open(file).unwrap().grayscale();
            let (_, length) = label.dimensions();
            let bytes = label.to_bytes();
            let bw = step_filter_normal(80, length, bytes);

            let printer = Printer::new(profile, media)
                .high_resolution(true)
                .cut_at_end(true)
                .two_colors(false)
                .enable_auto_cut(1);

            printer.print(vec![bw].into_iter()).unwrap();
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

            let matrix = grayscale_to_matrix(label);

            let printer = Printer::new(profile, media)
                .high_resolution(false)
                .cut_at_end(true)
                .two_colors(false)
                .enable_auto_cut(1);

            printer.print(vec![matrix].into_iter()).unwrap();
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
            let bytes = buffer.to_bytes();
            let bw = step_filter_normal(80, length, bytes);
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

            let bytes = buffer.to_luma().into_raw();
            let bw = step_filter_normal(80, length, bytes);
            self.counter = self.counter - 1;
            Some(bw)
        } else {
            None
        }
    }
}
