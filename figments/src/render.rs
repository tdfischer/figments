//! The core rendering engine types
use rgb::Rgb;

use core::fmt::Debug;
use super::geometry::*;

use crate::liber8tion::interpolate::Fract8Ops;

pub trait Renderable<U, Space: CoordinateSpace> {
    /// Draws the surfaces to the given sampler
    fn render_to<'a, S: Sample<'a, Space = Space>>(&self, output: &mut S, uniforms: &U);
}

/// Types that represent hardware-backed pixel formats (eg, RGB888, BGR888, etc)
pub trait HardwarePixel: Send + Sync + Copy + Default + From<Rgb<u8>> + Fract8Ops {}
impl<T> HardwarePixel for T where T: Send + Sync + Copy + Default + From<Rgb<u8>> + Fract8Ops {}

/// Types that can provide direct hardware access to individual pixels within a given [Virtual] rectangle shaped selection for reading and writing
pub trait Sample<'a> {
    type Space: CoordinateSpace;
    /// The type of pixel this sampler supports
    type Pixel: HardwarePixel + 'a;

    /// The iterator retuned by this sampler
    type PixelIterator: Iterator<Item = (Coordinates<Self::Space>, &'a mut Self::Pixel)> + Debug;

    /// Provides a [PixelView] over the given [Rectangle] selection
    fn sample(&mut self, rect: &Rectangle<Self::Space>) -> Self::PixelIterator;
}

/// Function type that can provide an RGB color given a location in [Virtual] space and global rendering state
pub trait Shader<Uniforms, Space: CoordinateSpace>: Send + 'static {
    /// Turns a [Virtual] coordinate into a real pixel color
    fn draw(&self, surface_coords: &Coordinates<Space>, uniforms: &Uniforms) -> Rgb<u8>;
}

impl<T, U, Space: CoordinateSpace> Shader<U, Space> for T where T: 'static + Send + Fn(&Coordinates<Space>, &U) -> Rgb<u8> {
    fn draw(&self, surface_coords: &Coordinates<Space>, uniforms: &U) -> Rgb<u8> {
        self(surface_coords, uniforms)
    }
}