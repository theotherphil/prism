
extern crate llvm_sys as llvm;

use std::mem;
use prism::*;
use prism::codegen::*;

fn run_process_image(context: &Context) {
    println!("Defining function");
    // These are the names of the buffers we'll pass to f after
    // it's generated, in order.
    let buffer_names = vec!["in", "f", "g"];
    let f = Func::new(
        "f",
        read("in", x(), y()) + 3
    );
    let g = Func::new(
        "g",
        read("f", x(), y()) * 2
    );
    // The generated IR looks sensible, but compilation fails with:
    //      "Unable to copy EFLAGS physical register!"
    // Searching for this produces a few LLVM bug reports, but it's also very possible
    // that I've messed something up
    //
    // EDIT: the optimisation step below fixes the compilation failure, which makes me
    // EDIT: more suspicious that this is an LLVM bug
    println!("Creating module");
    let mut module = create_process_image_module(context, &vec![&f, &g], &buffer_names);

    println!("Pre-optimise IR");
    module.dump_to_stdout();
    optimise(&mut module);
    println!("Post-optimise IR");
    module.dump_to_stdout();

    println!("Creating execution engine");
    let engine = ExecutionEngine::new(module);

    println!("Creating function");
    let process: extern "C" fn(*const u8, usize, usize, *mut u8, usize, usize, *mut u8, usize, usize)
        = unsafe { mem::transmute(engine.get_func_addr("process_image")) };

    println!("Running function");
    let in_image = gray_image!(1, 2, 3; 4, 5, 6; 7, 8, 9);
    let mut f_image = GrayImage::new(3, 3);
    let mut g_image = GrayImage::new(3, 3);
    process(
        in_image.buffer.as_ptr(), 3, 3,
        f_image.buffer.as_mut_ptr(), 3, 3,
        g_image.buffer.as_mut_ptr(), 3, 3
    );

    println!("{}", f.pretty_print());
    println!("{}", g.pretty_print());
    println!();
    println!("in: {:?}", in_image);
    println!("f: {:?}", f_image);
    println!("g: {:?}", g_image);
}

fn main() {
    initialise_llvm_jit();
    let context = Context::new();
    run_process_image(&context);
}

