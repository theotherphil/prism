
extern crate llvm_sys as llvm;

use prism::*;
use prism::codegen::*;

fn run_process_image(context: &Context) {
    println!("Defining function");
    // These are the names of the buffers we'll pass to f after
    // it's generated, in order.

    // TODO: want to be able to represent the entire function graph as a
    // TODO: single object, and pre-calculate any required info on inputs,
    // TODO: number of calculated images, etc.
    // TODO: also want to be able to write, e.g.
    // TODO: f(x, y) = g(x, y) + 1

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
    let funcs = [&f, &g];
    let mut module = create_process_image_module(context, &funcs, &buffer_names);

    println!("Pre-optimise IR");
    module.dump_to_stdout();
    optimise(&mut module);
    println!("Post-optimise IR");
    module.dump_to_stdout();

    println!("Creating execution engine");
    let engine = ExecutionEngine::new(module);

    println!("Creating function");
    let processor = engine.get_processor("process_image", &funcs);

    println!("Running function");
    let in_image = gray_image!(1, 2, 3; 4, 5, 6; 7, 8, 9);

    let results = processor.process(
        &[(String::from("in"), &in_image)],
        &funcs
    );

    println!("{}", f.pretty_print());
    println!("{}", g.pretty_print());
    println!();
    println!("in: {:?}", in_image);
    println!("f: {:?}", results["f"]);
    println!("g: {:?}", results["g"]);
}

fn main() {
    initialise_llvm_jit();
    let context = Context::new();
    run_process_image(&context);
}

