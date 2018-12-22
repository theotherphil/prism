
use std::fmt;
use crate::traits::*;

/// Trivial factory that just calls GrayImage::new
pub struct BufferFactory {}

impl BufferFactory {
    pub fn new() -> BufferFactory {
        BufferFactory { }
    }
}

impl Factory for BufferFactory {
    type Image = GrayImage;

    fn create_image(&mut self, width: usize, height: usize) -> GrayImage {
        GrayImage::new(width, height)
    }
}

/// For now we'll only consider greyscale images
// TODO: derived Eq checks for buffer equality, but we only care about
// TODO: the initial segment of length width * height
#[derive(Clone, PartialEq, Eq)]
pub struct ImageBuffer<T> {
    width: usize,
    height: usize,
    buffer: Vec<T>
}

impl<T: Zero + Copy + Clone> Image<T> for ImageBuffer<T> {
    #[inline]
    fn width(&self) -> usize {
        self.width
    }

    #[inline]
    fn height(&self) -> usize {
        self.height
    }

    #[inline]
    fn data(&self) -> &[T] {
        &self.buffer
    }

    fn clear(&mut self) {
        for e in &mut self.buffer {
            *e = T::zero();
        }
    }

    #[inline]
    fn get(&self, x: usize, y: usize) -> T {
        unsafe { *self.buffer.get_unchecked(y * self.width + x) }
    }

    #[inline]
    fn set(&mut self, x: usize, y: usize, c: T) {
        unsafe { *self.buffer.get_unchecked_mut(y * self.width + x) = c; }
    }

    fn active(&self, _: usize, _: usize, _: usize, _: usize) {
        // Do nothing
    }
}

pub type GrayImage = ImageBuffer<u8>;
// This is a stupid representation, but it'll do for now
pub type RgbImage = ImageBuffer<[u8; 3]>;

impl<T: Zero + Clone> ImageBuffer<T> {
    pub fn new(width: usize, height: usize) -> ImageBuffer<T> {
        let buffer = vec![T::zero(); width * height];
        ImageBuffer { width, height, buffer }
    }

    pub fn from_raw(width: usize, height: usize, buffer: Vec<T>) -> ImageBuffer<T> {
        assert!(buffer.len() >= width * height);
        ImageBuffer { width, height, buffer }
    }
}

impl<T: fmt::Debug + Zero + Copy + Clone> fmt::Debug for ImageBuffer<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Image(width: {:?}, height: {:?}, buffer: {{", self.width, self.height)?;
        for y in 0..self.height() {
            write!(f, "  ")?;
            for x in 0..self.width() {
                write!(f, "{:?}", self.get(x, y))?;
                if x < self.width() - 1 {
                    write!(f, ", ")?;
                }
            }
            writeln!(f, "")?;
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}

#[macro_export]
macro_rules! gray_image {
    // Empty image with default channel type u8
    () => {
        gray_image!(type: u8)
    };
    // Empty image with the given channel type
    (type: $channel_type:ty) => {
        {
            ImageBuffer { width: 0, height: 0, buffer: vec![] }
        }
    };
    // Non-empty image of default channel type u8
    ($( $( $x: expr ),*);*) => {
        gray_image!(type: u8, $( $( $x ),*);*)
    };
    // Non-empty image of given channel type
    (type: $channel_type:ty, $( $( $x: expr ),*);*) => {
        {
            let nested_array = [ $( [ $($x),* ] ),* ];
            let height = nested_array.len();
            let width = nested_array[0].len();

            let buffer: Vec<_> = nested_array.into_iter()
                .flat_map(|row| row.into_iter())
                .cloned()
                .collect();

            ImageBuffer { width, height, buffer }
        }
    }
}
