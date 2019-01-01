//!
//! Defines a 3x3 blur using Prism's DSL, compiles to native code, runs
//! the generated code on an example image, and dumps inputs, outputs and
//! intermediates to a user-provided directory.
//!
//! Example command line:
//!
//! $ cargo run --example jit -- -o /some/directory
//!

use std::{io::Result, path::{Path, PathBuf}};
use prism::{*, codegen::*};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Opts {
    /// Input and output images are written to this directory
    #[structopt(short = "o", long = "output", parse(from_os_str))]
    output_dir: PathBuf
}

fn main() -> Result<()> {
    initialise_llvm_jit();
    let context = Context::new();
    let opts = Opts::from_args();
    run(&context, &opts.output_dir)
}

fn run(context: &Context, dir: &Path) -> Result<()> {
    // Define the pipeline
    let (x, y) = (Var::X, Var::Y);
    source!(input);
    func!(blur_h = (input.at(x - 1, y) + input.at(x, y) + input.at(x + 1, y)) / 3);
    func!(blur_v = (blur_h.at(x, y - 1) + blur_h.at(x, y) + blur_h.at(x, y + 1)) / 3);
    let graph = Graph::new(vec![blur_h, blur_v]);

    // Generate LLVM IR
    let module = create_optimised_module(context, &graph);

    // Generate native code
    let engine = ExecutionEngine::new(module);
    let processor = engine.get_processor("process_image", &graph);

    // Run the generated code
    let inputs = [(&input, &example_image(20, 10))];
    let results = processor.process(&graph, &inputs);

    // Dump the inputs, outputs and intermediates
    for func in graph.funcs() {
        println!("{}", func.pretty_print());
    }
    for input in &inputs {
        println!("{:?}", input);
        save_to_png(&input.1, dir.join(&(input.0.name.clone() + ".png")))?;
    }
    for result in &results {
        println!("{}: {:?}", result.0, result.1);
        save_to_png(&result.1, dir.join(&(result.0.clone() + ".png")))?;
    }

    Ok(())
}

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

fn example_image(width: usize, height: usize) -> GrayImage {
    let mut image = GrayImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            image.set(x, y, ((x / 5 + y / 5) % 2) as u8 * 200);
        }
    }
    image
}
