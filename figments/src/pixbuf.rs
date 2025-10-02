#![doc = "Pixel buffer types"]
use crate::pixels::*;

/// Types that provide access to a buffer of pixels, which may or may not be hardware based
/// 
/// This trait requires [IndexMut] so you can acccess individual pixels by index
pub trait Pixbuf {
    /// The underlying hardware pixel type
    type Format: HardwarePixel;

    /// Blanks the pixels, usually to black
    fn blank(&mut self);

    /// Returns the number of pixels accessable through this buffer
    fn pixel_count(&self) -> usize;
}

impl<T: HardwarePixel, const PIXEL_NUM: usize> Pixbuf for [T; PIXEL_NUM] where for<'a> T: 'a {
    type Format = T;

    fn pixel_count(&self) -> usize {
        self.len()
    }

    fn blank(&mut self) {
        self.fill(T::default())
    }
}