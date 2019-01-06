//! A trivial wrapper type for an LLVM execution engine

use std::{
    ffi::CString,
    marker::PhantomData,
    mem
};
use llvm_sys::execution_engine::*;
use crate::llvm::module::Module;

pub struct ExecutionEngine<'c> {
    // Need lifetimes to make this safe - need to tie
    // lifetime of execution engine (and everything else)
    // to the lifetime of the LLVM context we're using
    engine: LLVMExecutionEngineRef,
    // This module must not outlive the context of the module it generates code for.
    // This isn't totally clear from the LLVM docs (or at least it wasn't in the docs I found),
    // but I get a segfault if I try to use the engine after dropping the context.
    pub(in crate::llvm) context: PhantomData<&'c crate::llvm::Context>
}

impl<'c> ExecutionEngine<'c> {
    pub fn new(module: Module<'_>) -> ExecutionEngine<'_> {
        unsafe {
            let mut engine = mem::uninitialized();
            let mut out = mem::zeroed();
            LLVMCreateExecutionEngineForModule(&mut engine, module.module, &mut out);
            ExecutionEngine { engine, context: module.context }
        }
    }

    /// Unsafe as the returned address is only valid for the lifetime of
    /// this ExecutionEngine.
    pub unsafe fn get_func_addr(&self, name: &str) -> u64 {
        let name = CString::new(name).unwrap();
        LLVMGetFunctionAddress(self.engine, name.as_ptr())
    }
}

impl<'c> Drop for ExecutionEngine<'c> {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeExecutionEngine(self.engine);
        }
    }
}
