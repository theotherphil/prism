
use std::fmt;
use std::ops::{Index, IndexMut};

pub trait Zero {
    fn zero() -> Self;
}

macro_rules! impl_zero {
    ($($t:ty),*) => {
        $(
            impl Zero for $t {
                fn zero() -> Self {
                    0
                }
            }
        )*
    }
}

impl_zero!(u8, i8, u16, i16, u32, i32, u64, i64);

/// For now we'll only consider greyscale images
// TODO: derived Eq checks for buffer equality, but we only care about
// TODO: the initial segment of length width * height
#[derive(Clone, PartialEq, Eq)]
pub struct Image<T> {
    pub width: usize,
    pub height: usize,
    pub buffer: Vec<T>
}

impl<T: Zero + Clone> Image<T> {
    pub fn new(width: usize, height: usize) -> Image<T> {
        let buffer = vec![T::zero(); width * height];
        Image { width, height, buffer }
    }
}

impl<T> Image<T> {
    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }
}

impl<T> Index<[usize; 2]> for Image<T> {
    type Output = T;

    fn index(&self, index: [usize; 2]) -> &T {
        &self.buffer[index[1] * self.width + index[0]]
    }
}

impl<T> IndexMut<[usize; 2]> for Image<T> {
    fn index_mut(&mut self, index: [usize; 2]) -> &mut T {
        &mut self.buffer[index[1] * self.width + index[0]]
    }
}

impl<T: fmt::Debug> fmt::Debug for Image<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Image(width: {:?}, height: {:?}, buffer: {{", self.width, self.height)?;
        for y in 0..self.height() {
            write!(f, "  ")?;
            for x in 0..self.width() {
                write!(f, "{:?}", self[[x, y]])?;
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
            Image { width: 0, height: 0, buffer: vec![] }
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

            Image { width, height, buffer }
        }
    }
}
