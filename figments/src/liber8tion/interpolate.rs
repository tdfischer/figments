use core::ops::{Add, BitOr, Div, Mul, Sub};

use num::traits::{WrappingAdd, WrappingMul};
use rgb::*;

use crate::liber8tion::trig::Trig8;

/// An alias for u8 to indicate that the value is a fraction from 0-255 where 0 is 0% and 255 is 100%
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Fract8(u8);

impl core::fmt::Display for Fract8 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Fract8 {
    pub const MAX: Fract8 = Fract8(u8::MAX);
    pub const MIN: Fract8 = Fract8(u8::MIN);

    pub const fn to_raw(self) -> u8 {
        self.0
    }

    pub const fn from_raw(bits: u8) -> Self {
        Fract8(bits)
    }

    pub const fn from_ratio(a: u8, b: u8) -> Self {
        Fract8(((a as u16 * 256) / b as u16) as u8)
    }

    pub const fn abs_diff(self, other: Self) -> Self {
        Fract8(self.0.abs_diff(other.0))
    }
}

impl WrappingAdd for Fract8 {
    fn wrapping_add(&self, v: &Self) -> Self {
        Fract8(self.0.wrapping_add(v.0))
    }
}

impl WrappingMul for Fract8 {
    fn wrapping_mul(&self, v: &Self) -> Self {
        Fract8(self.0.wrapping_mul(v.0))
    }
}

impl Trig8 for Fract8 {
    fn sin8(self) -> Fract8 {
        self.0.sin8()
    }

    fn cos8(self) -> Fract8 {
        self.0.cos8()
    }
}

impl Mul<Fract8> for Fract8 {
    type Output = Fract8;

    #[inline]
    fn mul(self, rhs: Fract8) -> Self::Output {
        Fract8(self.0 * rhs)
    }
}

impl Add<Fract8> for Fract8 {
    type Output = Self;

    fn add(self, rhs: Fract8) -> Self::Output {
        Fract8(self.0 + rhs.0)
    }
}

impl Sub<Fract8> for Fract8 {
    type Output = Self;

    fn sub(self, rhs: Fract8) -> Self::Output {
        Fract8(self.0 - rhs.0)
    }
}

impl Mul<u8> for Fract8 {
    type Output = u8;

    #[inline]
    fn mul(self, rhs: u8) -> Self::Output {
        rhs * self
    }
}

impl Mul<Fract8> for u8 {
    type Output = u8;

    #[inline]
    fn mul(self, rhs: Fract8) -> Self::Output {
        (self as f32 * (rhs.0 as f32 / 255f32)) as u8
    }
}

impl Div<u8> for Fract8 {
    type Output = Fract8;

    fn div(self, rhs: u8) -> Self::Output {
        Fract8(self.0 / rhs)
    }
}

impl Mul<Fract8> for f32 {
    type Output = f32;

    fn mul(self, rhs: Fract8) -> Self::Output {
        self * (rhs.0 as f32 / 255f32)
    }
}

impl Mul<Fract8> for usize {
    type Output = usize;

    #[inline(always)]
    fn mul(self, rhs: Fract8) -> Self::Output {
        ((self as u8) * rhs) as usize
    }
}

macro_rules! fract8_color_impl {
    ($color_type:tt $($component:ident),+) => {

        impl Mul<Fract8> for $color_type<u8> {
            type Output = Self;

            #[inline(always)]
            fn mul(self, rhs: Fract8) -> Self::Output {
                Self {
                    $($component: self.$component * rhs),*
                }
            }
        }

        impl<T> Fract8Ops for $color_type<T> where T: Fract8Ops {

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
    fn blend8(self, other: Self, scale: Fract8) -> Self;
    fn saturating_add(self, other: Self) -> Self;
    fn lerp8by8(self, other: Self, scale: Fract8) -> Self;
}

impl Fract8Ops for bool {
    
    fn blend8(self, other: Self, scale: Fract8) -> Self {
        if scale >= Fract8(128) {
            self && other
        } else {
            self
        }
    }
    
    fn saturating_add(self, other: Self) -> Self {
        self || other
    }
    
    fn lerp8by8(self, other: Self, scale: Fract8) -> Self {
        if scale >= Fract8(128) {
            other
        } else {
            self
        }
    }
}

impl Fract8Ops for u8 {

    #[inline(always)]
    fn blend8(self, other: Self, scale: Fract8) -> Self {
        match scale {
            Fract8::MIN => self,
            Fract8::MAX => other,
            _ => (((self as u16).wrapping_shl(8).bitor(other as u16)).wrapping_add(other as u16 * scale.0 as u16).wrapping_sub(self as u16 * scale.0 as u16)).wrapping_shr(8) as u8
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
            let scaled = delta * scale;
            self + scaled
        } else {
            let delta = self - other;
            let scaled = delta * scale;
            self - scaled
        }
    }
}

impl Fract8Ops for usize {

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
            let scaled = delta * scale;
            self + scaled
        } else {
            let delta = self - other;
            let scaled = delta * scale;
            self - scaled
        }
    }
}

#[cfg(feature="embedded-graphics")]
mod embedded_impl {
    use embedded_graphics::pixelcolor::BinaryColor;

    use super::Fract8Ops;
    impl Fract8Ops for BinaryColor {
    
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

pub fn lerp7by8(a: i8, b: i8, frac: Fract8) -> i8 {
    if b > a {
        let delta: u8 = b.wrapping_sub(a) as u8;
        let scaled: u8 = delta * frac;
        a.wrapping_add(scaled as i8)
    } else {
        let delta: u8 = a.wrapping_sub(b) as u8;
        let scaled: u8 = delta * frac;
        a.wrapping_sub(scaled as i8)
    }
}

pub fn lerp8by8<T: Fract8Ops>(a: T, b: T, frac: Fract8) -> T {
    a.lerp8by8(b, frac)
}

pub fn map8(x: Fract8, range_start: Fract8, range_end: Fract8) -> Fract8 {
    let range_width = range_end.0 - range_start.0;
    let mut out = x.0 * Fract8(range_width);
    out += range_start.0;
    Fract8(out)
}

pub fn ease_in_out_quad(i: Fract8) -> Fract8 {
    let j = if i.0 & 0x80 != 0 {
        255 - i.0
    } else {
        i.0
    };
    let jj = j * Fract8(j);
    let jj2 = jj.wrapping_shl(1);
    if i.0 & 0x80 == 0 {
        Fract8(jj2)
    } else {
        Fract8(255 - jj2)
    }
}