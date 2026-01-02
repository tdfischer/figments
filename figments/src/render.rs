//! The core rendering engine types
use super::geometry::*;

use crate::pixels::*;

/// Types that can provide direct hardware access to individual pixels within a given [Virtual] rectangle shaped selection for reading and writing
pub trait Sample<'a, Space: CoordinateSpace> {

    /// The type of pixel this sampler supports
    type Output: 'a;

    //FIXME: Moving 'a into sample<'a>() and type Iterator<'a>: Iterator<...> would allow implementations without unsafe {} blocks on basic arrays
    /// Provides a [PixelView] over the given [Rectangle] selection
    fn sample(&mut self, rect: &Rectangle<Space>) -> impl Iterator<Item = (Coordinates<Space>, &'a mut Self::Output)>;
}

/// Function type that can provide an RGB color given a location in [Virtual] space and global rendering state
pub trait Shader<Uniforms, Space: CoordinateSpace, Pixel>: Send {
    /// Turns a [Virtual] coordinate into a real pixel color
    fn draw(&self, surface_coords: &Coordinates<Space>, uniforms: &Uniforms) -> Pixel;
}

/// Types that can push pixels into samplers
pub trait RenderSource<Uniforms, Space: CoordinateSpace, Src, Dst> {
    /// Draws this source's pixels into the sampler
    fn render_to<'a, Smp>(&'a self, output: &'a mut Smp, uniforms: &Uniforms)
        where 
            Smp: Sample<'a, Space, Output = Dst>;
}

impl<T, U, Space: CoordinateSpace, Pixel> Shader<U, Space, Pixel> for T where T: Send + Fn(&Coordinates<Space>, &U) -> Pixel {
    fn draw(&self, surface_coords: &Coordinates<Space>, uniforms: &U) -> Pixel {
        self(surface_coords, uniforms)
    }
}

/// Types which can draw a shader over some pre-defined geometrical regions
pub trait Painter<U, Space: CoordinateSpace, Input> {
    /// Draws the shader over the entire area, eg Rectangle::everything()
    fn fill(&mut self, shader: &impl Shader<U, Space, Input>, uniforms: &U);

    /// Draws the shader over a given rectangle
    fn paint(&mut self, shader: &impl Shader<U, Space, Input>, uniforms: &U, rect: &Rectangle<Space>);
}

impl<'a, U, Space: CoordinateSpace, Input: 'static, T> Painter<U, Space, Input> for T where T: Sample<'a, Space>, T::Output: PixelSink<Input> {
    fn fill(&mut self, shader: &impl Shader<U, Space, Input>, uniforms: &U) {
        self.paint(shader, uniforms, &Rectangle::everything());
    }

    fn paint(&mut self, shader: &impl Shader<U, Space, Input>, uniforms: &U, rect: &Rectangle<Space>) {
        for (coords, pixel) in self.sample(rect) {
            pixel.set(&shader.draw(&coords, uniforms));
        }
    }
}