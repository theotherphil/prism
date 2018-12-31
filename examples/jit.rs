
extern crate llvm_sys as llvm;

use std::mem;
use prism::*;
use prism::codegen::*;

fn run_process_image(context: &Context) {
    println!("Defining function");
    let func = Func::new(
        "out",
        read("in", x(), y()) + 3
    );
    // The generated IR looks sensible, but compilation fails with:
    //      "Unable to copy EFLAGS physical register!"
    // Searching for this produces a few LLVM bug reports, but it's also very possible
    // that I've messed something up
    //
    // EDIT: the optimisation step below fixes the compilation failure, which makes me
    // EDIT: more suspicious that this is an LLVM bug
    println!("Creating module");
    let mut module = create_process_image_module(context, &func);

    println!("Pre-optimise IR");
    module.dump_to_stdout();
    optimise(&mut module);
    println!("Post-optimise IR");
    module.dump_to_stdout();

    println!("Creating execution engine");
    let engine = ExecutionEngine::new(module);

    println!("Creating function");
    let f: extern "C" fn(*const u8, usize, usize, *mut u8, usize, usize)
        = unsafe { mem::transmute(engine.get_func_addr("process_image")) };

    println!("Running function");
    let src = gray_image!(1, 2, 3; 4, 5, 6; 7, 8, 9);
    let mut dst = GrayImage::new(3, 3);
    f(src.buffer.as_ptr(), 3, 3, dst.buffer.as_mut_ptr(), 3, 3);

    println!("Func: {}", func.pretty_print());
    println!("src: {:?}", src);
    println!("dst: {:?}", dst);
}

fn main() {
    initialise_llvm_jit();
    let context = Context::new();
    run_process_image(&context);
}

