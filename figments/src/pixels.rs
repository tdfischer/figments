use rgb::{ComponentSlice, Rgb, Rgba};
use core::fmt::Debug;

use crate::{liber8tion::interpolate::Fract8, prelude::Fract8Ops};

/// Types that represent software or hardware based pixel formats
pub trait PixelFormat: PixelBlend<Self> + Send + Sync + Copy + Default {}
impl<T> PixelFormat for T where T: PixelBlend<Self> + Send + Sync + Copy + Default {}

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

impl PixelBlend<Rgba<u8>> for bool {
    fn blend_pixel(self, overlay: Rgba<u8>, opacity: Fract8) -> Self {
        if opacity >= 128 {
            (overlay.r as u32 + overlay.g as u32 + overlay.b as u32) >= 128
        } else {
            self
        }
    }
}