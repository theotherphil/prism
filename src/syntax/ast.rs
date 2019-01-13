
use std::fmt;
use crate::syntax::pretty_print::*;

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
pub enum Var { X, Y }

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
pub enum VarExpr {
    Var(Var),
    Const(i32),
    Add(Box<VarExpr>, Box<VarExpr>),
    Sub(Box<VarExpr>, Box<VarExpr>),
    Mul(Box<VarExpr>, Box<VarExpr>)
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

/// A runtime parameter to a function of type i32.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Param {
    pub name: String
}

impl Param {
    pub fn new(name: &str) -> Param {
        Param { name: name.to_string() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Access {
    /// The stage from which we're reading
    pub(crate) source: String,
    /// The x-coordinate to read from, in terms of
    /// variables X and Y.
    pub(crate) x: VarExpr,
    /// The y-coordinate to read from, in terms of
    /// variables X and Y.
    pub(crate) y: VarExpr
}

impl Access {
    pub fn new(source: &str, x: VarExpr, y: VarExpr) -> Access {
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
pub enum Definition {
    Access(Access),
    // All intermediate calculations happen at type i32 for now
    Const(i32),
    Param(String),
    // TODO: share code for printing and lowering arithmetic expressions
    // TODO: between VarExpr and Definition
    Add(Box<Definition>, Box<Definition>),
    Mul(Box<Definition>, Box<Definition>),
    Sub(Box<Definition>, Box<Definition>),
    Div(Box<Definition>, Box<Definition>)
}

impl Definition {
    pub(crate) fn sources(&self) -> Vec<String> {
        match self {
            Definition::Access(a) => vec![a.source.clone()],
            Definition::Const(_) => vec![],
            Definition::Param(_) => vec![],
            Definition::Add(l, r) => l.sources().into_iter().chain(r.sources()).collect(),
            Definition::Mul(l, r) => l.sources().into_iter().chain(r.sources()).collect(),
            Definition::Sub(l, r) => l.sources().into_iter().chain(r.sources()).collect(),
            Definition::Div(l, r) => l.sources().into_iter().chain(r.sources()).collect(),
        }
    }

    pub(crate) fn params(&self) -> Vec<String> {
        match self {
            Definition::Access(_) => vec![],
            Definition::Const(_) => vec![],
            Definition::Param(p) => vec![p.clone()],
            Definition::Add(l, r) => l.params().into_iter().chain(r.params()).collect(),
            Definition::Mul(l, r) => l.params().into_iter().chain(r.params()).collect(),
            Definition::Sub(l, r) => l.params().into_iter().chain(r.params()).collect(),
            Definition::Div(l, r) => l.params().into_iter().chain(r.params()).collect(),
        }
    }
}

impl PrettyPrint for Definition {
    fn pretty_print(&self) -> String {
        match self {
            Definition::Access(a) => a.pretty_print(),
            Definition::Const(c) => c.to_string(),
            Definition::Param(p) => p.clone(),
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

/// An image provided as an input
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub name: String
}

impl Source {
    pub fn new(name: &str) -> Source {
        Source { name: name.to_string() }
    }

    pub fn at<U, V>(&self, x: U, y: V) -> Definition
    where
        U: Into<VarExpr>,
        V: Into<VarExpr>
    {
        Definition::Access(Access::new(&self.name, x.into(), y.into()))
    }
}


#[derive(Debug, Clone)]
pub struct Func {
    pub(crate) name: String,
    pub(crate) definition: Definition
}

impl Func {
    pub fn new(name: &str, definition: Definition) -> Func {
        Func {
            name: name.to_string(),
            definition: definition
        }
    }

    /// Returns the name of all the sources mentioned
    /// in this func's definition.
    pub fn sources(&self) -> Vec<String> {
        self.definition.sources()
    }

    /// Returns the names of all the params mentioned
    /// in this func's definition.
    pub fn params(&self) -> Vec<String> {
        self.definition.params()
    }

    pub fn at<U, V>(&self, x: U, y: V) -> Definition
    where
        U: Into<VarExpr>,
        V: Into<VarExpr>
    {
        Definition::Access(Access::new(&self.name, x.into(), y.into()))
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

    fn assert_pretty_print<V: Into<VarExpr>>(expr: V, expected: &str) {
        let expr: VarExpr = expr.into();
        assert_eq!(expr.pretty_print(), expected);
    }

    #[test]
    fn test_var_expr_pretty_print() {
        let (x, y) = (Var::X, Var::Y);
        assert_pretty_print(x, "x");
        assert_pretty_print(y, "y");
        assert_pretty_print(x + y, "x + y");
        assert_pretty_print(3 * (x - 1), "3 * (x - 1)");
    }

    #[test]
    fn test_func_pretty_print() {
        let (x, y) = (Var::X, Var::Y);
        // f(x, y) = g(x + 1, y - 1) + g(x - 1, y) + 2
        let g = Source::new("g");
        let f = Func::new(
            "f",
            g.at(x + 1, y - 1) + g.at(x - 1, y) + 2
        );
        assert_eq!(f.pretty_print(), "f(x, y) = (g(x + 1, y - 1) + g(x - 1, y)) + 2");
    }
}