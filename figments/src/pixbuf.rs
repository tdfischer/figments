#![doc = "Pixel buffer types"]
use core::ops::IndexMut;
use super::render::HardwarePixel;

/// Types that provide access to a buffer of pixels, which may or may not be hardware based
/// 
/// This trait requires [IndexMut] so you can acccess individual pixels by index
pub trait Pixbuf: IndexMut<usize, Output=Self::Pixel> + Send {
    /// The underlying hardware pixel type
    type Pixel: HardwarePixel;
    /// Creates a new Pixbuf that may or may not contain default pixel values (eg, black)
    fn new() -> Self;

    /// Blanks the pixels, usually to black
    fn blank(&mut self);

    /// Returns the number of pixels accessable through this buffer
    fn pixel_count(&self) -> usize;
}

impl<T: HardwarePixel, const PIXEL_NUM: usize> Pixbuf for [T; PIXEL_NUM] {
    type Pixel = T;
    fn new() -> Self {
        [T::default(); PIXEL_NUM]
    }

    fn pixel_count(&self) -> usize {
        self.len()
    }

    fn blank(&mut self) {
        self.fill(T::default())
    }
}