//! A trivial wrapper type for an LLVM execution engine

use std::{ffi::CString, mem};
use llvm_sys::execution_engine::*;
use crate::llvm::module::Module;

pub struct ExecutionEngine {
    // Need lifetimes to make this safe - need to tie
    // lifetime of execution engine (and everything else)
    // to the lifetime of the LLVM context we're using
    engine: LLVMExecutionEngineRef
}

impl ExecutionEngine {
    pub fn new(module: Module) -> ExecutionEngine {
        unsafe {
            let mut engine = mem::uninitialized();
            let mut out = mem::zeroed();
            LLVMCreateExecutionEngineForModule(&mut engine, module.module, &mut out);
            ExecutionEngine { engine }
        }
    }

    pub fn get_func_addr(&self, name: &str) -> u64 {
        unsafe {
            let name = CString::new(name).unwrap();
            LLVMGetFunctionAddress(self.engine, name.as_ptr())
        }
    }
}

impl Drop for ExecutionEngine {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeExecutionEngine(self.engine);
        }
    }
}
