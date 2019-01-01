//! Functions for lowering the prism AST to LLVM IR

use llvm::prelude::*;
use std::collections::HashMap;
use crate::codegen::builder::*;
use crate::codegen::compile::*;
use crate::ast::*;

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

/// Return value is the value of the specified image at the given location,
/// sign extended to an i32, or 0i32 if the access is out of bounds
pub fn lower_access(
    builder: &Builder,
    llvm_func: LLVMValueRef,
    // e.g. in(3 * (x + 1) - y, 2 * x)
    access: &Access,
    // i32, width of input image
    width: LLVMValueRef,
    // i32, height of input image
    height: LLVMValueRef,
    symbols: &mut SymbolTable
    // return value has type i32
) -> LLVMValueRef {
    let input  = symbols.get(&access.source);
    let x = symbols.get("x");
    let y = symbols.get("y");
    let (x, y) = (
        lower_var_expr(builder, &access.x, x, y),
        lower_var_expr(builder, &access.y, x, y)
    );
    let result = builder.alloca(builder.type_i32(), 4);

    generate_if_then_else(
        builder,
        llvm_func,
        symbols,
        // if
        |_| {
            let x_positive = builder.icmp_sge(x, builder.const_i32(0));
            let x_lt_width = builder.icmp_slt(x, width);
            let y_positive = builder.icmp_sge(y, builder.const_i32(0));
            let y_lt_height = builder.icmp_slt(y, height);
            let x_valid = builder.and(x_positive, x_lt_width);
            let y_valid = builder.and(y_positive, y_lt_height);
            builder.and(x_valid, y_valid)
        },
        // then
        |_| {
            let offset = builder.add(builder.mul(y, width), x);
            let ptr = builder.in_bounds_gep(input, offset);
            let val = builder.load(ptr, 1);
            let ext = builder.zext(val, builder.type_i32());
            builder.store(ext, result, 4);
        },
        // else
        |_| {
            builder.store(builder.const_i32(0), result, 4);
        });

    builder.load(result, 4)
}

pub fn lower_definition(
    builder: &Builder,
    llvm_func: LLVMValueRef,
    // e.g. in(x, y) + in(x, y - 1)
    definition: &Definition,
    width: LLVMValueRef,
    height: LLVMValueRef,
    symbols: &mut SymbolTable
    // return value has type i32
) -> LLVMValueRef {
    match definition {
        Definition::Access(a) => lower_access(builder, llvm_func, a, width, height, symbols),
        Definition::Const(c) => builder.const_i32(*c),
        Definition::Add(l, r) => {
            let left = lower_definition(builder, llvm_func, l, width, height, symbols);
            let right = lower_definition(builder, llvm_func, r, width, height, symbols);
            builder.add(left, right)
        },
        Definition::Mul(l, r) => {
            let left = lower_definition(builder, llvm_func, l, width, height, symbols);
            let right = lower_definition(builder, llvm_func, r, width, height, symbols);
            builder.mul(left, right)
        },
        Definition::Sub(l, r) => {
            let left = lower_definition(builder, llvm_func, l, width, height, symbols);
            let right = lower_definition(builder, llvm_func, r, width, height, symbols);
            builder.sub(left, right)
        },
        Definition::Div(l, r) => {
            let left = lower_definition(builder, llvm_func, l, width, height, symbols);
            let right = lower_definition(builder, llvm_func, r, width, height, symbols);
            builder.sdiv(left, right)
        }
    }
}

pub fn lower_func(
    builder: &Builder,
    llvm_func: LLVMValueRef,
    func: &Func,
    // i32, width of input image
    width: LLVMValueRef,
    // i32, height of input image
    height: LLVMValueRef,
    // must contain symbols for all mentioned images and variables
    symbols: &mut SymbolTable
) {
    let val = lower_definition(builder, llvm_func, &func.definition, width, height, symbols);
    let offset = builder.add(builder.mul(symbols.get("y"), width), symbols.get("x"));
    let ptr = builder.in_bounds_gep(symbols.get(&func.name), offset);
    let trunc = builder.trunc(val, builder.type_i8());
    builder.store(trunc, ptr, 1);
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

pub fn create_process_image_module(context: &Context, graph: &Graph) -> Module {
    assert!(graph.funcs().len() > 0);
    let module = context.new_module("process_image");
    let builder = Builder::new(context);

    let buffer_names = graph.inputs().iter()
        .chain(graph.outputs())
        .collect::<Vec<_>>();

    let mut llvm_func_params = vec![];
    for _ in &buffer_names {
        llvm_func_params.push(builder.type_i8_ptr());
        llvm_func_params.push(builder.type_i64());
        llvm_func_params.push(builder.type_i64());
    }
    let llvm_func_type = builder.func_type(builder.type_void(), &mut llvm_func_params);
    let llvm_func = builder.add_func(module, "process_image", llvm_func_type);
    let params = builder.get_params(llvm_func);

    // We currently assume that all input buffers will have the same dimensions
    let width = params[1];
    let height = params[2];

    let entry = builder.new_block(llvm_func, "entry");
    builder.position_at_end(entry);
    let y_max = builder.trunc(height, builder.type_i32());
    let x_max = builder.trunc(width, builder.type_i32());

    let mut symbols = SymbolTable::new();
    for (i, b) in buffer_names.iter().enumerate() {
        symbols.add(b, params[3 * i]);
    }

    for func in graph.funcs() {
        let generate_x_body = |symbols: &mut SymbolTable| {
            lower_func(&builder, llvm_func, func, x_max, y_max, &mut *symbols);
        };
        let generate_y_body = |symbols| {
            generate_loop(&builder, "x", x_max, llvm_func, symbols, generate_x_body);
        };
        generate_loop(&builder, "y", y_max, llvm_func, &mut symbols, generate_y_body);
    }

    builder.ret_void();
    Module::new(module)
}

fn generate_loop<'s>(
    builder: &Builder,
    name: &str,
    bound: LLVMValueRef, // (open) upper bound on loop variable's value
    llvm_func: LLVMValueRef,
    symbols: &'s mut SymbolTable,
    mut generate_body: impl FnMut(&'s mut SymbolTable)
) {
    let pre_header = builder.get_insert_block();

    let header = builder.new_block(llvm_func, &(String::from(name) + ".header"));
    let body = builder.new_block(llvm_func, &(String::from(name) + ".loopbody"));
    let after = builder.new_block(llvm_func, &(String::from(name) + ".after"));

    // Add unconditional branch from the insertion block prior to
    // calling this function to the loop header
    builder.position_at_end(pre_header);
    builder.br(header);

    // header:
    builder.position_at_end(header);
    let is_empty = builder.icmp_eq(bound, builder.const_i32(0));
    builder.cond_br(is_empty, after, body);

    // body:
    builder.position_at_end(body);
    let loop_variable = builder.build_phi(builder.type_i32(), name);
    symbols.add(name, loop_variable);
    builder.add_phi_incoming(loop_variable, builder.const_i32(0), header);
    generate_body(symbols);
    let next = builder.add(loop_variable, builder.const_i32(1));
    builder.add_phi_incoming(loop_variable, next, builder.get_insert_block());
    let cont = builder.icmp_slt(next, bound);
    builder.cond_br(cont, body, after);

    // after:
    builder.position_at_end(after);
}

// The only way to call this function is to inline the closures
// directly into the call site - if the closures are first assigned
// to variables then the type system can't invent suitable types/borrow
// checker can't choose correct lifetimes. That's a bit sad...
fn generate_if_then_else(
    builder: &Builder,
    llvm_func: LLVMValueRef,
    symbols: & mut SymbolTable,
    mut generate_cond: impl FnMut(& mut SymbolTable) -> LLVMValueRef,
    mut generate_then: impl FnMut(& mut SymbolTable),
    mut generate_else: impl FnMut(& mut SymbolTable)
) {
    let pre_header = builder.get_insert_block();

    let if_block = builder.new_block(llvm_func, "cond.if");
    let then_block = builder.new_block(llvm_func, "cond.then");
    let else_block = builder.new_block(llvm_func, "cond.else");
    let after_block = builder.new_block(llvm_func, "cond.after");

    builder.position_at_end(pre_header);
    builder.br(if_block);
    builder.position_at_end(if_block);

    let cond = generate_cond(symbols);
    builder.cond_br(cond, then_block, else_block);

    builder.position_at_end(then_block);
    generate_then(symbols);
    builder.br(after_block);

    builder.position_at_end(else_block);
    // Might want to make this optional in general
    generate_else(symbols);
    builder.br(after_block);

    builder.position_at_end(after_block);
}

