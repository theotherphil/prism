
extern crate llvm_sys as llvm;

use prism::*;
use prism::codegen::*;

fn create_optimised_module(context: &Context, graph: &Graph) -> Module {
    let mut module = create_process_image_module(context, &graph);
    println!("Pre-optimise IR");
    module.dump_to_stdout();
    // Without this optimisation step the IR looks sensible, but compilation fails
    // for some examples with:
    //      "Unable to copy EFLAGS physical register!"
    // Searching for this produces a few LLVM bug reports, but it's also very possible
    // that I've messed something up
    optimise(&mut module);
    println!("Post-optimise IR");
    module.dump_to_stdout();
    module
}

fn run_process_image(context: &Context) {
    let graph = Graph::new(vec![
        Func::new("f", read("in", x(), y()) + 3),
        Func::new("g", read("f", x(), y()) * 2)
    ]);

    println!("Creating module");
    let module = create_optimised_module(context, &graph);

    println!("Creating execution engine");
    let engine = ExecutionEngine::new(module);

    println!("Creating processor");
    let processor = engine.get_processor("process_image", &graph);

    println!("Running function");
    let inputs = [(String::from("in"), &gray_image!(1, 2, 3; 4, 5, 6; 7, 8, 9))];
    let results = processor.process(&graph, &inputs);

    println!("Results");
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

