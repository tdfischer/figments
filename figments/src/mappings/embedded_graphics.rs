#![cfg(feature="embedded-graphics")]
use embedded_graphics::{pixelcolor::BinaryColor, prelude::{Dimensions, DrawTarget}, Pixel};

use crate::prelude::*;

#[derive(Default, Debug, Clone, Copy)]
pub struct Matrix2DSpace {}

impl CoordinateSpace for Matrix2DSpace {
    type Data = i32;
}

pub struct EmbeddedGraphicsSampler<'a, T: ?Sized>(pub &'a mut T, pub embedded_graphics::primitives::Rectangle);

impl<'a, T> Dimensions for EmbeddedGraphicsSampler<'a, T> {
    fn bounding_box(&self) -> embedded_graphics::primitives::Rectangle {
        self.1
    }
}

impl<'a, T> DrawTarget for EmbeddedGraphicsSampler<'a, T> where T: Sample<'a, Matrix2DSpace>, T::Output: PixelSink<BinaryColor> {
    type Color = BinaryColor;

    type Error = ();

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>> {
        for pix in pixels {
            let rect = Rectangle::new(Coordinates::new(pix.0.x, pix.0.y), Coordinates::new(pix.0.x, pix.0.y));
            for (coords, fpix) in self.0.sample(&rect) {
                fpix.set(&pix.1);
            }
        }

        Ok(())
    }
}

impl From<embedded_graphics::geometry::Point> for Coordinates<Matrix2DSpace> {
    fn from(value: embedded_graphics::geometry::Point) -> Self {
        Coordinates::new(value.x, value.y)
    }
}