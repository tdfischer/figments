#![no_std]
use figments::prelude::*;
use figments::liber8tion::trig::*;
use figments::liber8tion::noise::*;
use figments::liber8tion::interpolate::*;
use core::cmp::{max, min};
use rgb::*;

#[cfg(feature="micromath")]
use micromath::F32Ext;

#[derive(Default, Debug)]
pub struct FrameNumber(pub usize);

#[derive(Default, Debug)]
pub struct RgbWaves {}

impl<Space: CoordinateSpace<Data = usize>> Shader<FrameNumber, Space, Rgb<u8>> for RgbWaves {
    fn draw(&self, coords: &Coordinates<Space>, frame: &FrameNumber) -> Rgb<u8> {
        Rgb::new(
            sin8(coords.x.wrapping_mul(3).wrapping_add(frame.0)).wrapping_add(coords.y as u8),
            cos8(coords.x.wrapping_mul(5).wrapping_sub(frame.0)).wrapping_add(coords.y as u8),
            sin8(coords.x.wrapping_mul(2).wrapping_add(frame.0)).wrapping_add(coords.y as u8)
        )
    }
}


#[derive(Default, Debug)]
pub struct Thinking {}

impl<Space: CoordinateSpace<Data = usize>> Shader<FrameNumber, Space, Rgb<u8>> for Thinking {
    fn draw(&self, coords: &Coordinates<Space>, uniforms: &FrameNumber) -> Rgb<u8> {
        //let noise_x = sin8(sin8((frame % 255) as u8).wrapping_add(coords.x));
        //let noise_y = cos8(cos8((frame % 255) as u8).wrapping_add(coords.y));
        let offset_x = sin8(uniforms.0.wrapping_add(coords.x));
        let offset_y = cos8(uniforms.0.wrapping_add(coords.y));
        let noise_x = offset_x / 2;
        let noise_y = offset_y / 2;
        //let noise_x = coords.x.wrapping_add(offset_x);
        //let noise_y = coords.y.wrapping_add(offset_y);
        Hsv::new(
            inoise8(offset_x as i16, offset_y as i16),
            128_u8.saturating_add(inoise8(noise_y.into(), noise_x.into())),
            255
        ).into()
    }
}

#[derive(Default, Debug)]
pub struct ColorGlow {
    pub color: Hsv
}

impl<Space: CoordinateSpace<Data = usize>, Pixel> Shader<FrameNumber, Space, Pixel> for ColorGlow where Hsv: Into<Pixel> {
    fn draw(&self, coords: &Coordinates<Space>, uniforms: &FrameNumber) -> Pixel {
        let noise_y = sin8(uniforms.0);
        let noise_x = cos8(uniforms.0);

        let brightness = inoise8((noise_x.wrapping_add(coords.x as u8)).into(), (noise_y.wrapping_add(coords.y as u8)).into());
        let saturation = min(self.color.saturation, inoise8((noise_y.wrapping_add(coords.y as u8)).into(), (noise_x.wrapping_add(coords.x as u8)).into()));

        Hsv::new(self.color.hue.wrapping_add(scale8(16, sin8(uniforms.0))).wrapping_sub(8), max(128, saturation), brightness).into()
    }
}

#[derive(Default, Debug)]
pub struct RainbowSpiralShader {}
impl Shader<FrameNumber, Virtual, Rgba<u8>> for RainbowSpiralShader {
    fn draw(&self, coords: &VirtualCoordinates, uniforms: &FrameNumber) -> Rgba<u8> {
        let distance = (128f32 - coords.y as f32).hypot(128f32 - coords.x as f32);
        let angle = (((128f32 - coords.y as f32).atan2(128f32 - coords.x as f32)) * 255f32) as u8;
        let pixel_value = angle.wrapping_add((uniforms.0 % 255) as u8).wrapping_add(distance as u8);

        Rgba::new(sin8(pixel_value), sin8(pixel_value.wrapping_add(64)), sin8(pixel_value.wrapping_add(128)), 255)
    }
}