use std::cmp::max;
use std::cmp::min;

use figments::liber8tion::interpolate::Fract8Ops;
use figments::liber8tion::trig::sin8;
use figments::mappings::*;
use figments::prelude::*;

use ws2812_esp32_rmt_driver::{
    driver::color::LedPixelColorGrb24,
    LedPixelEsp32Rmt
};
use esp_idf_svc::hal::prelude::Peripherals;
use smart_leds::SmartLedsWrite;
use running_average::RealTimeRunningAverage;

pub type FastWs2812Esp32Rmt<'a> = LedPixelEsp32Rmt<'a, Rgb<u8>, LedPixelColorGrb24>;

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    
    // Take our peripherals
    let peripherals = Peripherals::take().unwrap();

    // First, let's create the underlying WS2812 smart-leds target on GPIO5 using RMT:
    let mut target = FastWs2812Esp32Rmt::new(peripherals.rmt.channel0, peripherals.pins.gpio5).unwrap();

    // The ESP32 doesn't have a GPU, so we create a regular memory-based pixel buffer for writing out over RMT later:
    let mut pixbuf = [Rgb::new(0, 0, 0); 256];

    // Create some surfaces to render to
    let mut surfaces = BufferedSurfacePool::default();

    // The real magic happens here: Creating a new surface and attaching a shader
    let _sfc = SurfaceBuilder::build(&mut surfaces)
        .shader(|coords: &Coordinates<Virtual>, frame: usize| {
            Hsv::new(sin8(coords.x.wrapping_add(frame as u8)), 255, 255).into_rgb8()
        })
        .finish().unwrap();

    // Every frame is numbered for replayability, and we will start at zero.
    let mut frame_idx = 0;

    // Maintaining the realtime average is somewhat heavyweight so we measure it in chunks of 1000 frames
    let mut frame_count = 0;
    let mut fps = RealTimeRunningAverage::default();

    loop {
        // 
        let mut sampler = LinearSampler::new(&mut pixbuf);
        surfaces.render_to(&mut sampler, frame_idx);

        let brightness = min(5, sin8((frame_idx / 5) as u8));
        target.write(pixbuf.iter().map(move |x| { x.scale8(brightness)})).unwrap();

        frame_idx = frame_idx.wrapping_add(1);
        frame_count += 1;
        if frame_count == 1000 {
            fps.insert(frame_count);
            frame_count = 0;
            log::info!("FPS: {}", fps.measurement().rate());
        }
    }
}