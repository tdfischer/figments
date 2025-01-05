use std::cmp::max;
use std::cmp::min;

use figments::liber8tion::interpolate::Fract8Ops;
use figments::liber8tion::trig::sin8;
use figments::mappings::*;
use figments::prelude::*;

use ws2812_spi::Ws2812;

use smart_leds::SmartLedsWrite;
use running_average::RealTimeRunningAverage;

use esp_idf_svc::hal::{
    prelude::*,
    gpio::AnyIOPin,
    spi::{
        config::{Config, DriverConfig},
        Dma,
        SpiBusDriver,
        SpiDriver
    }
};

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    
    // Take our peripherals
    let peripherals = Peripherals::take().unwrap();

    // First, let's create the underlying WS2812 smart-leds target on GPIO5 using RMT:
    let driver = SpiDriver::new_without_sclk(
        peripherals.spi2,
        peripherals.pins.gpio5,
        Option::<AnyIOPin>::None,
        &DriverConfig::new().dma(Dma::Auto(4092))
    ).unwrap();
    let cfg = Config::new().baudrate(3_800.kHz().into());
    let spi = SpiBusDriver::new(driver, &cfg).unwrap();
    let mut target = Ws2812::new(spi);

    // The ESP32 doesn't have a GPU, so we create a regular memory-based pixel buffer for writing out over RMT later:
    let mut pixbuf = [Rgb::new(0, 0, 0); 256];

    // Create some surfaces to render to
    let mut surfaces = BufferedSurfacePool::default();

    // The real magic happens here: Creating a new surface and attaching a shader
    let _sfc = SurfaceBuilder::build(&mut surfaces)
        .shader(|coords: &Coordinates<Virtual>, frame: &usize| {
            Hsv::new(sin8(coords.x.wrapping_add(*frame as u8)), 255, 255).into_rgb8()
        })
        .finish().unwrap();

    // Every frame is numbered for replayability, and we will start at zero.
    let mut frame_idx = 0;

    // Maintaining the realtime average is somewhat heavyweight so we measure it in chunks of 1000 frames
    let mut frame_count = 0;
    let mut fps = RealTimeRunningAverage::default();

    // For some reason, driving LEDs with SPI causes the idle task to never run, so lets disable the watchdog. Its just an example...
    unsafe {
        esp_idf_svc::hal::sys::esp_task_wdt_delete(esp_idf_svc::hal::sys::xTaskGetIdleTaskHandleForCore(esp_idf_svc::hal::cpu::core() as i32));
        esp_idf_svc::hal::sys::esp_task_wdt_add(esp_idf_svc::hal::sys::xTaskGetCurrentTaskHandle());
    }

    loop {
        // 
        let mut sampler = LinearSampler::new(&mut pixbuf);
        surfaces.render_to(&mut sampler, &frame_idx);

        // Scale the brightness down to 5/255, otherwise a full-size strip will trigger a brownout as soon as it lights up.
        let brightness = min(5, sin8((frame_idx / 5) as u8));

        target.write(pixbuf.iter().map(move |x| { x.scale8(brightness)})).unwrap();

        frame_idx = frame_idx.wrapping_add(1);
        frame_count += 1;
        if frame_count == 100 {
            fps.insert(frame_count);
            frame_count = 0;
            log::info!("FPS: {}", fps.measurement().rate());
        }

        // Pet the watchdog
        unsafe {esp_idf_svc::hal::sys::esp_task_wdt_reset();}
    }
}