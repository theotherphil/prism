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
    let module = unsafe {
        LLVMModuleCreateWithNameInContext(c_str!("sum"), context)
    };
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

fn create_process_image_module_via_builder(context: LLVMContextRef) -> LLVMModuleRef {
    let module = unsafe {
        LLVMModuleCreateWithNameInContext(c_str!("process_image"), context)
    };
    let builder = Builder::new(context);

    let i64t = builder.type_i64();
    let i32t = builder.type_i32();
    let i8pt = builder.type_i8_ptr();

    let function_type = builder.func_type(
        builder.type_void(),
        &mut [i8pt, i64t, i64t, i8pt, i64t, i64t]
    );
    let function = builder.add_func(module, c_str!("process_image"), function_type);

    let bb_entry = builder.new_block(function, c_str!("entry"));
    let bb_ycond = builder.new_block(function, c_str!("y.for.cond"));
    let bb_ybody = builder.new_block(function, c_str!("y.for.body"));
    let bb_yinc = builder.new_block(function, c_str!("y.for.inc"));
    let bb_yend = builder.new_block(function, c_str!("y.for.end"));
    let bb_xcond = builder.new_block(function, c_str!("x.for.cond"));
    let bb_xbody = builder.new_block(function, c_str!("x.for.body"));
    let bb_xinc = builder.new_block(function, c_str!("x.for.inc"));
    let bb_xend = builder.new_block(function, c_str!("x.for.end"));

    let params = builder.get_params(function);
    // We currently just assume that src and dst have the same dimensions
    // so ignore the last two params
    let (src, src_width, src_height, dst) = (
        params[0], params[1], params[2], params[3]
    );

    // entry:
    builder.position_at_end(bb_entry);
    let y = builder.alloca(i32t, c_str!("y"), 4);
    let x = builder.alloca(i32t, c_str!("x"), 4);
    let ymax = builder.trunc(src_height, i32t);
    let xmax = builder.trunc(src_width, i32t);
    builder.store(builder.const_i32(0), y, 4);
    builder.store(builder.const_i32(0), x, 4);
    builder.br(bb_ycond);
    // y.for.cond:
    builder.position_at_end(bb_ycond);
    let tmp_y_cond = builder.load(y, 4);
    let ycmp = builder.icmp(LLVMIntPredicate::LLVMIntSLT, tmp_y_cond, ymax);
    builder.cond_br(ycmp, bb_ybody, bb_yend);
    // y.for.body:
    builder.position_at_end(bb_ybody);
    let tmp1_y = builder.load(y, 4);
    builder.store(builder.const_i32(0), x, 4);
    builder.br(bb_xcond);
    // x.for.cond:
    builder.position_at_end(bb_xcond);
    let tmp_x_cond = builder.load(x, 4);
    let xcmp = builder.icmp(LLVMIntPredicate::LLVMIntSLT, tmp_x_cond, xmax);
    builder.cond_br(xcmp, bb_xbody, bb_xend);
    // x.for.body:
    builder.position_at_end(bb_xbody);
    let tmp1_x = builder.load(x, 4);
    let m = builder.mul(tmp1_y, xmax);
    let off = builder.add(m, tmp1_x);
    let sidx = builder.in_bounds_gep(src, off);
    let didx = builder.in_bounds_gep(dst, off);
    let val = builder.load(sidx, 1);
    let upd = builder.add(val, builder.const_i8(3));
    builder.store(upd, didx, 1);
    builder.br(bb_xinc);
    // x.for.inc:
    builder.position_at_end(bb_xinc);
    let tmp2_x = builder.load(x, 4);
    let inc_x = builder.add_nsw(tmp2_x, builder.const_i32(1));
    builder.store(inc_x, x, 4);
    builder.br(bb_xcond);
    // y.for.inc:
    builder.position_at_end(bb_yinc);
    let tmp2_y = builder.load(y, 4);
    let inc_y = builder.add_nsw(tmp2_y, builder.const_i32(1));
    builder.store(inc_y, y, 4);
    builder.br(bb_ycond);
    // x.for.end:
    builder.position_at_end(bb_xend);
    builder.br(bb_yinc);
    // y.for.end
    builder.position_at_end(bb_yend);
    builder.ret_void();

    module
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

