//! Handles compilation of pipeline definitions.
//! Uses the LLVM wrappers provided by the llvm module.

pub use self::lower::*;
pub use self::processor::*;

mod lower;
mod processor;
