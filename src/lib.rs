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

use std::rc::Rc;
use std::cell::RefCell;

// The stuff with storage and Rc<RefCell<S::Image>> is pretty horrible, but it lets us
// use the same code for testing the performance of different domain orders, storage orders, etc.
// and for producing visualisations. (Although the optimiser does appear to be massively fickle
// in what it decides to optimise away.)

// Running example: 3x3 box filter

/// 3x3 blur with no intermediate storage
pub fn blur3_inline<S: Storage>(storage: &mut S, image: Rc<RefCell<S::Image>>) -> Rc<RefCell<S::Image>> {
    let image = &*image.borrow();
    let result_ref = storage.create_image(image.width(), image.height());
    {
        let result = &mut *result_ref.borrow_mut();
        blur3_inline_impl(image, result);
    }
    result_ref
}

// A "raw" version of blur3_inline was 8x faster than a generic version using a Storage instance.
// Cutting and pasting the identical (post-preamble) method bodies into this method resulted in
// one the raw version getting 4x slower and the storage-based version getting 2x faster.
// So the diff was all down to the fickleness of the optimiser. As I'm just after consistency,
// I'm going to use this approach (delegating the bulk of the work to a function that relies on all
// intermediate storage being pre-allocated) for all my handwritten example programs.
fn blur3_inline_impl<I: Image<u8>>(image: &I, result: &mut I) {
    for y in 1..image.height() - 1 {
        for x in 1..image.width() - 1 {
            let t = (image.get(x - 1, y - 1) as u16 + image.get(x, y - 1) as u16 + image.get(x + 1, y - 1) as u16) / 3;
            let m = (image.get(x - 1, y) as u16 + image.get(x, y) as u16 + image.get(x + 1, y) as u16) / 3;
            let b = (image.get(x - 1, y + 1) as u16 + image.get(x, y + 1) as u16 + image.get(x + 1, y + 1) as u16) / 3;
            let p = (t + m + b) / 3;
            result.set(x, y, p as u8);
        }
    }
}

/// 3x3 blur where the horizontal blur is computed and stored before computing the vertical blur
pub fn blur3_intermediate<S: Storage>(storage: &mut S, image: Rc<RefCell<S::Image>>) -> Rc<RefCell<S::Image>> {
    let image = &*image.borrow();
    let h_ref = storage.create_image(image.width(), image.height());
    let v_ref = storage.create_image(image.width(), image.height());
    {
        let h = &mut *h_ref.borrow_mut();
        let v = &mut *v_ref.borrow_mut();
        blur3_full_intermediate_impl(image, h, v);
    }
    v_ref
}

fn blur3_full_intermediate_impl<I: Image<u8>>(image: &I, h: &mut I, v: &mut I) {
    for y in 0..image.height() {
        for x in 1..image.width() - 1 {
            h.set(x, y, ((image.get(x - 1, y) as u16 + image.get(x, y) as u16 + image.get(x + 1, y) as u16) / 3) as u8);
        }
    }
    for y in 1..image.height() - 1 {
        for x in 0..image.width() {
            v.set(x, y, ((h.get(x, y - 1) as u16 + h.get(x, y) as u16 + h.get(x, y + 1) as u16) / 3) as u8);
        }
    }
}

/// 3x3 blur where a strip of horizontal blur of height strip_height is computed and stored
pub fn blur3_split_y<S: Storage>(storage: &mut S, image: Rc<RefCell<S::Image>>, strip_height: usize) -> Rc<RefCell<S::Image>> {
    let image = &*image.borrow();
    assert!(image.height() % strip_height == 0);
    let strip_ref = storage.create_image(image.width(), strip_height + 2);
    let v_ref = storage.create_image(image.width(), image.height());
    {
        let strip = &mut *strip_ref.borrow_mut();
        let v = &mut *v_ref.borrow_mut();
        blur3_split_y_impl(image, strip, v, strip_height);
    }
    v_ref
}

fn blur3_split_y_impl<I: Image<u8>>(image: &I, strip: &mut I, v: &mut I, strip_height: usize) {
    for y_outer in 0..image.height() / strip_height {
        let y_offset = y_outer * strip_height;
        strip.clear();

        for y_buffer in 0..strip.height() {
            if y_buffer + y_offset == 0 || y_buffer + y_offset > image.height() {
                continue;
            }
            let y_image = y_buffer + y_offset - 1;
            for x in 1..image.width() - 1 {
                let p = (
                    image.get(x - 1, y_image) as u16
                    + image.get(x, y_image) as u16
                    + image.get(x + 1, y_image) as u16
                    ) / 3;
                strip.set(x, y_buffer, p as u8);
            }
        }

        for y_inner in 0..strip_height {
            if y_inner + y_offset == 0 || y_inner + y_offset == image.height() - 1 {
                continue;
            }
            for x in 0..image.width() {
                let y_buffer = y_inner + 1;
                let p = (
                    strip.get(x, y_buffer - 1) as u16
                    + strip.get(x, y_buffer) as u16
                    + strip.get(x, y_buffer + 1) as u16
                    ) / 3;
                v.set(x, y_inner + y_offset, p as u8);
            }
        }
    }
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

    fn blur3_reference<S: Storage>(storage: &mut S, image: Rc<RefCell<S::Image>>) -> Rc<RefCell<S::Image>> {
        let image = image.borrow();
        let result_ref = storage.create_image(image.width(), image.height());
        {
            let mut result = result_ref.borrow_mut();

            for y in 1..image.height() - 1 {
                for x in 1..image.width() - 1 {
                    let t = (image.get(x - 1, y - 1) + image.get(x, y - 1) + image.get(x + 1, y - 1)) / 3;
                    let m = (image.get(x - 1, y) + image.get(x, y) + image.get(x + 1, y)) / 3;
                    let b = (image.get(x - 1, y + 1) + image.get(x, y + 1) + image.get(x + 1, y + 1)) / 3;
                    result.set(x, y, (t + m + b) / 3);
                }
            }
        }
        result_ref
    }

    macro_rules! test_blur3 {
        ($blur_function:ident) => {
            paste::item! {
                #[test]
                fn [<test_ $blur_function>]() {
                    let i = black_box(image(5, 10));

                    let actual = {
                        let mut s = BufferStore::new();
                        let i = s.create_from_image(&i);
                        $blur_function(&mut s, i)
                    };
                    let expected = {
                        let mut s = BufferStore::new();
                        let i = s.create_from_image(&i);
                        blur3_reference(&mut s, i)
                    };
                    assert_eq!(&*actual.borrow(), &*expected.borrow());
                }
            }
        };
    }

    macro_rules! bench_blur3 {
        ($blur_function:ident) => {
            paste::item! {
                #[bench]
                fn [<bench_ $blur_function>](b: &mut Bencher) {
                    let mut s = BufferStore::new();
                    let i = s.create_from_image(&black_box(image(60, 60)));
                    b.iter(|| {
                        s.clear();
                        black_box($blur_function(&mut s, i.clone()))
                    });
                }
            }
        };
    }

    macro_rules! bench_and_test_blur3 {
        ($blur_function:ident) => {
            test_blur3!($blur_function);
            bench_blur3!($blur_function);
        }
    }

    fn blur3_split_y_5<S: Storage>(storage: &mut S, image: Rc<RefCell<S::Image>>) -> Rc<RefCell<S::Image>> {
        blur3_split_y(storage, image, 5)
    }

    fn blur3_split_y_2<S: Storage>(storage: &mut S, image: Rc<RefCell<S::Image>>) -> Rc<RefCell<S::Image>> {
        blur3_split_y(storage, image, 2)
    }

    bench_and_test_blur3!(blur3_inline);
    bench_and_test_blur3!(blur3_intermediate);
    bench_and_test_blur3!(blur3_split_y_5);
    bench_and_test_blur3!(blur3_split_y_2);
}
