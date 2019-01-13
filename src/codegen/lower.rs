//! Functions for lowering the prism AST to LLVM IR

use llvm_sys::prelude::*;
use crate::{syntax::*, codegen::*, llvm::*, tracing::*};

/// x and y are of type i32, return value has type i32
pub fn lower_var_expr(
    builder: &Builder,
    expr: &VarExpr,
    x: LLVMValueRef,
    y: LLVMValueRef
) -> LLVMValueRef {
    let recurse = |v| lower_var_expr(builder, v, x, y);
    match expr {
        VarExpr::Var(v) => match v { Var::X => x, Var::Y => y },
        VarExpr::Const(c) => builder.const_i32(*c),
        VarExpr::Add(l, r) => builder.add(recurse(l), recurse(r)),
        VarExpr::Sub(l, r) => builder.sub(recurse(l), recurse(r)),
        VarExpr::Mul(l, r) => builder.mul(recurse(l), recurse(r)),
    }
}

/// Return value is the value of the specified image at the given location,
/// sign extended to an i32, or 0i32 if the access is out of bounds.
/// Width and height are of type i32.
pub fn lower_access(
    builder: &Builder,
    llvm_func: LLVMValueRef,
    access: &Access,
    width: LLVMValueRef,
    height: LLVMValueRef,
    symbols: &mut SymbolTable
) -> LLVMValueRef {
    let input = symbols.get(&access.source);
    let x = symbols.get("x");
    let y = symbols.get("y");
    let log_read = symbols.get("log_read");
    let source = symbols.get(&global_buffer_string_name(&access.source));
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
            builder.build_function_call(
                log_read,
                &mut[source, x, y]);
            let ext = builder.zext(val, builder.type_i32());
            builder.store(ext, result, 4);
        },
        // else
        |_| {
            builder.store(builder.const_i32(0), result, 4);
        });

    builder.load(result, 4)
}

/// Return value has type i32
pub fn lower_definition(
    builder: &Builder,
    llvm_func: LLVMValueRef,
    definition: &Definition,
    width: LLVMValueRef,
    height: LLVMValueRef,
    symbols: &mut SymbolTable
) -> LLVMValueRef {
    let mut recurse = |v| lower_definition(builder, llvm_func, v, width, height, symbols);
    match definition {
        Definition::Access(a) => lower_access(builder, llvm_func, a, width, height, symbols),
        Definition::Const(c) => builder.const_i32(*c),
        Definition::Param(p) => symbols.get(&p),
        Definition::Add(l, r) => builder.add(recurse(l), recurse(r)),
        Definition::Mul(l, r) => builder.mul(recurse(l), recurse(r)),
        Definition::Sub(l, r) => builder.sub(recurse(l), recurse(r)),
        Definition::Div(l, r) => builder.sdiv(recurse(l), recurse(r))
    }
}

/// width and height are of type i32. symbols must contain entries for
/// all mentioned images and variables.
pub fn lower_func(
    builder: &Builder,
    llvm_func: LLVMValueRef,
    func: &Func,
    width: LLVMValueRef,
    height: LLVMValueRef,
    symbols: &mut SymbolTable
) {
    let val = lower_definition(builder, llvm_func, &func.definition, width, height, symbols);
    let (x, y) = (symbols.get("x"), symbols.get("y"));
    let offset = builder.add(builder.mul(y, width), x);
    let ptr = builder.in_bounds_gep(symbols.get(&func.name), offset);
    let trunc = builder.trunc(val, builder.type_i8());
    let log_write = symbols.get("log_write");
    let name = symbols.get(&global_buffer_string_name(&func.name));
    builder.build_function_call(
        log_write,
        &mut[name, x, y, trunc]);
    builder.store(trunc, ptr, 1);
}

/// Name of the global variable used to store the given buffer name.
fn global_buffer_string_name(name: &str) -> String {
    String::from(name) + "_name"
}

pub fn create_ir_module<'c, 'g>(context: &'c Context, graph: &'g Graph) -> Module<'c> {
    assert!(graph.funcs().len() > 0);
    let module = context.new_module(&graph.name);
    let builder = Builder::new(context);

    let mut symbols = SymbolTable::new();

    // Register tracing functions
    let log_read_type = builder.func_type(
        builder.type_void(),
        &mut [builder.type_i8_ptr(), builder.type_i32(), builder.type_i32()]
    );
    let log_write_type = builder.func_type(
        builder.type_void(),
        &mut [builder.type_i8_ptr(), builder.type_i32(), builder.type_i32(), builder.type_i8()]
    );
    builder.add_symbol("log_read", log_read as *const());
    builder.add_symbol("log_write", log_write as *const());
    let log_read = builder.add_func(&module, "log_read", log_read_type);
    let log_write = builder.add_func(&module, "log_write", log_write_type);
    symbols.add("log_read", log_read);
    symbols.add("log_write", log_write);

    let buffer_names = graph.inputs().iter()
        .chain(graph.outputs())
        .collect::<Vec<_>>();

    // Construct signature of generated function
    let mut llvm_func_params = vec![];
    for _ in &buffer_names {
        llvm_func_params.push(builder.type_i8_ptr());
        llvm_func_params.push(builder.type_i64());
        llvm_func_params.push(builder.type_i64());
    }
    // TODO: total hack that assumes there's always exactly one i32 parameter.
    // TODO: fix buffer and param passing to provide arrays.
    llvm_func_params.push(builder.type_i32());
    let llvm_func_type = builder.func_type(builder.type_void(), &mut llvm_func_params);
    let llvm_func = builder.add_func(&module, &graph.name, llvm_func_type);
    let params = builder.get_params(llvm_func);

    for (i, b) in buffer_names.iter().enumerate() {
        symbols.add(b, params[3 * i]);
    }

    // TODO: remove this hackery
    assert!(graph.params().len() == 1);
    for (i, p) in graph.params().iter().enumerate() {
        symbols.add(p, params[3 * buffer_names.len() + i]);
    }

    // We currently assume that all input buffers will have the same dimensions
    let width = params[1];
    let height = params[2];

    let entry = builder.new_block(llvm_func, "entry");
    builder.position_at_end(entry);

    for name in &buffer_names {
        let global = builder.global_string(name, name);
        symbols.add(&global_buffer_string_name(name), global);
    }

    let y_max = builder.trunc(height, builder.type_i32());
    let x_max = builder.trunc(width, builder.type_i32());

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
    module
}

/// bound is the open upper bound on the loop variable's value
fn generate_loop<'s>(
    builder: &Builder,
    name: &str,
    bound: LLVMValueRef,
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

