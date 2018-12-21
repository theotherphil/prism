//! Toy halide clone
// TODO: read up on halide, futhark, weld, https://suif.stanford.edu/papers/wolf91a.pdf
// TODO: http://dpzmick.com/2016/08/11/rust-jit-image-processing/

#![feature(test)]

#[macro_use]
pub mod buffer;
pub mod io;
pub mod tracer;
pub mod traits;
pub mod blur3;

extern crate test;

pub use crate::buffer::*;
pub use crate::io::*;
pub use crate::tracer::*;
pub use crate::traits::*;
pub use crate::blur3::*;