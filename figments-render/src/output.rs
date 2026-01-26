#![allow(async_fn_in_trait)]

use figments::{liber8tion::interpolate::Fract8, prelude::*};

use crate::gamma::GammaCurve;

pub trait Brightness {
    fn set_brightness(&mut self, brightness: Fract8);
    fn set_on(&mut self, is_on: bool);
}

pub trait GammaCorrected {
    fn set_gamma(&mut self, gamma: GammaCurve);
}

/// A hardware output that provides an interface to the underlying hardware pixels, including actually turning pixels into photons
pub trait Output<'a, SampleSpace: CoordinateSpace>: Sample<'a, SampleSpace> {
    type Error;
    type Controls: Brightness + GammaCorrected;

    /// Commits the contents of the underlying pixel buffers to hardware
    fn commit(&mut self)  -> Result<(), Self::Error>;

    fn controls(&mut self) -> Option<&mut Self::Controls>;
}

/// A hardware output that provides an interface to the underlying hardware pixels, including actually turning pixels into photons, but async flavored
pub trait OutputAsync<'a, SampleSpace: CoordinateSpace>: Sample<'a, SampleSpace> {
    type Error;
    type Controls: Brightness + GammaCorrected;

    /// Commits the contents of the underlying pixel buffers to hardware
    async fn commit_async(&mut self)  -> Result<(), Self::Error>;

    fn controls(&mut self) -> Option<&mut Self::Controls>;
}

#[derive(Default, Debug, Clone, Copy)]
pub struct NullControls {}

#[expect(unused_variables)]
impl Brightness for NullControls {
    fn set_brightness(&mut self, brightness: Fract8) {}
    fn set_on(&mut self, is_on: bool) {}
}

#[allow(unused_variables)]
impl GammaCorrected for NullControls {
    fn set_gamma(&mut self, gamma: GammaCurve) {}
}