//! Provides a `Builder` type for constructing LLVM IR that wraps
//! the raw C API provided by llvm-sys. rustc has a more fully-featured
//! version of this (that's also generic over the backend - something
//! that we don't need here).
//! See https://github.com/rust-lang/rust/blob/master/src/librustc_codegen_llvm/builder.rs

use llvm::prelude::*;
use llvm::core::*;
use llvm::*;
use libc::c_char;
use std::ffi::CString;
use crate::codegen::compile::*;

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

macro_rules! impl_llvm_binary_op {
    ($name:ident, $func:expr) => {
        pub fn $name(&self, lhs: LLVMValueRef, rhs: LLVMValueRef) -> LLVMValueRef {
            unsafe {
                $func(self.builder, lhs, rhs, noname())
            }
        }
    }
}

impl Builder {
    pub fn new(context: &Context) -> Builder {
        unsafe {
            Builder {
                builder: LLVMCreateBuilderInContext(context.context),
                context: context.context
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

    impl_llvm_binary_op!(add, LLVMBuildAdd);
    impl_llvm_binary_op!(add_nsw, LLVMBuildNSWAdd);
    impl_llvm_binary_op!(mul, LLVMBuildMul);
    impl_llvm_binary_op!(sub, LLVMBuildSub);
    impl_llvm_binary_op!(sdiv, LLVMBuildSDiv);

    pub fn func_type(&self, ret: LLVMTypeRef, args: &mut [LLVMTypeRef]) -> LLVMTypeRef {
        unsafe {
            const IS_VAR_ARG: LLVMBool = 0;
            LLVMFunctionType(ret, args.as_mut_ptr(), args.len() as u32, IS_VAR_ARG)
        }
    }

    pub fn add_func(
        &self,
        module: LLVMModuleRef,
        name: &str,
        func_type: LLVMTypeRef
    ) -> LLVMValueRef {
        unsafe {
            let name = CString::new(name).unwrap();
            LLVMAddFunction(module, name.as_ptr(), func_type)
        }
    }

    pub fn new_block(
        &self,
        function: LLVMValueRef,
        name: &str
    ) -> LLVMBasicBlockRef {
        unsafe {
            let name = CString::new(name).unwrap();
            let block = LLVMAppendBasicBlockInContext(self.context, function, name.as_ptr());
            LLVMPositionBuilderAtEnd(self.builder, block);
            block
        }
    }

    pub fn position_at_end(&self, block: LLVMBasicBlockRef) {
        unsafe { LLVMPositionBuilderAtEnd(self.builder, block); }
    }

    pub fn get_insert_block(&self) -> LLVMBasicBlockRef {
        unsafe { LLVMGetInsertBlock(self.builder) }
    }

    pub fn build_phi(&self, ty: LLVMTypeRef, name: &str) -> LLVMValueRef {
        unsafe {
            let name = CString::new(name).unwrap();
            LLVMBuildPhi(self.builder, ty, name.as_ptr())
        }
    }

    pub fn add_phi_incoming(
        &self,
        phi: LLVMValueRef,
        incoming_value: LLVMValueRef,
        incoming_block: LLVMBasicBlockRef
    ) {
        unsafe {
            let mut values = [incoming_value];
            let mut blocks = [incoming_block];
            LLVMAddIncoming(phi, values.as_mut_ptr(), blocks.as_mut_ptr(), 1);
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

    pub fn store(&self, value: LLVMValueRef, ptr: LLVMValueRef, align: u32) -> LLVMValueRef {
        unsafe {
            let s = LLVMBuildStore(self.builder, value, ptr);
            LLVMSetAlignment(s, align);
            s
        }
    }

    pub fn load(&self, ptr: LLVMValueRef, align: u32) -> LLVMValueRef {
        unsafe {
            let l = LLVMBuildLoad(self.builder, ptr, noname());
            LLVMSetAlignment(l, align);
            l
        }
    }

    pub fn const_i32(&self, value: i32) -> LLVMValueRef {
        unsafe {
            const SIGN_EXTEND: LLVMBool = 0;
            LLVMConstInt(self.type_i32(), value as ::libc::c_ulonglong, SIGN_EXTEND)
        }
    }

    pub fn const_i8(&self, value: i8) -> LLVMValueRef {
        unsafe {
            const SIGN_EXTEND: LLVMBool = 0;
            LLVMConstInt(self.type_i8(), value as ::libc::c_ulonglong, SIGN_EXTEND)
        }
    }

    pub fn alloca(&self, ty: LLVMTypeRef, name: &str, align: u32) -> LLVMValueRef {
        unsafe {
            let name = CString::new(name).unwrap();
            let a = LLVMBuildAlloca(self.builder, ty, name.as_ptr());
            LLVMSetAlignment(a, align);
            a
        }
    }

    pub fn br(&self, block: LLVMBasicBlockRef) -> LLVMValueRef {
        unsafe {
            LLVMBuildBr(self.builder, block)
        }
    }

    pub fn cond_br(
        &self,
        cond: LLVMValueRef,
        then_block: LLVMBasicBlockRef,
        else_block: LLVMBasicBlockRef
    ) -> LLVMValueRef {
        unsafe {
            LLVMBuildCondBr(self.builder, cond, then_block, else_block)
        }
    }

    pub fn trunc(&self, value: LLVMValueRef, ty: LLVMTypeRef) -> LLVMValueRef {
        unsafe {
            LLVMBuildTrunc(self.builder, value, ty, noname())
        }
    }

    pub fn icmp(&self, op: LLVMIntPredicate, lhs: LLVMValueRef, rhs: LLVMValueRef) -> LLVMValueRef {
        unsafe {
            LLVMBuildICmp(self.builder, op, lhs, rhs, noname())
        }
    }

    pub fn in_bounds_gep(&self, ptr: LLVMValueRef, offset: LLVMValueRef) -> LLVMValueRef {
        unsafe {
            let mut indices = [offset];
            LLVMBuildInBoundsGEP(self.builder, ptr, indices.as_mut_ptr(), 1, noname())
        }
    }
}
