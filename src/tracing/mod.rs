//! Functions for recording reads from and writes to images and visualizing
//! image processing pipelines.

pub use self::global_trace::*;
pub use self::replay::*;
pub use self::trace_image::*;

mod global_trace;
mod replay;
mod trace_image;