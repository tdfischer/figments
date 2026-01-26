use num::traits::WrappingAdd;

use crate::liber8tion::{interpolate::Fract8, trig::Trig8};


fn beat88(now: u32, bpm: u16, timebase: u32) -> u16 {
    (((now - timebase).wrapping_mul(bpm as u32).wrapping_mul(280)).wrapping_shr(16)) as u16
}

fn beat16(now: u32, bpm: u16, timebase: u32) -> u16 {
    let adj_bpm = if bpm < 256 {
        bpm.wrapping_shl(8)
    } else {
        bpm
    };
    beat88(now, adj_bpm, timebase)
}

fn beat8(now: u32, bpm: u16, timebase: u32) -> Fract8 {
    Fract8::from_raw(beat16(now, bpm, timebase).wrapping_shr(8) as u8)
}

pub fn beatsin8(now: u32, bpm: u16, lowest: Fract8, highest: Fract8, timebase: u32, phase: Fract8) -> Fract8 {
    let beat = beat8(now, bpm, timebase);
    let beatsin = beat.wrapping_add(&phase).sin8();
    let width = highest - lowest;
    let scaledbeat = beatsin * width;
    
    lowest + scaledbeat
} 