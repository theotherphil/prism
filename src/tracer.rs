use crate::image::*;
use crate::io::*;
use std::rc::Rc;
use std::cell::{Cell, RefCell};

/// Records the set of trace images, so that reads and writes can be ordered
/// across multiple images.
pub struct Tracer {
    /// Counts the number of calls to get or set for any image produced by this Tracer.
    count: Rc<Cell<usize>>,
    store: Vec<Rc<RefCell<TraceImage>>>
}

impl Storage for Tracer {
    type Image = TraceImage;

    fn create_image(&mut self, width: usize, height: usize) -> Rc<RefCell<TraceImage>> {
        self.add_image(TraceImage::new(self.count.clone(), width, height))
    }

    fn images(self) -> Vec<TraceImage> {
        self.store.into_iter().map(|i| Rc::try_unwrap(i).unwrap().into_inner()).collect()
    }
}

impl Tracer {
    pub fn new() -> Tracer {
        Tracer {
            count: Rc::new(Cell::new(0)),
            store: vec![]
        }
    }

    pub fn create_from_image(&mut self, image: &GrayImage) -> Rc<RefCell<TraceImage>> {
        self.add_image(TraceImage::from_image(self.count.clone(), image))
    }

    fn add_image(&mut self, image: TraceImage) -> Rc<RefCell<TraceImage>> {
        let image = Rc::new(RefCell::new(image));
        self.store.push(image.clone());
        image
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Read(usize, (usize, usize)),
    Write(usize, (usize, usize), u8),
    Clear(usize)
}

impl Action {
    fn step_count(&self) -> usize {
        match self {
            Action::Read(n, _) => *n,
            Action::Write(n, _, _) => *n,
            Action::Clear(n) => *n
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TraceImage {
    count: Rc<Cell<usize>>,
    initial_image: GrayImage,
    current_image: GrayImage,
    trace: RefCell<Vec<Action>>
}

impl Image<u8> for TraceImage {
    fn width(&self) -> usize {
        self.initial_image.width()
    }

    fn height(&self) -> usize {
        self.initial_image.height()
    }

    fn get(&self, x: usize, y: usize) -> u8 {
        self.trace.borrow_mut().push(Action::Read(self.incr_count(), (x, y)));
        self.current_image.get(x, y)
    }

    fn set(&mut self, x: usize, y: usize, c: u8) {
        self.trace.borrow_mut().push(Action::Write(self.incr_count(), (x, y), c));
        self.current_image.set(x, y, c);
    }

    fn clear(&mut self) {
        self.trace.borrow_mut().push(Action::Clear(self.incr_count()));
        self.current_image.clear();
    }

    fn data(&self) -> &[u8] {
        self.current_image.data()
    }
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
            trace: RefCell::new(vec![])
        }
    }

    fn incr_count(&self) -> usize {
        (*self.count).set(self.count.get() + 1);
        self.count.get()
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

pub fn replay(images: &[TraceImage]) -> Vec<RgbImage> {
    // Determine how to embed the individual images into a single combined image
    let dimensions: Vec<(usize, usize)> = images.iter()
        .map(|t| t.initial_image.dimensions())
        .collect();
    let layout = layout(&dimensions, 1);

    // Combine traces and label each action with the index of the image to which it applies
    let mut full_trace: Vec<(usize, Action)> = images.iter()
        .enumerate()
        .flat_map(|(n, img)| img.trace.borrow().iter().map(move |t| (n, t.clone())).collect::<Vec<_>>())
        .collect();
    full_trace.sort_by_key(|e| e.1.step_count());

    // Arrange the initial images onto a single canvas
    let initial_images: Vec<RgbImage> = images.iter()
        .map(|i| gray_to_rgb(&i.initial_image))
        .collect();
    let mut current_image = combine(&initial_images, &layout);

    let mut frames = vec![];
    frames.push(current_image.clone());

    let red = [255, 0, 0];
    let green = [0, 255, 0];
    let black = [0, 0, 0];

    for (image_index, action) in &full_trace {
        let offset = layout.offsets[*image_index];

        match action {
            Action::Read(_, (x, y)) => {
                let x = *x + offset.0;
                let y = *y + offset.1;

                let current = current_image.get(x, y);
                current_image.set(x, y, green);
                frames.push(current_image.clone());

                current_image.set(x, y, current);
                frames.push(current_image.clone());
            },
            Action::Write(_, (x, y), c) => {
                let x = *x + offset.0;
                let y = *y + offset.1;

                current_image.set(x, y, red);
                frames.push(current_image.clone());

                current_image.set(x, y, [*c, *c, *c]);
                frames.push(current_image.clone());
            },
            Action::Clear(_) => {
                let (w, h) = dimensions[*image_index];

                for y in 0..h {
                    for x in 0..w {
                        current_image.set(x + offset.0, y + offset.1, red);
                    }
                }
                frames.push(current_image.clone());
                for y in 0..h {
                    for x in 0..w {
                        current_image.set(x + offset.0, y + offset.1, black);
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
