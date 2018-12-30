extern crate llvm_sys as llvm;

use std::mem;
use llvm::*;
use llvm::prelude::*;
use llvm::core::*;
use llvm::execution_engine::*;
use llvm::target::*;
use llvm::ir_reader::*;
use std::ffi::CString;
use prism::*;
use prism::builder::*;

/// Call a function that returns an integer error code and panic
/// if the result is non-zero
macro_rules! c_try {
    ($f:expr, $message:expr) => { if $f() != 0 { panic!($message); } };
}

/// Do the global setup necessary to create execution engines which compile to native code
fn initialise_jit() {
    unsafe {
        LLVMLinkInMCJIT();
        c_try!(LLVM_InitializeNativeTarget, "Failed to initialise native target");
        c_try!(LLVM_InitializeNativeAsmPrinter, "Failed to initialise native assembly printer");
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

const SUM_IR: &str = "define i64 @sum(i64, i64, i64) {
entry:
    %sum.1 = add i64 %0, %1
    %sum.2 = add i64 %sum.1, %2
    ret i64 %sum.2
}";

fn create_sum_module_via_builder(context: LLVMContextRef) -> LLVMModuleRef {
    unsafe {
        let module = LLVMModuleCreateWithNameInContext(c_str!("sum"), context);
        let builder = Builder::new(context);
        let i64t = builder.type_i64();
        let function_type = builder.func_type(i64t, &mut [i64t, i64t, i64t]);
        let function = builder.add_func(module, c_str!("sum"), function_type);
        let _ = builder.new_block(function, c_str!("entry"));
        let params = builder.get_params(function);
        let (x, y, z) = (params[0], params[1], params[2]);
        let sum = builder.add(x, y);
        let sum = builder.add(sum, z);
        builder.ret(sum);
        module
    }
}

fn create_process_image_module_via_builder(context: LLVMContextRef) -> LLVMModuleRef {
    unsafe {
        let module = LLVMModuleCreateWithNameInContext(c_str!("process_image"), context);
        let builder = LLVMCreateBuilderInContext(context);

        let voidt = LLVMVoidTypeInContext(context);
        let i64t = LLVMInt64TypeInContext(context);
        let i32t = LLVMInt32TypeInContext(context);
        let i8t = LLVMInt8TypeInContext(context);
        let i8pt = LLVMPointerType(i8t, 0);
        let zero_i32 = LLVMConstInt(i32t, 0, 0);
        let one_i32 = LLVMConstInt(i32t, 1, 0);

        let mut argts = [i8pt, i64t, i64t, i8pt, i64t, i64t];
        let function_type = LLVMFunctionType(voidt, argts.as_mut_ptr(), argts.len() as u32, 0);
        let function = LLVMAddFunction(module, c_str!("process_image"), function_type);

        let bb_entry = LLVMAppendBasicBlockInContext(context, function, c_str!("entry"));
        let bb_ycond = LLVMAppendBasicBlockInContext(context, function, c_str!("y.for.cond"));
        let bb_ybody = LLVMAppendBasicBlockInContext(context, function, c_str!("y.for.body"));
        let bb_yinc = LLVMAppendBasicBlockInContext(context, function, c_str!("y.for.inc"));
        let bb_yend = LLVMAppendBasicBlockInContext(context, function, c_str!("y.for.end"));
        let bb_xcond = LLVMAppendBasicBlockInContext(context, function, c_str!("x.for.cond"));
        let bb_xbody = LLVMAppendBasicBlockInContext(context, function, c_str!("x.for.body"));
        let bb_xinc = LLVMAppendBasicBlockInContext(context, function, c_str!("x.for.inc"));
        let bb_xend = LLVMAppendBasicBlockInContext(context, function, c_str!("x.for.end"));

        let src = LLVMGetParam(function, 0);
        let src_width = LLVMGetParam(function, 1);
        let src_height = LLVMGetParam(function, 2);
        let dst = LLVMGetParam(function, 3);
        // We currently just assume that src and dst have the same dimensions
        //let dst_width = LLVMGetParam(function, 4);
        //let dst_height = LLVMGetParam(function, 5);

        // entry:
        LLVMPositionBuilderAtEnd(builder, bb_entry);
        let y = LLVMBuildAlloca(builder, i32t, c_str!("y"));
        LLVMSetAlignment(y, 4);
        let x = LLVMBuildAlloca(builder, i32t, c_str!("x"));
        LLVMSetAlignment(x, 4);

        let ymax = LLVMBuildTrunc(builder, src_height, i32t, c_str!("ymax"));
        let xmax = LLVMBuildTrunc(builder, src_width, i32t, c_str!("xmax"));
        let s = LLVMBuildStore(builder, zero_i32, y);
        LLVMSetAlignment(s, 4);
        let s = LLVMBuildStore(builder, zero_i32, x);
        LLVMSetAlignment(s, 4);
        LLVMBuildBr(builder, bb_ycond);

        // y.for.cond:
        LLVMPositionBuilderAtEnd(builder, bb_ycond);
        let tmp_y_cond = LLVMBuildLoad(builder, y, c_str!("tmp.y.cond"));
        LLVMSetAlignment(tmp_y_cond, 4);
        let ycmp = LLVMBuildICmp(builder, LLVMIntPredicate::LLVMIntSLT, tmp_y_cond, ymax, c_str!("cmp.y"));
        LLVMBuildCondBr(builder, ycmp, bb_ybody, bb_yend);

        // y.for.body:
        LLVMPositionBuilderAtEnd(builder, bb_ybody);
        let tmp1_y = LLVMBuildLoad(builder, y, c_str!("tmp1.y"));
        LLVMSetAlignment(tmp1_y, 4);
        let s = LLVMBuildStore(builder, zero_i32, x);
        LLVMSetAlignment(s, 4);
        LLVMBuildBr(builder, bb_xcond);

        // x.for.cond:
        LLVMPositionBuilderAtEnd(builder, bb_xcond);
        let tmp_x_cond = LLVMBuildLoad(builder, x, c_str!("tmp.x.cond"));
        LLVMSetAlignment(tmp_x_cond, 4);
        let xcmp = LLVMBuildICmp(builder, LLVMIntPredicate::LLVMIntSLT, tmp_x_cond, xmax, c_str!("cmp.x"));
        LLVMBuildCondBr(builder, xcmp, bb_xbody, bb_xend);

        // x.for.body:
        LLVMPositionBuilderAtEnd(builder, bb_xbody);
        let tmp1_x = LLVMBuildLoad(builder, x, c_str!("tmp1.x"));
        LLVMSetAlignment(tmp1_x, 4);
        let m = LLVMBuildMul(builder, tmp1_y, xmax, c_str!("m"));
        let off = LLVMBuildAdd(builder, m, tmp1_x, c_str!("off"));
        let mut idxs = [off];
        let sidx = LLVMBuildInBoundsGEP(builder, src, idxs.as_mut_ptr(), 1, c_str!("sidx"));
        let didx = LLVMBuildInBoundsGEP(builder, dst, idxs.as_mut_ptr(), 1, c_str!("didx"));
        let val = LLVMBuildLoad(builder, sidx, c_str!("val"));
        let three_i8 = LLVMConstInt(i8t, 3, 0);
        let upd = LLVMBuildAdd(builder, val, three_i8, c_str!("upd"));
        LLVMBuildStore(builder, upd, didx);
        LLVMBuildBr(builder, bb_xinc);

        // x.for.inc:
        LLVMPositionBuilderAtEnd(builder, bb_xinc);
        let tmp2_x = LLVMBuildLoad(builder, x, c_str!("tmp2.x"));
        LLVMSetAlignment(tmp2_x, 4);
        let inc_x = LLVMBuildNSWAdd(builder, tmp2_x, one_i32, c_str!("inc.x"));
        let s = LLVMBuildStore(builder, inc_x, x);
        LLVMSetAlignment(s, 4);
        LLVMBuildBr(builder, bb_xcond);

        // y.for.inc:
        LLVMPositionBuilderAtEnd(builder, bb_yinc);
        let tmp2_y = LLVMBuildLoad(builder, y, c_str!("tmp2_y"));
        LLVMSetAlignment(tmp2_y, 4);
        let inc_y = LLVMBuildNSWAdd(builder, tmp2_y, one_i32, c_str!("inc.y"));
        let s = LLVMBuildStore(builder, inc_y, y);
        LLVMSetAlignment(s, 4);
        LLVMBuildBr(builder, bb_ycond);

        // x.for.end:
        LLVMPositionBuilderAtEnd(builder, bb_xend);
        LLVMBuildBr(builder, bb_yinc);

        // y.for.end
        LLVMPositionBuilderAtEnd(builder, bb_yend);
        LLVMBuildRetVoid(builder);

        LLVMDisposeBuilder(builder);
        module
    }
}

const PROCESS_IMAGE_IR: &str = "define void @process_image(
    i8* nocapture readonly %src, i64 %src_width, i64 %src_height,
    i8* nocapture %dst, i64 %dst_width, i64 %dst_height) {
; TODO: try using phi nodes instead of alloca for loop variables
; TODO: this code assumes that src and dst have the same dimensions. add validation
entry:
  %y = alloca i32, align 4
  %x = alloca i32, align 4
  %ymax = trunc i64 %src_height to i32
  %xmax = trunc i64 %src_width to i32
  store i32 0, i32* %y, align 4
  store i32 0, i32* %x, align 4
  br label %y.for.cond
y.for.cond:
  %tmp.y.cond = load i32, i32* %y, align 4
  %cmp.y = icmp slt i32 %tmp.y.cond, %ymax
  br i1 %cmp.y, label %y.for.body, label %y.for.end
y.for.body:
  %tmp1.y = load i32, i32* %y, align 4
  store i32 0, i32* %x, align 4
  br label %x.for.cond
x.for.cond:
  %tmp.x.cond = load i32, i32* %x, align 4
  %cmp.x = icmp slt i32 %tmp.x.cond, %xmax
  br i1 %cmp.x, label %x.for.body, label %x.for.end
x.for.body:
  %tmp1.x = load i32, i32* %x, align 4
  %m = mul i32 %tmp1.y, %xmax
  %off = add i32 %m, %tmp1.x
  %sidx = getelementptr i8, i8* %src, i32 %off
  %didx = getelementptr i8, i8* %dst, i32 %off
  %val = load i8, i8* %sidx
  %upd = add i8 %val, 3
  store i8 %upd, i8* %didx
  br label %x.for.inc
x.for.inc:
  %tmp2.x = load i32, i32* %x, align 4
  %inc.x = add nsw i32 %tmp2.x, 1
  store i32 %inc.x, i32* %x, align 4
  br label %x.for.cond
y.for.inc:
  %tmp2.y = load i32, i32* %y, align 4
  %inc.y = add nsw i32 %tmp2.y, 1
  store i32 %inc.y, i32* %y, align 4
  br label %y.for.cond
x.for.end:
  br label %y.for.inc
y.for.end:
  ret void
}";

fn run_process_image_example(codegen: Codegen) {
    unsafe {
        let context = LLVMContextCreate();
        let module = match codegen {
            Codegen::Handwritten => create_module_from_handwritten_ir(context, PROCESS_IMAGE_IR),
            Codegen::Builder => create_process_image_module_via_builder(context)
        };
        // Dump the module as IR to stdout.
        LLVMDumpModule(module);
        let mut ee = mem::uninitialized();
        let mut out = mem::zeroed();
        println!("Execution engine creation: PENDING");
        LLVMCreateExecutionEngineForModule(&mut ee, module, &mut out);
        println!("Execution engine creation: COMPLETE");
        println!("Function creation: PENDING");
        let addr = LLVMGetFunctionAddress(ee, c_str!("process_image"));
        let f: extern "C" fn(*const u8, usize, usize, *mut u8, usize, usize) = mem::transmute(addr);
        println!("Function creation: COMPLETE");
        let x = gray_image!(1, 2, 3; 4, 5, 6; 7, 8, 9);
        let mut y = GrayImage::new(3, 3);
        println!("Function execution: PENDING");
        f(x.buffer.as_ptr(), 3, 3, y.buffer.as_mut_ptr(), 3, 3);
        println!("Function execution: COMPLETE");
        println!("map(+3, {:?}) = {:?}", x, y);
        LLVMDisposeExecutionEngine(ee);
        LLVMContextDispose(context);
    }
}

fn run_sum_example(codegen: Codegen) {
    unsafe {
        let context = LLVMContextCreate();
        let module = match codegen {
            Codegen::Handwritten => create_module_from_handwritten_ir(context, SUM_IR),
            Codegen::Builder => create_sum_module_via_builder(context),
        };
        // Dump the module as IR to stdout.
        LLVMDumpModule(module);

        let mut ee = mem::uninitialized();
        let mut out = mem::zeroed();
        LLVMCreateExecutionEngineForModule(&mut ee, module, &mut out);

        let addr = LLVMGetFunctionAddress(ee, c_str!("sum"));
        let f: extern "C" fn(u64, u64, u64) -> u64 = mem::transmute(addr);
        let (x, y, z) = (1, 1, 1);
        let res = f(x, y, z);
        println!("{} + {} + {} = {}", x, y, z, res);

        LLVMDisposeExecutionEngine(ee);
        LLVMContextDispose(context);
    }
}

/// Whether to use handwritten IR or an LLVM builder
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Codegen {
    Handwritten,
    Builder
}

/// Which example to use
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Example {
    Sum,
    ProcessImage
}

use structopt::StructOpt;

fn codegen_from_str(d: &str) -> Codegen {
    match d {
        "handwritten" => Codegen::Handwritten,
        "builder" => Codegen::Builder,
        _ => panic!("invalid codegen flag")
    }
}

fn example_from_str(d: &str) -> Example {
    match d {
        "sum" => Example::Sum,
        "image" => Example::ProcessImage,
        _ => panic!("invalid example flag")
    }
}

#[derive(StructOpt, Debug)]
struct Opts {
    #[structopt(short = "e", long = "example")]
    example: String,

    #[structopt(short = "c", long = "codegen")]
    codegen: String,
}

fn main() {
    initialise_jit();
    
    let opts = Opts::from_args();
    let example = example_from_str(&opts.example);
    let codegen = codegen_from_str(&opts.codegen);
    match example {
        Example::Sum => run_sum_example(codegen),
        Example::ProcessImage => run_process_image_example(codegen)
    };
}

