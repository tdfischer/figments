use core::marker::Copy;
use core::convert::AsRef;
use core::result::Result;
use core::iter::Iterator;

use smart_leds_trait::{SmartLedsWrite, SmartLedsWriteAsync};

use figments::{mappings::linear::LinearSpace, prelude::*};

use crate::{gamma::{GammaCurve, WithGamma}, output::{Brightness, GammaCorrected, Output, OutputAsync}, power::*};

#[derive(Debug)]
pub struct PowerControls {
    max_mw: u32,
    brightness: u8,
    is_on: bool,
    gamma_curve: GammaCurve,
    cur_mw: u32
}

impl PowerControls {
    pub fn new(max_mw: u32) -> Self {
        Self {
            max_mw,
            brightness: 255,
            is_on: true,
            gamma_curve: GammaCurve::default(),
            cur_mw: 0
        }
    }

    pub fn iter_brightness<'a, Color, P: AsRef<[Color]> + ?Sized>(&'a mut self, pixbuf: &'a P) -> impl Iterator<Item = Color> + use<'a, Color, P> where Color: 'a + Copy + WithGamma + AsMilliwatts + Fract8Ops {
        self.cur_mw = pixbuf.as_ref().iter().map(|x| { x.with_gamma(&self.gamma_curve).as_milliwatts() }).sum();
        let b = brightness_for_mw(self.cur_mw, self.brightness, self.max_mw);
        pixbuf.as_ref().iter().map(move |x| { x.with_gamma(&self.gamma_curve).scale8(b) })
    }
}

impl Brightness for PowerControls {
    fn set_brightness(&mut self, brightness: u8) {
        self.brightness = brightness;
    }

    fn set_on(&mut self, is_on: bool) {
        self.is_on = is_on;
    }
}

impl GammaCorrected for PowerControls {
    fn set_gamma(&mut self, gamma: GammaCurve) {
        self.gamma_curve = gamma
    }
}

#[derive(Debug)]
pub struct PowerManagedWriter<T> {
    target: T,
    controls: PowerControls
}

impl<T> PowerManagedWriter<T> {
    pub fn new(target: T, max_mw: u32) -> Self {
        Self {
            target,
            controls: PowerControls::new(max_mw)
        }
    }

    pub fn write<P: AsRef<[T::Color]> + ?Sized>(&mut self, pixbuf: &P) -> Result<(), T::Error> where T: SmartLedsWrite, T::Color: Fract8Ops + Copy + WithGamma + AsMilliwatts + core::fmt::Debug {
        if self.controls.is_on {
            self.target.write(self.controls.iter_brightness(pixbuf))
        } else {
            self.target.write(pixbuf.as_ref().iter().map(|x| { x.scale8(0) }))
        }
    }


    pub async fn write_async<P: AsRef<[T::Color]> + ?Sized>(&mut self, pixbuf: &P) -> Result<(), T::Error> where T: SmartLedsWriteAsync, T::Color: Fract8Ops + Copy + WithGamma + AsMilliwatts + core::fmt::Debug {
        if self.controls.is_on {
            self.target.write(self.controls.iter_brightness(pixbuf)).await
        } else {
            self.target.write(pixbuf.as_ref().iter().map(|x| { x.scale8(0) })).await
        }
    }

    pub fn controls(&mut self) -> &mut PowerControls {
        &mut self.controls
    }

    /// Returns the total power required to display the previous write at full brightness. This is /not/ the actual power consumption, only a theoretical maximum useful for designing power supplies.
    pub const fn max_mw(&self) -> u32 {
        self.controls.cur_mw
    }
}

pub struct SmartLedsOutput<'a, T, Pixbuf> {
    writer: PowerManagedWriter<T>,
    pixbuf: &'a mut Pixbuf,
    buf_idx: usize,
    clip: Rectangle<LinearSpace>
}

impl<'a, T, Pixel, const PIXEL_COUNT: usize> SmartLedsOutput<'a, T, [Pixel; PIXEL_COUNT]> {
    pub fn new(target: T, pixbuf: &'a mut [Pixel; PIXEL_COUNT], max_mw: u32) -> Self {
        Self {
            writer: PowerManagedWriter::new(target, max_mw),
            pixbuf,
            buf_idx: 0,
            clip: Rectangle::everything()
        }
    }

    pub const fn pixbuf(&mut self) -> &mut [Pixel; PIXEL_COUNT] {
        self.pixbuf
    }

    pub fn set_clip(&mut self, clip: Rectangle<LinearSpace>) {
        self.clip = clip;
    }

    // TODO: We could just put this into a DoubleBufferedPixbuf, then there isn't a need to call this ever with SmartLedsOutput, as you could do output.pixbuf().swap(&mut next) with that.
    pub fn swap_buffer(&mut self, pixbuf: &'a mut [Pixel; PIXEL_COUNT]) -> &'a mut [Pixel; PIXEL_COUNT] {
        self.buf_idx = (self.buf_idx + 1) % self.pixbuf.as_ref().len();
        core::mem::replace(&mut self.pixbuf, pixbuf)
    }
}

impl<'a, T: SmartLedsWrite + 'a, Pixbuf: AsRef<[T::Color]>> Output<'a, LinearSpace> for SmartLedsOutput<'a, T, Pixbuf> where Self: Sample<'a, LinearSpace>, T::Color: core::fmt::Debug + AsMilliwatts + Fract8Ops + Copy + WithGamma {
    type Error = T::Error;

    type Controls = PowerControls;

    fn commit(&mut self)  -> Result<(), Self::Error> {
        self.writer.write(&self.pixbuf)
    }

    fn controls(&mut self) -> Option<&mut Self::Controls> {
        Some(self.writer.controls())
    }
}

impl<'a, T: SmartLedsWriteAsync + 'a, Pixbuf: AsRef<[T::Color]>> OutputAsync<'a, LinearSpace> for SmartLedsOutput<'a, T, Pixbuf> where Self: Sample<'a, LinearSpace>, T::Color: core::fmt::Debug + AsMilliwatts + Fract8Ops + Copy + WithGamma {
    type Error = T::Error;

    type Controls = PowerControls;

    async fn commit_async(&mut self)  -> Result<(), Self::Error> {
        self.writer.write_async(&self.pixbuf).await
    }

    fn controls(&mut self) -> Option<&mut Self::Controls> {
        Some(self.writer.controls())
    }
}

impl<'a, T, Color, const PIXEL_COUNT: usize> Sample<'a, LinearSpace> for SmartLedsOutput<'a, T, [Color; PIXEL_COUNT]> where Color: 'a {
    type Output = Color;

    fn sample(&mut self, rect: &figments::prelude::Rectangle<LinearSpace>) -> impl Iterator<Item = (figments::prelude::Coordinates<LinearSpace>, &'a mut Self::Output)> {
        let start = self.clip.top_left.x.clamp(0, self.pixbuf.len() - 1);
        let end = self.clip.bottom_right.x.clamp(0, self.pixbuf.len() - 1);
        self.pixbuf[start..=end].sample(rect)
    }
}