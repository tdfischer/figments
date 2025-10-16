use rgb::Rgb;

pub trait AsMilliwatts {
    fn as_milliwatts(&self) -> u32;
}

// Values are calculated base on the WS2812B chip
impl AsMilliwatts for Rgb<u8> {
    fn as_milliwatts(&self) -> u32 {
        const RED_MW : u32   = 16 * 5; //< 16mA @ 5v = 80mW
        const GREEN_MW : u32 = 11 * 5; //< 11mA @ 5v = 55mW
        const BLUE_MW : u32  = 15 * 5; //< 15mA @ 5v = 75mW
        const DARK_MW : u32  =      5; //<  1mA @ 5v =  5mW

        let red = (self.r as u32 * RED_MW).wrapping_shr(8);
        let green = (self.g as u32 * GREEN_MW).wrapping_shr(8);
        let blue = (self.b as u32 * BLUE_MW).wrapping_shr(8);

        red + green + blue + DARK_MW
    }
}

impl<T> AsMilliwatts for [T] where T: AsMilliwatts {
    fn as_milliwatts(&self) -> u32 {
        self.iter().map(|p| { p.as_milliwatts() }).sum()
    }
}

impl<T, const S: usize> AsMilliwatts for [T; S] where T: AsMilliwatts {
    fn as_milliwatts(&self) -> u32 {
        self.iter().map(|p| { p.as_milliwatts() }).sum()
    }
}

pub fn brightness_for_mw(total_mw : u32, target : u8, max_power: u32) -> u8 {
    let target32 = target as u32;
    let requested_mw = (total_mw * target32) / 256;

    if requested_mw > max_power {
        ((target32 * max_power) / requested_mw) as u8
    } else {
        target
    }
}