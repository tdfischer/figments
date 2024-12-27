use rgb::Rgb;
use running_average::RealTimeRunningAverage;
use core::fmt::Debug;

use super::geometry::*;

use crate::lib8::interpolate::Fract8Ops;
use crate::microtick::events::*;
use crate::microtick::properties::*;
use crate::microtick::task::Environment;
use crate::microtick::task::Task;
use crate::time::Periodically;

pub trait HardwarePixel: Send + Sync + Copy + Default + From<Rgb<u8>> + Fract8Ops {}
impl<T> HardwarePixel for T where T: Send + Sync + Copy + Default + From<Rgb<u8>> + Fract8Ops {}

pub trait PixelView {
    type Pixel: HardwarePixel;
    fn next(&mut self) -> Option<(Coordinates<Virtual>, &mut Self::Pixel)>;
}

pub trait Sample {
    type Pixel: HardwarePixel;

    fn sample(&mut self, rect: &Rectangle<Virtual>) -> impl PixelView<Pixel = Self::Pixel>;
}

pub trait Shader: Send + 'static {
    fn draw(&self, surface_coords: &VirtualCoordinates, frame: usize) -> Rgb<u8>;
}

impl<T> Shader for T where T: 'static + Send + Fn(&VirtualCoordinates, usize) -> Rgb<u8> {
    fn draw(&self, surface_coords: &VirtualCoordinates, frame: usize) -> Rgb<u8> {
        self(surface_coords, frame)
    }
}

pub trait Surfaces: Send {
    type Surface: Surface;
    type Error: Debug;
    fn new_surface(&mut self, area: Rectangle<Virtual>) -> Result<Self::Surface, Self::Error>;
    fn render_to<S: Sample>(&self, output: &mut S, frame: usize);
}

pub trait Visible {
    fn set_opacity(&mut self, opacity: u8);
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

pub struct SurfaceBuilder<'a, S: Surface, SS: Surfaces<Surface = S>, SF: Shader> {
    surfaces: &'a mut SS,
    rect: Option<Rectangle<Virtual>>,
    opacity: Option<u8>,
    shader: Option<SF>,
    visible: Option<bool>
}

impl<'a, S: Surface, SS: Surfaces<Surface = S>, SF: Shader> SurfaceBuilder<'a, S, SS, SF> {
    pub fn build(surfaces: &'a mut SS) -> Self {
        Self {
            surfaces,
            opacity: None,
            shader: None,
            rect: None,
            visible: None
        }
    }

    pub fn opacity(mut self, opacity: u8) -> Self {
        self.opacity = Some(opacity);
        self
    }

    pub fn rect(mut self, rect: Rectangle<Virtual>) -> Self {
        self.rect = Some(rect);
        self
    }

    pub fn shader(mut self, shader: SF) -> Self {
        self.shader = Some(shader);
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = Some(visible);
        self
    }

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

pub trait Surface: Send + Visible {
    fn set_shader<T: Shader>(&mut self, shader: T);

    fn clear_shader(&mut self);

    fn set_rect(&mut self, rect: Rectangle<Virtual>);
}

pub trait Output: Sample + Send {
    fn on_event(&mut self, event: &Event);
    fn blank(&mut self);
    fn commit(&mut self);
}

#[derive(Debug)]
pub struct Renderer<T: Output, S: Surfaces> {
    output: T,
    surfaces: S,
    fps: RealTimeRunningAverage<u32>,
    fps_display: Periodically,
    frame: usize,
    frame_count: usize
}

impl<T: Output, S: Surfaces> Renderer<T, S> {
    pub fn new(output: T, surfaces: S) -> Self {
        Self {
            output,
            surfaces: surfaces,
            fps: RealTimeRunningAverage::default(),
            fps_display: Periodically::new_every_n_seconds(5),
            frame: 0,
            frame_count: 0
        }
    }
}

impl<T: Output, S: Surfaces> Task for Renderer<T, S> {
    fn on_property_change(&mut self, key: PropertyID, value: &Variant, _env: &mut Environment) {
        self.output.on_event(&Event::new_property_change(key, value.clone()));
    }

    fn on_tick(&mut self, env: &mut Environment) {
        self.output.blank();

        self.surfaces.render_to(&mut self.output, self.frame);

        self.output.commit();

        self.frame += 1;
        self.fps_display.run(|| {
            self.fps.insert((self.frame - self.frame_count) as u32);
            self.frame_count = self.frame;
            let fps = self.fps.measurement();
            env.set_property(super::render::props::Output::FPS, fps.rate() as u32);
        });
    }
}

pub mod props {
    use crate::property_namespace;

    property_namespace!(
        Output,
        FPS => 0,
        Brightness => 255
    );
}