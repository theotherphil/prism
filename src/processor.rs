//! A load of utter rubbish to run JIT-ed functions. Will be totally replaced
//! but for now this lets us experiment with image chains.

use crate::traits::*;
use crate::buffer::*;
use crate::ast::*;
use std::collections::HashMap;
use std::mem;

pub struct Processor {
    // TODO: needs lifetimes - can't use this after the execution engine is dropped
    function_pointer: u64,
    inputs: Vec<String>,
    outputs: Vec<String>
}

impl Processor {
    pub fn new(graph: &Graph, function_pointer: u64) -> Processor {
        let inputs = graph.inputs().to_vec();
        let outputs = graph.outputs().to_vec();
        Processor { function_pointer, inputs, outputs }
    }

    pub fn process(&self, inputs: &[(&Source, &GrayImage)]) -> HashMap<String, GrayImage> {
        for source in &self.inputs {
            match inputs.iter().find(|i| &i.0.name == source) {
                None => panic!(
                    "Required source {} is not calculated and is not provided as an input",
                    source
                ),
                _ => { }
            }
        }

        // Assume that all images are the same size for now. This will not be true in general
        let (w, h) = inputs[0].1.dimensions();
        
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

        calculated_images.into_iter().collect()
    }
}
