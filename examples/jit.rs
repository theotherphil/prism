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

fn create_loop1_module_from_handwritten_ir(context: LLVMContextRef) -> LLVMModuleRef {
    let ir =
"define void @loop1(i8* nocapture readonly %src, i8* nocapture %dst, i64 %len)
{
entry:
  %i = alloca i32, align 4
  %bound = trunc i64 %len to i32
  store i32 0, i32* %i, align 4
  br label %for.cond

for.cond:
  %0 = load i32, i32* %i, align 4
  %cmp = icmp slt i32 %0, %bound
  br i1 %cmp, label %for.body, label %for.end

for.body:
  %1 = load i32, i32* %i, align 4
  %sidx = getelementptr i8, i8* %src, i32 %1
  %didx = getelementptr i8, i8* %dst, i32 %1
  %val = load i8, i8* %sidx
  %upd = add i8 %val, 3
  store i8 %upd, i8* %didx
  br label %for.inc

for.inc:
  %2 = load i32, i32* %i, align 4
  %inc = add nsw i32 %2, 1
  store i32 %inc, i32* %i, align 4
  br label %for.cond

for.end:
  ret void
}";
    create_module_from_handwritten_ir(context, ir)
}

fn loop_1d_example() {
    unsafe {
        let context = LLVMContextCreate();
        let module = create_loop1_module_from_handwritten_ir(context);
        let mut ee = mem::uninitialized();
        let mut out = mem::zeroed();
        println!("Creating execution engine from module");
        LLVMCreateExecutionEngineForModule(&mut ee, module, &mut out);
        println!("Created execution engine from module");
        let addr = LLVMGetFunctionAddress(ee, b"loop1\0".as_ptr() as *const _);
        let f: extern "C" fn(*const u8, *mut u8, usize) = mem::transmute(addr);
        let x = [1, 2, 3];
        let mut y = [0u8; 3];
        f(x.as_ptr(), y.as_mut_ptr(), x.len());
        println!("map(+1, {:?}) = {:?}", x, y);
        LLVMDisposeExecutionEngine(ee);
        LLVMContextDispose(context);
    }
}

fn jit_sum_example(variation: Variation) {
    unsafe {
        let context = LLVMContextCreate();
        let module = match variation {
            Variation::Handwritten => create_sum_module_from_handwritten_ir(context),
            Variation::Jit => create_sum_module_via_builder(context),
            _ => panic!("Nope")
        };
        // Dump the module as IR to stdout.
        LLVMDumpModule(module);

        let mut ee = mem::uninitialized();
        let mut out = mem::zeroed();
        LLVMCreateExecutionEngineForModule(&mut ee, module, &mut out);

        let addr = LLVMGetFunctionAddress(ee, b"sum\0".as_ptr() as *const _);
        let f: extern "C" fn(u64, u64, u64) -> u64 = mem::transmute(addr);
        let (x, y, z) = (1, 1, 1);
        let res = f(x, y, z);
        println!("{} + {} + {} = {}", x, y, z, res);

        LLVMDisposeExecutionEngine(ee);
        LLVMContextDispose(context);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Variation {
    Handwritten,
    Jit,
    Loop1d
}

use structopt::StructOpt;

fn variation_from_str(d: &str) -> Variation {
    match d {
        "hand" => Variation::Handwritten,
        "jit" => Variation::Jit,
        "1d" => Variation::Loop1d,
        _ => panic!("invalid Variation variant")
    }
}

#[derive(StructOpt, Debug)]
struct Opts {
    #[structopt(short = "v", long = "variation")]
    variation: String
}

fn main() {
    initialise_jit();
    
    let opts = Opts::from_args();
    let variation = variation_from_str(&opts.variation);
    match variation {
        Variation::Loop1d => loop_1d_example(),
        _ => jit_sum_example(variation)
    }
}

