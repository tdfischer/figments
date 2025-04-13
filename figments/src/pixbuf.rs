#![doc = "Pixel buffer types"]
use core::iter::Enumerate;
use core::ops::IndexMut;
use core::fmt::Debug;
use core::slice::Iter;
use crate::prelude::{CoordinateSpace, Coordinates};
use crate::render::Sample;
use crate::pixels::*;

/// Types that provide access to a buffer of pixels, which may or may not be hardware based
/// 
/// This trait requires [IndexMut] so you can acccess individual pixels by index
pub trait Pixbuf: Send + Debug {
    /// The underlying hardware pixel type
    type Format: HardwarePixel;
    /// Creates a new Pixbuf that may or may not contain default pixel values (eg, black)
    fn new() -> Self;

    /// Blanks the pixels, usually to black
    fn blank(&mut self);

    /// Returns the number of pixels accessable through this buffer
    fn pixel_count(&self) -> usize;
}

impl<T: HardwarePixel, const PIXEL_NUM: usize> Pixbuf for [T; PIXEL_NUM] where for<'a> T: 'a {
    type Format = T;
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