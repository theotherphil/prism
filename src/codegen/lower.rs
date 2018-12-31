//! Functions for lowering the prism AST to LLVM IR

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

pub fn create_process_image_module(context: &Context, func: &Func) -> Module {
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

    let entry = builder.new_block(function, "entry");
    let y_header = builder.new_block(function, "y.header");
    let y_loop = builder.new_block(function, "y.loop");
    let y_after = builder.new_block(function, "y.after");
    let x_header = builder.new_block(function, "x.header");
    let x_loop = builder.new_block(function, "x.loop");
    let x_after = builder.new_block(function, "x.after");

    // entry:
    builder.position_at_end(entry);
    let y_max = builder.trunc(src_height, builder.type_i32());
    let x_max = builder.trunc(src_width, builder.type_i32());
    builder.br(y_header);
    // y.header:
    builder.position_at_end(y_header);
    let no_rows = builder.icmp_eq(y_max, builder.const_i32(0));
    builder.cond_br(no_rows, y_after, y_loop);
    // y.loop:
    builder.position_at_end(y_loop);
    let y = builder.build_phi(builder.type_i32(), "y");
    builder.add_phi_incoming(y, builder.const_i32(0), y_header);
    builder.br(x_header);
    // x.header:
    builder.position_at_end(x_header);
    let no_cols = builder.icmp_eq(x_max, builder.const_i32(0));
    builder.cond_br(no_cols, x_after, x_loop);
    // x.loop:
    builder.position_at_end(x_loop);
    let x = builder.build_phi(builder.type_i32(), "x");
    builder.add_phi_incoming(x, builder.const_i32(0), x_header);
    lower_func(&builder, func, src, dst, src_width, x, y);
    let x_next = builder.add(x, builder.const_i32(1));
    builder.add_phi_incoming(x, x_next, builder.get_insert_block());
    let x_continue = builder.icmp_slt(x_next, x_max);
    builder.cond_br(x_continue, x_loop, x_after);
    // x.after:
    builder.position_at_end(x_after);
    let y_next = builder.add(y, builder.const_i32(1));
    builder.add_phi_incoming(y, y_next, builder.get_insert_block());
    let y_continue = builder.icmp_slt(y_next, y_max);
    builder.cond_br(y_continue, y_loop, y_after);
    // y.after:
    builder.position_at_end(y_after);
    builder.ret_void();

    Module::new(module)
}