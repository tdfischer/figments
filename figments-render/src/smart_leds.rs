use core::marker::Copy;
use core::convert::AsRef;
use core::result::Result;
use core::iter::Iterator;

use smart_leds_trait::{SmartLedsWrite, SmartLedsWriteAsync};

use figments::prelude::*;

use crate::{gamma::{WithGamma, GammaCurve}, output::{Brightness, GammaCorrected}, power::*};

pub struct PowerControls {
    max_mw: u32,
    brightness: u8,
    is_on: bool,
    gamma_curve: GammaCurve
}

impl PowerControls {
    pub fn new(max_mw: u32) -> Self {
        Self {
            max_mw,
            brightness: 255,
            is_on: true,
            gamma_curve: GammaCurve::default()
        }
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

pub struct PowerManagedWriterAsync<T: SmartLedsWriteAsync> {
    target: T,
    controls: PowerControls
}

impl<T: SmartLedsWriteAsync> PowerManagedWriterAsync<T> where T::Color: PixelBlend<Rgb<u8>> + PixelFormat + WithGamma, T::Error: core::fmt::Debug {
    pub async fn write<P: AsMilliwatts + AsRef<[T::Color]> + WithGamma + Copy>(&mut self, pixbuf: &P) -> Result<(), T::Error> {
        if self.controls.is_on {
            let with_gamma = pixbuf.with_gamma(&self.controls.gamma_curve);
            let b = brightness_for_mw(with_gamma.as_milliwatts(), self.controls.brightness, self.controls.max_mw);

            // FIXME: Should be able to just replace this with a greyscale u8 value, which would let us drop PixelBlend<Rgb<u8>> from the trait
            let blend_color = Rgb::new(b, b, b);
            let iter = with_gamma.as_ref().iter().map(|x| { x.multiply(blend_color) });
            self.target.write(iter).await
        } else {
            self.target.write(pixbuf.as_ref().iter().map(|x| { x.multiply(Rgb::new(0, 0, 0)) })).await
        }
    }

    pub fn new(target: T, max_mw: u32) -> Self {
        Self {
            target,
            controls: PowerControls::new(max_mw)
        }
    }

    pub fn controls(&mut self) -> &mut PowerControls {
        &mut self.controls
    }
}

pub struct PowerManagedWriter<T: SmartLedsWrite> {
    target: T,
    controls: PowerControls
}

impl<T: SmartLedsWrite> PowerManagedWriter<T> where T::Color: PixelFormat + WithGamma + AsMilliwatts, T::Error: core::fmt::Debug {
    pub fn new(target: T, max_mw: u32) -> Self {
        Self {
            target,
            controls: PowerControls::new(max_mw)
        }
    }

    pub fn write<P: AsMilliwatts + AsRef<[T::Color]> + WithGamma + Copy>(&mut self, pixbuf: &P) -> Result<(), T::Error> where T::Color: PixelBlend<Rgb<u8>> {
        if self.controls.is_on {
            let with_gamma = pixbuf.as_ref().iter().map(|pix| {
                pix.with_gamma(&self.controls.gamma_curve)
            });
            let mw: u32 = with_gamma.clone().map(|pix| { pix.as_milliwatts() }).sum();
            let b = brightness_for_mw(mw, self.controls.brightness, self.controls.max_mw);
            let blend_color = Rgb::new(b, b, b);
            let gamma_iter = with_gamma.map(|x| { x.multiply(blend_color)});
            self.target.write(gamma_iter)
        } else {
            self.target.write(pixbuf.as_ref().iter().map(|x| { x.multiply(Rgb::new(0, 0, 0)) }))
        }
    }

    pub fn controls(&mut self) -> &mut PowerControls {
        &mut self.controls
    }
}