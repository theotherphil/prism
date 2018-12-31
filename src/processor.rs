//! A load of utter rubbish to run JIT-ed functions. Will be totally replaced
//! but for now this lets us experiment with image chains.

use crate::traits::*;
use crate::buffer::*;
use crate::ast::*;
use std::collections::{HashMap, HashSet};
use std::mem;

pub struct Processor {
    function: FunctionPointer
}

// Returns (num inputs, num outputs)
fn sources(funcs: &[&Func]) -> (usize, usize) {
    // All this function-runtime rubbish has to go
    let func_sources: HashSet<String> = funcs
        .iter()
        .flat_map(|f| f.sources())
        .collect();

    let names: HashSet<String> = funcs
        .iter()
        .map(|f| f.name.clone())
        .collect();

    (func_sources.difference(&names).count(), funcs.len())
}

impl Processor {
    pub fn new(funcs: &[&Func], addr: u64) -> Processor {
        unsafe {
            match sources(funcs) {
                (1, 1) => {
                    Processor { function: FunctionPointer::OneInOneOut(mem::transmute(addr)) }
                },
                (1, 2) => {
                    Processor { function: FunctionPointer::OneInTwoOut(mem::transmute(addr)) }
                },
                (2, 1) => {
                    Processor { function: FunctionPointer::TwoInOneOut(mem::transmute(addr)) }
                },
                (2, 2) => {
                    Processor { function: FunctionPointer::TwoInTwoOut(mem::transmute(addr)) }
                },
                (_, _) => {
                    panic!("Unsupported signature")
                }
            }
        }
        
    }

    pub fn process(
        &self,
        inputs: &[(String, &GrayImage)],
        funcs: &[&Func]
    ) -> HashMap<String, GrayImage> {
        assert_eq!(inputs.len(), self.function.num_inputs());
        assert_eq!(funcs.len(), self.function.num_outputs());

        let mut func_sources: Vec<String> = funcs
            .iter()
            .flat_map(|f| f.sources())
            .collect();

        func_sources.sort();
        func_sources.dedup();

        let calculated_funcs: HashSet<String> = funcs
            .iter()
            .map(|f| f.name.clone())
            .collect();

        let input_funcs: HashSet<String> = inputs
            .iter()
            .map(|i| i.0.clone())
            .collect();

        for source in func_sources {
            if !calculated_funcs.contains(&source) && !input_funcs.contains(&source) {
                panic!(
                    "Required source {} is not calculated and is not provided as an input",
                    source
                );
            }
        }

        // Assume that all images are the same size for now. This will not be true in general
        let (w, h) = inputs[0].1.dimensions();
        
        let mut calculated_images: Vec<(String, GrayImage)> = funcs
            .iter()
            .map(|f| (f.name.clone(), GrayImage::new(w, h)))
            .collect();

        match self.function {
            FunctionPointer::OneInOneOut(f) => {
                let i0 = &inputs[0].1;
                let r0 = &mut calculated_images[0].1;
                f(
                    i0.buffer.as_ptr(), i0.width, i0.height,
                    r0.buffer.as_mut_ptr(), r0.width, r0.height
                );
            },
            FunctionPointer::OneInTwoOut(f) => {
                let i0 = &inputs[0].1;
                let (rl, rr) = calculated_images.split_at_mut(1);
                let r0 = &mut rl[0].1;
                let r1 = &mut rr[0].1;
                f(
                    i0.buffer.as_ptr(), i0.width, i0.height,
                    r0.buffer.as_mut_ptr(), r0.width, r0.height,
                    r1.buffer.as_mut_ptr(), r1.width, r1.height
                );
            },
            FunctionPointer::TwoInOneOut(f) => {
                let i0 = &inputs[0].1;
                let i1 = &inputs[0].1;
                let r0 = &mut calculated_images[0].1;
                f(
                    i0.buffer.as_ptr(), i0.width, i0.height,
                    i1.buffer.as_ptr(), i1.width, i1.height,
                    r0.buffer.as_mut_ptr(), r0.width, r0.height
                );
            },
            FunctionPointer::TwoInTwoOut(f) => {
                let i0 = &inputs[0].1;
                let i1 = &inputs[0].1;
                let (rl, rr) = calculated_images.split_at_mut(1);
                let r0 = &mut rl[0].1;
                let r1 = &mut rr[0].1;
                f(
                    i0.buffer.as_ptr(), i0.width, i0.height,
                    i1.buffer.as_ptr(), i1.width, i1.height,
                    r0.buffer.as_mut_ptr(), r0.width, r0.height,
                    r1.buffer.as_mut_ptr(), r1.width, r1.height
                );
            }
        };

        calculated_images.into_iter().collect()
    }
}

// Will do for now...
pub enum FunctionPointer {
    OneInOneOut(extern "C" fn(
        *const u8, usize, usize,
        *mut u8, usize, usize
    )),
    OneInTwoOut(extern "C" fn(
        *const u8, usize, usize,
        *mut u8, usize, usize,
        *mut u8, usize, usize
    )),
    TwoInOneOut(extern "C" fn(
        *const u8, usize, usize,
        *const u8, usize, usize,
        *mut u8, usize, usize
    )),
    TwoInTwoOut(extern "C" fn(
        *const u8, usize, usize,
        *const u8, usize, usize,
        *mut u8, usize, usize,
        *mut u8, usize, usize
    )),
}

impl FunctionPointer {
    fn num_inputs(&self) -> usize {
        self.signature().0
    }

    fn num_outputs(&self) -> usize {
        self.signature().1
    }

    // Returns (num_input, num_outputs)
    fn signature(&self) -> (usize, usize) {
        match self {
            FunctionPointer::OneInOneOut(_) => (1, 1),
            FunctionPointer::OneInTwoOut(_) => (1, 2),
            FunctionPointer::TwoInOneOut(_) => (2, 1),
            FunctionPointer::TwoInTwoOut(_) => (2, 2)
        }
    }
}