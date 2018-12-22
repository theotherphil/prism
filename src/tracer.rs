//! Image and Factory implementations that trace all reads and traces to an image, to
//! allow replay visualisations to be created.

use std::rc::Rc;
use std::cell::RefCell;
use crate::traits::*;
use crate::buffer::*;

pub struct Tracer {
    pub trace: Rc<Trace>
}

impl Factory for Tracer {
    type Image = TraceImage;

    fn create_image(&mut self, width: usize, height: usize) -> TraceImage {
        TraceImage::new(self.trace.clone(), width, height)
    }
}

impl Tracer {
    pub fn new() -> Tracer {
        Tracer {
            trace: Rc::new(Trace::new())
        }
    }

    pub fn create_from_image(&mut self, image: &GrayImage) -> TraceImage {
        TraceImage::from_image(self.trace.clone(), image)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ActiveRegion {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// A pixel in an image has been read from
    Read(TraceId, usize, usize),
    /// A pixel in an image has been written to
    Write(TraceId, usize, usize, u8),
    /// The contents of an image have been cleared
    Clear(TraceId),
    /// An area of an image is now "active" - this
    /// concept is only used for generating more
    /// helpful visualisations
    Active(TraceId, ActiveRegion)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TraceId(pub usize);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trace {
    pub actions: RefCell<Vec<Action>>,
    pub initial_images: RefCell<Vec<GrayImage>>
}

impl Trace {
    fn new() -> Trace {
        Trace {
            actions: RefCell::new(vec![]),
            initial_images: RefCell::new(vec![])
        }
    }

    pub fn create_trace_id(&self, initial_image: &GrayImage) -> TraceId {
        let id = TraceId(self.initial_images.borrow().len());
        self.initial_images.borrow_mut().push(initial_image.clone());
        id
    }

    pub fn trace_get(&self, id: TraceId, x: usize, y: usize) {
        self.actions.borrow_mut().push(Action::Read(id, x, y));
    }

    pub fn trace_set(&self, id: TraceId, x: usize, y: usize, c: u8) {
        self.actions.borrow_mut().push(Action::Write(id, x, y, c));
    }

    pub fn trace_clear(&self, id: TraceId) {
        self.actions.borrow_mut().push(Action::Clear(id));
    }

    pub fn trace_active(&self, id: TraceId, x: usize, y: usize, width: usize, height: usize) {
        self.actions.borrow_mut().push(Action::Active(id, ActiveRegion { x, y, width, height }));
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
