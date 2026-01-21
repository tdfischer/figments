use core::ops::BitOr;

use rgb::*;

/// An alias for u8 to indicate that the value is a fraction from 0-255 where 0 is 0% and 255 is 100%
pub type Fract8 = u8;

macro_rules! fract8_color_impl {
    ($color_type:tt $($component:ident),+) => {
        impl<T> Fract8Ops for $color_type<T> where T: Fract8Ops {
            #[inline(always)]
            fn scale8(self, scale: Fract8) -> Self {
                Self {
                    $($component: self.$component.scale8(scale)),*
                }
            }

            #[inline(always)]
            fn blend8(self, other: Self, scale: Fract8) -> Self {
                Self {
                    $($component: self.$component.blend8(other.$component, scale)),*
                }
            }

            #[inline(always)]
            fn saturating_add(self, other: Self) -> Self {
                Self {
                    $($component: self.$component.saturating_add(other.$component)),*
                }
            }

            #[inline(always)]
            fn lerp8by8(self, other: Self, scale: Fract8) -> Self {
                Self {
                    $($component: self.$component.lerp8by8(other.$component, scale)),*
                }
            }
        }
    };
}

fract8_color_impl!(Rgb r,g,b);
fract8_color_impl!(Grb g,r,b);
fract8_color_impl!(Bgr b,g,r);
fract8_color_impl!(Rgba r,g,b,a);
fract8_color_impl!(Bgra r,g,b,a);
fract8_color_impl!(GrayA a,v);

pub trait Fract8Ops {
    fn scale8(self, scale: Fract8) -> Self;
    fn blend8(self, other: Self, scale: Fract8) -> Self;
    fn saturating_add(self, other: Self) -> Self;
    fn lerp8by8(self, other: Self, scale: Fract8) -> Self;
}

impl Fract8Ops for bool {
    #[inline]
    fn scale8(self, scale: Fract8) -> Self {
        self && scale >= 128
    }
    
    fn blend8(self, other: Self, scale: Fract8) -> Self {
        if scale >= 128 {
            self && other
        } else {
            self
        }
    }
    
    fn saturating_add(self, other: Self) -> Self {
        self || other
    }
    
    fn lerp8by8(self, other: Self, scale: Fract8) -> Self {
        if scale >= 128 {
            other
        } else {
            self
        }
    }
}

impl Fract8Ops for u8 {
    #[inline(always)]
    fn scale8(self, scale: Fract8) -> Self {
        (self as f32 * (scale as f32 / 255f32)) as u8
    }

    #[inline(always)]
    fn blend8(self, other: Self, scale: Fract8) -> Self {
        match scale {
            0 => self,
            255 => other,
            _ => (((self as u16).wrapping_shl(8).bitor(other as u16)).wrapping_add(other as u16 * scale as u16).wrapping_sub(self as u16 * scale as u16)).wrapping_shr(8) as u8
        }
    }

    #[inline(always)]
    fn saturating_add(self, other: Self) -> Self {
        self.saturating_add(other)
    }
    
    #[inline(always)]
    fn lerp8by8(self, other: Self, scale: Fract8) -> Self {
        if other > self {
            let delta = other - self;
            let scaled = scale8(delta, scale);
            self + scaled
        } else {
            let delta = self - other;
            let scaled = scale8(delta, scale);
            self - scaled
        }
    }
}

impl Fract8Ops for usize {
    #[inline]
    fn scale8(self, scale: Fract8) -> Self {
        (self as u8).scale8(scale) as usize
    }

    #[inline]
    fn blend8(self, other: Self, scale: Fract8) -> Self {
        (self as u8).blend8(other as u8, scale) as usize
    }

    #[inline]
    fn saturating_add(self, other: Self) -> Self {
        (self as usize).saturating_add(other)
    }
    
    fn lerp8by8(self, other: Self, scale: Fract8) -> Self {
        if other > self {
            let delta = other - self;
            let scaled = scale8(delta, scale);
            self + scaled
        } else {
            let delta = self - other;
            let scaled = scale8(delta, scale);
            self - scaled
        }
    }
}

#[cfg(feature="embedded-graphics")]
mod embedded_impl {
    use embedded_graphics::pixelcolor::BinaryColor;

    use super::Fract8Ops;
    impl Fract8Ops for BinaryColor {
        fn scale8(self, scale: super::Fract8) -> Self {
            self
        }
    
        fn blend8(self, other: Self, scale: super::Fract8) -> Self {
            self
        }
    
        fn saturating_add(self, other: Self) -> Self {
            self
        }
    
        fn lerp8by8(self, other: Self, scale: super::Fract8) -> Self {
            self
        }
    }
}

#[inline(always)]
pub fn scale8<T: Fract8Ops>(i: T, scale: Fract8) -> T {
    i.scale8(scale)
}

#[inline]
pub fn avg7(i: i8, j: i8) -> i8 {
    i.wrapping_shr(1).wrapping_add(j.wrapping_shr(1)).wrapping_add(i & 0x1)
}

pub fn grad8(hash: u8, x: i8, y: i8) -> i8 {
    let mut u: i8;
    let mut v: i8;

    if hash & 4 != 0 {
        u = y; v = x;
    } else {
        u = x; v = y;
    }

    if hash & 1 != 0 {
        u = u.wrapping_neg();
    }
    if hash & 2 != 0 {
        v = v.wrapping_neg();
    }

    avg7(u, v)
}

pub fn lerp7by8(a: i8, b: i8, frac: u8) -> i8 {
    if b > a {
        let delta: u8 = b.wrapping_sub(a) as u8;
        let scaled: u8 = scale8(delta, frac);
        a.wrapping_add(scaled as i8)
    } else {
        let delta: u8 = a.wrapping_sub(b) as u8;
        let scaled: u8 = scale8(delta, frac);
        a.wrapping_sub(scaled as i8)
    }
}

pub fn lerp8by8<T: Fract8Ops>(a: T, b: T, frac: u8) -> T {
    a.lerp8by8(b, frac)
}

pub fn map8(x: u8, range_start: u8, range_end: u8) -> u8 {
    let range_width = range_end - range_start;
    let mut out = scale8(x, range_width);
    out += range_start;
    out
}

pub fn ease_in_out_quad(i: u8) -> u8 {
    let j = if i & 0x80 != 0 {
        255 - i
    } else {
        i
    };
    let jj = scale8(j, j);
    let jj2 = jj.wrapping_shl(1);
    if i & 0x80 == 0 {
        jj2
    } else {
        255 - jj2
    }
}