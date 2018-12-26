
// Used to generate IR for basic functions to nick for use in handwritten/generated IR
// Without the cargo incremental flag the following warning is produced:
//  warning: ignoring emit path because multiple .ll files were produced
//
// CARGO_INCREMENTAL=0 RUSTFLAGS="--emit=llvm-ir" cargo run --release --example scratch

pub fn process(src: *const u8, dst: *mut u8, width: usize, height: usize, ) {
    unsafe {
        for y in 0..height {
            for x in 0..width {
                dst
                    .offset((y * width + x) as isize)
                    .write(*src.offset((y * width + x) as isize));
            }
        }
    }
}

fn main() {
    let src = vec![0u8; 100 * 100];
    let mut dst = vec![0u8; 100 * 100];

    process(src.as_ptr(), dst.as_mut_ptr(), 100, 100);
    println!("{:?}", dst.iter().take(10).collect::<Vec<_>>());
}