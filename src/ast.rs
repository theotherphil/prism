#![allow(dead_code)]

use std::fmt;
use std::ops::{Add, Div, Mul, Sub};

// [NOTE: AST terminology]
//
//             VarExpr
//               |
//             v~~~v
// f(x, y) = g(x + 1, y - 1) + g(x - 1, y) + 2
//           ^~~~~~~~~~~~~~^
//                   |
//                 Access
//
//           ^~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~^
//                           |
//                      Definition

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Var { X, Y }

impl fmt::Display for Var {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Var::X => write!(f, "x")?,
            Var::Y => write!(f, "y")?
        }
        Ok(())
    }
}

/// An expression defining the coordinate to access an input image at.
#[derive(Debug, Clone, PartialEq, Eq)]
enum VarExpr {
    Var(Var),
    Const(i32),
    Add(Box<VarExpr>, Box<VarExpr>),
    Sub(Box<VarExpr>, Box<VarExpr>),
    Mul(Box<VarExpr>, Box<VarExpr>)
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
    };
}

impl_var_expr_bin_op!(Add, add, VarExpr::Add);
impl_var_expr_bin_op!(Sub, sub, VarExpr::Sub);
impl_var_expr_bin_op!(Mul, mul, VarExpr::Mul);

// We could also define static X and Y variables
const fn x() -> VarExpr { VarExpr::Var(Var::X) }
const fn y() -> VarExpr { VarExpr::Var(Var::Y) }

trait PrettyPrint {
    fn pretty_print(&self) -> String;
    fn is_leaf(&self) -> bool;
}

fn combine_with_op<P: PrettyPrint>(op: &str, left: &P, right: &P) -> String {
    let left = pretty_print_with_parens(left);
    let right = pretty_print_with_parens(right);
    format!("{} {} {}", left, op, right)
}

fn pretty_print_with_parens<P: PrettyPrint>(p: &P) -> String {
    let pp = p.pretty_print();
    if p.is_leaf() { pp } else { format!("({})", pp) }
}

impl<P: PrettyPrint> PrettyPrint for Box<P> {
    fn pretty_print(&self) -> String {
        (**self).pretty_print()
    }

    fn is_leaf(&self) -> bool {
        (**self).is_leaf()
    }
}

impl PrettyPrint for VarExpr {
    fn pretty_print(&self) -> String {
        match self {
            VarExpr::Var(v) => v.to_string(),
            VarExpr::Const(c) => c.to_string(),
            VarExpr::Add(l, r) => combine_with_op("+", l, r),
            VarExpr::Sub(l, r) => combine_with_op("-", l, r),
            VarExpr::Mul(l, r) => combine_with_op("*", l, r)
        }
    }

    fn is_leaf(&self) -> bool {
        match self {
            VarExpr::Var(_) | VarExpr::Const(_) => true,
            _ => false
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Access {
    /// The stage from which we're reading
    source: String,
    /// The x-coordinate to read from, in terms of
    /// variables X and Y.
    x: VarExpr,
    /// The y-coordinate to read from, in terms of
    /// variables X and Y.
    y: VarExpr
}

impl Access {
    fn new(source: &str, x: VarExpr, y: VarExpr) -> Access {
        let source = source.to_string();
        Access { source, x, y }
    }
}

impl PrettyPrint for Access {
    fn pretty_print(&self) -> String {
        format!("{}({}, {})", self.source, self.x.pretty_print(), self.y.pretty_print())
    }

    fn is_leaf(&self) -> bool {
        true
    }
}

/// An expression defining the value to set an image pixel to
#[derive(Debug, Clone)]
enum Definition {
    Access(Access),
    Const(i8),
    Add(Box<Definition>, Box<Definition>),
    Mul(Box<Definition>, Box<Definition>),
    Sub(Box<Definition>, Box<Definition>),
    Div(Box<Definition>, Box<Definition>)
}

macro_rules! impl_definition_bin_op {
    ($trait_name:ident, $trait_op:ident, $ctor:expr) => {
        impl $trait_name<Self> for Definition {
            type Output = Definition;
            fn $trait_op(self, rhs: Self) -> Definition {
                $ctor(Box::new(self), Box::new(rhs))
            }
        }

        impl $trait_name<i8> for Definition {
            type Output = Definition;
            fn $trait_op(self, rhs: i8) -> Definition {
                $ctor(Box::new(self), Box::new(Definition::Const(rhs)))
            }
        }

        impl $trait_name<Definition> for i8 {
            type Output = Definition;
            fn $trait_op(self, rhs: Definition) -> Definition {
                $ctor(Box::new(Definition::Const(self)), Box::new(rhs))
            }
        }
    };
}

impl_definition_bin_op!(Add, add, Definition::Add);
impl_definition_bin_op!(Sub, sub, Definition::Sub);
impl_definition_bin_op!(Mul, mul, Definition::Mul);
impl_definition_bin_op!(Div, div, Definition::Div);

fn read(source: &str, x: VarExpr, y: VarExpr) -> Definition {
    Definition::Access(Access::new(source, x, y))
}

impl PrettyPrint for Definition {
    fn pretty_print(&self) -> String {
        match self {
            Definition::Access(a) => a.pretty_print(),
            Definition::Const(c) => c.to_string(),
            Definition::Add(l, r) => combine_with_op("+", l, r),
            Definition::Sub(l, r) => combine_with_op("-", l, r),
            Definition::Mul(l, r) => combine_with_op("*", l, r),
            Definition::Div(l, r) => combine_with_op("/", l, r)
        }
    }

    fn is_leaf(&self) -> bool {
        match self {
            Definition::Access(_) | Definition::Const(_) => true,
            _ => false
        }
    }
}

struct Func {
    name: String,
    definition: Definition
}

impl Func {
    fn new(name: &str, definition: &Definition) -> Func {
        Func {
            name: name.to_string(),
            definition: definition.clone()
        }
    }
}

impl PrettyPrint for Func {
    fn pretty_print(&self) -> String {
        format!("{}(x, y) = {}", self.name, self.definition.pretty_print())
    }

    fn is_leaf(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_pretty_print(expr: VarExpr, expected: &str) {
        assert_eq!(expr.pretty_print(), expected);
    }

    #[test]
    fn test_var_expr_pretty_print() {
        assert_pretty_print(x(), "x");
        assert_pretty_print(y(), "y");
        assert_pretty_print(x() + y(), "x + y");
        assert_pretty_print(3 * (x() - 1), "3 * (x - 1)");
    }

    #[test]
    fn test_func_pretty_print() {
        // f(x, y) = g(x + 1, y - 1) + g(x - 1, y) + 2
        let d = read("g", x() + 1, y() - 1) + read("g", x() - 1, y()) + 2;
        let f = Func::new("f", &d);
        assert_eq!(f.pretty_print(), "f(x, y) = (g(x + 1, y - 1) + g(x - 1, y)) + 2");
    }
}