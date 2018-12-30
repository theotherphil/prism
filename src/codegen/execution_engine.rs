
use std::mem;
use llvm::prelude::*;
use llvm::execution_engine::*;
use std::ffi::CString;

pub struct ExecutionEngine {
    // Need lifetimes to make this safe - need to tie
    // lifetime of execution engine (and everything else)
    // to the lifetime of the LLVM context we're using
    engine: LLVMExecutionEngineRef
}

impl ExecutionEngine {
    pub fn new(module: LLVMModuleRef) -> ExecutionEngine {
        unsafe {
            let mut engine = mem::uninitialized();
            let mut out = mem::zeroed();
            LLVMCreateExecutionEngineForModule(&mut engine, module, &mut out);
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
