
use prism::*;
use std::time::{Duration, SystemTime};

fn compare_blur_perf(width: usize) {
    let mut image = GrayImage::new(width, width);
    for y in 0..image.height() {
        for x in 0..image.width() {
            image.set(x, y, (10 * (x % 10 + y % 10) as u8) + 50);
        }
    }

    {
        let now = SystemTime::now();
        let r = blur3_inline(&image);
        println!("{:?} RAW: {:?}", r.width(), now.elapsed().unwrap());
    }
    {
        let mut store = BufferStore::new();
        let image = store.create_from_image(&image);
        let now = SystemTime::now();
        let r = trace_blur3_inline(&mut store, image);
        println!("{:?} TRACE: {:?}", r.borrow().width(), now.elapsed().unwrap());
    }
}

fn main() {
    for i in &[10, 100, 1000, 10000] {
        compare_blur_perf(*i);
    }
}