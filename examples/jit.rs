
extern crate llvm_sys as llvm;

use prism::*;
use prism::codegen::*;

fn run_process_image(context: &Context) {
    println!("Defining function");
    let graph = Graph::new(vec![
        Func::new("f", read("in", x(), y()) + 3),
        Func::new("g", read("f", x(), y()) * 2)
    ]);
    // The generated IR looks sensible, but compilation fails with:
    //      "Unable to copy EFLAGS physical register!"
    // Searching for this produces a few LLVM bug reports, but it's also very possible
    // that I've messed something up
    //
    // EDIT: the optimisation step below fixes the compilation failure, which makes me
    // EDIT: more suspicious that this is an LLVM bug
    println!("Creating module");
    let mut module = create_process_image_module(context, &graph);

    println!("Pre-optimise IR");
    module.dump_to_stdout();
    optimise(&mut module);
    println!("Post-optimise IR");
    module.dump_to_stdout();

    println!("Creating execution engine");
    let engine = ExecutionEngine::new(module);

    println!("Creating function");
    let processor = engine.get_processor("process_image", &graph);

    println!("Running function");
    let in_image = gray_image!(1, 2, 3; 4, 5, 6; 7, 8, 9);
    let inputs = [(String::from("in"), &in_image)];
    let results = processor.process(&graph, &inputs);

    for func in graph.funcs() {
        println!("{}", func.pretty_print());
    }
    for input in &inputs {
        println!("{:?}", input);
    }
    for result in &results {
        println!("{}: {:?}", result.0, result.1);
    }
}

fn main() {
    initialise_llvm_jit();
    let context = Context::new();
    run_process_image(&context);
}

