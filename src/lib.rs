//! Toy halide clone
// TODO: read up on halide, futhark, weld
// TODO: http://dpzmick.com/2016/08/11/rust-jit-image-processing/

#![feature(test)]

#[macro_use]
pub mod image;

extern crate test;

use crate::image::*;

pub type GrayImage = Image<u8>;

// Running example: 3x3 box filter

pub fn blur3_inline(image: &GrayImage) -> GrayImage {
    let mut result = Image::new(image.width(), image.height());
    for y in 1..image.height() - 1 {
        for x in 1..image.width() - 1 {
            // Need to handle overflow and loss of precision better
            let mut temp = [0; 3];
            temp[0] = (image[[x - 1, y - 1]] + image[[x, y - 1]] + image[[x + 1, y - 1]]) / 3;
            temp[1] = (image[[x - 1, y]] + image[[x, y]] + image[[x + 1, y]]) / 3;
            temp[2] = (image[[x - 1, y + 1]] + image[[x, y + 1]] + image[[x + 1, y + 1]]) / 3;
            let p = (temp[0] + temp[1] + temp[2]) / 3;
            result[[x, y]] = p;
        }
    }
    result
}

pub fn blur3_full_intermediate(image: &GrayImage) -> GrayImage {
    let mut h = GrayImage::new(image.width(), image.height());
    for y in 0..image.height() {
        for x in 1..image.width() - 1 {
            h[[x, y]] = (image[[x - 1, y]] + image[[x, y]] + image[[x + 1, y]]) / 3;
        }
    }
    let mut v = GrayImage::new(image.width(), image.height());
    for y in 1..image.height() - 1 {
        for x in 0..image.width() {
            v[[x, y]] = (h[[x, y - 1]] + h[[x, y]] + h[[x, y + 1]]) / 3;
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::test::*;

    #[bench]
    fn bench_blur3_inline(b: &mut Bencher) {
        let i = gray_image!(1, 2, 3; 4, 5, 6; 7, 8, 9);
        b.iter(|| black_box(blur3_inline(&i)));
    }

    #[bench]
    fn bench_blur3_full_intermediate(b: &mut Bencher) {
        let i = gray_image!(1, 2, 3; 4, 5, 6; 7, 8, 9);
        b.iter(|| black_box(blur3_full_intermediate(&i)));
    }
}
