//! Used to generate IR for basic functions to nick for use in handwritten/generated IR
//! Without the cargo incremental flag the following warning is produced:
//!  warning: ignoring emit path because multiple .ll files were produced
//!
//! CARGO_INCREMENTAL=0 RUSTFLAGS="--emit=llvm-ir" cargo run --release --example scratch
//!
//! Check target/release/examples/scratch_*.ll to see the generated IR

#![feature(test)]
extern crate test;
use test::black_box;
use libc::c_char;
use std::ffi::{CStr, CString};

#[inline(never)]
#[no_mangle]
pub extern "C" fn log(msg: *const c_char) {
    let msg = unsafe { CStr::from_ptr(msg).to_string_lossy().to_string() };
    println!("{}", msg);
}

#[inline(never)]
#[no_mangle]
pub extern "C" fn process(src: *const u8, dst: *mut u8, width: usize, height: usize, ) {
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

#[inline(never)]
#[no_mangle]
pub fn call_process() {
    let w = black_box(100usize);
    let h = black_box(100usize);
    let p = black_box(0u8);
    let src = vec![p; w * h];
    let mut dst = vec![0u8; w * h];

    process(src.as_ptr(), dst.as_mut_ptr(), w, h);
    println!("{:?}", dst.iter().take(10).collect::<Vec<_>>());
}

#[inline(never)]
#[no_mangle]
pub fn call_log(msg: &str) -> u32 {
    let msg = CString::new(msg).unwrap();
    log(msg.as_ptr() as *const _);
    let x = 5;
    x + 1
}

fn main() {
    let mut msg = String::new();
    for _ in 0..10 {
        msg.push_str("SPAM");
    }
    let _ = call_log(&msg);
}