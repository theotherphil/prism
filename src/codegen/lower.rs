//! Functions for lowering the prism AST to LLVM IR

use llvm::prelude::*;
use std::collections::HashMap;
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

#[derive(Debug)]
pub struct SymbolTable {
    symbols: HashMap<String, LLVMValueRef>
}

impl SymbolTable {
    pub fn new() -> SymbolTable {
        SymbolTable { symbols: HashMap::new() }
    }

    pub fn add(&mut self, name: &str, value: LLVMValueRef) {
        self.symbols.insert(name.to_string(), value);
    }

    pub fn remove(&mut self, name: &str) {
        match self.symbols.remove(name) {
            None => panic!("Remove failed - symbol {} not found", name),
            _ => {}
        };
    }

    pub fn get(&self, name: &str) -> LLVMValueRef {
        match self.symbols.get(name) {
            Some(v) => *v,
            None => panic!("Get failed - symbol {} not found", name)
        }
    }
}

pub fn create_process_image_module(context: &Context, func: &Func) -> Module {
    let module = context.new_module("process_image");
    let builder = Builder::new(context);

    let llvm_func_type = builder.func_type(
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
    let llvm_func = builder.add_func(module, "process_image", llvm_func_type);
    let params = builder.get_params(llvm_func);
    // We currently just assume that src and dst have the same dimensions
    // so ignore the last two params
    let (src, src_width, src_height, dst) = (
        params[0], params[1], params[2], params[3]
    );

    let mut symbols = SymbolTable::new();
    symbols.add("src", src);
    symbols.add("dst", dst);
    symbols.add("src_width", src_width);

    let entry = builder.new_block(llvm_func, "entry");
    builder.position_at_end(entry);
    let y_max = builder.trunc(src_height, builder.type_i32());
    let x_max = builder.trunc(src_width, builder.type_i32());
    symbols.add("y_max", y_max);
    symbols.add("x_max", x_max);

    let generate_y_body = |symbols| {
        generate_x_loop(&builder, func, llvm_func, symbols)
    };
    generate_y_loop(&builder, llvm_func, &mut symbols, generate_y_body);

    builder.ret_void();
    Module::new(module)
}

fn generate_y_loop<'s>(
    builder: &Builder,
    llvm_func: LLVMValueRef,
    symbols: &'s mut SymbolTable,
    mut generate_body: impl FnMut(&'s mut SymbolTable)
) {
    let pre_header = builder.get_insert_block();

    let y_header = builder.new_block(llvm_func, "y.header");
    let y_loop = builder.new_block(llvm_func, "y.loop");
    let y_after = builder.new_block(llvm_func, "y.after");

    builder.position_at_end(pre_header);
    builder.br(y_header);

    // load symbols
    let y_max = symbols.get("y_max");

    // y.header:
    builder.position_at_end(y_header);
    let no_rows = builder.icmp_eq(y_max, builder.const_i32(0));
    builder.cond_br(no_rows, y_after, y_loop);
    // y.loop:
    builder.position_at_end(y_loop);
    let y = builder.build_phi(builder.type_i32(), "y");
    symbols.add("y", y);
    builder.add_phi_incoming(y, builder.const_i32(0), y_header);
    generate_body(symbols);
    let y_next = builder.add(y, builder.const_i32(1));
    builder.add_phi_incoming(y, y_next, builder.get_insert_block());
    let y_continue = builder.icmp_slt(y_next, y_max);
    builder.cond_br(y_continue, y_loop, y_after);
    // y.after:
    builder.position_at_end(y_after);
}

fn generate_x_loop(
    builder: &Builder,
    func: &Func,
    llvm_func: LLVMValueRef,
    symbols: &mut SymbolTable
) {
    let pre_header = builder.get_insert_block();

    let x_header = builder.new_block(llvm_func, "x.header");
    let x_loop = builder.new_block(llvm_func, "x.loop");
    let x_after = builder.new_block(llvm_func, "x.after");

    builder.position_at_end(pre_header);
    builder.br(x_header);

    // load symbols
    let x_max = symbols.get("x_max");
    let y = symbols.get("y");
    let src = symbols.get("src");
    let dst = symbols.get("dst");
    let src_width = symbols.get("src_width");

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
    builder.position_at_end(x_after);
}
