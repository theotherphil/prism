
use prism::*;

fn main() {
    let mut image = GrayImage::new(5, 5);
    for y in 0..image.height() {
        for x in 0..image.width() {
            image.set(x, y, (10 * (x % 10 + y % 10) as u8) + 50);
        }
    }

    {
        let mut store = BufferStore::new();
        let image = store.create_from_image(&image);
        let r = trace_blur3_inline(&mut store, image);
        println!("{:?}", r.borrow().dimensions());
    }
    {
        let r = blur3_inline(&image);
        println!("{:?}", r.dimensions());
    }
}