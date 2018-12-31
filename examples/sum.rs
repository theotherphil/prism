//! Reproduction of the jit-function example from llvm-sys but using
//! this crate's Builder type.

extern crate llvm_sys as llvm;

use std::mem;
use llvm::prelude::*;
use llvm::core::*;
use prism::codegen::*;

const SUM_IR: &str = "define i64 @sum(i64, i64, i64) {
entry:
    %sum.1 = add i64 %0, %1
    %sum.2 = add i64 %sum.1, %2
    ret i64 %sum.2
}";

fn create_sum_module_via_builder(context: &Context) -> LLVMModuleRef {
    let module = context.new_module("sum_builder");
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

fn run_sum(context: &Context, codegen: Codegen) {
    println!("*** Running {:?}\n", codegen);
    let module = match codegen {
        Codegen::Handwritten => create_module_from_handwritten_ir(context, SUM_IR),
        Codegen::Builder => create_sum_module_via_builder(context),
    };
    println!("** Module IR:");
    unsafe { LLVMDumpModule(module); }

    let engine = ExecutionEngine::new(module);

    let addr = engine.get_func_addr("sum");
    let f: extern "C" fn(u64, u64, u64) -> u64 = unsafe { mem::transmute(addr) };
    let (x, y, z) = (1, 1, 1);
    let res = f(x, y, z);
    println!("{} + {} + {} = {}", x, y, z, res);
}

/// Whether to use handwritten IR or an LLVM builder
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Codegen { Handwritten, Builder }

fn main() {
    initialise_llvm_jit();
    run_sum(&Context::new(), Codegen::Handwritten);
    run_sum(&Context::new(), Codegen::Builder);
}