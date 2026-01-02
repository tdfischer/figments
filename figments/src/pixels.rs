use rgb::{Grb, Rgb, Rgba};

use crate::{liber8tion::interpolate::Fract8, prelude::Fract8Ops};

/// Types that represent RAM based pixel formats
pub trait PixelFormat: PixelBlend<Self> + Send + Copy + Default {}
impl<T> PixelFormat for T where T: PixelBlend<Self> + Send + Copy + Default {}

/// Types that can blend the value of another pixel
pub trait PixelBlend<OverlayPixel> {
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

pub trait AdditivePixelSink<Src> {
    fn add(&mut self, pixel: Src, opacity: Fract8);
}

/*impl<T: Copy + Fract8Ops + From<X>, X> AdditivePixelSink<X> for T {
    #[inline]
    fn add(&mut self, pixel: X, opacity: Fract8) {
        *self = self.blend8(From::from(pixel), opacity);
    }
}*/

impl AdditivePixelSink<Rgb<u8>> for Grb<u8> {
    #[inline(always)]
    fn add(&mut self, pixel: Rgb<u8>, opacity: Fract8) {
        match opacity {
            0 => (),
            255 => *self = pixel.into(),
            _ => *self = self.blend8(pixel.into(), opacity)
        }
    }
}

impl AdditivePixelSink<Rgba<u8>> for Rgb<u8> {
    #[inline(always)]
    fn add(&mut self, pixel: Rgba<u8>, opacity: Fract8) {
        *self = self.blend8(Self::new(pixel.r, pixel.g, pixel.b), pixel.a.scale8(opacity))
    }
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

impl PixelSink<Rgb<u8>> for Grb<u8> {
    fn set(&mut self, pixel: &Rgb<u8>) {
        *self = pixel.clone().into()
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