//! Toy halide clone
// TODO: read up on halide, futhark, weld
// TODO: https://suif.stanford.edu/papers/wolf91a.pdf
// TODO: http://dpzmick.com/2016/08/11/rust-jit-image-processing/
// TODO: https://github.com/jauhien/iron-kaleidoscope

#![feature(test)]
extern crate test;

pub use crate::image::*;
pub use crate::codegen::*;
pub use crate::ast::*;
pub use crate::blur3::*;
pub use crate::llvm::*;
pub use crate::pretty_print::*;
pub use crate::tracing::*;

#[macro_use]
mod image;
#[macro_use]
mod codegen;
mod ast;
mod blur3;
mod llvm;
mod pretty_print;
mod tracing;



