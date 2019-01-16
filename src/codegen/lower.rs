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
    symbols: &mut SymbolTable
) -> LLVMValueRef {
    let input = symbols.get(&access.source);
    let width = symbols.get(&width_symbol_name(&access.source));
    let height = symbols.get(&height_symbol_name(&access.source));

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
    symbols: &mut SymbolTable
) -> LLVMValueRef {
    let mut recurse = |v| lower_definition(builder, llvm_func, v, symbols);
    match definition {
        Definition::Access(a) => lower_access(builder, llvm_func, a, symbols),
        Definition::Const(c) => builder.const_i32(*c),
        Definition::Param(p) => symbols.get(&p),
        Definition::Cond(c) => {
            let left = recurse(&*c.lhs);
            let right = recurse(&*c.rhs);
            let if_true = recurse(&*c.if_true);
            let if_false = recurse(&*c.if_false);
            let result = builder.alloca(builder.type_i32(), 4);
            generate_if_then_else(
                builder,
                llvm_func,
                symbols,
                // if
                |_| {
                    match c.cmp {
                        Comparison::EQ => builder.icmp_eq(left, right),
                        Comparison::GT => builder.icmp_sgt(left, right),
                        Comparison::GTE => builder.icmp_sge(left, right),
                        Comparison::LT => builder.icmp_slt(left, right),
                        Comparison::LTE => builder.icmp_sle(left, right)
                    }
                },
                // then
                |_| { builder.store(if_true, result, 4); },
                // else
                |_| { builder.store(if_false, result, 4); });

            builder.load(result, 4)
        }
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
    symbols: &mut SymbolTable
) {
    let val = lower_definition(builder, llvm_func, &func.definition, symbols);
    let (x, y) = (symbols.get("x"), symbols.get("y"));
    let width = symbols.get(&width_symbol_name(&func.name));
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

/// Name of the symbol used to store the width of a given buffer.
fn width_symbol_name(buffer_name: &str) -> String {
    String::from(buffer_name) + "_width"
}

/// Name of the symbol used to store the height of a given buffer.
fn height_symbol_name(buffer_name: &str) -> String {
    String::from(buffer_name) + "_height"
}

/// Add symbols for the static log_read and log_write functions and add these functions to `module`.
fn register_trace_functions(builder: &Builder, module: &Module<'_>) -> (LLVMValueRef, LLVMValueRef) {
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
    (log_read, log_write)
}

/// Creates the type of the generated function and adds it to `module`.
fn construct_func(builder: &Builder, module: &Module<'_>, graph: &Graph) -> LLVMValueRef {
    let mut llvm_func_params = vec![
        builder.ptr_type(builder.type_i8_ptr()), // buffers
        builder.ptr_type(builder.type_i64()),    // widths
        builder.ptr_type(builder.type_i64()),    // heights
        builder.ptr_type(builder.type_i32())     // params
    ];
    let llvm_func_type = builder.func_type(builder.type_void(), &mut llvm_func_params);
    builder.add_func(&module, &graph.name, llvm_func_type)
}

/// Parameters to the generated image processing function
struct ProcessingParams {
    // i8**
    buffers: LLVMValueRef,
    // i64*
    widths: LLVMValueRef,
    // i64*
    heights: LLVMValueRef,
    // i32*
    params: LLVMValueRef
}

impl ProcessingParams {
    fn new(params: Vec<LLVMValueRef>) -> ProcessingParams {
        assert_eq!(params.len(), 4);
        ProcessingParams {
            buffers: params[0],
            widths: params[1],
            heights: params[2],
            params: params[3]
        }
    }

    fn nth_buffer(&self, builder: &Builder, n: usize) -> (LLVMValueRef, LLVMValueRef, LLVMValueRef) {
        let offset = builder.const_i32(n as i32);
        let buffer = builder.load(builder.in_bounds_gep(self.buffers, offset), 8);
        let width = builder.load(builder.in_bounds_gep(self.widths, offset), 8);
        let height = builder.load(builder.in_bounds_gep(self.heights, offset), 8);
        (buffer, width, height)
    }

    fn nth_param(&self, builder: &Builder, n: usize) -> LLVMValueRef {
        let offset = builder.const_i32(n as i32);
        builder.load(builder.in_bounds_gep(self.params, offset), 4)
    }
}

pub fn create_ir_module<'c, 'g>(context: &'c Context, graph: &'g Graph) -> Module<'c> {
    assert!(graph.funcs().len() > 0);

    let module = context.new_module(&graph.name);
    let builder = Builder::new(context);
    let mut symbols = SymbolTable::new();

    // Set up tracing
    let (log_read, log_write) = register_trace_functions(&builder, &module);
    symbols.add("log_read", log_read);
    symbols.add("log_write", log_write);

    // Construct the LLVM object for the generated function
    let llvm_func = construct_func(&builder, &module, &graph);
    let params = ProcessingParams::new(builder.get_params(llvm_func));

    // Create first basic block in generated function and start writing to it
    let entry = builder.new_block(llvm_func, "entry");
    builder.position_at_end(entry);
    
    // Add expressions for each buffer and param to the symbol table.
    for (i, b) in graph.input_then_outputs().iter().enumerate() {
        // Global variable holding the name of this buffer, to use when tracing
        symbols.add(&global_buffer_string_name(b), builder.global_string(b, b));
        // Construct expressions for accessing the nth buffer
        let (buffer, buffer_width, buffer_height) = params.nth_buffer(&builder, i);
        symbols.add(b, buffer);
        let width = builder.trunc(buffer_width, builder.type_i32());
        let height = builder.trunc(buffer_height, builder.type_i32());
        symbols.add(&width_symbol_name(b), width);
        symbols.add(&height_symbol_name(b), height);
    }
    for (i, p) in graph.params().iter().enumerate() {
        let param = params.nth_param(&builder, i);
        symbols.add(p, param);
    }

    // TODO: need to switch from processing a graph to computing a single
    // TODO: designated output image, and compute loop bounds by working
    // TODO: backwards from it
    let final_func_name = &graph.funcs().iter().last().unwrap().name;
    let y_max = symbols.get(&height_symbol_name(final_func_name));
    let x_max = symbols.get(&width_symbol_name(final_func_name));

    for func in graph.funcs() {
        let sched = graph.schedule.get_func_schedule(func);
        // Hack hack hack
        let y_outer = sched.variables[0] == Var::Y;
        let (outer_variable, outer_max, inner_variable, inner_max) = if y_outer {
            ("y", y_max, "x", x_max)
        } else {
            ("x", x_max, "y", y_max)
        };
        let generate_inner_body = |symbols: &mut SymbolTable| {
            lower_func(&builder, llvm_func, func, &mut *symbols);
        };
        let generate_outer_body = |symbols| {
            generate_loop(&builder, inner_variable, inner_max, llvm_func, symbols, generate_inner_body);
        };
        generate_loop(&builder, outer_variable, outer_max, llvm_func, &mut symbols, generate_outer_body);
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

