#![allow(async_fn_in_trait)]

use figments::prelude::*;

use crate::gamma::GammaCurve;

pub trait Brightness {
    fn set_brightness(&mut self, brightness: u8);
    fn set_on(&mut self, is_on: bool);
}

pub trait GammaCorrected {
    fn set_gamma(&mut self, gamma: GammaCurve);
}

/// A hardware output that provides an interface to the underlying hardware pixels, including actually turning pixels into photons
pub trait Output<'a, SampleSpace: CoordinateSpace>: Sample<'a, SampleSpace, Output = Self::HardwarePixel> + 'a {
    type HardwarePixel: PixelFormat;
    type Error;
    type Controls: Brightness + GammaCorrected;

    /// Commits the contents of the underlying pixel buffers to hardware
    fn commit(&mut self)  -> Result<(), Self::Error>;

    fn controls(&self) -> Option<&Self::Controls>;
}

/// A hardware output that provides an interface to the underlying hardware pixels, including actually turning pixels into photons, but async flavored
pub trait OutputAsync<'a, SampleSpace: CoordinateSpace>: Sample<'a, SampleSpace, Output = Self::HardwarePixel> + 'a {
    type HardwarePixel: PixelFormat;
    type Error;
    type Controls: Brightness + GammaCorrected;

    /// Commits the contents of the underlying pixel buffers to hardware
    async fn commit_async(&mut self)  -> Result<(), Self::Error>;

    fn controls(&self) -> Option<&Self::Controls>;
}

#[derive(Default, Debug, Clone, Copy)]
pub struct NullControls {}

impl Brightness for NullControls {
    fn set_brightness(&mut self, brightness: u8) {}
    fn set_on(&mut self, is_on: bool) {}
}

impl GammaCorrected for NullControls {
    fn set_gamma(&mut self, gamma: GammaCurve) {}
}