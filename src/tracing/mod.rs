//! Functions for recording reads from and writes to images and visualizing
//! image processing pipelines.

pub use self::replay::*;
pub use self::trace_image::*;

mod replay;
mod trace_image;