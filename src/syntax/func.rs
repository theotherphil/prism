
use crate::syntax::{ast::*, pretty_print::*};

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
