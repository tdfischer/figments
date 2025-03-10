//! The core rendering engine types
use rgb::Rgb;

use super::geometry::*;

use crate::liber8tion::interpolate::Fract8Ops;

/// Types that represent hardware pixel formats (eg, RGB888, BGR888, etc)
pub trait HardwarePixel: Send + Sync + Copy + Default + From<Rgb<u8>> + Fract8Ops {}
impl<T> HardwarePixel for T where T: Send + Sync + Copy + Default + From<Rgb<u8>> + Fract8Ops {}

/// Types that can provide direct hardware access to individual pixels within a given [Virtual] rectangle shaped selection
pub trait Sample<'a> {
    /// The underlying hardware pixel type
    type Pixel: HardwarePixel + 'a;
    type PixelView: Iterator<Item = (Coordinates<Virtual>, &'a mut Self::Pixel)>;

    /// Provides a [PixelView] over the given [Rectangle] selection
    //fn sample(&mut self, rect: &Rectangle<Virtual>) -> impl PixelView<Pixel = Self::Pixel>;
    fn sample(&mut self, rect: &Rectangle<Virtual>) -> Self::PixelView;
}

pub trait Renderable {
    type Uniforms;
    type Pixel: HardwarePixel;
    fn render_to<'a, S: Sample<'a, Pixel = Self::Pixel>>(&self, output: &mut S, uniforms: &Self::Uniforms);
}

/// Function type that can provide an RGB color given a location in [Virtual] space and global rendering state
pub trait Shader<Uniforms>: Send + 'static {
    /// Turns a [Virtual] coordinate into a real pixel color
    fn draw(&self, surface_coords: &VirtualCoordinates, uniforms: &Uniforms) -> Rgb<u8>;
}

impl<T, U> Shader<U> for T where T: 'static + Send + Fn(&VirtualCoordinates, &U) -> Rgb<u8> {
    fn draw(&self, surface_coords: &VirtualCoordinates, uniforms: &U) -> Rgb<u8> {
        self(surface_coords, uniforms)
    }
}