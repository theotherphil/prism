//! Functions for compiling LLVM IR to binary

use std::mem;
use llvm::prelude::*;
use llvm::core::*;
use llvm::execution_engine::*;
use llvm::target::*;
use llvm::ir_reader::*;
use llvm::transforms::pass_manager_builder::*;
use std::ffi::CString;

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

/// Call a function that returns an integer error code and panic
/// if the result is non-zero
macro_rules! c_try {
    ($f:expr, $message:expr) => { if $f() != 0 { panic!($message); } };
}

/// Do the global setup necessary to create execution engines which compile to native code
pub fn initialise_llvm_jit() {
    unsafe {
        LLVMLinkInMCJIT();
        c_try!(LLVM_InitializeNativeTarget, "Failed to initialise native target");
        c_try!(LLVM_InitializeNativeAsmPrinter, "Failed to initialise native assembly printer");
    }
}

pub struct ExecutionEngine {
    // Need lifetimes to make this safe - need to tie
    // lifetime of execution engine (and everything else)
    // to the lifetime of the LLVM context we're using
    engine: LLVMExecutionEngineRef
}

pub fn optimise(module: LLVMModuleRef) {
    unsafe {
        let pass_manager_builder = LLVMPassManagerBuilderCreate();
        LLVMPassManagerBuilderSetOptLevel(pass_manager_builder, 3 as ::libc::c_uint);
        LLVMPassManagerBuilderSetSizeLevel(pass_manager_builder, 0 as ::libc::c_uint);

        let pass_manager = LLVMCreatePassManager();
        LLVMPassManagerBuilderPopulateModulePassManager(pass_manager_builder, pass_manager);
        LLVMPassManagerBuilderDispose(pass_manager_builder);
        LLVMRunPassManager(pass_manager, module);
    }
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

pub fn create_module_from_handwritten_ir(context: &Context, ir: &str) -> LLVMModuleRef {
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

        module
    }
}