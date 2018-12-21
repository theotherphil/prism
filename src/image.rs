
use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;

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

impl Zero for [u8; 3] {
    fn zero() -> [u8; 3] {
        [0, 0, 0]
    }
}

pub trait Storage {
    type Image: Image<u8>;
    fn create_image(&mut self, width: usize, height: usize) -> Rc<RefCell<Self::Image>>;
    fn images(self) -> Vec<Self::Image>;
}

pub struct BufferStore {
    store: Vec<Rc<RefCell<GrayImage>>>
}

impl BufferStore {
    pub fn new() -> BufferStore {
        BufferStore { store: vec![] }
    }

    pub fn create_from_image(&mut self, image: &GrayImage) -> Rc<RefCell<GrayImage>> {
        self.add_image(image)
    }

    pub fn clear(&mut self) {
        self.store.clear();
    }

    fn add_image(&mut self, image: &GrayImage) -> Rc<RefCell<GrayImage>> {
        let image = Rc::new(RefCell::new(image.clone()));
        self.store.push(image.clone());
        image
    }
}

impl Storage for BufferStore {
    type Image = GrayImage;

    fn create_image(&mut self, width: usize, height: usize) -> Rc<RefCell<GrayImage>> {
        self.add_image(&GrayImage::new(width, height))
    }

    fn images(self) -> Vec<GrayImage> {
        self.store.into_iter().map(|i| Rc::try_unwrap(i).unwrap().into_inner()).collect()
    }
}

pub trait Image<T> {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    
    fn dimensions(&self) -> (usize, usize) {
        (self.width(), self.height())
    }

    fn get(&self, x: usize, y: usize) -> T;
    fn set(&mut self, x: usize, y: usize, c: T);
    fn clear(&mut self);
    fn data(&self) -> &[T];
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
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn data(&self) -> &[T] {
        &self.buffer
    }

    fn clear(&mut self) {
        for e in &mut self.buffer {
            *e = T::zero();
        }
    }

    fn get(&self, x: usize, y: usize) -> T {
        unsafe { *self.buffer.get_unchecked(y * self.width + x) }
    }

    fn set(&mut self, x: usize, y: usize, c: T) {
        unsafe { *self.buffer.get_unchecked_mut(y * self.width + x) = c; }
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
