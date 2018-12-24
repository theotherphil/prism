//! Toy halide clone
// TODO: read up on halide, futhark, weld
// TODO: https://suif.stanford.edu/papers/wolf91a.pdf
// TODO: http://dpzmick.com/2016/08/11/rust-jit-image-processing/

#![feature(test)]



//#######################################
//#######################################
//
// Next steps: 
// * write an example where an input and output
// buffer are created and passed to a function
// hand written in LLVM ir
//
// * create trivial AST and generate IR from it
// (can initially just map any AST to exactly 
// the hand-written IR), call this IR from example
//
//#######################################
//#######################################





#[macro_use]
pub mod buffer;
pub mod io;
pub mod tracer;
pub mod replay;
pub mod traits;
pub mod blur3;

extern crate test;

pub use crate::buffer::*;
pub use crate::io::*;
pub use crate::tracer::*;
pub use crate::traits::*;
pub use crate::replay::*;
pub use crate::blur3::*;
