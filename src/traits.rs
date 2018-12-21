
use std::rc::Rc;
use std::cell::RefCell;

/// Allocates images. This trait exists solely so that we can track
/// all allocated images when producing tracing visualisations.
pub trait Storage {
    type Image: Image<u8>;
    fn create_image(&mut self, width: usize, height: usize) -> Rc<RefCell<Self::Image>>;
    fn images(self) -> Vec<Self::Image>;
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

/// Any type with a "zero" value - used when initialising and clearing images
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
