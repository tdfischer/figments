use num::PrimInt;
use core::ops::BitOr;

use rgb::Rgb;

pub type Fract8 = u8;

pub trait Fract8Ops {
    fn scale8(self, scale: Fract8) -> Self;
    fn blend8(self, other: Self, scale: Fract8) -> Self;
}

impl Fract8Ops for u8 {
    #[inline]
    fn scale8(self, scale: Fract8) -> Self {
        match scale {
            0 => 0,
            255 => self,
            _ => 
                // borrowed from FastLED
                (self as u16 * (1 + scale as u16)).unsigned_shr(8) as u8
        }
    }

    #[inline]
    fn blend8(self, other: Self, scale: Fract8) -> Self {
        match scale {
            0 => self,
            255 => other,
            _ => (((self as u16).unsigned_shl(8).bitor(other as u16)).wrapping_add(other as u16 * scale as u16).wrapping_sub(self as u16 * scale as u16)).unsigned_shr(8) as u8
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
}

impl Fract8Ops for Rgb<u8> {
    #[inline]
    fn scale8(self, scale: Fract8) -> Self {
        Rgb::new(
            self.r.scale8(scale),
            self.g.scale8(scale),
            self.b.scale8(scale)
        )
    }

    #[inline]
    fn blend8(self, other: Self, scale: Fract8) -> Self {
        match scale {
            0 => self,
            255 => other,
            _ => match (other.r, other.g, other.b) {
                (0, 0, 0) => self,
                _ => Rgb::new(
                    self.r.blend8(other.r, scale),
                    self.g.blend8(other.g, scale),
                    self.b.blend8(other.b, scale)
                )
            }
        }
    }
}

#[inline]
pub fn scale8<T: Fract8Ops>(i: T, scale: Fract8) -> T {
    i.scale8(scale)
}

#[inline]
pub fn avg7(i: i8, j: i8) -> i8 {
    i.unsigned_shr(1).wrapping_add(j.unsigned_shr(1)).wrapping_add(i & 0x1)
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

pub fn lerp8by8(a: u8, b: u8, frac: u8) -> u8 {
    if b > a {
        let delta = b - a;
        let scaled = scale8(delta, frac);
        a + scaled
    } else {
        let delta = a - b;
        let scaled = scale8(delta, frac);
        a - scaled
    }
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
    let jj2 = jj.unsigned_shl(1);
    if i & 0x80 == 0 {
        jj2
    } else {
        255 - jj2
    }
}