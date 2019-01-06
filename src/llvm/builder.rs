//! Provides a `Builder` type for constructing LLVM IR that wraps
//! the raw C API provided by llvm-sys. rustc has a more fully-featured
//! version of this (that's also generic over the backend - something
//! that we don't need here).
//! See https://github.com/rust-lang/rust/blob/master/src/librustc_codegen_llvm/builder.rs

use llvm_sys::{
    *,
    prelude::*,
    core::*,
    support::*
};
use std::mem;
use std::ptr;
use libc::{c_char, c_void};
use std::ffi::CString;
use crate::*;

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

macro_rules! impl_icmp {
    ($name:ident, $op:ident) => {
        pub fn $name(&self, lhs: LLVMValueRef, rhs: LLVMValueRef) -> LLVMValueRef {
            unsafe {
                LLVMBuildICmp(self.builder, LLVMIntPredicate::$op, lhs, rhs, noname())
            }
        }
    };
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

    impl_llvm_binary_op!(add, LLVMBuildAdd);
    impl_llvm_binary_op!(add_nsw, LLVMBuildNSWAdd);
    impl_llvm_binary_op!(mul, LLVMBuildMul);
    impl_llvm_binary_op!(sub, LLVMBuildSub);
    impl_llvm_binary_op!(sdiv, LLVMBuildSDiv);
    impl_llvm_binary_op!(and, LLVMBuildAnd);
    impl_llvm_binary_op!(or, LLVMBuildOr);
    impl_llvm_binary_op!(xor, LLVMBuildXor);

    impl_icmp!(icmp_eq, LLVMIntEQ);
    impl_icmp!(icmp_ne, LLVMIntNE);
    impl_icmp!(icmp_ugt, LLVMIntUGT);
    impl_icmp!(icmp_uge, LLVMIntUGE);
    impl_icmp!(icmp_ult, LLVMIntULT);
    impl_icmp!(icmp_ule, LLVMIntULE);
    impl_icmp!(icmp_sgt, LLVMIntSGT);
    impl_icmp!(icmp_sge, LLVMIntSGE);
    impl_icmp!(icmp_slt, LLVMIntSLT);
    impl_icmp!(icmp_sle, LLVMIntSLE);

    pub fn const_i32(&self, value: i32) -> LLVMValueRef {
        unsafe {
            const SIGN_EXTEND: LLVMBool = 0;
            LLVMConstInt(self.type_i32(), value as ::libc::c_ulonglong, SIGN_EXTEND)
        }
    }

    pub fn const_i64(&self, value: i64) -> LLVMValueRef {
        unsafe {
            const SIGN_EXTEND: LLVMBool = 0;
            LLVMConstInt(self.type_i64(), value as ::libc::c_ulonglong, SIGN_EXTEND)
        }
    }

    pub fn const_i8(&self, value: i8) -> LLVMValueRef {
        unsafe {
            const SIGN_EXTEND: LLVMBool = 0;
            LLVMConstInt(self.type_i8(), value as ::libc::c_ulonglong, SIGN_EXTEND)
        }
    }

    pub fn const_string(&self, value: &str) -> LLVMValueRef {
        unsafe {
            let value = CString::new(value).unwrap();
            LLVMConstStringInContext(
                self.context,
                value.as_ptr(),
                value.as_bytes_with_nul().len() as u32,
                1
            )
        }
    }

    pub fn const_null(&self, ty: LLVMTypeRef) -> LLVMValueRef {
        unsafe { LLVMConstNull(ty) }
    }

    pub fn global_string(&self, value: &str, name: &str) -> LLVMValueRef {
        unsafe {
            let value = CString::new(value).unwrap();
            let name = CString::new(name).unwrap();
            LLVMBuildGlobalStringPtr(self.builder, value.as_ptr(), name.as_ptr())
        }
    }

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

    pub fn alloca(&self, ty: LLVMTypeRef, align: u32) -> LLVMValueRef {
        self.named_alloca(ty, "", align)
    }

    pub fn named_alloca(&self, ty: LLVMTypeRef, name: &str, align: u32) -> LLVMValueRef {
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

    pub fn in_bounds_gep(&self, ptr: LLVMValueRef, offset: LLVMValueRef) -> LLVMValueRef {
        unsafe {
            let mut indices = [offset];
            LLVMBuildInBoundsGEP(self.builder, ptr, indices.as_mut_ptr(), 1, noname())
        }
    }

    pub fn sext(&self, val: LLVMValueRef, dest_ty: LLVMTypeRef) -> LLVMValueRef {
        unsafe {
            LLVMBuildSExt(self.builder, val, dest_ty, noname())
        }
    }

    pub fn zext(&self, val: LLVMValueRef, dest_ty: LLVMTypeRef) -> LLVMValueRef {
        unsafe {
            LLVMBuildZExt(self.builder, val, dest_ty, noname())
        }
    }

    pub fn build_function_call(
        &self,
        func: LLVMValueRef,
        args: &mut[LLVMValueRef]
    ) -> LLVMValueRef {
        unsafe {
            LLVMBuildCall(self.builder, func, args.as_mut_ptr(), args.len() as u32, noname())
        }
    }

    pub fn add_symbol(&self, name: &str, ptr: *const ()) {
        unsafe {
            let name = CString::new(name).unwrap();
            let addr = ptr as *mut c_void;
            LLVMAddSymbol(name.as_ptr(), addr);
        }
    }

    pub fn address_of_symbol(&self, symbol: &str) -> Option<u64> {
        unsafe {
            let symbol = CString::new(symbol).unwrap();
            let addr = LLVMSearchForAddressOfSymbol(symbol.as_ptr());
            if addr == ptr::null_mut() {
                return None;
            }
            let addr = addr as *mut u64;
            Some(mem::transmute(*addr))
        }
    }
}
