use rgb::{Rgb, Rgba};

use crate::{liber8tion::interpolate::Fract8, prelude::Fract8Ops};

/// Types that represent RAM based pixel formats
pub trait PixelFormat: PixelBlend<Self> + Send + Copy + Default {}
impl<T> PixelFormat for T where T: PixelBlend<Self> + Send + Copy + Default {}

/// Types that can blend the value of another pixel
pub trait PixelBlend<OverlayPixel: PixelFormat> {
    /// Blend a given pixel as an overlay by a given percentage
    fn blend_pixel(self, overlay: OverlayPixel, opacity: Fract8) -> Self;

    /// Blend a pixel by multiplying it by another pixel
    fn multiply(self, overlay: OverlayPixel) -> Self;
}

pub trait PixelSrc<Output> {
    fn get_pixel(&self) -> Output;
}

pub trait PixelSink<Src> {
    fn set(&mut self, pixel: &Src);
}

impl<Src> PixelSink<Src> for Src where Src: Clone {
    fn set(&mut self, pixel: &Src) {
        *self = pixel.clone();
    }
}

impl<Output> PixelSrc<Output> for Output where Output: Clone {
    fn get_pixel(&self) -> Output {
        self.clone()
    }
}

impl PixelSink<Rgba<u8>> for Rgb<u8> {
    fn set(&mut self, pixel: &Rgba<u8>) {
        *self = pixel.rgb()
    }
}

impl PixelBlend<Rgb<u8>> for Rgb<u8> {
    fn blend_pixel(self, overlay: Rgb<u8>, opacity: Fract8) -> Self {
        self.blend8(overlay, opacity)
    }
    
    fn multiply(self, overlay: Rgb<u8>) -> Self {
        Self::new(
            self.r.scale8(overlay.r),
            self.g.scale8(overlay.g),
            self.b.scale8(overlay.b),
        )
    }
}

impl PixelBlend<Rgba<u8>> for Rgba<u8> {
    fn blend_pixel(self, overlay: Rgba<u8>, opacity: Fract8) -> Self {
        self.blend8(overlay, opacity)
    }
    
    fn multiply(self, overlay: Rgba<u8>) -> Self {
        Self::new(
            self.r.scale8(overlay.r),
            self.g.scale8(overlay.g),
            self.b.scale8(overlay.b),
            self.a.scale8(overlay.a)
        )
    }
}

impl PixelBlend<Rgba<u8>> for Rgb<u8> {
    fn blend_pixel(self, overlay: Rgba<u8>, opacity: Fract8) -> Self {
        self.blend8(Rgb::new(overlay.r, overlay.g, overlay.b), overlay.a.scale8(opacity))
    }
    
    fn multiply(self, overlay: Rgba<u8>) -> Self {
        Self::new(
            self.r.scale8(overlay.r),
            self.g.scale8(overlay.g),
            self.b.scale8(overlay.b),
        )
    }
}

impl PixelBlend<Rgba<u8>> for bool {
    fn blend_pixel(self, overlay: Rgba<u8>, opacity: Fract8) -> Self {
        if opacity >= 128 {
            (overlay.r as u32 + overlay.g as u32 + overlay.b as u32) >= 128
        } else {
            self
        }
    }
    
    fn multiply(self, overlay: Rgba<u8>) -> Self {
        overlay.iter().map(|x| { x as u16 }).sum::<u16>() / 4 >= 128
    }
}

#[cfg(feature="embedded-graphics")]
mod embedded_impl {
    use embedded_graphics::pixelcolor::BinaryColor;
    use super::*;

    impl PixelBlend<BinaryColor> for BinaryColor {
        fn blend_pixel(self, overlay: BinaryColor, opacity: Fract8) -> Self {
            overlay
        }

        fn multiply(self, overlay: BinaryColor) -> Self {
            overlay
        }
    }
}