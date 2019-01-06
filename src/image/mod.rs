//! Defines the basic image traits, and a buffer-based implementation of them.

pub use self::buffer::*;
pub use self::io::*;
pub use self::traits::*;

#[macro_use]
mod buffer;
mod io;
mod traits;
