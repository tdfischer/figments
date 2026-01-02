#![no_std]
#![no_main]

/*
    This example is intended to show you two of the principles of figments: sampling and shaders.

    Running this program should produce a fun animation of colors that fade in and out at a very high FPS with precice power consumption.

 */

use esp_backtrace as _;
use esp_hal::{clock::CpuClock, delay::Delay, main, rmt::Rmt, time::{Instant, Rate}};
use log::info;
use rgb::{Grb,Rgb};
use figments::prelude::*;
use figments::liber8tion::trig::*;
use figments_render::{output::Brightness, power::AsMilliwatts, smart_leds::PowerManagedWriter};
use core::num::Wrapping;

use esp_hal_smartled::{smart_led_buffer, SmartLedsAdapter};
use smart_leds::{
    brightness, gamma,
    hsv::{hsv2rgb, Hsv},
    SmartLedsWrite, RGB8,
};

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    // Even though we don't use any heap allocations in this example, we can't remove the "alloc" feature flag from figments for just one example, sorry.
    esp_alloc::heap_allocator!(size: 128 * 1024);

    // Set up the low level hardware we'll be using for rendering pixels
    let p = esp_hal::init(esp_hal::Config::default());
    esp_println::logger::init_logger_from_env();

    // Change this number to use a different number of LEDs
    const NUM_LEDS: usize = 256;

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

    // Here we create the smart-leds driver on top of the RMT interface, which allows us to write RGB values out to the hardware. It requires 
    let mut rmt_buffer = smart_led_buffer!(NUM_LEDS);

    // By default, SmartLedsAdapter works with GRB pixels, but you could also change the color space by changing the type of the pixbuf later on,
    // then change this to new_with_color(). Either way, a color conversion happens at rendering time in the most efficient way possible, which could mean no conversion at all.
    let mut target = SmartLedsAdapter::new(rmt_channel, p.GPIO5, &mut rmt_buffer);

    // LEDs can get extremely bright very quickly. Sticking a power management API on top of it prevents brownouts.
    let mut writer = PowerManagedWriter::new(target, MAX_POWER_MW);

    // This value is used as the 'seed' for rendering each frame, allowing us to do things like run the animation backwards, frames for double FPS, or even use system uptime for more human-paced animations
    // Try setting it to Instant::now() inside the loop, for example.
    let mut frame = Wrapping(0);

    // Finally, lets create a pixbuf that we will be drawing everything into before it is sent out to the hardware.
    // We can change this to specify a custom color format, or we can let the trait system figure out which is the most compatible with the code that generates your images, and what the hardware expects.
    // This minimizes the need for expensive color conversions and lets you write your graphics in any color format or space you want, as long as it can eventually get converted to the driver's format.
    let mut pixbuf = [Default::default(); NUM_LEDS];

    loop {
        // Clear the pixbuf to black
        pixbuf = [Default::default(); NUM_LEDS];

        // Mark the time we start rendering for performance reporting
        let start = Instant::now();

        // Render the frame to the pixbuf by sampling the entire pixbuf and manipulating each pixel in the buffer
        for (coords, pix) in pixbuf.sample(&Rectangle::everything()) {
            // Calculate the color for this pixel using some fun wave functions that take coordinates along the pixel strip as an input
            let rendered = Rgb::new(
                sin8(coords.x.wrapping_mul(3).wrapping_add(frame.0).wrapping_add(coords.x)),
                cos8(coords.x.wrapping_mul(5).wrapping_sub(frame.0).wrapping_add(coords.x)),
                sin8(coords.x.wrapping_mul(2).wrapping_add(frame.0).wrapping_add(coords.x))
            );

            // We apply a color format conversion here from RGB to whatever the hardware ends up spporting via into()
            *pix = rendered.into();
        }

        let draw_time = start.elapsed();

        // Adjust the brightness along a sine wave pattern so the whole display fades in and out
        writer.controls().set_brightness(sin8(frame.0 / 3));

        // Finally, write out the rendered frame
        writer.write(&pixbuf).expect("Failed to write to LEDs!");

        // Print out our rendering times
        let write_time = start.elapsed();
        info!("draw={draw_time} write={write_time} total={} power={}mw / {MAX_POWER_MW}mw", draw_time + write_time, writer.max_mw());

        // Increment the frame counter
        frame += 1;
    }
}