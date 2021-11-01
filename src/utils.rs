/// choose a step filter depending on the priter width.
///
use crate::Matrix;
use image::GenericImageView;

#[cfg(feature = "image")]
use image::imageops::FilterType;

#[cfg(feature = "image")]
pub fn grayscale_to_matrix(image: image::DynamicImage) -> Matrix {
    let (w, h) = image.dimensions();

    // resize the image to fit the label width.
    // For the high-resolution printing, set the factor to 2.
    let length = crate::NORMAL_PRINTER_WIDTH * h / w;
    let factor = 1;
    let resized = image.resize_exact(
        crate::NORMAL_PRINTER_WIDTH,
        length * factor,
        FilterType::Nearest,
    );

    let color_map = image::imageops::colorops::BiLevel;
    let mut buffer = resized.into_luma8();
    image::imageops::colorops::dither(&mut buffer, &color_map);

    step_filter_normal(127, buffer.dimensions().1, buffer.to_vec())
}

pub fn step_filter_normal(threashold: u8, length: u32, bytes: Vec<u8>) -> Matrix {
    step_filter(threashold, crate::NORMAL_PRINTER_WIDTH, length, bytes)
}

pub fn step_filter_wide(threashold: u8, length: u32, bytes: Vec<u8>) -> Matrix {
    step_filter(threashold, crate::WIDE_PRINTER_WIDTH, length, bytes)
}

fn step_filter(threashold: u8, width: u32, length: u32, bytes: Vec<u8>) -> Matrix {
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

#[cfg(test)]
mod tests {
    #[test]
    fn check_step_filter_with_dots() {
        let (w, h): (u32, u32) = (16, 16);
        let mut buf = vec![255; (w * h) as usize];
        for j in 0..h {
            for i in 0..w {
                // buf[(i + j * w) as usize] = (255 * i / w) as u8;
                buf[(i + j * w) as usize] = if i % 2 == 0 { 255 } else { 0 };
            }
        }

        let filtered = super::step_filter(127, w, h, buf);

        let bw = vec![vec![170, 170]; 16];

        assert_eq!(filtered, bw);
    }
}
