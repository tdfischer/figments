use core::marker::Copy;
use core::convert::AsRef;
use core::result::Result;
use core::iter::Iterator;

use smart_leds_trait::{SmartLedsWrite, SmartLedsWriteAsync};

use figments::{liber8tion::interpolate::Fract8, prelude::*};

use crate::{gamma::{GammaCurve, WithGamma}, output::{Brightness, GammaCorrected, Output}, power::*};

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