//! A `Trace` is a record of actions performed on images, to use for debugging or
//! generating animated replays.

use std::cell::RefCell;
use crate::image::GrayImage;

/// Used to highlight an image region when generating visualisations.
/// Currently only used in the hand-written blur3x3 examples and doesn't
/// have a precise definition in terms of schedules.
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
    pub fn new() -> Trace {
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
