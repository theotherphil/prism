//! A trivial wrapper type for an LLVM context

use std::ffi::CString;
use llvm_sys::{core::*, prelude::*};

pub struct Context {
    // TODO: lifetimes
    pub(crate) context: LLVMContextRef
}

impl Context {
    pub fn new() -> Context {
        let context = unsafe { LLVMContextCreate() };
        Context { context }
    }

    pub fn new_module(&self, name: &str) -> LLVMModuleRef {
        unsafe {
            let name = CString::new(name).unwrap();
            LLVMModuleCreateWithNameInContext(name.as_ptr(), self.context)
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { LLVMContextDispose(self.context); }
    }
}
