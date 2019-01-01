# Prism

A toy [halide](http://halide-lang.org/) clone.

Not a lot is implemented yet, but it is possible to JIT and run a basic image pipeline. The details are in flux, but as of the time of writing the following example works (and produces terrible code).

```rust
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
```

See [examples/jit.rs](https://github.com/theotherphil/prism/blob/master/examples/jit.rs) for the latest runnable version of this.

This library also defines some basic functionality for tracing image processing operations, although these aren't yet integrated with the JIT functionality. The following examples were generated from handwritten examples in [src/blur3.rs](https://github.com/theotherphil/prism/blob/master/src/blur3.rs), using the `TraceImage` implementation of the `Image` trait. The goal is to add support for generating (and tracing) these schedules and more automatically. But I've not even started on that yet...

Dimensions: y, x. Compute at blur_v.x, store at blur_v.x
<br/><br/>
<img src="data/inline.gif" alt="inline blur" width="300" />

Dimensions: y, x. Compute at root, store at root
<br/><br/>
<img src="data/intermediate.gif" alt="blur with intermediate" width="500" />

Dimensions: y, x. Compute at blur_v.x, store at root
<br/><br/>
<img src="data/local_intermediate.gif" alt="blur with intermediate" width="500" />

Dimensions: yo, y, x. Compute at blur_v.yo, store at blur_v.yo
<br/><br/>
<img src="data/stripped.gif" alt="blur with striping" width="500" />

Dimension: yo, xo, y, x. Compute at blur_v.xo, store at blur_v.xo
<br/><br/>
<img src="data/tiled.gif" alt="blur with striping" width="500" />