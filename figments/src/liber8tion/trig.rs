use super::sin_table::SIN_TABLE;

const B_M16_INTERLEAVE: [u8; 8] = [0, 49, 49, 41, 90, 27, 117, 10];

pub trait Trig8 {
    fn sin8(self) -> u8;
    fn cos8(self) -> u8;
}

impl Trig8 for u8 {
    fn sin8(self) -> u8 {
        SIN_TABLE[self as usize]
    }

    fn cos8(self) -> u8 {
        sin8(self.wrapping_add(64))
    }
}

impl Trig8 for usize {
    fn sin8(self) -> u8 {
        ((self % 255) as u8).sin8()
    }

    fn cos8(self) -> u8 {
        ((self % 255) as u8).cos8()
    }
}

impl Trig8 for i32 {
    fn sin8(self) -> u8 {
        ((self % 255) as u8).sin8()
    }

    fn cos8(self) -> u8 {
        ((self % 255) as u8).cos8()
    }
}

pub fn sin8<T: Trig8>(theta: T) -> u8 {
    theta.sin8()
}

pub fn cos8<T: Trig8>(theta: T) -> u8 {
    theta.cos8()
}