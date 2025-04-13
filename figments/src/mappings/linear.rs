use core::cmp::min;
use core::fmt::Debug;

use crate::geometry::*;
use crate::pixels::*;
use crate::render::Sample;

/// Linear coordinate space where Y is meaningless and X points to a unique pixel
#[derive(Debug, Clone, Copy)]
pub struct LinearSpace {}
impl CoordinateSpace for LinearSpace {
    type Data = usize;
}

impl<'a, Pixel: HardwarePixel + 'a, const SIZE: usize> Sample<'a, LinearSpace> for [Pixel; SIZE]{
    type Output = Pixel;

    fn sample(&mut self, rect: &Rectangle<LinearSpace>) -> impl Iterator<Item = (Coordinates<LinearSpace>, &'a mut Self::Output)> {
        let (_, rest) = self.split_at_mut(min(rect.left(), SIZE));
        let (subset, _) = rest.split_at_mut(min(rect.width(), rest.len()));
        let bufref = unsafe {
            (subset as *mut [Pixel]).as_mut().unwrap()
        };
        bufref.iter_mut().enumerate().map(|(idx, pix)| {
            (Coordinates::new(idx, 0), pix)
        })
    }
}