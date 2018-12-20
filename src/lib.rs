//! Toy halide clone
// TODO: read up on halide, futhark, weld, https://suif.stanford.edu/papers/wolf91a.pdf
// TODO: http://dpzmick.com/2016/08/11/rust-jit-image-processing/

#![feature(test)]

#[macro_use]
pub mod image;
pub mod io;

extern crate test;

pub use crate::image::*;
pub use crate::io::*;

pub type GrayImage = Image<u8>;

// Running example: 3x3 box filter

/// 3x3 blur with no intermediate storage
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

/// 3x3 blur where the horizontal blur is computed and stored before computing the vertical blur
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

/// 3x3 blur where a 5 row slice is computed at a time, and each 5 row
/// slice of the horizontal blur is computed before computing the corresponding
/// section of the vertical blur
pub fn blur3_split_y_5(image: &GrayImage) -> GrayImage {
    const STRIP_HEIGHT: usize = 5;
    assert!((image.height() - 2) % (STRIP_HEIGHT - 2) == 0); // consecutive strips have a two row overlap

    let mut v = GrayImage::new(image.width(), image.height());

    for yo in 0..(image.height() - 1) / (STRIP_HEIGHT - 2) {
        let y_offset = yo * (STRIP_HEIGHT - 2);

        // store "at yo", i.e. at the top of the yo loop body
        let mut strip = GrayImage::new(image.width(), STRIP_HEIGHT);

        // Populate the whole strip's worth of horizontal blur before computing vertical blur
        for yi in 0..STRIP_HEIGHT {
            for x in 1..image.width() - 1 {
                let y = yi + y_offset;
                strip[[x, yi]] = (image[[x - 1, y]] + image[[x, y]] + image[[x + 1, y]]) / 3;
            }
        }

        for yi in 1..STRIP_HEIGHT - 1 {
            for x in 0..image.width() {
                v[[x, yi + y_offset]] = (strip[[x, yi - 1]] + strip[[x, yi]] + strip[[x, yi + 1]]) / 3;
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
                i[[x, y]] = ((x + y) % 17) as u8;
            }
        }
        i
    }

    fn blur3_reference(image: &GrayImage) -> GrayImage {
        let mut result = Image::new(image.width(), image.height());
        for y in 1..image.height() - 1 {
            for x in 1..image.width() - 1 {
                let t = (image[[x - 1, y - 1]] + image[[x, y - 1]] + image[[x + 1, y - 1]]) / 3;
                let m = (image[[x - 1, y]] + image[[x, y]] + image[[x + 1, y]]) / 3;
                let b = (image[[x - 1, y + 1]] + image[[x, y + 1]] + image[[x + 1, y + 1]]) / 3;
                result[[x, y]] = (t + m + b) / 3;
            }
        }
        result
    }

    macro_rules! test_blur3 {
        ($name:ident, $blur_function:ident) => {
            #[test]
            fn $name() {
                let i = image(3, 11);
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
                let i = black_box(image(64, 62));
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

    bench_and_test_blur3!(blur3_inline);
    bench_and_test_blur3!(blur3_full_intermediate);
    bench_and_test_blur3!(blur3_split_y_5);
}
