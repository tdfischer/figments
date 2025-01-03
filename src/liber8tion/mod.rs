#![doc = "A partial rust implementation of FastLED's lib8tion for fast 8 bit math on microcontrollers"]
pub mod interpolate;
pub mod noise;
pub mod trig;

use rgb::Rgb;

use crate::liber8tion::interpolate::scale8;

pub trait IntoRgb8 {
    fn into_rgb8(self) -> Rgb<u8>;
}

impl IntoRgb8 for Rgb<u8> {
    fn into_rgb8(self) -> Rgb<u8> {
        self
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Hsv {
    pub hue: u8,
    pub saturation: u8,
    pub value: u8
}

impl Hsv {
    pub fn new(hue: u8, saturation: u8, value: u8) -> Self {
        Hsv {
            hue,
            saturation,
            value
        }
    }
}

impl IntoRgb8 for Hsv {
    //TODO: Borrowed from FastLED
    fn into_rgb8(self) -> Rgb<u8> {
        const HSV_SECTION_3: u8 = 0x40;

        if self.saturation == 0 {
            return Rgb::new(self.value, self.value, self.value)
        }

        let mock_hue = scale8(191, self.hue);
        let value: u8 = self.value;
        let saturation: u8 = self.saturation;
        let invsat: u8 = 255 - saturation;
        let brightness_floor: u8 = (value as u16 * invsat as u16 / 256) as u8;

        let color_amplitude: u8 = value - brightness_floor;
        let section: u8 = mock_hue / HSV_SECTION_3;
        let offset: u8 = mock_hue % HSV_SECTION_3;

        let rampup: u8 = offset;
        let rampdown: u8 = (HSV_SECTION_3 - 1) - offset;

        let rampup_amp_adj: u8 = (rampup as u16 * color_amplitude as u16 / 64) as u8;
        let rampdown_amp_adj: u8 = (rampdown as u16 * color_amplitude as u16 / 64) as u8;

        let rampup_adj_with_floor: u8 = rampup_amp_adj.saturating_add(brightness_floor);
        let rampdown_adj_with_floor: u8 = rampdown_amp_adj.saturating_add(brightness_floor);

        match section {
            1 => Rgb::new(brightness_floor, rampdown_adj_with_floor, rampup_adj_with_floor),
            0 => Rgb::new(rampdown_adj_with_floor, rampup_adj_with_floor, brightness_floor),
            _ => Rgb::new(rampup_adj_with_floor, brightness_floor, rampdown_adj_with_floor)
        }
    }
}