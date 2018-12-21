//! Toy halide clone
// TODO: read up on halide, futhark, weld, https://suif.stanford.edu/papers/wolf91a.pdf
// TODO: http://dpzmick.com/2016/08/11/rust-jit-image-processing/

#![feature(test)]

#[macro_use]
pub mod image;
pub mod io;
pub mod tracer;

extern crate test;

pub use crate::image::*;
pub use crate::io::*;
pub use crate::tracer::*;

// Running example: 3x3 box filter

/// 3x3 blur with no intermediate storage
pub fn blur3_inline(image: &GrayImage) -> GrayImage {
    let mut result = Image::new(image.width(), image.height());
    for y in 1..image.height() - 1 {
        for x in 1..image.width() - 1 {
            let mut temp = [0; 3];
            temp[0] = (image.get(x - 1, y - 1) as u16 + image.get(x, y - 1) as u16 + image.get(x + 1, y - 1) as u16) / 3;
            temp[1] = (image.get(x - 1, y) as u16 + image.get(x, y) as u16 + image.get(x + 1, y) as u16) / 3;
            temp[2] = (image.get(x - 1, y + 1) as u16 + image.get(x, y + 1) as u16 + image.get(x + 1, y + 1) as u16) / 3;
            let p = (temp[0] + temp[1] + temp[2]) / 3;
            result.set(x, y, p as u8);
        }
    }
    result
}

/// 3x3 blur where the horizontal blur is computed and stored before computing the vertical blur
pub fn blur3_full_intermediate(image: &GrayImage) -> GrayImage {
    let mut h = GrayImage::new(image.width(), image.height());
    for y in 0..image.height() {
        for x in 1..image.width() - 1 {
            h.set(x, y, ((image.get(x - 1, y) as u16 + image.get(x, y) as u16 + image.get(x + 1, y) as u16) / 3) as u8);
        }
    }
    let mut v = GrayImage::new(image.width(), image.height());
    for y in 1..image.height() - 1 {
        for x in 0..image.width() {
            v.set(x, y, ((h.get(x, y - 1) as u16 + h.get(x, y) as u16 + h.get(x, y + 1) as u16) / 3) as u8);
        }
    }
    v
}

/// 3x3 blur where a strip of horizontal blur of height strip_height is computed and stored
pub fn blur3_split_y(image: &GrayImage, strip_height: usize) -> GrayImage {
    assert!(image.height() % strip_height == 0);
    let buffer_height = strip_height + 2;

    let mut v = GrayImage::new(image.width(), image.height());

    for y_outer in 0..image.height() / strip_height {
        let y_offset = y_outer * strip_height;

        // store "at yo", i.e. at the top of the yo loop body
        let mut strip = GrayImage::new(image.width(), buffer_height);

        // Populate the whole strip's worth of horizontal blur before computing vertical blur
        for y_buffer in 0..buffer_height {
            if y_buffer + y_offset == 0 || y_buffer + y_offset > image.height() {
                continue;
            }
            let y_image = y_buffer + y_offset - 1;
            for x in 1..image.width() - 1 {
                let p = (image.get(x - 1, y_image) as u16 + image.get(x, y_image) as u16 + image.get(x + 1, y_image) as u16) / 3;
                strip.set(x, y_buffer, p as u8);
            }
        }

        for y_inner in 0..strip_height {
            if y_inner + y_offset == 0 || y_inner + y_offset == image.height() - 1 {
                continue;
            }
            for x in 0..image.width() {
                let y_buffer = y_inner + 1;
                let p = (strip.get(x, y_buffer - 1) as u16 + strip.get(x, y_buffer) as u16 + strip.get(x, y_buffer + 1) as u16) / 3;
                v.set(x, y_inner + y_offset, p as u8);
            }
        }
    }

    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::test::*;

    fn image(width: usize, height: usize) -> GrayImage {
        let mut i = GrayImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                i.set(x, y, ((x + y) % 17) as u8);
            }
        }
        i
    }

    fn blur3_reference(image: &GrayImage) -> GrayImage {
        let mut result = Image::new(image.width(), image.height());
        for y in 1..image.height() - 1 {
            for x in 1..image.width() - 1 {
                let t = (image.get(x - 1, y - 1) + image.get(x, y - 1) + image.get(x + 1, y - 1)) / 3;
                let m = (image.get(x - 1, y) + image.get(x, y) + image.get(x + 1, y)) / 3;
                let b = (image.get(x - 1, y + 1) + image.get(x, y + 1) + image.get(x + 1, y + 1)) / 3;
                result.set(x, y, (t + m + b) / 3);
            }
        }
        result
    }

    macro_rules! test_blur3 {
        ($name:ident, $blur_function:ident) => {
            #[test]
            fn $name() {
                let i = image(3, 10);
                let actual = $blur_function(&i);
                let expected = blur3_reference(&i);
                assert_eq!(actual, expected);
            }
        };
    }

    macro_rules! bench_blur3 {
        ($name:ident, $blur_function:ident) => {
            #[bench]
            fn $name(b: &mut Bencher) {
                let i = black_box(image(64, 60));
                b.iter(|| black_box($blur_function(&i)));
            }
        };
    }

    macro_rules! bench_and_test_blur3 {
        ($blur_function:ident) => {
            paste::item! {
                bench_blur3!([<bench_ $blur_function>], $blur_function);
                test_blur3!([<test_ $blur_function>], $blur_function);
            }
        }
    }

    fn blur3_split_y_5(image: &GrayImage) -> GrayImage {
        blur3_split_y(image, 5)
    }

    fn blur3_split_y_2(image: &GrayImage) -> GrayImage {
        blur3_split_y(image, 2)
    }

    bench_and_test_blur3!(blur3_inline);
    bench_and_test_blur3!(blur3_full_intermediate);
    bench_and_test_blur3!(blur3_split_y_5);
    bench_and_test_blur3!(blur3_split_y_2);
}
