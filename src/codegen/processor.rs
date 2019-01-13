//! A load of utter rubbish to run JIT-ed functions. Will be totally replaced
//! but for now this lets us experiment with image chains.

use std::collections::HashMap;
use std::mem;
use crate::{image::*, syntax::*, llvm::*, tracing::*};

pub struct Processor<'c> {
    /// This fields exists solely to ensure the engine
    /// isn't dropped while we're still using it.
    /// We could have a reference instead, but then this class
    /// would have two lifetimes - one for the reference to the
    /// engine and one for the context.
    _engine: ExecutionEngine<'c>,
    function_pointer: u64,
    inputs: Vec<String>,
    outputs: Vec<String>
}

/// Compile IR and return an object which supports calling the generated function
pub fn create_processor<'c, 'g>(module: Module<'c>, graph: &'g Graph) -> Processor<'c> {
    let engine = ExecutionEngine::new(module);
    Processor::new(engine, &graph)
}

impl<'c> Processor<'c> {
    pub fn new<'d>(engine: ExecutionEngine<'d>, graph: &Graph) -> Processor<'d> {
        let function_pointer = unsafe { engine.get_func_addr(&graph.name) };
        let inputs = graph.inputs().to_vec();
        let outputs = graph.outputs().to_vec();
        Processor { _engine: engine, function_pointer, inputs, outputs }
    }

    pub fn process(
        &self,
        inputs: &[(&Source, &GrayImage)],
        params: &HashMap<Param, i32>
    ) -> HashMap<String, GrayImage> {
        self.process_impl(inputs, params, false).0
    }

    /// Uses horrible global state for tracing.
    pub fn process_with_tracing(
        &self,
        inputs: &[(&Source, &GrayImage)],
        params: &HashMap<Param, i32>
    ) -> (HashMap<String, GrayImage>, Trace) {
        let r = self.process_impl(inputs, params, true);
        (r.0, r.1.unwrap())
    }

    fn process_impl(
        &self,
        inputs: &[(&Source, &GrayImage)],
        params: &HashMap<Param, i32>,
        trace: bool
    ) -> (HashMap<String, GrayImage>, Option<Trace>) {
        // Assume that all images are the same size for now. This will not be true in general
        let (w, h) = inputs[0].1.dimensions();

        // Initialise trace, set up the mapping from buffer names to trace ids
        if trace {
            let mut ids = HashMap::new();
            let tr = Trace::new();

            for input in inputs {
                let name = input.0.name.clone();
                let image = input.1;
                ids.insert(name, tr.create_trace_id(image));
            }
            for output in &self.outputs {
                ids.insert(output.to_string(), tr.create_trace_id(&GrayImage::new(w, h)));
            }

            unsafe { set_global_trace(ids, tr); }
        }

        // Check that all required inputs have been provided
        for source in &self.inputs {
            match inputs.iter().find(|i| &i.0.name == source) {
                None => panic!(
                    "Required source {} is not calculated and is not provided as an input",
                    source
                ),
                _ => { }
            }
        }

        // Allocate intermediate and result buffers
        let calculated_images: Vec<(String, GrayImage)> = self.outputs
            .iter()
            .map(|name| (name.clone(), GrayImage::new(w, h)))
            .collect();

        let mut buffers = vec![];
        let mut widths = vec![];
        let mut heights = vec![];

        for input in inputs {
            let image = input.1;
            buffers.push(image.buffer.as_ptr());
            widths.push(image.width());
            heights.push(image.height());
        }
        for calculated in &calculated_images {
            let image = &calculated.1;
            buffers.push(image.buffer.as_ptr());
            widths.push(image.width());
            heights.push(image.height());
        }

        // Sort params by name
        let mut params: Vec<(Param, i32)> = params.iter().map(|e| (e.0.clone(), *e.1)).collect();
        params.sort_by_key(|e| e.0.name.clone());
        let params: Vec<i32> = params.iter().map(|e| e.1).collect();

        // The generated function takes a single array containing all buffers,
        // both inputs and outputs. We claim all the pointers are const here, but
        // the output buffers are actually mutable.
        let f: extern "C" fn(
            *const *const u8, // buffers
            *const usize,     // widths
            *const usize,     // heights
            *const i32        // params
        ) = unsafe { mem::transmute(self.function_pointer) };

        f(
            buffers.as_ptr(),
            widths.as_ptr(),
            heights.as_ptr(),
            params.as_ptr()
        );

        let tr = unsafe { get_global_trace() };
        if trace {
            unsafe { clear_global_trace(); }
        }

        (calculated_images.into_iter().collect(), tr)
    }
}
