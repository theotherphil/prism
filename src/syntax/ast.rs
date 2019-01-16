
use std::{
    collections::HashMap,
    fmt
};
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

pub struct Schedule {
    /// Schedules indexed by function name.
    pub(crate) func_schedules: HashMap<String, FuncSchedule>
}

impl Schedule {
    pub fn new() -> Schedule {
        Schedule { func_schedules: HashMap::new() }
    }

    pub fn add_func(&mut self, func: &Func, sched: FuncSchedule) {
        self.func_schedules.insert(func.name.to_string(), sched);
    }

    pub fn add_source(&mut self, func: &Source, sched: FuncSchedule) {
        self.func_schedules.insert(func.name.to_string(), sched);
    }

    pub fn get_func_schedule(&self, func: &Func) -> &FuncSchedule {
        self.func_schedules.get(&func.name).unwrap()
    }

    pub fn get_source_schedule(&self, func: &Source) -> &FuncSchedule {
        self.func_schedules.get(&func.name).unwrap()
    }
}

// TODO: implement real schedules. 
// Need iteration order, compute location and storage
// location for each func. Compute location determins how a function's
// loops nest inside those of its callers, storage location determines
// the point in the loop nest where its storage is allocated, and iteration
// order defines the nesting order of its loops
pub struct FuncSchedule {
    // TODO support more than just X and Y!
    pub(crate) variables: Vec<Var>
}

impl FuncSchedule {
    /// By default the y variable is iterated in the outer loop
    pub fn by_row() -> FuncSchedule {
        FuncSchedule { variables: vec![Var::Y, Var::X] }
    }

    /// Iterates over the x variable in the outer loop
    pub fn by_column() -> FuncSchedule {
        FuncSchedule { variables: vec![Var::X, Var::Y] }
    }
}

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

impl VarExpr {
    pub fn evaluate(&self, x: i32, y: i32) -> i32 {
        match self {
            VarExpr::Var(v) => {
                match v {
                    Var::X => x,
                    Var::Y => y
                }
            },
            VarExpr::Const(c) => *c,
            VarExpr::Add(l, r) => l.evaluate(x, y) + r.evaluate(x, y),
            VarExpr::Sub(l, r) => l.evaluate(x, y) - r.evaluate(x, y),
            VarExpr::Mul(l, r) => l.evaluate(x, y) * r.evaluate(x, y)
        }
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Comparison {
    /// Left equal to right.
    EQ,
    /// Left strictly greater than right.
    GT,
    /// Left greater than or equal to right.
    GTE,
    /// Left strictly less than right.
    LT,
    /// Left less than or equal to right.
    LTE
}

impl PrettyPrint for Comparison {
    fn pretty_print(&self) -> String {
        let s = match *self {
            Comparison::EQ => "==",
            Comparison::GT => ">",
            Comparison::GTE => ">=",
            Comparison::LT => "<",
            Comparison::LTE => "<="
        };
        String::from(s)
    }

    fn is_leaf(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone)]
pub struct Condition {
    pub cmp: Comparison,
    pub lhs: Box<Definition>,
    pub rhs: Box<Definition>,
    pub if_true: Box<Definition>,
    pub if_false: Box<Definition>
}

impl Condition {
    pub fn new(
        cmp: Comparison,
        lhs: Definition,
        rhs: Definition,
        if_true: Definition,
        if_false: Definition
    ) -> Condition {
        Condition {
            cmp,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            if_true: Box::new(if_true),
            if_false: Box::new(if_false)
        }
    }
}

/// An expression defining the value to set an image pixel to
#[derive(Debug, Clone)]
pub enum Definition {
    Access(Access),
    // All intermediate calculations happen at type i32 for now
    Const(i32),
    Param(String),
    Cond(Condition),
    // TODO: share code for printing and lowering arithmetic expressions
    // TODO: between VarExpr and Definition
    Add(Box<Definition>, Box<Definition>),
    Mul(Box<Definition>, Box<Definition>),
    Sub(Box<Definition>, Box<Definition>),
    Div(Box<Definition>, Box<Definition>)
}

fn sources(definitions: &[&Box<Definition>]) -> Vec<String> {
    let mut sources = vec![];
    for definition in definitions {
        sources.extend(definition.sources().into_iter());
    }
    sources
}

fn params(definitions: &[&Box<Definition>]) -> Vec<String> {
    let mut params = vec![];
    for definition in definitions {
        params.extend(definition.params().into_iter());
    }
    params
}

impl Definition {
    pub(crate) fn sources(&self) -> Vec<String> {
        match self {
            Definition::Access(a) => vec![a.source.clone()],
            Definition::Const(_) => vec![],
            Definition::Param(_) => vec![],
            Definition::Cond(c) => sources(&vec![&c.lhs, &c.rhs, &c.if_true, &c.if_false]),
            Definition::Add(l, r) => sources(&vec![l, r]),
            Definition::Mul(l, r) => sources(&vec![l, r]),
            Definition::Sub(l, r) => sources(&vec![l, r]),
            Definition::Div(l, r) => sources(&vec![l, r]),
        }
    }

    pub(crate) fn params(&self) -> Vec<String> {
        match self {
            Definition::Access(_) => vec![],
            Definition::Const(_) => vec![],
            Definition::Param(p) => vec![p.clone()],
            Definition::Cond(c) => params(&vec![&c.lhs, &c.rhs, &c.if_true, &c.if_false]),
            Definition::Add(l, r) => params(&vec![l, r]),
            Definition::Mul(l, r) => params(&vec![l, r]),
            Definition::Sub(l, r) => params(&vec![l, r]),
            Definition::Div(l, r) => params(&vec![l, r]),
        }
    }
}

impl PrettyPrint for Definition {
    fn pretty_print(&self) -> String {
        match self {
            Definition::Access(a) => a.pretty_print(),
            Definition::Const(c) => c.to_string(),
            Definition::Param(p) => p.clone(),
            Definition::Cond(c) => {
                let l = pretty_print_with_parens(&*c.lhs);
                let op = pretty_print_with_parens(&c.cmp);
                let r = pretty_print_with_parens(&*c.rhs);
                let t = pretty_print_with_parens(&*c.if_true);
                let f = pretty_print_with_parens(&*c.if_false);
                format!("if {} {} {} {{{}}} else {{{}}}", l, op, r, t, f)
            },
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