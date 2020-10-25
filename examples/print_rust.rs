use image::GenericImage;
use image::{GenericImageView, ImageFormat};
use ptouch::{Config, Media, Model, Printer};
use std::path::Path;

fn main() {
    // let file = "examples/rust-logo-256x256-blk.png";
    // let file = "examples/test-text.png";
    // let file = "examples/print-sample.png";
    let file = "examples/sample2.png";

    let im: image::DynamicImage = image::open(file).unwrap();
    let (_, height) = im.dimensions();


    let width = 720;
    let length = height; // 480;

    let gray = im.grayscale();
    let mut buffer = image::DynamicImage::new_luma8(width, length);
    buffer.invert();
    buffer.copy_from(&gray, 0, 0).unwrap();
    buffer.invert();
    let bytes = buffer.to_bytes();

    println!("{:?}", buffer.dimensions());

    // buffer.save("examples/out.png").unwrap();

    // println!("{:?}", gray.dimensions());
    // gray.invert();
    // let bytes = gray.to_bytes();

    let mut bw: Vec<Vec<u8>> = Vec::new();

    for y in 0..length {
        let mut buf: Vec<u8> = Vec::new();
        for x in 0..(width / 8) {
            // 0 1 ... width / 8 (max 90)
            // let index = (width - 8 - x * 8 + y * width) as usize;
            let index = (1 + y) * width - (1 + x) * 8;
            let mut tmp: u8 = 0x00;
            for i in 0..8 {
                let pixel = bytes[(index + i) as usize];
                let value: u8 = if pixel > 100 { 1 } else { 0 };
                tmp = tmp | (value << i);
            }
            buf.push(tmp);
        }
        /*
        let x = width / 8;
        let res = width % 8;
        if res > 0 {
            let index = (width - 8 - x * 8 + y * width) as usize;
            let mut tmp: u8 = 0x00;
            for i in 0..res {
                tmp = tmp | (bytes[index + i as usize] as u8 & 0xF0u8) >> (7 - i);
            }
            buf.push(tmp);
        }
        */
        bw.push(buf);
    }

    if true {
        match Printer::new(Model::QL800, "000G0Z714634".to_string()) {
            Ok(printer) => {
                printer.request_status().unwrap();
                let result = printer.read_status();
                println!("status before: {:?}", result);

                let config: Config = Config::new().change_resolution(true);

                match printer.print_label(bw, config) {
                    Ok(_) => println!("success"),
                    Err(err) => println!("ERROR {:?}", err),
                }
                let result = printer.read_status();
                println!("status after: {:?}", result);
            }
            Err(err) => panic!("read error {}", err),
        }
    }
}

// fn to_bw() -> Vec<u8> {}
