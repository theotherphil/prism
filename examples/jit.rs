//!
//! Defines a 3x3 blur using Prism's DSL, compiles to native code, runs
//! the generated code on an example image, and dumps raw and optimised IR,
//! and the function's inputs, outputs, and intermediates to a user-provided directory.
//!
//! Example command line:
//!
//! $ cargo run --example jit -- -o /some/directory
//!

use std::{
    fs::File,
    io::{Result, Write},
    path::{Path, PathBuf}
};
use prism::{
    func,
    source,
    syntax::*,
    codegen::*,
    image::*,
    llvm::*,
    tracing::*
};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Opts {
    /// Input and output images are written to this directory
    #[structopt(short = "o", long = "output", parse(from_os_str))]
    output_dir: PathBuf
}

fn main() -> Result<()> {
    initialise_llvm_jit();
    let opts = Opts::from_args();
    run(&opts.output_dir)
}

fn run(dir: &Path) -> Result<()> {
    // Define the pipeline
    let (x, y) = (Var::X, Var::Y);
    source!(input);
    func!(blur_h = (input.at(x - 1, y) + input.at(x, y) + input.at(x + 1, y)) / 3);
    func!(blur_v = (blur_h.at(x, y - 1) + blur_h.at(x, y) + blur_h.at(x, y + 1)) / 3);
    let graph = Graph::new("blur3x3", vec![blur_h, blur_v]);

    // Generate LLVM IR
    let context = Context::new();
    let module = create_optimised_module(&context, &graph, dir);

    // Generate native code
    let processor = create_processor(module, &graph);

    // Run the generated code
    let inputs = [(&input, &example_image(6, 6))];
    let (results, trace) = processor.process_with_tracing(&inputs);

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

    // Dump a text trace of all the reads and writes...
    let mut f = File::create(dir.join("replay.txt"))?;
    for action in trace.actions.borrow().iter() {
        writeln!(f, "{:?}", action)?;
    }
    // ... and an animated gif showing them.
    write_replay_animation(dir.join("replay.gif"), &trace, 60)?;

    Ok(())
}

fn create_optimised_module<'c, 'g, 'p>(
    context: &'c Context,
    graph: &'g Graph,
    dir: &'p Path
) -> Module<'c> {
    let mut module = create_ir_module(context, &graph);

    module.dump_to_file(dir.join(graph.name.clone() + ".original.txt")).unwrap();
    // Without this optimisation step the IR looks sensible, but compilation fails
    // for some examples with:
    //      "Unable to copy EFLAGS physical register!"
    // Searching for this produces a few LLVM bug reports, but it's also very possible
    // that I've messed something up
    optimise(&mut module);
    module.dump_to_file(dir.join(graph.name.clone() + ".optimised.txt")).unwrap();
    module
}

fn example_image(width: usize, height: usize) -> GrayImage {
    let mut image = GrayImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            image.set(x, y, 100 * (x / 2 % 2 + y / 2 % 2) as u8);
        }
    }
    image
}
