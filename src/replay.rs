//! Functions for creating animations visualising traces of image reads and writes

use crate::buffer::*;
use crate::traits::*;
use crate::tracer::*;
use crate::io::*;
use std::collections::HashMap;

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

pub fn create_gif_palette() -> GifPalette {
    let mut palette = vec![];
    // Greyscale pixels where each value has an even intensity no more than 250u8
    for i in 0..126u8 {
        palette.extend([2 * i, 2 * i, 2 * i].iter().cloned());
    }
    // Their blue-tinted equivalents
    for i in 0..126u8 {
        let tint = compute_tint(2 * i);
        palette.extend([2 * i, 2 * i, 2 * i + tint].iter().cloned());
    }
    // Red, green, blue, yellow
    palette.extend([255, 0, 0].iter().cloned());
    palette.extend([0, 255, 0].iter().cloned());
    palette.extend([0, 255, 255].iter().cloned());
    palette.extend([255, 255, 0].iter().cloned());

    let compute_palette_index = |p: [u8; 3]| {
        if p == [255u8, 0, 0] {
            252
        }
        else if p == [0, 255u8, 0] {
            253
        }
        else if p == [0, 0, 255u8] {
            254
        }
        else if p == [255u8, 255u8, 0] {
            255
        }
        else if p[0] == p[1] && p[1] == p[2] && p[0] <= 250  {
            // Round down to even values in each channel
            p[0] / 2
        }
        else if p[0] == p[1] {
            // Check if this is a blue-tinted version of an accepted greyscale value
            let t = compute_tint(p[0]);
            let b = p[0] + t;
            if b == p[2] && p[0] <= 250 {
                p[0] / 2 + 126
            } else {
                panic!("Invalid trace image RGB value {:?}", p)
            }
        }
        else {
            panic!("Invalid trace image RGB value {:?}", p)
        }
    };

    GifPalette::new(&palette, Box::new(compute_palette_index))
}

fn compute_tint(c: u8) -> u8 {
    (255 - c) / 3
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



