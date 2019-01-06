//! Used to experiment with code generation using the Builder type.
//!
//! $ cargo run --example build
//!

use prism::llvm::*;
use libc::c_char;
use std::mem;
use std::ffi::CStr;

fn main() {
    initialise_llvm_jit();
    run(&Context::new());
}

#[no_mangle]
extern "C" fn log(name: *const c_char) {
    let name = unsafe { CStr::from_ptr(name).to_string_lossy().to_string() };
    println!("LOGGED({})", name);
}

fn run(context: &Context) {
    // Create module
    let module_name = "call_log";
    let mut module = context.new_module(&module_name);

    let builder = Builder::new(context);

    // Declare extern log function and give symbol address
    builder.add_symbol("log", log as *const());
    let log_func_type = builder.func_type(
        builder.type_void(),
        &mut [builder.type_i8_ptr()]
    );
    let log = builder.add_func(&module, "log", log_func_type);

    // Create call_log function
    let call_func_type = builder.func_type(
        builder.type_void(),
        &mut[]
    );
    let call_func = builder.add_func(&module, "call_log", call_func_type);

    // Declare msg global and generate call to the log function
    let entry = builder.new_block(call_func, "entry");
    builder.position_at_end(entry);
    let msg = builder.global_string("a message", "msg");
    builder.build_function_call(log, &mut [msg]);
    builder.ret_void();

    // Dump generated IR
    println!("{}", module.dump_to_string());
    optimise(&mut module);
    println!("{}", module.dump_to_string());

    // JIT and run
    let engine = ExecutionEngine::new(module);
    let func: fn() = unsafe { mem::transmute(engine.get_func_addr("call_log")) };
    func();
}
