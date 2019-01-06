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

    pub fn process(&self, inputs: &[(&Source, &GrayImage)]) -> HashMap<String, GrayImage> {
        self.process_impl(inputs, false).0
    }

    /// Uses horrible global state for tracing.
    pub fn process_with_tracing(
        &self,
        inputs: &[(&Source, &GrayImage)]
    ) -> (HashMap<String, GrayImage>, Trace) {
        let r = self.process_impl(inputs, true);
        (r.0, r.1.unwrap())
    }

    fn process_impl(
        &self,
        inputs: &[(&Source, &GrayImage)],
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

        for source in &self.inputs {
            match inputs.iter().find(|i| &i.0.name == source) {
                None => panic!(
                    "Required source {} is not calculated and is not provided as an input",
                    source
                ),
                _ => { }
            }
        }

        let mut calculated_images: Vec<(String, GrayImage)> = self.outputs
            .iter()
            .map(|name| (name.clone(), GrayImage::new(w, h)))
            .collect();

        match (self.inputs.len(), self.outputs.len()) {
            (1, 1) => {
                let i0 = &inputs[0].1;
                let r0 = &mut calculated_images[0].1;
                let f: extern "C" fn(
                    *const u8, usize, usize,
                    *mut u8, usize, usize
                ) = unsafe { mem::transmute(self.function_pointer) };
                f(
                    i0.buffer.as_ptr(), i0.width, i0.height,
                    r0.buffer.as_mut_ptr(), r0.width, r0.height
                );
            },
            (1, 2) => {
                let i0 = &inputs[0].1;
                let (rl, rr) = calculated_images.split_at_mut(1);
                let r0 = &mut rl[0].1;
                let r1 = &mut rr[0].1;
                let f: extern "C" fn(
                    *const u8, usize, usize,
                    *mut u8, usize, usize,
                    *mut u8, usize, usize
                ) = unsafe { mem::transmute(self.function_pointer) };
                f(
                    i0.buffer.as_ptr(), i0.width, i0.height,
                    r0.buffer.as_mut_ptr(), r0.width, r0.height,
                    r1.buffer.as_mut_ptr(), r1.width, r1.height
                );
            },
            (2, 1) => {
                let i0 = &inputs[0].1;
                let i1 = &inputs[0].1;
                let r0 = &mut calculated_images[0].1;
                let f: extern "C" fn(
                    *const u8, usize, usize,
                    *const u8, usize, usize,
                    *mut u8, usize, usize
                ) = unsafe { mem::transmute(self.function_pointer) };
                f(
                    i0.buffer.as_ptr(), i0.width, i0.height,
                    i1.buffer.as_ptr(), i1.width, i1.height,
                    r0.buffer.as_mut_ptr(), r0.width, r0.height
                );
            },
            (2, 2) => {
                let i0 = &inputs[0].1;
                let i1 = &inputs[0].1;
                let (rl, rr) = calculated_images.split_at_mut(1);
                let r0 = &mut rl[0].1;
                let r1 = &mut rr[0].1;
                let f: extern "C" fn(
                    *const u8, usize, usize,
                    *const u8, usize, usize,
                    *mut u8, usize, usize,
                    *mut u8, usize, usize
                ) = unsafe { mem::transmute(self.function_pointer) };
                f(
                    i0.buffer.as_ptr(), i0.width, i0.height,
                    i1.buffer.as_ptr(), i1.width, i1.height,
                    r0.buffer.as_mut_ptr(), r0.width, r0.height,
                    r1.buffer.as_mut_ptr(), r1.width, r1.height
                );
            },
            (_, _) => panic!("Unsupported signature")
        };

        let tr = unsafe { get_global_trace() };
        if trace {
            unsafe { clear_global_trace(); }
        }

        (calculated_images.into_iter().collect(), tr)
    }
}
