//! Provides a `Builder` type for constructing LLVM IR that wraps
//! the raw C API provided by llvm-sys. rustc has a more fully-featured
//! version of this (that's also generic over the backend - something
//! that we don't need here).
//! See https://github.com/rust-lang/rust/blob/master/src/librustc_codegen_llvm/builder.rs

use llvm::prelude::*;
use llvm::core::*;
use libc::c_char;

fn noname() -> *const c_char {
    static CNULL: c_char = 0;
    &CNULL
}

/// Creates a nul-terminated c string literal
#[macro_export]
macro_rules! c_str {
    ($s:expr) => { concat!($s, "\0").as_ptr() as *const _ };
}

pub struct Builder {
    // I need lifetimes to make this safe, and the functions
    // returning raw pointers should return references. But I'll
    // worry about that later...
    builder: LLVMBuilderRef,
    context: LLVMContextRef
}

impl Drop for Builder {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeBuilder(self.builder);
        }
    }
}

macro_rules! impl_llvm_type_getter {
    ($name:ident, $func:expr) => {
        pub fn $name(&self) -> LLVMTypeRef { unsafe { $func(self.context) } }
    };
}

impl Builder {
    pub fn new(context: LLVMContextRef) -> Builder {
        unsafe {
            Builder {
                builder: LLVMCreateBuilderInContext(context),
                context: context
            }
        }
    }

    impl_llvm_type_getter!(type_void, LLVMVoidTypeInContext);
    impl_llvm_type_getter!(type_i8, LLVMInt8TypeInContext);
    impl_llvm_type_getter!(type_i16, LLVMInt16TypeInContext);
    impl_llvm_type_getter!(type_i32, LLVMInt32TypeInContext);
    impl_llvm_type_getter!(type_i64, LLVMInt64TypeInContext);

    pub fn type_i8_ptr(&self) -> LLVMTypeRef {
        unsafe { LLVMPointerType(self.type_i8(), 0) }
    }

    pub fn func_type(&self, ret: LLVMTypeRef, args: &mut [LLVMTypeRef]) -> LLVMTypeRef {
        unsafe {
            const IS_VAR_ARG: LLVMBool = 0;
            LLVMFunctionType(ret, args.as_mut_ptr(), args.len() as u32, IS_VAR_ARG)
        }
    }

    pub fn add_func(
        &self,
        module: LLVMModuleRef,
        name: *const c_char,
        func_type: LLVMTypeRef
    ) -> LLVMValueRef {
        unsafe { LLVMAddFunction(module, name, func_type) }
    }

    pub fn new_block(
        &self,
        function: LLVMValueRef,
        name: *const c_char
    ) -> LLVMBasicBlockRef {
        unsafe {
            let block = LLVMAppendBasicBlockInContext(self.context, function, name);
            LLVMPositionBuilderAtEnd(self.builder, block);
            block
        }
    }

    pub fn add(&self, lhs: LLVMValueRef, rhs: LLVMValueRef) -> LLVMValueRef {
        unsafe {
            LLVMBuildAdd(self.builder, lhs, rhs, noname())
        }
    }

    pub fn get_params(&self, func: LLVMValueRef) -> Vec<LLVMValueRef> {
        unsafe {
            let num = LLVMCountParams(func);
            let mut params = vec![];
            for i in 0..num {
                params.push(LLVMGetParam(func, i));
            }
            params
        }
    }

    pub fn ret_void(&self) {
        unsafe { LLVMBuildRetVoid(self.builder); }
    }

    pub fn ret(&self, value: LLVMValueRef) {
        unsafe { LLVMBuildRet(self.builder, value); }
    }
}

