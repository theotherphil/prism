//! Toy halide clone.

// TODO: read up on halide, futhark, weld
// TODO: https://suif.stanford.edu/papers/wolf91a.pdf
// TODO: http://dpzmick.com/2016/08/11/rust-jit-image-processing/
// TODO: https://github.com/jauhien/iron-kaleidoscope

#![feature(test)]
extern crate test;

#[macro_use]
pub mod image;
#[macro_use]
pub mod codegen;
#[macro_use]
pub mod syntax;
pub mod blur3;
pub mod llvm;
pub mod tracing;
