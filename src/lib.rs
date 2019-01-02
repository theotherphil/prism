//! Toy halide clone
// TODO: read up on halide, futhark, weld
// TODO: https://suif.stanford.edu/papers/wolf91a.pdf
// TODO: http://dpzmick.com/2016/08/11/rust-jit-image-processing/
// TODO: https://github.com/jauhien/iron-kaleidoscope

#![feature(test)]
extern crate test;
extern crate llvm_sys as llvm;

#[macro_use]
pub mod buffer;
pub mod io;
pub mod tracer;
pub mod replay;
pub mod traits;
pub mod blur3;
#[macro_use]
pub mod codegen;
pub mod ast;
pub mod pretty_print;
pub mod processor;

pub use crate::buffer::*;
pub use crate::io::*;
pub use crate::tracer::*;
pub use crate::traits::*;
pub use crate::replay::*;
pub use crate::blur3::*;
pub use crate::ast::*;
pub use crate::pretty_print::*;
pub use crate::processor::*;

