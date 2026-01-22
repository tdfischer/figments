#![doc = "A partial rust implementation of FastLED's lib8tion for fast 8 bit math on microcontrollers"]
pub mod interpolate;
pub mod noise;
pub mod trig;
pub mod rhythm;
mod sin_table;

use rgb::{Rgb, Rgba};

use crate::liber8tion::interpolate::Fract8Ops;

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Hsv {
    pub hue: u8,
    pub saturation: u8,
    pub value: u8
}

impl Hsv {
    pub const fn new(hue: u8, saturation: u8, value: u8) -> Self {
        Hsv {
            hue,
            saturation,
            value
        }
    }
}

fn sqrt16(x: u16) -> u16 {
    if x <= 1 {
        return x;
    }

    let mut low = 1; // lower bound

    let mut hi = if x > 7904 {
        255
    } else {
        (x >> 5) + 8 // initial estimate for upper bound
    };

    loop {
        let mid = (low + hi) >> 1;
        if (mid as u16 * mid as u16) > x {
            hi = mid - 1;
        } else {
            if mid == 255 {
                return 255;
            }
            low = mid + 1;
        }
        if hi >= low {
            break
        }
    }

    return low - 1;
}

#[inline]
fn qadd8(i: u8, j: u8) -> u8 {
    let t= i as u16 + j as u16;
    if t > 255 {
        255
    } else {
        t as u8
    }
}

#[inline]
fn qsub8(i: u8, j: u8) -> u8 {
    let t = i as i16 - j as i16;
    if t < 0 {
        0
    } else {
        t as u8
    }
}

macro_rules! FIXFRAC8 {
    ($a:expr, $b:expr) => {
        (($a as u16 * 256) / $b as u16) as u8
    };
}

// Pre-defined hue values for CHSV objects
const HUE_RED: u8 = 0;       ///< Red (0°)
const HUE_ORANGE: u8 = 32;   ///< Orange (45°)
const HUE_YELLOW: u8 = 64;   ///< Yellow (90°)
const HUE_GREEN: u8 = 96;    ///< Green (135°)
const HUE_AQUA: u8 = 128;    ///< Aqua (180°)
const HUE_BLUE: u8 = 160;    ///< Blue (225°)
const HUE_PURPLE: u8 = 192;  ///< Purple (270°)
const HUE_PINK: u8 = 224;     ///< Pink (315°)

impl From<Rgba<u8>> for Hsv {
    fn from(value: Rgba<u8>) -> Self {
        From::from(Rgb::new(value.r, value.g, value.b))
    }
}

impl From<Rgb<u8>> for Hsv {
    fn from(rgb: Rgb<u8>) -> Self { //FIXME: it is broken :(
        let mut r = rgb.r;
        let mut g = rgb.g;
        let mut b = rgb.b;
        let mut h: u8;
        let mut s: u8;
        let mut v: u8;
        
        // find desaturation
        let mut desat = 255;
        if r < desat {
            desat = r;
        }
        if g < desat {
            desat = g;
        }
        if b < desat {
            desat = b;
        }
        
        // remove saturation from all channels
        r -= desat;
        g -= desat;
        b -= desat;
        
        //Serial.print("desat="); Serial.print(desat); Serial.println("");
        
        //uint8_t orig_desat = sqrt16( desat * 256);
        //Serial.print("orig_desat="); Serial.print(orig_desat); Serial.println("");
        
        // saturation is opposite of desaturation
        s = 255 - desat;
        //Serial.print("s.1="); Serial.print(s); Serial.println("");
        
        if s != 255 {
            // undo 'dimming' of saturation
            s = 255 - sqrt16( (255-s as u16) * 256) as u8;
        }
        // without lib8tion: float ... ew ... sqrt... double ew, or rather, ew ^ 0.5
        // if( s != 255 ) s = (255 - (256.0 * sqrt( (float)(255-s) / 256.0)));
        //Serial.print("s.2="); Serial.print(s); Serial.println("");
        
        
        // at least one channel is now zero
        // if all three channels are zero, we had a
        // shade of gray.
        if (r.wrapping_add(g).wrapping_add(b)) == 0 {
            // we pick hue zero for no special reason
            return Hsv::new( 0, 0, 255 - s);
        }
        
        // scale all channels up to compensate for desaturation
        if s < 255 {
            if s == 0 {
                s = 1;
            }
            let scaleup = 65535 / (s as u32);
            r = ((r as u32 * scaleup) / 256) as u8;
            g = ((g as u32 * scaleup) / 256) as u8;
            b = ((b as u32 * scaleup) / 256) as u8;
        }
        //Serial.print("r.2="); Serial.print(r); Serial.println("");
        //Serial.print("g.2="); Serial.print(g); Serial.println("");
        //Serial.print("b.2="); Serial.print(b); Serial.println("");
        
        let mut total: u16 = r as u16 + g as u16 + b as u16;
        
        //Serial.print("total="); Serial.print(total); Serial.println("");
        
        // scale all channels up to compensate for low values
        if total < 255 {
            if total == 0 {
                total = 1;
            }
            let scaleup = 65535 / (total as u32);
            r = ((r as u32 * scaleup) / 256) as u8;
            g = ((g as u32 * scaleup) / 256) as u8;
            b = ((b as u32 * scaleup) / 256) as u8;
        }
        //Serial.print("r.3="); Serial.print(r); Serial.println("");
        //Serial.print("g.3="); Serial.print(g); Serial.println("");
        //Serial.print("b.3="); Serial.print(b); Serial.println("");
        
        if total > 255 {
            v = 255;
        } else {
            v = qadd8(desat,total as u8);
            // undo 'dimming' of brightness
            if v != 255 {
                v = sqrt16( v as u16 * 256) as u8;
            }
            // without lib8tion: float ... ew ... sqrt... double ew, or rather, ew ^ 0.5
            // if( v != 255) v = (256.0 * sqrt( (float)(v) / 256.0));
            
        }
        
        // since this wasn't a pure shade of gray,
        // the interesting question is what hue is it
        
        
        
        // start with which channel is highest
        // (ties don't matter)
        let mut highest = r;
        if g > highest {
            highest = g;
        }
        if b > highest {
            highest = b;
        }
        
        if highest == r {
            // Red is highest.
            // Hue could be Purple/Pink-Red,Red-Orange,Orange-Yellow
            if g == 0 {
                // if green is zero, we're in Purple/Pink-Red
                h = (HUE_PURPLE.wrapping_add(HUE_PINK)) / 2;
                h += qsub8(r, 128).scale8(FIXFRAC8!(48,128));
            } else if (r - g) > g {
                // if R-G > G then we're in Red-Orange
                h = HUE_RED;
                h += g.scale8(FIXFRAC8!(32,85));
            } else {
                // R-G < G, we're in Orange-Yellow
                h = HUE_ORANGE;
                h += qsub8((g.wrapping_sub(85)).wrapping_add(171u8.wrapping_sub(r)), 4).scale8(FIXFRAC8!(32,85)); //221
            }
            
        } else if highest == g {
            // Green is highest
            // Hue could be Yellow-Green, Green-Aqua
            if b == 0 {
                // if Blue is zero, we're in Yellow-Green
                //   G = 171..255
                //   R = 171..  0
                h = HUE_YELLOW;
                let radj = qsub8(171,r).scale8(47); //171..0 -> 0..171 -> 0..31
                let gadj = qsub8(g,171).scale8(96); //171..255 -> 0..84 -> 0..31;
                let rgadj = radj + gadj;
                let hueadv = rgadj / 2;
                h += hueadv;
                //h += scale8( qadd8( 4, qadd8((g - 128), (128 - r))),
                //             FIXFRAC8(32,255)); //
            } else {
                // if Blue is nonzero we're in Green-Aqua
                if (g-b) > b {
                    h = HUE_GREEN;
                    h += b.scale8(FIXFRAC8!(32,85));
                } else {
                    h = HUE_AQUA;
                    h += qsub8(b, 85).scale8(FIXFRAC8!(8,42));
                }
            }
            
        } else /* highest == b */ {
            // Blue is highest
            // Hue could be Aqua/Blue-Blue, Blue-Purple, Purple-Pink
            if r == 0 {
                // if red is zero, we're in Aqua/Blue-Blue
                h = HUE_AQUA + ((HUE_BLUE - HUE_AQUA) / 4);
                h += qsub8(b, 128).scale8(FIXFRAC8!(24,128));
            } else if (b-r) > r {
                // B-R > R, we're in Blue-Purple
                h = HUE_BLUE;
                h += r.scale8(FIXFRAC8!(32,85));
            } else {
                // B-R < R, we're in Purple-Pink
                h = HUE_PURPLE;
                h += qsub8(r, 85).scale8(FIXFRAC8!(32,85));
            }
        }
        
        h += 1;
        return Hsv::new( h, s, v);
    }
}

impl Into<Rgba<u8>> for Hsv {
    fn into(self) -> Rgba<u8> {
        let rgb: Rgb<u8> = Into::into(self);
        Rgba::new(rgb.r, rgb.g, rgb.b, 255)
    }
}

impl Into<Rgb<u8>> for Hsv {
    //TODO: Borrowed from FastLED
    fn into(self) -> Rgb<u8> {
        const HSV_SECTION_3: u8 = 0x40;

        if self.saturation == 0 {
            return Rgb::new(self.value, self.value, self.value)
        }

        let mock_hue = 191.scale8(self.hue);
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