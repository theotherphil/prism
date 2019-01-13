//! The syntax used to represent image processing pipelines.

pub use self::ast::*;
pub use self::func::*;
pub use self::graph::*;
pub use self::pretty_print::*;

#[macro_use]
mod ast;
mod func;
mod graph;
mod pretty_print;

/// Shorthand for creating a new `Source`.
///
/// The following code samples are equivalent.
/// 
/// ```source!(input);```
///
/// ```let input = Source::new("input");```
#[macro_export]
macro_rules! source {
    ($name:ident) => {
        let $name = Source::new(stringify!($name));
    }
}

/// Shorthand for creating a new `Func`.
///
/// The following code samples are equivalent.
/// 
/// ```func!(g = f.at(x, y));```
///
/// ```let g = Func::new("g", f.at(x, y));```
#[macro_export]
macro_rules! func {
    ($name:ident = $($rest:tt)*) => {
        let $name = Func::new(stringify!($name), $($rest)*);
    }
}

/// Shorthand for creating a new `Param`.
#[macro_export]
macro_rules! param {
    ($name:ident) => {
        let $name = Param::new(stringify!($name));
    }
}
