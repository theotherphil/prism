//! Some handwritten examples of 3x3 blur functions

use crate::traits::*;

// Running example: 3x3 box filter

fn mean(a: u8, b: u8, c: u8) -> u8 {
    ((a as u16 + b as u16 + c as u16) / 3) as u8
}

macro_rules! continue_if_outside_range {
    ($x:expr, $lower:expr, $upper:expr) => {
        let (x, l, u) = ($x, $lower, $upper);
        if x < l || x > u {
            continue;
        }
    };
}

/// 3x3 blur with no intermediate storage
pub fn blur3_inline<F: Factory>(factory: &mut F, image: &F::Image) -> F::Image {
    let mut result = factory.create_image(image.width(), image.height());
    blur3_inline_body(image, &mut result);
    result
}

fn blur3_inline_body<I: Image<u8>>(image: &I, result: &mut I) {
    for y in 1..image.height() - 1 {
        for x in 1..image.width() - 1 {
            result.active(x, y, 1, 1);
            let t = mean(image.get(x - 1, y - 1), image.get(x, y - 1), image.get(x + 1, y - 1));
            let m = mean(image.get(x - 1, y), image.get(x, y), image.get(x + 1, y));
            let b = mean(image.get(x - 1, y + 1), image.get(x, y + 1), image.get(x + 1, y + 1));
            let p = mean(t, m, b);
            result.set(x, y, p);
        }
    }
}

/// 3x3 blur where the horizontal blur is computed and stored before computing the vertical blur
pub fn blur3_intermediate<F: Factory>(factory: &mut F, image: &F::Image) -> F::Image {
    let mut h = factory.create_image(image.width(), image.height());
    let mut v = factory.create_image(image.width(), image.height());
    blur3_intermediate_body(image, &mut h, &mut v);
    v
}

fn blur3_intermediate_body<I: Image<u8>>(image: &I, h: &mut I, v: &mut I) {
    v.active(0, 0, image.width(), image.height());
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

/// 3x3 blur where we allocate storage for the entire horizontal blur image, but consume
/// these values as soon as they're created.
pub fn blur3_local_intermediate<F: Factory>(factory: &mut F, image: &F::Image) -> F::Image {
    assert!(image.height() > 2);
    let mut h = factory.create_image(image.width(), image.height());
    let mut v = factory.create_image(image.width(), image.height());
    blur3_local_intermediate_body(image, &mut h, &mut v);
    v
}

fn blur3_local_intermediate_body<I: Image<u8>>(image: &I, h: &mut I, v: &mut I) {
    for x in 1..image.width() - 1 {
        v.active(x, 1, 1, 1);
        h.set(x, 0, mean(image.get(x - 1, 0), image.get(x, 0), image.get(x + 1, 0)));
        h.set(x, 1, mean(image.get(x - 1, 1), image.get(x, 1), image.get(x + 1, 1)));
        h.set(x, 2, mean(image.get(x - 1, 2), image.get(x, 2), image.get(x + 1, 2)));
        v.set(x, 1, mean(h.get(x, 0), h.get(x, 1), h.get(x, 2)));
    }
    for y in 3..image.height() {
        for x in 1..image.width() - 1 {
            v.active(x, y - 1, 1, 1);
            h.set(x, y, mean(image.get(x - 1, y), image.get(x, y), image.get(x + 1, y)));
            v.set(x, y - 1, mean(h.get(x, y - 2), h.get(x, y - 1), h.get(x, y)));
        }
    }
}

/// 3x3 blur where a strip of horizontal blur of height strip_height is computed and stored
pub fn blur3_split_y<F: Factory>(factory: &mut F, image: &F::Image, strip_height: usize) -> F::Image {
    assert!(image.height() % strip_height == 0);
    let mut strip = factory.create_image(image.width(), strip_height + 2);
    let mut v = factory.create_image(image.width(), image.height());
    blur3_split_y_body(image, &mut strip, &mut v, strip_height);
    v
}

fn blur3_split_y_body<I: Image<u8>>(image: &I, strip: &mut I, v: &mut I, strip_height: usize) {
    for y_outer in 0..image.height() / strip_height {
        let y_offset = y_outer * strip_height;
        strip.clear();
        v.active(0, y_offset, image.width(), strip_height);

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
pub fn blur3_tiled<F: Factory>(
    factory: &mut F,
    image: &F::Image,
    tile_width: usize,
    tile_height: usize
) -> F::Image {
    assert!(image.height() % tile_width == 0);
    assert!(image.height() % tile_height == 0);
    let mut tile = factory.create_image(tile_width, tile_height + 2);
    let mut result = factory.create_image(image.width(), image.height());
    blur3_tiled_body(image, &mut tile, &mut result, tile_width, tile_height);
    result
}

// The bounds checking here is awful. Need to do something more sensible
fn blur3_tiled_body<I: Image<u8>>(image: &I, tile: &mut I, result: &mut I, tile_width: usize, tile_height: usize) {
    // tile height is tile_height
    // tile width is tile_width + 2
    for y_outer in 0..image.height() / tile_height {
        let y_offset = y_outer * tile_height;

        for x_outer in 0..image.width() / tile_width {
            let x_offset = x_outer * tile_width;
            tile.clear();
            result.active(x_offset, y_offset, tile_width, tile_height);

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

    fn image(width: usize, height: usize) -> GrayImage {
        let mut i = GrayImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                i.set(x, y, ((x + y) % 17) as u8);
            }
        }
        black_box(i)
    }

    fn blur3_reference<F: Factory>(factory: &mut F, image: F::Image) -> F::Image {
        let mut result = factory.create_image(image.width(), image.height());

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
        ($blur_function:ident) => {
            paste::item! {
                #[test]
                fn [<test_ $blur_function>]() {
                    let i = image(10, 10);
                    let mut f = BufferFactory::new();
                    let actual = $blur_function(&mut f, &i);
                    let expected = blur3_reference(&mut f, i);
                    assert_eq!(actual, expected);
                }
            }
        };
    }

    macro_rules! bench_blur3 {
        ($blur_function:ident) => {
            paste::item! {
                #[bench]
                fn [<bench_ $blur_function>](b: &mut Bencher) {
                    let mut f = BufferFactory::new();
                    let i = image(180, 180);
                    b.iter(|| {
                        black_box($blur_function(&mut f, &i))
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

    fn blur3_split_y_5<F: Factory>(factory: &mut F, image: &F::Image) -> F::Image {
        blur3_split_y(factory, image, 5)
    }

    fn blur3_split_y_2<F: Factory>(factory: &mut F, image: &F::Image) -> F::Image {
        blur3_split_y(factory, image, 2)
    }

    fn blur3_tiled_5<F: Factory>(factory: &mut F, image: &F::Image) -> F::Image {
        blur3_tiled(factory, image, 5, 5)
    }

    bench_and_test_blur3!(blur3_inline);
    bench_and_test_blur3!(blur3_intermediate);
    bench_and_test_blur3!(blur3_local_intermediate);
    bench_and_test_blur3!(blur3_split_y_5);
    bench_and_test_blur3!(blur3_split_y_2);
    bench_and_test_blur3!(blur3_tiled_5);
}
