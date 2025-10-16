use esp_idf_svc::hal::prelude::Peripherals;
use figments::{liber8tion::trig::{cos8, sin8}, prelude::*};
use rgb::Grb;
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

use figments_render::smart_leds::PowerManagedWriter;

fn main() {
    let peripherals = Peripherals::take().expect("Failed to grab the peripherals");
    let rmt = peripherals.rmt.channel0;

    // Change this to use a different pin for the WS2812 LED strip
    let led_pin = peripherals.pins.gpio5;

    // Change this to adjust the power available; the USB spec says 500ma is the standard limit, but sometimes you can draw more from a power brick
    const POWER_MA : u32 = 500;
    
    // You probably don't need to change these values, unless your LED strip is somehow not 5 volts
    const POWER_VOLTS : u32 = 5;
    const MAX_POWER_MW : u32 = POWER_VOLTS * POWER_MA;

    // Change this number to use a different number of LEDs
    let mut pixbuf = [Default::default(); 255];

    // Construct the smart-led interface
    let mut target = Ws2812Esp32Rmt::new(rmt, led_pin).expect("Failed to construct WS2812 RMT driver");

    // Stick a power management API on top of it
    let mut writer = PowerManagedWriter::new(target, MAX_POWER_MW);

    // This value is used as the 'seed' for rendering each frame, allowing us to do things like run the animation backwards, frames for double FPS, or even use system uptime for more human-paced animations
    let mut frame = 0;

    loop {
        // Clear the pixbuf to black
        pixbuf.fill_with(|| { Default::default() });

        // Render the frame to the pixbuf
        for (coords, pix) in pixbuf.sample(&Rectangle::everything()) {
            let angle_x = coords.x.wrapping_mul(3).wrapping_add(coords.y.wrapping_mul(3)).wrapping_add(frame);
            let angle_y = coords.y.wrapping_mul(3).wrapping_add(coords.x.wrapping_mul(3)).wrapping_add(frame.wrapping_div(2));
            *pix = Grb::new_grb(
                sin8(angle_y),
                cos8(angle_x),
                angle_x.wrapping_add(angle_y) as u8
            ).into();
        }

        // Finally, write out the rendered frame
        writer.write(&pixbuf).expect("Failed to write to LEDs!");

        // Increment the frame counter
        frame += 1;
    }
}