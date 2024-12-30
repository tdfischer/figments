//! The core rendering engine types
use rgb::Rgb;
use core::fmt::Debug;

use super::geometry::*;

use crate::liber8tion::interpolate::Fract8Ops;

/// Types that represent hardware pixel formats (eg, RGB888, BGR888, etc)
pub trait HardwarePixel: Send + Sync + Copy + Default + From<Rgb<u8>> + Fract8Ops {}
impl<T> HardwarePixel for T where T: Send + Sync + Copy + Default + From<Rgb<u8>> + Fract8Ops {}

/// Similiar to a [CoordinateView], but it maps [Virtual] coordinates to hardware pixels for writing
pub trait PixelView {
    /// The underlying hardware pixel type
    type Pixel: HardwarePixel;

    /// Returns the next pixel in this view, or None otherwise
    fn next(&mut self) -> Option<(Coordinates<Virtual>, &mut Self::Pixel)>;
}

/// Types that can provide direct hardware access to individual pixels within a given [Virtual] rectangle shaped selection
pub trait Sample {
    /// The underlying hardware pixel type
    type Pixel: HardwarePixel;

    /// Provides a [PixelView] over the given [Rectangle] selection
    fn sample(&mut self, rect: &Rectangle<Virtual>) -> impl PixelView<Pixel = Self::Pixel>;
}

/// Types that can provide an RGB color given a location in [Virtual] space
pub trait Shader: Send + 'static {
    /// Turns a [Virtual] coordinate into a real pixel color
    fn draw(&self, surface_coords: &VirtualCoordinates, frame: usize) -> Rgb<u8>;
}

impl<T> Shader for T where T: 'static + Send + Fn(&VirtualCoordinates, usize) -> Rgb<u8> {
    fn draw(&self, surface_coords: &VirtualCoordinates, frame: usize) -> Rgb<u8> {
        self(surface_coords, frame)
    }
}

/// Types that can provide [Surface]s and render their surfaces to a [Sample]-able type
pub trait Surfaces: Send {
    type Surface: Surface;
    type Error: Debug;
    fn new_surface(&mut self, area: Rectangle<Virtual>) -> Result<Self::Surface, Self::Error>;
    fn render_to<S: Sample>(&self, output: &mut S, frame: usize);
}

/// Helper trait for allowing some [Surface] properties to be set when they are in a slice or array 
pub trait Visible {
    /// Sets the opacity of this surface, where 0 is completely transparent and 255 is completely opaque
    fn set_opacity(&mut self, opacity: u8);

    /// Sets the visibility of the surface without adjusting the stored opacity
    fn set_visible(&mut self, visible: bool);
}

impl<T: Visible> Visible for [T] {
    fn set_opacity(&mut self, opacity: u8) {
        for v in self.iter_mut() {
            v.set_opacity(opacity);
        }
    }

    fn set_visible(&mut self, visible: bool) {
        for v in self.iter_mut() {
            v.set_visible(visible);
        }
    }
}

/// Builder pattern API for creating surfaces
pub struct SurfaceBuilder<'a, S: Surface, SS: Surfaces<Surface = S>, SF: Shader> {
    surfaces: &'a mut SS,
    rect: Option<Rectangle<Virtual>>,
    opacity: Option<u8>,
    shader: Option<SF>,
    visible: Option<bool>
}

impl<'a, S: Surface, SS: Surfaces<Surface = S>, SF: Shader> SurfaceBuilder<'a, S, SS, SF> {
    /// Starts building a surface
    pub fn build(surfaces: &'a mut SS) -> Self {
        Self {
            surfaces,
            opacity: None,
            shader: None,
            rect: None,
            visible: None
        }
    }

    /// Sets the initial opacity
    pub fn opacity(mut self, opacity: u8) -> Self {
        self.opacity = Some(opacity);
        self
    }

    /// Sets the initial size of the surface
    pub fn rect(mut self, rect: Rectangle<Virtual>) -> Self {
        self.rect = Some(rect);
        self
    }

    /// Sets the shader attached to the surface
    pub fn shader(mut self, shader: SF) -> Self {
        self.shader = Some(shader);
        self
    }

    /// Sets the initial visibility of the surface
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = Some(visible);
        self
    }

    /// Constructs the surface
    pub fn finish(self) -> Result<S, SS::Error> {
        let sfc = self.surfaces.new_surface(match self.rect {
            None => Rectangle::everything(),
            Some(r) => r
        });

        match sfc {
            Ok(mut s) => {
                if self.opacity.is_some() {
                    s.set_opacity(self.opacity.unwrap());
                }
                if self.shader.is_some() {
                    s.set_shader(self.shader.unwrap());
                }
                if self.visible.is_some() {
                    s.set_visible(self.visible.unwrap());
                }

                Ok(s)
            },
            err => err
        }
    }
}

/// A rectangular set of pixels that can be drawn on with a [Shader]
pub trait Surface: Send + Visible {
    /// Sets the shader for this surface
    fn set_shader<T: Shader>(&mut self, shader: T);

    /// Clears the shader
    fn clear_shader(&mut self);

    /// Changes the size of the surface
    fn set_rect(&mut self, rect: Rectangle<Virtual>);
}