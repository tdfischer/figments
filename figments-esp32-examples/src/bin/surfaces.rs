#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::peripherals::RNG;
use esp_hal::time::Instant;
use esp_hal::{clock::CpuClock, delay::Delay, main, rmt::Rmt, time::Rate};
use figments::prelude::Hsv;
use log::info;
use rgb::{Grb,Rgb};
use figments::{mappings::linear::LinearSpace, prelude::*};
use figments::liber8tion::trig::sin8;
use figments_render::{output::Brightness, power::AsMilliwatts, smart_leds::PowerManagedWriter};
use core::num::Wrapping;
use figments_sample_shaders::*;

use esp_hal_smartled::{smart_led_buffer, SmartLedsAdapter};
use smart_leds::{
    brightness, gamma,
    hsv::{hsv2rgb},
    SmartLedsWrite, RGB8,
};

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    esp_alloc::heap_allocator!(size: 128 * 1024);

    let p = esp_hal::init(esp_hal::Config::default());

    esp_println::logger::init_logger_from_env();

    let rng = esp_hal::rng::Rng::new();

    // Change this number to use a different number of LEDs
    // bathroom = 120
    const NUM_LEDS: usize = 256;

    // Change this to adjust the power available; the USB spec says 500ma is the standard limit,
    // but sometimes you can draw more from a power brick.
    // We also need to take a small 100ma off the top for the MCU, therefore we aim for 400ma.
    const POWER_MA : u32 = 400;

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

    // By default, SmartLedsAdapter works with GRB pixels, but you could also change the hardware color space by changing the type of the pixbuf above
    let mut target = SmartLedsAdapter::new(rmt_channel, p.GPIO5, &mut rmt_buffer);

    // Stick a power management API on top of it
    let mut writer = PowerManagedWriter::new(target, MAX_POWER_MW);

    let mut surfaces = BufferedSurfacePool::default();

    // Our scene will have three separate layers that have their opacities animated around based on the frame.
    // Layers are rendered from first to last, meaning the first layer is the 'bottom' layer on top of which others are drawn.
    let mut layers = [
        SurfaceBuilder::build(&mut surfaces).shader(RgbWaves::default()).finish().unwrap(),
        SurfaceBuilder::build(&mut surfaces).shader(Thinking::default()).finish().unwrap(),
    ];

    // Additionally, a glowing background color is always visible below all the layers, so we draw it separately
    let mut background_shader = ColorGlow { color: Hsv::new(0, 255, 255) };

    // This value is used as the 'seed' for rendering each frame, allowing us to do things like run the animation backwards, frames for double FPS, or even use system uptime for more human-paced animations
    // Try setting it to Instant::now() inside the loop, for example.
    let mut frame = Wrapping(0);

    let mut last_rotation = 0;

    loop {
        let start = Instant::now();

        // Clear the pixbuf back to a blank slate
        pixbuf = [Default::default(); NUM_LEDS];

        frame.0 = Instant::now().duration_since_epoch().as_millis() as usize / 100;

        // Draw the background shader which is always visible
        Painter::<_, _, Rgb<u8>>::paint(&mut pixbuf, &background_shader, &FrameNumber(frame.0), &Rectangle::everything());

        // Adjust the opacity for each layer using a basic oscilating wave function based on layer order
        for (idx, layer) in layers.iter_mut().enumerate() {
            layer.set_opacity(sin8(frame.0.wrapping_mul(idx + 2)));
        }

        surfaces.commit();

        // Render the layers to the pixbuf
        surfaces.render_to(&mut pixbuf, &FrameNumber(frame.0));
        let draw_time = start.elapsed();

        // Finally, write out the rendered frame
        writer.write(&pixbuf).expect("Failed to write to LEDs!");
        let flush_time = start.elapsed();

        let cur_second = Instant::now().duration_since_epoch().as_secs();
        if last_rotation != cur_second {
            info!("frame={frame:?} draw={draw_time} flush={flush_time} total={} power={}mw", draw_time + flush_time, writer.max_mw());
            // Set a different color on the colorglow shader every couple of frames
            //background_shader.color.hue = background_shader.color.hue.wrapping_add(rng.random() as u8);

            last_rotation = cur_second;
        }

        background_shader.color.hue = background_shader.color.hue.wrapping_add(1);

        Delay::new().delay_millis(13);

        // Increment the frame counter
        frame += 1;
    }
}