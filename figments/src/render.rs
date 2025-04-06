//! The core rendering engine types
use core::fmt::Debug;
use super::geometry::*;

use crate::liber8tion::interpolate::Fract8Ops;

pub trait Renderable<'a, U, Space: CoordinateSpace, Pixel: PixelFormat> {
    /// Draws the surfaces to the given sampler
    fn render_to<S: Sample<'a, Space = Space, Pixel = Pixel>>(&self, output: &mut S, uniforms: &U);
}

pub trait PixelFormat: Send + Sync + Copy + Default {}
impl<T> PixelFormat for T where T: Send + Sync + Copy + Default {}

/// Types that represent hardware-backed pixel formats (eg, RGB888, BGR888, etc)
pub trait HardwarePixel: PixelFormat + Fract8Ops {}
impl<T> HardwarePixel for T where T: PixelFormat + Fract8Ops {}

/// Types that can provide direct hardware access to individual pixels within a given [Virtual] rectangle shaped selection for reading and writing
pub trait Sample<'a> {
    type Space: CoordinateSpace;
    /// The type of pixel this sampler supports
    type Pixel: PixelFormat + 'a;

    /// The iterator retuned by this sampler
    type PixelIterator: Iterator<Item = (Coordinates<Self::Space>, &'a mut Self::Pixel)> + Debug;

    /// Provides a [PixelView] over the given [Rectangle] selection
    fn sample(&mut self, rect: &Rectangle<Self::Space>) -> Self::PixelIterator;
}

/// Function type that can provide an RGB color given a location in [Virtual] space and global rendering state
pub trait Shader<Uniforms, Space: CoordinateSpace, Pixel: PixelFormat>: Send + 'static {
    /// Turns a [Virtual] coordinate into a real pixel color
    fn draw(&self, surface_coords: &Coordinates<Space>, uniforms: &Uniforms) -> Pixel;
}

impl<T, U, Space: CoordinateSpace, Pixel: PixelFormat> Shader<U, Space, Pixel> for T where T: 'static + Send + Fn(&Coordinates<Space>, &U) -> Pixel {
    fn draw(&self, surface_coords: &Coordinates<Space>, uniforms: &U) -> Pixel {
        self(surface_coords, uniforms)
    }
}