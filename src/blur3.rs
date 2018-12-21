//! Some handwritten examples of 3x3 blur functions

use std::rc::Rc;
use std::cell::RefCell;
use crate::traits::*;

// The stuff with storage and Rc<RefCell<S::Image>> is pretty horrible, but it lets us
// use the same code for testing the performance of different domain orders, storage orders, etc.
// and for producing visualisations. (Although the optimiser does appear to be massively fickle
// in what it decides to optimise away.)

// Running example: 3x3 box filter

fn mean(a: u8, b: u8, c: u8) -> u8 {
    ((a as u16 + b as u16 + c as u16) / 3) as u8
}

macro_rules! continue_if_outside_range {
    ($x:expr, $lower:expr, $upper:expr) => {
        let (x, l, u) = ($x, $upper, $lower);
        if x < l || x > u {
            continue;
        }
    };
}

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
            let t = mean(image.get(x - 1, y - 1), image.get(x, y - 1), image.get(x + 1, y - 1));
            let m = mean(image.get(x - 1, y), image.get(x, y), image.get(x + 1, y));
            let b = mean(image.get(x - 1, y + 1), image.get(x, y + 1), image.get(x + 1, y + 1));
            let p = mean(t, m, b);
            result.set(x, y, p);
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
            h.set(x, y, mean(image.get(x - 1, y), image.get(x, y), image.get(x + 1, y)));
        }
    }
    for y in 1..image.height() - 1 {
        for x in 0..image.width() {
            v.set(x, y, mean(h.get(x, y - 1), h.get(x, y), h.get(x, y + 1)));
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
            continue_if_outside_range!(y_buffer + y_offset, 1, image.height());
            let y_image = y_buffer + y_offset - 1;
            for x in 1..image.width() - 1 {
                let p = mean(image.get(x - 1, y_image), image.get(x, y_image), image.get(x + 1, y_image));
                strip.set(x, y_buffer, p);
            }
        }

        for y_inner in 0..strip_height {
            continue_if_outside_range!(y_inner + y_offset, 1, image.height() - 2);
            let y_buffer = y_inner + 1;

            for x in 0..image.width() {
                let p = mean(strip.get(x, y_buffer - 1), strip.get(x, y_buffer), strip.get(x, y_buffer + 1));
                v.set(x, y_inner + y_offset, p);
            }
        }
    }
}

/// 3x3 blur where a strip of horizontal blur of height strip_height is computed and stored
pub fn blur3_tiled<S: Storage>(
    storage: &mut S,
    image: Rc<RefCell<S::Image>>,
    tile_width: usize,
    tile_height: usize
) -> Rc<RefCell<S::Image>> {
    let image = &*image.borrow();
    assert!(image.height() % tile_width == 0);
    assert!(image.height() % tile_height == 0);

    let tile_ref = storage.create_image(tile_width, tile_height + 2);
    let result_ref = storage.create_image(image.width(), image.height());
    {
        let tile = &mut *tile_ref.borrow_mut();
        let result = &mut *result_ref.borrow_mut();
        blur3_tiled_impl(image, tile, result, tile_width, tile_height);
    }
    result_ref
}

// The bounds checking here is awful. Need to do something more sensible
fn blur3_tiled_impl<I: Image<u8>>(image: &I, tile: &mut I, result: &mut I, tile_width: usize, tile_height: usize) {
    // tile height is tile_height
    // tile width is tile_width + 2
    for y_outer in 0..image.height() / tile_height {
        let y_offset = y_outer * tile_height;

        for x_outer in 0..image.width() / tile_width {
            let x_offset = x_outer * tile_width;
            tile.clear();

            // Populate the tile with the horizontal blur
            for y_buffer in 0..tile.height() {
                continue_if_outside_range!(y_buffer + y_offset, 1, image.height());
                let y_image = y_buffer + y_offset - 1;

                for x_buffer in 0..tile.width() {
                    continue_if_outside_range!(x_buffer + x_offset, 1, image.width());
                    let x_image = x_buffer + x_offset;

                    let p = mean(
                        image.get(x_image - 1, y_image), image.get(x_image, y_image), image.get(x_image + 1, y_image)
                    );
                    tile.set(x_buffer, y_buffer, p);
                }
            }

            // Compute vertical blur using tile contents
            for y_inner in 0..tile_height {
                continue_if_outside_range!(y_inner + y_offset, 1, image.height() - 2);
                let y_buffer = y_inner + 1;

                for x_inner in 0..tile_width {
                    continue_if_outside_range!(x_inner + x_offset, 1, image.width() - 2);
                    let x_buffer = x_inner;
                    let p = mean(
                        tile.get(x_buffer, y_buffer - 1), tile.get(x_buffer, y_buffer), tile.get(x_buffer, y_buffer + 1)
                    );
                    result.set(x_buffer + x_offset, y_inner + y_offset, p);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::test::*;
    use crate::buffer::*;

    fn image(storage: &mut BufferStore, width: usize, height: usize) -> Rc<RefCell<GrayImage>> {
        let i_ref = storage.create_image(width, height);
        {
            let i = &mut *i_ref.borrow_mut();
            for y in 0..height {
                for x in 0..width {
                    i.set(x, y, ((x + y) % 17) as u8);
                }
            }
        }
        black_box(i_ref)
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
                    let mut s = BufferStore::new();
                    let i = image(&mut s, 10, 10);
                    let actual = $blur_function(&mut s, i.clone());
                    let expected = blur3_reference(&mut s, i);
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
                    let i = image(&mut s, 60, 60);
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

    fn blur3_tiled_5<S: Storage>(storage: &mut S, image: Rc<RefCell<S::Image>>) -> Rc<RefCell<S::Image>> {
        blur3_tiled(storage, image, 5, 5)
    }

    bench_and_test_blur3!(blur3_inline);
    bench_and_test_blur3!(blur3_intermediate);
    bench_and_test_blur3!(blur3_split_y_5);
    bench_and_test_blur3!(blur3_split_y_2);
    bench_and_test_blur3!(blur3_tiled_5);
}
