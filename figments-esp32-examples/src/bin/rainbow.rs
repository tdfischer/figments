#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{main, rmt::Rmt, time::Rate, clock::CpuClock};
use log::info;
use rgb::{Grb,Rgb};
use figments::prelude::*;
use figments::liber8tion::trig::sin8;
use figments_render::{power::AsMilliwatts, smart_leds::PowerManagedWriter};
use core::num::Wrapping;

use esp_hal_smartled::{smart_led_buffer, SmartLedsAdapter};
use smart_leds::{
    brightness, gamma,
    hsv::{hsv2rgb, Hsv},
    SmartLedsWrite, RGB8,
};

#[main]
fn main() -> ! {
    let p = esp_hal::init(esp_hal::Config::default());

    esp_println::logger::init_logger_from_env();

    // Change this number to use a different number of LEDs
    const NUM_LEDS: usize = 50;

    // Change this to adjust the power available; the USB spec says 500ma is the standard limit, but sometimes you can draw more from a power brick
    const POWER_MA : u32 = 500;
    
    // You probably don't need to change these values, unless your LED strip is somehow not 5 volts
    const POWER_VOLTS : u32 = 5;
    const MAX_POWER_MW : u32 = POWER_VOLTS * POWER_MA;

        // Configure the RMT driver
    let frequency: Rate = Rate::from_mhz(80);
    let rmt: Rmt<'_, esp_hal::Blocking> = Rmt::new(p.RMT, frequency)
        .expect("Failed to initialize RMT");

    let rmt_channel = rmt.channel0;

    // Construct the actual smart-leds output
    let mut pixbuf = [Default::default(); NUM_LEDS];
    let mut rmt_buffer = smart_led_buffer!(NUM_LEDS);
    let mut target = SmartLedsAdapter::new(rmt_channel, p.GPIO5, &mut rmt_buffer);

    // Stick a power management API on top of it
    let mut writer = PowerManagedWriter::new(target, MAX_POWER_MW);

    // This value is used as the 'seed' for rendering each frame, allowing us to do things like run the animation backwards, frames for double FPS, or even use system uptime for more human-paced animations
    // Try setting it to Instant::now() inside the loop, for example.
    let mut frame = Wrapping(0);

    loop {
        // Clear the pixbuf to black
        pixbuf.fill_with(|| { Default::default() });

        // Render the frame to the pixbuf
        for (coords, pix) in pixbuf.sample(&Rectangle::everything()) {
            *pix = Rgb::new(
                sin8(coords.x.wrapping_mul(3).wrapping_add(frame.0)).wrapping_add(coords.x as u8),
                sin8(coords.x.wrapping_mul(5).wrapping_sub(frame.0)).wrapping_add(coords.x as u8),
                sin8(coords.x.wrapping_mul(2).wrapping_add(frame.0)).wrapping_add(coords.x as u8)
            ).into();
        }

        // Finally, write out the rendered frame
        writer.write(&pixbuf).expect("Failed to write to LEDs!");

        // Increment the frame counter
        frame += 1;
    }
}