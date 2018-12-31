//! Functions for lowering the prism AST to LLVM IR

use llvm::*;
use llvm::prelude::*;
use crate::codegen::builder::*;
use crate::codegen::compile::*;
use crate::ast::*;

/// TODO: bounds checking!
pub fn lower_var_expr(
    builder: &Builder,
    // e.g. 3 * (x + 1) - y
    expr: &VarExpr,
    // i32, current value of x variable
    x: LLVMValueRef,
    // i32, current value of y variable
    y: LLVMValueRef
    // return value has type i32
) -> LLVMValueRef {
    match expr {
        VarExpr::Var(v) => {
            match v {
                Var::X => x,
                Var::Y => y
            }
        },
        VarExpr::Const(c) => builder.const_i32(*c),
        VarExpr::Add(l, r) => {
            let left = lower_var_expr(builder, l, x, y);
            let right = lower_var_expr(builder, r, x, y);
            builder.add(left, right)
        },
        VarExpr::Sub(l, r) => {
            let left = lower_var_expr(builder, l, x, y);
            let right = lower_var_expr(builder, r, x, y);
            builder.sub(left, right)
        },
        VarExpr::Mul(l, r) => {
            let left = lower_var_expr(builder, l, x, y);
            let right = lower_var_expr(builder, r, x, y);
            builder.mul(left, right)
        },
    }
}

/// We only support loading from the special "in" source for now.
/// Return value is the value of the specified image at the given location.
/// TODO: bounds checking!
pub fn lower_access(
    builder: &Builder,
    // e.g. in(3 * (x + 1) - y, 2 * x)
    access: &Access,
    // i8* pointing to start of input image buffer
    input: LLVMValueRef,
    // i32, width of input image
    width: LLVMValueRef,
    // i32, current value of x variable
    x: LLVMValueRef,
    // i32, current value of y variable
    y: LLVMValueRef
    // return value has type i8
) -> LLVMValueRef {
    assert_eq!(access.source, "in");
    let (x, y) = (
        lower_var_expr(builder, &access.x, x, y),
        lower_var_expr(builder, &access.y, x, y)
    );
    let offset = builder.add(builder.mul(y, width), x);
    let ptr = builder.in_bounds_gep(input, offset);
    builder.load(ptr, 1)
}

pub fn lower_definition(
    builder: &Builder,
    // e.g. in(x, y) + in(x, y - 1)
    definition: &Definition,
    // i8* pointing to start of input image buffer
    input: LLVMValueRef,
    // i32, width of input image
    width: LLVMValueRef,
    // i32, current value of x variable
    x: LLVMValueRef,
    // i32, current value of y variable
    y: LLVMValueRef
    // return value has type i8
) -> LLVMValueRef {
    match definition {
        Definition::Access(a) => lower_access(builder, a, input, width, x, y),
        Definition::Const(c) => builder.const_i8(*c),
        Definition::Add(l, r) => {
            let left = lower_definition(builder, l, input, width, x, y);
            let right = lower_definition(builder, r, input, width, x, y);
            builder.add(left, right)
        },
        Definition::Mul(l, r) => {
            let left = lower_definition(builder, l, input, width, x, y);
            let right = lower_definition(builder, r, input, width, x, y);
            builder.mul(left, right)
        },
        Definition::Sub(l, r) => {
            let left = lower_definition(builder, l, input, width, x, y);
            let right = lower_definition(builder, r, input, width, x, y);
            builder.sub(left, right)
        },
        Definition::Div(l, r) => {
            let left = lower_definition(builder, l, input, width, x, y);
            let right = lower_definition(builder, r, input, width, x, y);
            builder.sdiv(left, right)
        }
    }
}

/// Only support reading from source "in" and writing to "out"
pub fn lower_func(
    builder: &Builder,
    // e.g. out(x, y) = in(x, y) + in(x, y - 1)
    func: &Func,
    // i8* pointing to start of input image buffer
    input: LLVMValueRef,
    // i8* pointing to start of output image buffer
    output: LLVMValueRef,
    // i32, width of input image
    width: LLVMValueRef,
    // i32, current value of x variable
    x: LLVMValueRef,
    // i32, current value of y variable
    y: LLVMValueRef
) {
    assert_eq!(func.name, "out");
    let val = lower_definition(builder, &func.definition, input, width, x, y);
    let offset = builder.add(builder.mul(y, width), x);
    let ptr = builder.in_bounds_gep(output, offset);
    builder.store(val, ptr, 1);
}

pub fn create_process_image_module(context: &Context, func: &Func) -> LLVMModuleRef {
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
    builder.store(builder.const_i32(0), x, 4);
    builder.br(bb_xcond);
    // x.for.cond:
    builder.position_at_end(bb_xcond);
    let tmp_x_cond = builder.load(x, 4);
    let xcmp = builder.icmp(LLVMIntPredicate::LLVMIntSLT, tmp_x_cond, xmax);
    builder.cond_br(xcmp, bb_xbody, bb_xend);
    // x.for.body:
    builder.position_at_end(bb_xbody);
    lower_func(&builder, func, src, dst, src_width, builder.load(x, 4), builder.load(y, 4));
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