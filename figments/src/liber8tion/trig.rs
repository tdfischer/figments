use crate::liber8tion::interpolate::Fract8;

use super::sin_table::SIN_TABLE;

pub trait Trig8 {
    fn sin8(self) -> Fract8;
    fn cos8(self) -> Fract8;
}

impl Trig8 for u8 {
    fn sin8(self) -> Fract8 {
        Fract8::from_raw(SIN_TABLE[self as usize])
    }

    fn cos8(self) -> Fract8 {
        self.wrapping_add(64).sin8()
    }
}

impl Trig8 for usize {
    fn sin8(self) -> Fract8 {
        ((self % 255) as u8).sin8()
    }

    fn cos8(self) -> Fract8 {
        ((self % 255) as u8).cos8()
    }
}

impl Trig8 for i32 {
    fn sin8(self) -> Fract8 {
        ((self % 255) as u8).sin8()
    }

    fn cos8(self) -> Fract8 {
        ((self % 255) as u8).cos8()
    }
}