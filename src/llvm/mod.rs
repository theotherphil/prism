//! Wrapper types for the LLVM bindings in llvm-sys

pub use self::builder::*;
pub use self::context::*;
pub use self::execution_engine::*;
pub use self::module::*;

mod builder;
mod context;
mod execution_engine;
mod module;

/// Do the global setup necessary to create execution engines which compile to native code
pub fn initialise_llvm_jit() {
    unsafe {
        llvm_sys::execution_engine::LLVMLinkInMCJIT();
        if llvm_sys::target::LLVM_InitializeNativeTarget() != 0 {
            panic!("Failed to initialise native target");
        }
        if llvm_sys::target::LLVM_InitializeNativeAsmPrinter() != 0 {
            panic!("Failed to initialise native assembly printer");
        }
    }
}

/// Parse a string containing a textual representation of an IR module into an in-memory module.
pub fn create_module_from_ir_string(context: &Context, ir: &str) -> Module {
    use std::{ffi::CString, mem};
    use llvm_sys::{
        core::LLVMCreateMemoryBufferWithMemoryRange,
        ir_reader::LLVMParseIRInContext
    };

    unsafe {
        let ir = CString::new(ir).unwrap();

        let ir_buffer = LLVMCreateMemoryBufferWithMemoryRange(
            ir.as_ptr(), ir.as_bytes_with_nul().len(), std::ptr::null(), 1);

        let mut module = mem::uninitialized();
        let mut message = mem::zeroed();
        let res = LLVMParseIRInContext(context.context, ir_buffer, &mut module, &mut message);

        if res != 0 {
            let message_str = CString::from_raw(message);
            panic!("IR parsing failed: {:?}", message_str);
        }

        Module::new(module)
    }
}

pub fn optimise(module: &mut Module) {
    use llvm_sys::{core::*, transforms::pass_manager_builder::*};

    unsafe {
        let pass_manager_builder = LLVMPassManagerBuilderCreate();
        LLVMPassManagerBuilderSetOptLevel(pass_manager_builder, 3 as ::libc::c_uint);
        LLVMPassManagerBuilderSetSizeLevel(pass_manager_builder, 0 as ::libc::c_uint);

        let pass_manager = LLVMCreatePassManager();
        LLVMPassManagerBuilderPopulateModulePassManager(pass_manager_builder, pass_manager);
        LLVMPassManagerBuilderDispose(pass_manager_builder);
        LLVMRunPassManager(pass_manager, module.module);
    }
}