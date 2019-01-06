//! A trivial wrapper type for an LLVM context

use std::{
    ffi::CString,
    marker::PhantomData
};
use llvm_sys::{core::*, prelude::*};
use crate::llvm::*;

pub struct Context {
    pub(crate) context: LLVMContextRef
}

impl Context {
    pub fn new() -> Context {
        let context = unsafe { LLVMContextCreate() };
        Context { context }
    }

    pub fn new_module(&self, name: &str) -> Module<'_> {
        let module = unsafe {
            let name = CString::new(name).unwrap();
            LLVMModuleCreateWithNameInContext(name.as_ptr(), self.context)
        };
        Module { module, context: PhantomData }
    }

    pub unsafe fn wrap_llvm_module(&self, module: LLVMModuleRef) -> Module<'_> {
        Module { module, context: PhantomData }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { LLVMContextDispose(self.context); }
    }
}
