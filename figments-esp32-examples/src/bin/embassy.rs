#![no_std]
#![no_main]

use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_hal::peripherals::RNG;
use embassy_time::{Duration, Instant};
use esp_hal::{clock::CpuClock, delay::Delay, main, rmt::Rmt, time::Rate};
use figments::prelude::Hsv;
use log::{info, warn};
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

use esp_hal::gpio::AnyPin;
use esp_hal::gpio::Pin;
use esp_hal::timer::systimer::SystemTimer;

use embassy_time::Timer;

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    esp_alloc::heap_allocator!(size: 128 * 1024);

    let p = esp_hal::init(esp_hal::Config::default());

    let sys_timer = SystemTimer::new(p.SYSTIMER);
    esp_rtos::start(sys_timer.alarm1);

    esp_println::logger::init_logger_from_env();

    let rng = esp_hal::rng::Rng::new();

    // The surface pool will become the single point of contact between both tasks. The pool is created here and stays with the task for the rendering, while individual surfaces can get handed out to other tasks
    let mut surfaces = BufferedSurfacePool::default();

    // Our scene will have three separate layers that have their opacities animated around based on the frame.
    // Layers are rendered from first to last, meaning the first layer is the 'bottom' layer on top of which others are drawn.

    // Additionally, a glowing background color is always visible below all the layers, so we draw it separately
    let mut background_shader = SurfaceBuilder::build(&mut surfaces).shader(ColorGlow::default()).finish().unwrap();

    let mut layers = [
        SurfaceBuilder::build(&mut surfaces).shader(RgbWaves::default()).finish().unwrap(),
        SurfaceBuilder::build(&mut surfaces).shader(Thinking::default()).finish().unwrap(),
    ];

    // From here, we separate the rendering and hardware writing from the UI updating into two tasks. On the esp32s3, you could consider running the layering task on the second core.
    spawner.spawn(layer_task(layers, background_shader)).unwrap();
    spawner.spawn(render_task(surfaces, p.RMT, p.GPIO5.degrade())).unwrap();
}

#[embassy_executor::task]
async fn layer_task(mut layers: [BufferedSurface<FrameNumber, LinearSpace, Rgb<u8>>; 2], mut background: BufferedSurface<FrameNumber, LinearSpace, Rgb<u8>>) {
    let mut background_color = Hsv::new(0, 255, 255);
    let mut frame_idx: usize = 0;
    loop {
        // Adjust the opacity for each layer using a basic oscilating wave function based on layer order
        for (idx, layer) in layers.iter_mut().enumerate() {
            layer.set_opacity(sin8(frame_idx.wrapping_mul(idx + 2)));
        }

        background_color.hue = background_color.hue.wrapping_add(21);

        // To change the properties of a shader, we must re-upload the entire shader back into the surface's memory, which is later picked up by surfaces.commit() in the rendering task.
        background.set_shader(ColorGlow { color: background_color });
        info!("background={background_color:?}");

        // We only update the shaders and surfaces once per second. Conceptually, you could use a series of such delays to create an animation by setting shaders in sequence
        Timer::after_secs(1).await;

        frame_idx += 1;
    }
}

#[embassy_executor::task]
async fn render_task(mut surfaces: BufferedSurfacePool<FrameNumber, LinearSpace, Rgb<u8>>, rmt: esp_hal::peripherals::RMT<'static>, pin: AnyPin<'static>) {
    // Configure the RMT driver
    let frequency: Rate = Rate::from_mhz(80);
    let rmt: Rmt<'_, esp_hal::Blocking> = Rmt::new(rmt, frequency)
        .expect("Failed to initialize RMT");

    let rmt_channel = rmt.channel0;

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

    // Our goal should be to get 30 frames per second
    const FPS: u64 = 30;
    const RENDER_BUDGET: Duration = Duration::from_millis(1000 / FPS);

    // Meanwhile, the actual animations (which are based on the frame number) should run much faster, meaning not every frame gets rendered
    const ANIMATION_TPS: u64 = 120;
    const ANIMATION_FRAME_TIME: Duration = Duration::from_millis(1000 / ANIMATION_TPS);

    // Construct the actual smart-leds output
    let mut pixbuf = [Default::default(); NUM_LEDS];
    let mut rmt_buffer = smart_led_buffer!(NUM_LEDS);

    // By default, SmartLedsAdapter works with GRB pixels, but you could also change the hardware color space by changing the type of the pixbuf above
    let mut target = SmartLedsAdapter::new(rmt_channel, pin, &mut rmt_buffer);

    // Stick a power management API on top of it
    let mut writer = PowerManagedWriter::new(target, MAX_POWER_MW);

    // We use this so we only print out our stats once every second
    let mut last_print = 0;

    loop {
        let start = Instant::now();

        // Clear the pixbuf back to a blank slate
        pixbuf = [Default::default(); NUM_LEDS];

        let frame = (Instant::now().as_millis() / ANIMATION_FRAME_TIME.as_millis()) as usize;

        // Apply any changes that the other layer task might have prepared
        surfaces.commit();

        // Render the layers to the pixbuf
        surfaces.render_to(&mut pixbuf, &FrameNumber(frame));
        let draw_time = start.elapsed();

        // Finally, write out the rendered frame
        writer.write(&pixbuf).expect("Failed to write to LEDs!");
        let flush_time = start.elapsed();

        let cur_second = start.as_secs();
        if cur_second != last_print {
            last_print = cur_second;
            info!("frame={frame} draw={}ms flush={}ms", draw_time.as_millis(), flush_time.as_millis())
        }

        // Now we calculate how long it took to draw everything
        let render_time = start.elapsed();

        // If we did it fast, then we simply wait until the next frame is due.
        if render_time < RENDER_BUDGET {
            let delay = RENDER_BUDGET - render_time;
            Timer::after(delay).await;
        }

    }
}