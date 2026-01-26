use rgb::{Rgb, Rgba, Bgr, Bgra, Grb};

use crate::{liber8tion::interpolate::Fract8, prelude::Fract8Ops};

/// Types that can add the color of another pixel to itself
pub trait AdditivePixelSink<Src> {
    /// Blend a given pixel as an overlay by a given percentage
    fn add(&mut self, pixel: Src, opacity: Fract8);
}

macro_rules! rgb_pixel_sink {
    ($dest_pixel:ident $src_pixel:ident) => {
        impl AdditivePixelSink<$src_pixel<u8>> for $dest_pixel<u8> {
            #[inline(always)]
            fn add(&mut self, pixel: $src_pixel<u8>, opacity: Fract8) {
                match opacity {
                    Fract8::MIN => (),
                    Fract8::MAX => *self = Self { r: pixel.r, g: pixel.g, b: pixel.b },
                    _ => *self = self.blend8(Self { r: pixel.r, g: pixel.g, b: pixel.b }, opacity)
                }
            }
        }
    };
}

macro_rules! rgba_pixel_sink {
    ($dest_pixel:ident $src_pixel:ident) => {
        impl AdditivePixelSink<$src_pixel<u8>> for $dest_pixel<u8> {
            #[inline(always)]
            fn add(&mut self, pixel: $src_pixel<u8>, opacity: Fract8) {
                match opacity {
                    Fract8::MIN => (),
                    Fract8::MAX => *self = Self { r: pixel.r, g: pixel.g, b: pixel.b },
                    _ => *self = self.blend8(Self { r: pixel.r, g: pixel.g, b: pixel.b }, Fract8::from_raw(pixel.a * opacity))
                }
            }
        }
    };
}

rgb_pixel_sink!(Grb Grb);
rgb_pixel_sink!(Grb Rgb);
rgb_pixel_sink!(Grb Bgr);
rgba_pixel_sink!(Grb Rgba);
rgba_pixel_sink!(Grb Bgra);

rgb_pixel_sink!(Rgb Rgb);
rgb_pixel_sink!(Rgb Grb);
rgb_pixel_sink!(Rgb Bgr);
rgba_pixel_sink!(Rgb Rgba);
rgba_pixel_sink!(Rgb Bgra);

rgb_pixel_sink!(Bgr Bgr);
rgb_pixel_sink!(Bgr Rgb);
rgb_pixel_sink!(Bgr Grb);
rgba_pixel_sink!(Bgr Rgba);
rgba_pixel_sink!(Bgr Bgra);