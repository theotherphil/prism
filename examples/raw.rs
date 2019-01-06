//! An image processing example using handwritten IR

extern crate llvm_sys as llvm;

use std::mem;
use prism::*;

/// Variables can't be reassigned in LLVM IR, so this version allocates the
/// loop variables in a stack-allocated array and updates them via loads and
/// stores. PROCESS_IMAGE_IR_PHI uses phi nodes instead, which appears to be
/// the more common approach.
const PROCESS_IMAGE_IR: &str = "define void @process_image(
    i8* nocapture readonly %src, i64 %src_width, i64 %src_height,
    i8* nocapture %dst, i64 %dst_width, i64 %dst_height) {
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

const PROCESS_IMAGE_IR_PHI: &str = "define void @process_image(
    i8* nocapture readonly %src, i64 %src_width, i64 %src_height,
    i8* nocapture %dst, i64 %dst_width, i64 %dst_height) {
; TODO: this code assumes that src and dst have the same dimensions. add validation
entry:
  %ymax = trunc i64 %src_height to i32
  %xmax = trunc i64 %src_width to i32
  br label %y.header
y.header:
  %norows = icmp eq i32 %ymax, 0
  br i1 %norows, label %y.after, label %y.loop
y.loop:
  %y = phi i32 [0, %y.header], [%ynext, %x.after]
  br label %x.header
x.header:
  %nocols = icmp eq i32 %xmax, 0
  br i1 %nocols, label %x.after, label %x.loop
x.loop:
  %x = phi i32 [0, %x.header], [%xnext, %x.loop]
  %rowoff = mul i32 %y, %xmax
  %off = add i32 %rowoff, %x
  %srcptr = getelementptr i8, i8* %src, i32 %off
  %dstptr = getelementptr i8, i8* %dst, i32 %off
  %val = load i8, i8* %srcptr
  %upd = add i8 %val, 3
  store i8 %upd, i8* %dstptr
  %xnext = add i32 %x, 1
  %xcontinue = icmp slt i32 %xnext, %xmax
  br i1 %xcontinue, label %x.loop, label %x.after
x.after:
  %ynext = add i32 %y, 1
  %ycontinue = icmp slt i32 %ynext, %ymax
  br i1 %ycontinue, label %y.loop, label %y.after
y.after:
  ret void
}";

fn run_process_image(context: &Context, ir: &str) {
    println!("* Creating module");
    let mut module = create_module_from_ir_string(context, ir);

    println!("* Raw IR");
    module.dump_to_stdout();
    optimise(&mut module);
    println!("* Optimised IR");
    module.dump_to_stdout();

    println!("* Creating execution engine");
    let engine = ExecutionEngine::new(module);

    println!("* Creating function");
    let f: extern "C" fn(*const u8, usize, usize, *mut u8, usize, usize)
        = unsafe { mem::transmute(engine.get_func_addr("process_image")) };

    println!("* Executing function");
    let src = gray_image!(1, 2, 3; 4, 5, 6; 7, 8, 9);
    let mut dst = GrayImage::new(3, 3);
    f(src.buffer.as_ptr(), 3, 3, dst.buffer.as_mut_ptr(), 3, 3);
    println!("src: {:?}", src);
    println!("dst: {:?}", dst);
}

fn main() {
    initialise_llvm_jit();
    const USE_PHI_VERSION: bool = true;
    let ir = if USE_PHI_VERSION { PROCESS_IMAGE_IR_PHI } else { PROCESS_IMAGE_IR };
    run_process_image(&Context::new(), ir);
}

