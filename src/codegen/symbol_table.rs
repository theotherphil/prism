//! A simple wrapper for a HashMap that maps names of symbols to an LLVMValueRef for their value.

use std::collections::HashMap;
use llvm_sys::prelude::LLVMValueRef;

#[derive(Debug)]
pub struct SymbolTable {
    symbols: HashMap<String, LLVMValueRef>
}

impl SymbolTable {
    pub fn new() -> SymbolTable {
        SymbolTable { symbols: HashMap::new() }
    }

    pub fn add(&mut self, name: &str, value: LLVMValueRef) {
        self.symbols.insert(name.to_string(), value);
    }

    pub fn remove(&mut self, name: &str) {
        match self.symbols.remove(name) {
            None => panic!("Remove failed - symbol {} not found", name),
            _ => {}
        };
    }

    pub fn get(&self, name: &str) -> LLVMValueRef {
        match self.symbols.get(name) {
            Some(v) => *v,
            None => panic!("Get failed - symbol {} not found", name)
        }
    }
}
