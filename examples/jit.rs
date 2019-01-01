
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
    println!("Optimising");
    optimise(&mut module);
    println!("Post-optimise IR");
    module.dump_to_stdout();
    module
}

fn example_image(width: usize, height: usize) -> GrayImage {
    let mut image = GrayImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            image.set(x, y, (10 * (x % 10 + y % 10) as u8) + 50);
        }
    }
    image
}

fn run_process_image(context: &Context) {
    let graph = Graph::new(vec![
        Func::new(
            "blur_h",
            (read("in", x() - 1, y()) + read("in", x(), y()) + read("in", x() + 1, y())) / 3
        ),
        Func::new(
            "blur_v",
            (read("blur_h", x(), y() - 1) + read("blur_h", x(), y()) + read("blur_h", x(), y() + 1)) / 3
        )
    ]);

    println!("Creating module");
    let module = create_optimised_module(context, &graph);

    println!("Creating execution engine");
    let engine = ExecutionEngine::new(module);

    println!("Creating processor");
    let processor = engine.get_processor("process_image", &graph);

    println!("Running function");
    let image = example_image(100, 50);
    let inputs = [(String::from("in"), &image)];
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

    save_to_png(&image, "/Users/philip/dev/data/blur_jit_input.png").unwrap();
    save_to_png(&results["blur_h"], "/Users/philip/dev/data/blur_jit_h.png").unwrap();
    save_to_png(&results["blur_v"], "/Users/philip/dev/data/blur_jit_v.png").unwrap();
}

fn main() {
    initialise_llvm_jit();
    let context = Context::new();
    run_process_image(&context);
}

