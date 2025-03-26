use super::{interpolate::scale8, trig::sin8};

fn beat88(bpm: u16, timebase: u32) -> u16 {
    ((timebase.wrapping_mul(bpm as u32).wrapping_mul(280)).wrapping_shr(16)) as u16
}

fn beat16(bpm: u16, timebase: u32) -> u16 {
    let adj_bpm = if bpm < 256 {
        bpm.wrapping_shl(8)
    } else {
        bpm
    };
    beat88(adj_bpm, timebase)
}

fn beat8(bpm: u16, timebase: u32) -> u8 {
    beat16(bpm, timebase).wrapping_shr(8) as u8
}

pub fn beatsin8(bpm: u16, lowest: u8, highest: u8, timebase: u32, phase: u8) -> u8 {
    let beat = beat8(bpm, timebase);
    let beatsin = sin8(beat.wrapping_add(phase));
    let width = highest - lowest;
    let scaledbeat = scale8(beatsin, width);
    
    lowest + scaledbeat
} 