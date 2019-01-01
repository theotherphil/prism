
extern crate llvm_sys as llvm;

use std::path::{Path, PathBuf};
use prism::*;
use prism::codegen::*;
use structopt::StructOpt;

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
            image.set(x, y, ((x / 5 + y / 5) % 2) as u8 * 200);
        }
    }
    image
}

fn run_process_image(context: &Context, dir: &Path) {
    let (x, y) = (Var::X, Var::Y);
    let input = Source::new("in");

    let blur_h = Func::new(
        "blur_h", {
        let sum = input.at(x - 1, y) + input.at(x, y) + input.at(x + 1, y);
        sum / 3
    });

    let blur_v = Func::new(
        "blur_v", {
        let sum = blur_h.at(x, y - 1) + blur_h.at(x, y) + blur_h.at(x, y + 1);
        sum / 3
    });

    let graph = Graph::new(vec![blur_h, blur_v]);

    println!("Creating module");
    let module = create_optimised_module(context, &graph);

    println!("Creating execution engine");
    let engine = ExecutionEngine::new(module);

    println!("Creating processor");
    let processor = engine.get_processor("process_image", &graph);

    println!("Running function");
    let inputs = [(&input, &example_image(20, 10))];
    let results = processor.process(&graph, &inputs);

    println!("Results");
    for func in graph.funcs() {
        println!("{}", func.pretty_print());
    }
    for input in &inputs {
        println!("{:?}", input);
        save_to_png(&input.1, dir.join(&(input.0.name.clone() + ".png"))).unwrap();
    }
    for result in &results {
        println!("{}: {:?}", result.0, result.1);
        save_to_png(&result.1, dir.join(&(result.0.clone() + ".png"))).unwrap();
    }
}

#[derive(StructOpt, Debug)]
struct Opts {
    /// Input and output images are written to this directory
    #[structopt(short = "o", long = "output", parse(from_os_str))]
    output_dir: PathBuf
}

fn main() {
    initialise_llvm_jit();
    let context = Context::new();
    let opts = Opts::from_args();
    run_process_image(&context, &opts.output_dir);
}

