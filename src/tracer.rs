use crate::image::*;
use crate::io::*;
use std::rc::Rc;
use std::cell::Cell;

/// Records the set of trace images, so that reads and writes can be ordered
/// across multiple images.
pub struct Tracer {
    count: Rc<Cell<usize>>
}

impl Tracer {
    pub fn new() -> Tracer {
        Tracer {
            count: Rc::new(Cell::new(0))
        }
    }

    pub fn create_new(&mut self, name: &'static str, width: usize, height: usize) -> TraceImage {
        TraceImage::new(self.count.clone(), width, height)
    }

    pub fn create_from_image(&mut self, name: &'static str, image: &GrayImage) -> TraceImage {
        TraceImage::from_image(self.count.clone(), image)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Read(usize, (usize, usize)),
    Write(usize, (usize, usize), u8)
}

pub struct TraceImage {
    count: Rc<Cell<usize>>,
    initial_image: GrayImage,
    current_image: GrayImage,
    pub trace: Vec<Action>
}

impl TraceImage {
    pub fn new(count: Rc<Cell<usize>>, width: usize, height: usize) -> TraceImage {
        Self::from_image(count, &GrayImage::new(width, height))
    }

    pub fn from_image(count: Rc<Cell<usize>>, image: &GrayImage) -> TraceImage {
        TraceImage {
            count: count,
            initial_image: image.clone(),
            current_image: image.clone(),
            trace: vec![]
        }
    }

    pub fn get(&mut self, x: usize, y: usize) -> u8 {
        self.trace.push(Action::Read(self.incr_count(), (x, y))); // reading requires mutable access... use a RefCell?
        self.current_image[[x, y]]
    }

    pub fn set(&mut self, x: usize, y: usize, c: u8) {
        self.trace.push(Action::Write(self.incr_count(), (x, y), c));
        self.current_image[[x, y]] = c;
    }

    pub fn incr_count(&self) -> usize {
        (*self.count).set(self.count.get() + 1);
        self.count.get()
    }
}

pub fn upscale<T: Copy + Zero>(image: &Image<T>, factor: u8) -> Image<T> {
    let (w, h) = (factor as usize * image.width(), factor as usize * image.height());
    let mut result = Image::new(w, h);
    for y in 0..h {
        for x in 0..w {
            result[[x, y]] = image[[x / factor as usize, y / factor as usize]];
        }
    }
    result
}

// This needs to return counts in order to sync multiple images
// The flash for a read or write should occur on the count of the action
pub fn replay(image: &TraceImage, scale_factor: u8) -> Vec<RgbImage> {
    let mut current_image = gray_to_rgb(&image.initial_image);

    let mut frames = vec![];
    frames.push(upscale(&current_image, scale_factor));

    for action in &image.trace {
        match action {
            Action::Read(_, (x, y)) => { 
                let current = current_image[[*x, *y]];
                current_image[[*x, *y]] = [0, 255, 0];
                frames.push(upscale(&current_image, scale_factor));

                current_image[[*x, *y]] = current;
                frames.push(upscale(&current_image, scale_factor));
            },
            Action::Write(_, (x, y), c) => {
                current_image[[*x, *y]] = [255, 0, 0];
                frames.push(upscale(&current_image, scale_factor));

                current_image[[*x, *y]] = [*c, *c, *c];
                frames.push(upscale(&current_image, scale_factor));
            }
        }
    }

    frames
}
