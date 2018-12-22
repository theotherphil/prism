
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
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
pub struct ActiveRegion {
    x: usize,
    y: usize,
    width: usize,
    height: usize
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

pub fn compute_tint(c: u8) -> u8 {
    (255 - c) / 3
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

    let current_image = combine(&initial_images, &layout);

    let mut frames = vec![];
    frames.push(current_image.clone());

    let red = [255, 0, 0];
    let green = [0, 255, 0];
    let black = [0, 0, 0];

    struct Tinter {
        current_image: RgbImage,
        layout: Layout,
        // Tinting isn't invertible due to saturation, so we need to track
        // the tints we're currently applying to each pixel in order to undo
        // them when a region becomes inactive. These coordinates are locations
        // in the combined image.
        active_tints: HashMap<TraceId, HashMap<(usize, usize), u8>>,
        // This will need to change when there can be multiple active regions
        // at once. The Image trait will also need to acquire a new function
        // to deactive a region.
        active_regions: HashMap<TraceId, ActiveRegion>
    };

    impl Tinter {
        fn get(&self, id: TraceId, x: usize, y: usize) -> [u8; 3] {
            let (x, y) = self.layout.apply_offset(id.0, x, y);
            self.current_image.get(x, y)
        }

        fn activate(&mut self, id: TraceId, region: ActiveRegion) {
            // Remove existing tints
            if let Some(tints) = self.active_tints.get(&id) {
                for ((x, y), tint) in tints {
                    let [r, g, b] = self.current_image.get(*x, *y);
                    self.current_image.set(*x, *y, [r, g, b - tint]);
                }
            };
            self.active_tints.remove(&id);
            // Add new tints
            let mut tints = HashMap::new();
            for y in 0..region.height {
                let ya = y + region.y;
                for x in 0..region.width {
                    let xa = x + region.x;
                    let (x, y) = self.layout.apply_offset(id.0, xa, ya);
                    let [r, g, b] = self.current_image.get(x, y);
                    let tint = compute_tint(b);
                    self.current_image.set(x, y, [r, g, b + tint]);
                    tints.insert((x, y), tint);
                }
            }
            self.active_tints.insert(id, tints);
            // Update the active region
            self.active_regions.insert(id, region);
        }

        fn set_with_tint(&mut self, id: TraceId, x: usize, y: usize, c: [u8; 3]) {
            let c = if let Some(region) = self.active_regions.get(&id) {
                let x_active = x >= region.x && x <= region.x + region.width;
                let y_active = y >= region.y && y <= region.y + region.height;

                if x_active && y_active {
                    let (x, y) = self.layout.apply_offset(id.0, x, y);
                    let tint = compute_tint(c[2]);
                    self.active_tints.get_mut(&id).unwrap().insert((x, y), tint);
                    [c[0], c[1], c[2] + tint]
                } else {
                    c
                }
            } else {
                c
            };
            let (x, y) = self.layout.apply_offset(id.0, x, y);
            self.current_image.set(x, y, c);
        }

        fn set_without_tint(&mut self, id: TraceId, x: usize, y: usize, c: [u8; 3]) {
            let (x, y) = self.layout.apply_offset(id.0, x, y);
            self.current_image.set(x, y, c);
        }

        fn frame(&self) -> RgbImage {
            self.current_image.clone()
        }
    };

    let mut tinter = Tinter {
        current_image: current_image,
        layout: layout,
        active_tints: HashMap::new(),
        active_regions: HashMap::new()
    };

    for action in trace.actions.borrow().iter() {
        match action {
            Action::Read(id, x, y) => {
                let (id, x, y) = (*id, *x, *y);
                let current = tinter.get(id, x, y);
                tinter.set_without_tint(id, x, y, green);
                frames.push(tinter.frame());
                tinter.set_without_tint(id, x, y, current);
                frames.push(tinter.frame());
            },
            Action::Write(id, x, y, c) => {
                let (id, x, y, c) = (*id, *x, *y, *c);
                tinter.set_without_tint(id, x, y, red);
                frames.push(tinter.frame());
                tinter.set_with_tint(id, x, y, [c, c, c]);
                frames.push(tinter.frame());
            },
            Action::Clear(id) => {
                let id = *id;
                let (w, h) = dimensions[id.0];
                for y in 0..h {
                    for x in 0..w {
                        tinter.set_without_tint(id, x, y, red);
                    }
                }
                frames.push(tinter.frame());
                for y in 0..h {
                    for x in 0..w {
                        tinter.set_without_tint(id, x, y, black);
                    }
                }
                frames.push(tinter.frame());
            },
            Action::Active(id, region) => {
                tinter.activate(*id, *region);
                frames.push(tinter.frame());
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
