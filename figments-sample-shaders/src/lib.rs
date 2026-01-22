#![no_std]
use figments::prelude::*;
use figments::liber8tion::trig::*;
use figments::liber8tion::noise::*;
use core::cmp::max;
use rgb::*;

#[cfg(feature="micromath")]
use micromath::F32Ext;

#[derive(Default, Debug)]
pub struct FrameNumber(pub usize);

#[derive(Default, Debug)]
pub struct RgbWaves {}

impl<Space: CoordinateSpace<Data = usize>> Shader<FrameNumber, Space, Rgb<u8>> for RgbWaves {
    fn draw(&self, coords: &Coordinates<Space>, frame: &FrameNumber) -> Rgb<u8> {
        // Scroll the entire pattern sideways, so it repeats less often
        let offset_x = coords.x.wrapping_add(frame.0 / 30);
        // The color is just some simple wave functions with varying frequencies, with the Y coordinate as a phase offset
        Rgb::new(
            offset_x.wrapping_mul(3).wrapping_add(frame.0).sin8().wrapping_add(coords.y as u8),
            offset_x.wrapping_mul(5).wrapping_sub(frame.0).cos8().wrapping_add(coords.y as u8),
            offset_x.wrapping_mul(2).wrapping_add(frame.0).sin8().wrapping_add(coords.y as u8)
        )
    }
}


#[derive(Default, Debug)]
pub struct Thinking {}

impl<Space: CoordinateSpace<Data = usize>> Shader<FrameNumber, Space, Rgb<u8>> for Thinking {
    fn draw(&self, coords: &Coordinates<Space>, uniforms: &FrameNumber) -> Rgb<u8> {
        //let noise_x = sin8(sin8((frame % 255) as u8).wrapping_add(coords.x));
        //let noise_y = cos8(cos8((frame % 255) as u8).wrapping_add(coords.y));
        let offset_x = uniforms.0.wrapping_add(coords.x).sin8();
        let offset_y = uniforms.0.wrapping_add(coords.y).cos8();
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
        let noise_y = uniforms.0.sin8();
        let noise_x = uniforms.0.cos8();

        let brightness = inoise8((noise_x.wrapping_add(coords.x as u8)).into(), (noise_y.wrapping_add(coords.y as u8)).into());

        // Saturation will be +/- 15 from the requested color
        let saturation_min = self.color.saturation.saturating_sub(15);
        let saturation_shift = 30.scale8(inoise8((noise_y.wrapping_add(coords.y as u8)).into(), (noise_x.wrapping_add(coords.x as u8)).into()));
        let saturation = saturation_min.saturating_add(saturation_shift);

        Hsv::new(self.color.hue.wrapping_add(16.scale8(uniforms.0.sin8())).wrapping_sub(8), saturation, brightness).into()
    }
}

#[derive(Default, Debug)]
pub struct RainbowSpiralShader {}
impl Shader<FrameNumber, Virtual, Rgba<u8>> for RainbowSpiralShader {
    fn draw(&self, coords: &VirtualCoordinates, uniforms: &FrameNumber) -> Rgba<u8> {
        let distance = (128f32 - coords.y as f32).hypot(128f32 - coords.x as f32);
        let angle = (((128f32 - coords.y as f32).atan2(128f32 - coords.x as f32)) * 255f32) as u8;
        let pixel_value = angle.wrapping_add((uniforms.0 % 255) as u8).wrapping_add(distance as u8);

        Rgba::new(pixel_value.sin8(), pixel_value.wrapping_add(64).sin8(), pixel_value.wrapping_add(128).sin8(), 255)
    }
}

#[derive(Default, Debug)]
pub struct Chimes {}
impl<Space: CoordinateSpace<Data = usize>, Pixel> Shader<FrameNumber, Space, Pixel> for Chimes where Hsv: Into<Pixel> {
    fn draw(&self, surface_coords: &Coordinates<Space>, uniforms: &FrameNumber) -> Pixel {
        const CHIME_LENGTH: usize = 8;

        let animation_frame = uniforms.0 / 5;
        let local_x = surface_coords.x.wrapping_add(animation_frame / 300);

        let chime_idx = (local_x / CHIME_LENGTH) % 32;
        let chime_pos = local_x % CHIME_LENGTH;

        let brightness = (animation_frame.wrapping_mul(chime_idx + 1) / 3).sin8();
        let saturation = (chime_pos.wrapping_add(animation_frame / 3)).sin8();
        let hue = chime_idx.wrapping_add(animation_frame / 30) as u8;

        Hsv::new(
            hue,
            saturation,
            brightness
        ).into()
    }
}

#[derive(Default, Debug)]
pub struct Flashlight {}

impl<Pixel, Space: CoordinateSpace<Data = usize>> Shader<FrameNumber, Space, Pixel> for Flashlight where Hsv: Into<Pixel> {
    fn draw(&self, coords: &Coordinates<Space>, uniforms: &FrameNumber) -> Pixel {
        let noise_y = uniforms.0.sin8();
        let noise_x = uniforms.0.cos8();

        let brightness = inoise8((noise_x.wrapping_add(coords.x as u8)).into(), (noise_y.wrapping_add(coords.y as u8)).into());
        let saturation = inoise8((noise_y.wrapping_add(coords.y as u8)).into(), (noise_x.wrapping_add(coords.x as u8)).into());
        let hue = 16u8.scale8(uniforms.0.sin8());

        Hsv::new(hue, max(128, saturation), brightness).into()
    }
}