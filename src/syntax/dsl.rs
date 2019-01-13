//! Contains operator impls and macros to allow ergonomic construction of AST nodes.

use std::ops::{Add, Div, Mul, Sub};
use crate::syntax::ast::*;

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

macro_rules! impl_var_expr_bin_op {
    ($trait_name:ident, $trait_op:ident, $ctor:expr) => {
        impl $trait_name<Self> for VarExpr {
            type Output = VarExpr;
            fn $trait_op(self, rhs: Self) -> VarExpr {
                $ctor(Box::new(self), Box::new(rhs))
            }
        }

        impl $trait_name<i32> for VarExpr {
            type Output = VarExpr;
            fn $trait_op(self, rhs: i32) -> VarExpr {
                $ctor(Box::new(self), Box::new(VarExpr::Const(rhs)))
            }
        }

        impl $trait_name<VarExpr> for i32 {
            type Output = VarExpr;
            fn $trait_op(self, rhs: VarExpr) -> VarExpr {
                $ctor(Box::new(VarExpr::Const(self)), Box::new(rhs))
            }
        }

        impl $trait_name<VarExpr> for Var {
            type Output = VarExpr;
            fn $trait_op(self, rhs: VarExpr) -> VarExpr {
                $ctor(Box::new(VarExpr::Var(self)), Box::new(rhs))
            }
        }

        impl $trait_name<Var> for VarExpr {
            type Output = VarExpr;
            fn $trait_op(self, rhs: Var) -> VarExpr {
                $ctor(Box::new(self), Box::new(VarExpr::Var(rhs)))
            }
        }

        impl $trait_name<i32> for Var {
            type Output = VarExpr;
            fn $trait_op(self, rhs: i32) -> VarExpr {
                $ctor(Box::new(VarExpr::Var(self)), Box::new(VarExpr::Const(rhs)))
            }
        }

        impl $trait_name<Var> for i32 {
            type Output = VarExpr;
            fn $trait_op(self, rhs: Var) -> VarExpr {
                $ctor(Box::new(VarExpr::Const(self)), Box::new(VarExpr::Var(rhs)))
            }
        }

        impl $trait_name<Var> for Var {
            type Output = VarExpr;
            fn $trait_op(self, rhs: Var) -> VarExpr {
                $ctor(Box::new(VarExpr::Var(self)), Box::new(VarExpr::Var(rhs)))
            }
        }
    };
}

impl_var_expr_bin_op!(Add, add, VarExpr::Add);
impl_var_expr_bin_op!(Sub, sub, VarExpr::Sub);
impl_var_expr_bin_op!(Mul, mul, VarExpr::Mul);

impl Into<VarExpr> for Var {
    fn into(self) -> VarExpr {
        VarExpr::Var(self)
    }
}

macro_rules! impl_definition_bin_op {
    ($trait_name:ident, $trait_op:ident, $ctor:expr) => {
        impl $trait_name<Self> for Definition {
            type Output = Definition;
            fn $trait_op(self, rhs: Self) -> Definition {
                $ctor(Box::new(self), Box::new(rhs))
            }
        }

        impl $trait_name<i32> for Definition {
            type Output = Definition;
            fn $trait_op(self, rhs: i32) -> Definition {
                $ctor(Box::new(self), Box::new(Definition::Const(rhs)))
            }
        }

        impl $trait_name<Definition> for i32 {
            type Output = Definition;
            fn $trait_op(self, rhs: Definition) -> Definition {
                $ctor(Box::new(Definition::Const(self)), Box::new(rhs))
            }
        }

        impl $trait_name<&Param> for Definition {
            type Output = Definition;
            fn $trait_op(self, rhs: &Param) -> Definition {
                $ctor(Box::new(self), Box::new(Definition::Param(rhs.name.clone())))
            }
        }

        impl $trait_name<Definition> for &Param {
            type Output = Definition;
            fn $trait_op(self, rhs: Definition) -> Definition {
                $ctor(Box::new(Definition::Param(self.name.clone())), Box::new(rhs))
            }
        }
    };
}

impl_definition_bin_op!(Add, add, Definition::Add);
impl_definition_bin_op!(Sub, sub, Definition::Sub);
impl_definition_bin_op!(Mul, mul, Definition::Mul);
impl_definition_bin_op!(Div, div, Definition::Div);
