
// To shut up the warning when log_action is used
// for a function with void return type.
#![allow(path_statements)]

extern crate llvm_sys as llvm;

use std::mem;
use std::ops::{Add, Mul};
use llvm::*;
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

#[derive(Debug, Clone)]
enum Func {
    // Value read from the input image
    Input,
    Const(i8),
    Add(Box<Func>, Box<Func>),
    Mul(Box<Func>, Box<Func>)
}

impl Func {
    fn id() -> Func {
        Func::Input
    }
}

macro_rules! impl_bin_op {
    ($trait_name:ident, $trait_op:ident, $ctor:expr) => {
        impl $trait_name<Self> for Func {
            type Output = Func;
            fn $trait_op(self, rhs: Self) -> Func {
                $ctor(Box::new(self), Box::new(rhs))
            }
        }

        impl $trait_name<i8> for Func {
            type Output = Func;
            fn $trait_op(self, rhs: i8) -> Func {
                $ctor(Box::new(self), Box::new(Func::Const(rhs)))
            }
        }

        impl $trait_name<Func> for i8 {
            type Output = Func;
            fn $trait_op(self, rhs: Func) -> Func {
                $ctor(Box::new(Func::Const(self)), Box::new(rhs))
            }
        }
    };
}

impl_bin_op!(Add, add, Func::Add);
impl_bin_op!(Mul, mul, Func::Mul);

impl Func {
    // For now we'll just support defining out(x,y) = f(in(x, y)),
    // where f is defined via an instance of Func
    fn compile(&self, builder: &Builder, input: LLVMValueRef) -> LLVMValueRef {
        match self {
            Func::Input => input,
            Func::Const(c) => builder.const_i8(*c),
            Func::Add(l, r) => {
                let left = l.compile(builder, input);
                let right = r.compile(builder, input);
                builder.add(left, right)
            },
            Func::Mul(l, r) => {
                let left = l.compile(builder, input);
                let right = r.compile(builder, input);
                builder.mul(left, right)
            }
        }
    }

    fn pretty_print(&self) -> String {
        match self {
            Func::Input => "v".into(),
            Func::Const(c) => c.to_string(),
            Func::Add(l, r) => {
                let left = l.pretty_print_with_parens();
                let right = r.pretty_print_with_parens();
                format!("{} + {}", left, right)
            }
            Func::Mul(l, r) => {
                let left = l.pretty_print_with_parens();
                let right = r.pretty_print_with_parens();
                format!("{} * {}", left, right)
            }
        }
    }

    fn pretty_print_with_parens(&self) -> String {
        let pp = self.pretty_print();
        match self {
            Func::Input => pp,
            Func::Const(_) => pp,
            Func::Add(_, _) => format!("({})", pp),
            Func::Mul(_, _)  => format!("({})", pp)
        }
    }
}

fn create_process_image_module_via_builder(context: &Context, func: &Func) -> LLVMModuleRef {
    let module = context.new_module("process_image");
    let builder = Builder::new(context);

    let function_type = builder.func_type(
        builder.type_void(),
        &mut [
            builder.type_i8_ptr(),
            builder.type_i64(),
            builder.type_i64(),
            builder.type_i8_ptr(),
            builder.type_i64(),
            builder.type_i64()
        ]
    );
    let function = builder.add_func(module, "process_image", function_type);
    let params = builder.get_params(function);
    // We currently just assume that src and dst have the same dimensions
    // so ignore the last two params
    let (src, src_width, src_height, dst) = (
        params[0], params[1], params[2], params[3]
    );

    let bb_entry = builder.new_block(function, "entry");
    let bb_ycond = builder.new_block(function, "y.for.cond");
    let bb_ybody = builder.new_block(function, "y.for.body");
    let bb_yinc = builder.new_block(function, "y.for.inc");
    let bb_yend = builder.new_block(function, "y.for.end");
    let bb_xcond = builder.new_block(function, "x.for.cond");
    let bb_xbody = builder.new_block(function, "x.for.body");
    let bb_xinc = builder.new_block(function, "x.for.inc");
    let bb_xend = builder.new_block(function, "x.for.end");

    // entry:
    builder.position_at_end(bb_entry);
    let y = builder.alloca(builder.type_i32(), "y", 4);
    let x = builder.alloca(builder.type_i32(), "x", 4);
    let ymax = builder.trunc(src_height, builder.type_i32());
    let xmax = builder.trunc(src_width, builder.type_i32());
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
    let upd = func.compile(&builder, val);
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

fn run_process_image_example(context: &Context, codegen: Codegen) {
    unsafe {
        let func = (Func::id() + 1i8) * 2i8;
        let module = match codegen {
            Codegen::Handwritten => create_module_from_handwritten_ir(context, PROCESS_IMAGE_IR),
            Codegen::Builder => create_process_image_module_via_builder(context, &func)
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

