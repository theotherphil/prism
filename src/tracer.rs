
use std::rc::Rc;
use std::cell::RefCell;
use crate::io::*;
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
pub enum Action {
    Read(TraceId, (usize, usize)),
    Write(TraceId, (usize, usize), u8),
    Clear(TraceId)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TraceId(usize);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trace {
    actions: RefCell<Vec<Action>>,
    initial_images: RefCell<Vec<GrayImage>>
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
        self.actions.borrow_mut().push(Action::Read(id, (x, y)));
    }

    pub fn trace_set(&self, id: TraceId, x: usize, y: usize, c: u8) {
        self.actions.borrow_mut().push(Action::Write(id, (x, y), c));
    }

    pub fn trace_clear(&self, id: TraceId) {
        self.actions.borrow_mut().push(Action::Clear(id));
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

pub fn upscale<T: Copy + Zero>(image: &ImageBuffer<T>, factor: u8) -> ImageBuffer<T> {
    let (w, h) = (factor as usize * image.width(), factor as usize * image.height());
    let mut result = ImageBuffer::new(w, h);
    for y in 0..h {
        for x in 0..w {
            result.set(x, y, image.get(x / factor as usize, y / factor as usize));
        }
    }
    result
}

pub fn replay(trace: &Trace) -> Vec<RgbImage> {
    // Determine how to embed the individual images into a single combined image
    let dimensions: Vec<(usize, usize)> = trace.initial_images
        .borrow()
        .iter()
        .map(|i| i.dimensions())
        .collect();
    
    let layout = layout(&dimensions, 1);

    // Arrange the initial images onto a single canvas
    let initial_images: Vec<RgbImage> = trace.initial_images
        .borrow()
        .iter()
        .map(|i| gray_to_rgb(i))
        .collect();

    let mut current_image = combine(&initial_images, &layout);

    let mut frames = vec![];
    frames.push(current_image.clone());

    let red = [255, 0, 0];
    let green = [0, 255, 0];
    let black = [0, 0, 0];

    for action in trace.actions.borrow().iter() {
        match action {
            Action::Read(id, (x, y)) => {
                let (x, y) = layout.apply_offset(id.0, *x, *y);
                let current = current_image.get(x, y);
                current_image.set(x, y, green);
                frames.push(current_image.clone());
                current_image.set(x, y, current);
                frames.push(current_image.clone());
            },
            Action::Write(id, (x, y), c) => {
                let (x, y) = layout.apply_offset(id.0, *x, *y);
                current_image.set(x, y, red);
                frames.push(current_image.clone());
                current_image.set(x, y, [*c, *c, *c]);
                frames.push(current_image.clone());
            },
            Action::Clear(id) => {
                let (w, h) = dimensions[id.0];
                for y in 0..h {
                    for x in 0..w {
                        let (xo, yo) = layout.apply_offset(id.0, x, y);
                        current_image.set(xo, yo, red);
                    }
                }
                frames.push(current_image.clone());
                for y in 0..h {
                    for x in 0..w {
                        let (xo, yo) = layout.apply_offset(id.0, x, y);
                        current_image.set(xo, yo, black);
                    }
                }
                frames.push(current_image.clone());
            }
        }
    }

    frames
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Layout {
    width: usize,
    height: usize,
    offsets: Vec<(usize, usize)>
}

impl Layout {
    fn apply_offset(&self, index: usize, original_x: usize, original_y: usize) -> (usize, usize) {
        (original_x + self.offsets[index].0, original_y + self.offsets[index].1)
    }
}

// Given a series of trace images, create a new image to contain them all and a mapping
// from locations in each image to the corresponding location in the combined image
fn layout(dimensions: &[(usize, usize)], margin: usize) -> Layout {
    assert!(dimensions.len() > 0);

    let height = dimensions.iter().map(|d| d.1).max().unwrap() + 2 * margin;
    let width = dimensions.iter().map(|d| d.0).sum::<usize>() + (dimensions.len() + 1) * margin;

    let mut offsets = vec![(margin, margin)];
    let mut left = 2 * margin + dimensions[0].0;

    for d in dimensions.iter().skip(1) {
        offsets.push((left, margin));
        left += d.0 + margin;
    }

    Layout { width, height, offsets }
}

fn combine(images: &[RgbImage], layout: &Layout) -> RgbImage {
    let mut result = RgbImage::new(layout.width, layout.height);

    let background = [120, 120, 120];
    for y in 0..result.height() {
        for x in 0..result.width() {
            result.set(x, y, background);
        }
    }

    for (n, image) in images.iter().enumerate() {
        let offset = layout.offsets[n];
        for y in 0..image.height() {
            for x in 0..image.width() {
                result.set(x + offset.0, y + offset.1, image.get(x, y));
            }
        }
    }
    result
}
