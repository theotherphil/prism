
// To shut up the warning when log_action is used
// for a function with void return type.
#![allow(path_statements)]

extern crate llvm_sys as llvm;

use std::mem;
use llvm::prelude::*;
use llvm::core::*;
use prism::*;
use prism::codegen::*;

macro_rules! log_action {
    ($name:expr, $action:expr) => {{
        print!($name);
        println!(": PENDING");
        let r = $action();
        print!($name);
        println!(": COMPLETE");
        r
    }};
}

const SUM_IR: &str = "define i64 @sum(i64, i64, i64) {
entry:
    %sum.1 = add i64 %0, %1
    %sum.2 = add i64 %sum.1, %2
    ret i64 %sum.2
}";

fn create_sum_module_via_builder(context: &Context) -> LLVMModuleRef {
    let module = context.new_module("sum");
    let builder = Builder::new(context);
    let i64t = builder.type_i64();
    let function_type = builder.func_type(i64t, &mut [i64t, i64t, i64t]);
    let function = builder.add_func(module, "sum", function_type);
    let _ = builder.new_block(function, "entry");
    let params = builder.get_params(function);
    let (x, y, z) = (params[0], params[1], params[2]);
    let sum = builder.add(x, y);
    let sum = builder.add(sum, z);
    builder.ret(sum);
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

fn run_process_image_example(context: &Context, codegen: Codegen) {
    unsafe {
        let def = read("in", x(), y()) + 1;
        let func = Func::new("out", &def);
        let module = match codegen {
            Codegen::Handwritten => create_module_from_handwritten_ir(context, PROCESS_IMAGE_IR),
            // The generated IR looks sensible, but compilation fails with:
            //      "Unable to copy EFLAGS physical register!"
            // Searching for this produces a few LLVM bug reports, but it's also very possible
            // that I've messed something up
            Codegen::Builder => create_process_image_module(context, &func)
        };
        // Dump the module as IR to stdout.
        LLVMDumpModule(module);
        let engine = log_action!(
            "Execution engine creation",
            || ExecutionEngine::new(module)
        );
        let f: extern "C" fn(*const u8, usize, usize, *mut u8, usize, usize) = log_action!(
            "Function creation",
            || mem::transmute(engine.get_func_addr("process_image"))
        );
        let x = gray_image!(1, 2, 3; 4, 5, 6; 7, 8, 9);
        let mut y = GrayImage::new(3, 3);
        log_action! (
            "Function execution",
            || f(x.buffer.as_ptr(), 3, 3, y.buffer.as_mut_ptr(), 3, 3)
        );
        println!("map({}, {:?}) = {:?}", func.pretty_print(), x, y);
    }
}

fn run_sum_example(context: &Context, codegen: Codegen) {
    unsafe {
        let module = match codegen {
            Codegen::Handwritten => create_module_from_handwritten_ir(context, SUM_IR),
            Codegen::Builder => create_sum_module_via_builder(context),
        };
        // Dump the module as IR to stdout.
        LLVMDumpModule(module);

        let engine = ExecutionEngine::new(module);

        let addr = engine.get_func_addr("sum");
        let f: extern "C" fn(u64, u64, u64) -> u64 = mem::transmute(addr);
        let (x, y, z) = (1, 1, 1);
        let res = f(x, y, z);
        println!("{} + {} + {} = {}", x, y, z, res);
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
    initialise_llvm_jit();
    let context = Context::new();
    
    let opts = Opts::from_args();
    let example = example_from_str(&opts.example);
    let codegen = codegen_from_str(&opts.codegen);
    match example {
        Example::Sum => run_sum_example(&context, codegen),
        Example::ProcessImage => run_process_image_example(&context, codegen)
    };
}

