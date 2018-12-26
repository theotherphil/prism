extern crate llvm_sys as llvm;

use std::mem;

use llvm::prelude::*;
use llvm::core::*;
use llvm::execution_engine::*;
use llvm::target::*;
use llvm::ir_reader::*;

use std::ffi::CString;

/// Do the global setup necessary to create execution engines which compile to native code
fn initialise_jit() {
    unsafe {
        LLVMLinkInMCJIT();
        let res = LLVM_InitializeNativeTarget();
        if res != 0 {
            panic!("Failed to initialise native target");
        }
        let res = LLVM_InitializeNativeAsmPrinter();
        if res != 0 {
            panic!("Failed to initialise native assembly printer");
        }
    }
}

fn create_module_from_handwritten_ir(context: LLVMContextRef, ir: &str) -> LLVMModuleRef {
    unsafe {
        let ir = CString::new(ir).unwrap();

        let ir_buffer = LLVMCreateMemoryBufferWithMemoryRange(
            ir.as_ptr(), ir.as_bytes_with_nul().len(), std::ptr::null(), 1);

        let mut module = mem::uninitialized();
        let mut message = mem::zeroed();
        let res = LLVMParseIRInContext(context, ir_buffer, &mut module, &mut message);

        if res != 0 {
            let message_str = CString::from_raw(message);
            panic!("IR parsing failed: {:?}", message_str);
        }

        module
    }
}

fn create_sum_module_from_handwritten_ir(context: LLVMContextRef) -> LLVMModuleRef {
    let ir = "define i64 @sum(i64, i64, i64) {
entry:
    %sum.1 = add i64 %0, %1
    %sum.2 = add i64 %sum.1, %2
    ret i64 %sum.2
}";
    create_module_from_handwritten_ir(context, ir)
}

fn create_sum_module_via_builder(context: LLVMContextRef) -> LLVMModuleRef {
    unsafe {
        let module = LLVMModuleCreateWithNameInContext(b"sum\0".as_ptr() as *const _, context);
        let builder = LLVMCreateBuilderInContext(context);

        // get a type for sum function
        let i64t = LLVMInt64TypeInContext(context);
        let mut argts = [i64t, i64t, i64t];
        let function_type = LLVMFunctionType(i64t, argts.as_mut_ptr(), argts.len() as u32, 0);

        // add it to our module
        let function = LLVMAddFunction(module, b"sum\0".as_ptr() as *const _, function_type);

        // Create a basic block in the function and set our builder to generate
        // code in it.
        let bb = LLVMAppendBasicBlockInContext(context, function, b"entry\0".as_ptr() as *const _);

        LLVMPositionBuilderAtEnd(builder, bb);

        // get the function's arguments
        let x = LLVMGetParam(function, 0);
        let y = LLVMGetParam(function, 1);
        let z = LLVMGetParam(function, 2);

        let sum = LLVMBuildAdd(builder, x, y, b"sum.1\0".as_ptr() as *const _);
        let sum = LLVMBuildAdd(builder, sum, z, b"sum.2\0".as_ptr() as *const _);

        // Emit a `ret void` into the function
        LLVMBuildRet(builder, sum);

        // done building
        LLVMDisposeBuilder(builder);

        module
    }
}

fn jit_sum_example(definition_source: SumDefinition) {
    unsafe {
        // Set up a context, module and builder in that context.
        let context = LLVMContextCreate();
        let module = match definition_source {
            SumDefinition::Handwritten => create_sum_module_from_handwritten_ir(context),
            SumDefinition::Jit => create_sum_module_via_builder(context)
        };

        // Dump the module as IR to stdout.
        LLVMDumpModule(module);

        // build an execution engine
        let mut ee = mem::uninitialized();
        let mut out = mem::zeroed();

        // takes ownership of the module
        LLVMCreateExecutionEngineForModule(&mut ee, module, &mut out);

        let addr = LLVMGetFunctionAddress(ee, b"sum\0".as_ptr() as *const _);

        let f: extern "C" fn(u64, u64, u64) -> u64 = mem::transmute(addr);

        let (x, y, z) = (1, 1, 1);
        let res = f(x, y, z);

        println!("{} + {} + {} = {}", x, y, z, res);

        // Clean up the rest.
        LLVMDisposeExecutionEngine(ee);
        LLVMContextDispose(context);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum SumDefinition {
    Handwritten,
    Jit
}

use structopt::StructOpt;

fn sum_definition_from_str(d: &str) -> SumDefinition {
    match d {
        "hand" => SumDefinition::Handwritten,
        "jit" => SumDefinition::Jit,
        _ => panic!("invalid SumDefinition variant")
    }
}

#[derive(StructOpt, Debug)]
struct Opts {
    /// Example files are written to this directory.
    #[structopt(short = "s", long = "definition_source")]
    definition_source: String
}

fn main() {
    let opts = Opts::from_args();
    let definition_type = sum_definition_from_str(&opts.definition_source);

    initialise_jit();
    jit_sum_example(definition_type);
}

