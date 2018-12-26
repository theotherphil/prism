
// Used to generate IR for basic functions to nick for use in handwritten/generated IR
// Without the cargo incremental flag the following warning is produced:
//  warning: ignoring emit path because multiple .ll files were produced
//
// CARGO_INCREMENTAL=0 RUSTFLAGS="--emit=llvm-ir" cargo run --release --example scratch

#![feature(test)]
extern crate test;
use test::black_box;

#[inline(never)]
pub fn process(src: *const u8, dst: *mut u8, width: usize, height: usize, ) {
    unsafe {
        for y in 0..height {
            for x in 0..width {
                dst
                    .offset((y * width + x) as isize)
                    .write(*src.offset((y * width + x) as isize) + 3);
            }
        }
    }
}

fn main() {
    let w = black_box(100usize);
    let h = black_box(100usize);
    let p = black_box(0u8);
    let src = vec![p; w * h];
    let mut dst = vec![0u8; w * h];

    process(src.as_ptr(), dst.as_mut_ptr(), w, h);
    println!("{:?}", dst.iter().take(10).collect::<Vec<_>>());
}