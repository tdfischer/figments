//! The core rendering engine types
use core::fmt::Debug;
use rgb::{Rgb, Rgba};

use super::geometry::*;

use crate::liber8tion::interpolate::{Fract8, Fract8Ops};

/// Types that can blend the values of two pixels together (eg, overlaying RGBA8 on top of plain RGB8)
pub trait PixelBlend<OverlayPixel: PixelFormat> {
    /// Blend a given pixel as an overlay by a given percentage
    fn blend_pixel(self, overlay: OverlayPixel, opacity: Fract8) -> Self;
}


impl PixelBlend<Rgb<u8>> for Rgb<u8> {
    fn blend_pixel(self, overlay: Rgb<u8>, opacity: Fract8) -> Self {
        self.blend8(overlay, opacity)
    }
}

impl PixelBlend<Rgba<u8>> for Rgb<u8> {
    fn blend_pixel(self, overlay: Rgba<u8>, opacity: Fract8) -> Self {
        self.blend8(Rgb::new(overlay.r, overlay.g, overlay.b), overlay.a.scale8(opacity))
    }
}


/// Types that can draw something to a Sampler's pixels
pub trait Renderable<'a, U, Space: CoordinateSpace, Pixel: PixelFormat> {
    /// Draws the surfaces to the given sampler
    fn render_to<S: Sample<'a, Space = Space, Pixel = Pixel>>(&self, output: &mut S, uniforms: &U);
}

/// Types that represent software or hardware based pixel formats
pub trait PixelFormat: Send + Sync + Copy + Default {}
impl<T> PixelFormat for T where T: Send + Sync + Copy + Default {}

/// Types that represent hardware-backed pixel formats (eg, RGB888, BGR888, etc)
pub trait HardwarePixel: PixelFormat + Fract8Ops + Debug {}
impl<T> HardwarePixel for T where T: PixelFormat + Fract8Ops + Debug {}

/// Types that can provide direct hardware access to individual pixels within a given [Virtual] rectangle shaped selection for reading and writing
pub trait Sample<'a> {
    /// The coordinate space supported by this sampler
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

/// Types which can draw a shader over some pre-defined geometrical regions
pub trait Painter<U, Space: CoordinateSpace, Pixel: PixelFormat> {
    /// Draws the shader over the entire area, eg Rectangle::everything()
    fn fill(&mut self, shader: &impl Shader<U, Space, Pixel>, uniforms: &U);

    /// Draws the shader over a given rectangle
    fn draw(&mut self, shader: &impl Shader<U, Space, Pixel>, uniforms: &U, rect: &Rectangle<Space>);
}

// FIXME: We need to be able to split input formats from output formats here, so the BufferedSurfacePool can use this trait
impl<'a, U, Space: CoordinateSpace, Pixel: PixelFormat + 'static, T> Painter<U, Space, Pixel> for T where T: Sample<'a, Space = Space, Pixel = Pixel>, T::Pixel: PixelBlend<Pixel> {
    fn fill(&mut self, shader: &impl Shader<U, Space, Pixel>, uniforms: &U) {
        self.draw(shader, uniforms, &Rectangle::everything());
    }

    fn draw(&mut self, shader: &impl Shader<U, Space, Pixel>, uniforms: &U, rect: &Rectangle<Space>) {
        for (coords, pixel) in self.sample(rect) {
            *pixel = pixel.blend_pixel(shader.draw(&coords, uniforms), 255);
        }
    }
}