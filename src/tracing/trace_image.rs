//! Image and Factory implementations that record all reads and writes to a shared `Trace`.

use std::rc::Rc;
use crate::{image::*, tracing::*};

pub struct TraceImageFactory {
    pub trace: Rc<Trace>
}

impl Factory for TraceImageFactory {
    type Image = TraceImage;

    fn create_image(&mut self, width: usize, height: usize) -> TraceImage {
        TraceImage::new(self.trace.clone(), width, height)
    }
}

impl TraceImageFactory {
    pub fn new() -> TraceImageFactory {
        TraceImageFactory {
            trace: Rc::new(Trace::new())
        }
    }

    pub fn create_from_image(&mut self, image: &GrayImage) -> TraceImage {
        TraceImage::from_image(self.trace.clone(), image)
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TraceImage {
    image: GrayImage,
    trace_id: TraceId,
    trace: Rc<Trace>
}

impl Image<u8> for TraceImage {
    fn width(&self) -> usize {
        self.image.width()
    }

    fn height(&self) -> usize {
        self.image.height()
    }

    fn get(&self, x: usize, y: usize) -> u8 {
        self.trace.trace_get(self.trace_id, x, y);
        self.image.get(x, y)
    }

    fn set(&mut self, x: usize, y: usize, c: u8) {
        self.trace.trace_set(self.trace_id, x, y, c);
        self.image.set(x, y, c);
    }

    fn clear(&mut self) {
        self.trace.trace_clear(self.trace_id);
        self.image.clear();
    }

    fn data(&self) -> &[u8] {
        self.image.data()
    }

    fn active(&self, x: usize, y: usize, active_width: usize, active_height: usize) {
        self.trace.trace_active(self.trace_id, x, y, active_width, active_height);
    }
}

impl TraceImage {
    pub fn new(trace: Rc<Trace>, width: usize, height: usize) -> TraceImage {
        Self::from_image(trace, &GrayImage::new(width, height))
    }

    pub fn from_image(trace: Rc<Trace>, image: &GrayImage) -> TraceImage {
        TraceImage {
            image: image.clone(),
            trace_id: trace.create_trace_id(image),
            trace: trace.clone()
        }
    }
}
