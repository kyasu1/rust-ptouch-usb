/// choose a step filter depending on the priter width.
///
use crate::Matrix;
use image::GenericImageView;
use std::os::unix::thread;

#[cfg(feature = "image")]
use image::{imageops::FilterType, ImageBuffer};

#[cfg(feature = "image")]
pub fn grayscale_to_matrix(mut image: image::DynamicImage) -> Matrix {
    use std::io::Read;

    use image::DynamicImage;

    // crop
    let mut cropped = image.crop(0, 0, crate::NORMAL_PRINTER_WIDTH, image.dimensions().1);

    // shrink
    let mut resized = image.resize(
        crate::NORMAL_PRINTER_WIDTH,
        cropped.dimensions().1 * 2,
        FilterType::Lanczos3,
    );

    let color_map = image::imageops::colorops::BiLevel;
    let mut buffer = cropped.into_luma8();
    image::imageops::colorops::dither(&mut buffer, &color_map);

    println!("dimensions: {:?}", buffer.dimensions());

    let (_, length) = buffer.dimensions();
    step_filter_normal(80, length, buffer.to_vec())
}

pub fn step_filter_normal(threashold: u8, length: u32, bytes: Vec<u8>) -> Matrix {
    step_filter(threashold, crate::NORMAL_PRINTER_WIDTH, length, bytes)
}

pub fn step_filter_wide(threashold: u8, length: u32, bytes: Vec<u8>) -> Matrix {
    step_filter(threashold, crate::WIDE_PRINTER_WIDTH, length, bytes)
}

fn step_filter(threashold: u8, width: u32, length: u32, bytes: Vec<u8>) -> Matrix {
    // convert a grayscale image to 1-bit black and white data
    // threashold = 80 seems to work fine if original data is monochrome.
    // TODO: Add support for a dithering algorithm to print photos
    //
    // width must be
    let mut bw: Vec<Vec<u8>> = Vec::new();

    for y in 0..length {
        let mut buf: Vec<u8> = Vec::new();
        for x in 0..(width / 8) {
            let index = (1 + y) * width - (1 + x) * 8;
            let mut tmp: u8 = 0x00;
            for i in 0..8 {
                let pixel = bytes[(index + i) as usize];
                let value: u8 = if pixel > threashold { 0 } else { 1 };
                tmp |= value << i;
            }
            buf.push(tmp);
        }
        bw.push(buf);
    }

    bw
}
