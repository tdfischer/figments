use num::PrimInt;

const B_M16_INTERLEAVE: [u8; 8] = [0, 49, 49, 41, 90, 27, 117, 10];

pub trait Trig8 {
    fn sin8(self) -> u8;
    fn cos8(self) -> u8;
}

impl Trig8 for u8 {
    fn sin8(self) -> u8 {
        let mut offset: u8 = self;
        if self & 0x40 != 0 {
            offset = 255 - offset;
        }
        offset &= 0x3f;

        let mut secoffset: u8 = offset & 0x0f;
        if self & 0x40 != 0 {
            secoffset += 1;
        }

        let section: u8 = offset.unsigned_shr(4);
        let s2: u8 = section * 2;
        let b: u8 = B_M16_INTERLEAVE[s2 as usize];
        let m16: u8 = B_M16_INTERLEAVE[s2 as usize + 1];
        let mx: u8 = m16.wrapping_mul(secoffset).unsigned_shr(4);
        let mut y: i8 = mx as i8 + b as i8;
        if self & 0x80 != 0 {
            y = -y;
        }
        y = y.wrapping_add(128u8 as i8);
        return y as u8;
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

pub fn sin8<T: Trig8>(theta: T) -> u8 {
    theta.sin8()
}

pub fn cos8<T: Trig8>(theta: T) -> u8 {
    theta.cos8()
}