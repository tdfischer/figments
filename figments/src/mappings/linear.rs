use core::cmp::min;
use core::fmt::Debug;

use crate::geometry::*;
use crate::pixels::*;
use crate::render::Sample;

/// Linear coordinate space where Y is meaningless and X points to a unique pixel
#[derive(Debug, Clone, Copy, Default)]
pub struct LinearSpace {}
impl CoordinateSpace for LinearSpace {
    type Data = usize;
}

impl<'a, Pixel: 'a> Sample<'a, LinearSpace> for [Pixel] {
    type Output = Pixel;

    fn sample(&mut self, rect: &Rectangle<LinearSpace>) -> impl Iterator<Item = (Coordinates<LinearSpace>, &'a mut Self::Output)> {
        let size = self.len();
        // Clip the pixbuf at the left side of the rectangle
        let (_, rest) = self.split_at_mut(min(rect.left(), size));
        // Clip again on the other end of the rectangle
        let (subset, _) = rest.split_at_mut(min(rect.width(), rest.len()));
        // Trick the borrow checker, until we can rewrite the sample trait to use a lifetime generic parameter
        let bufref = unsafe {
            (subset as *mut [Pixel]).as_mut().unwrap()
        };
        // Enumerate each pixel into an absolute (x, 0) coordinate along with the sampled pixel
        bufref.iter_mut().enumerate().map(|(idx, pix)| {
            (Coordinates::new(idx + rect.left(), 0), pix)
        })
    }
}

impl<'a, Pixel: 'a, const N: usize> Sample<'a, LinearSpace> for [Pixel; N] {
    type Output = Pixel;

    fn sample(&mut self, rect: &Rectangle<LinearSpace>) -> impl Iterator<Item = (Coordinates<LinearSpace>, &'a mut Self::Output)> {
        let size = self.len();
        // Clip the pixbuf at the left side of the rectangle
        let (_, rest) = self.split_at_mut(min(rect.left(), size));
        // Clip again on the other end of the rectangle
        let (subset, _) = rest.split_at_mut(min(rect.width(), rest.len()));
        // Trick the borrow checker, until we can rewrite the sample trait to use a lifetime generic parameter
        let bufref = unsafe {
            (subset as *mut [Pixel]).as_mut().unwrap()
        };
        // Enumerate each pixel into an absolute (x, 0) coordinate along with the sampled pixel
        bufref.iter_mut().enumerate().map(|(idx, pix)| {
            (Coordinates::new(idx + rect.left(), 0), pix)
        })
    }
}

#[cfg(test)]
mod test {
    use core::array;
    use core::cmp::min;

    use rgb::Rgb;
    use crate::{mappings::linear::LinearSpace, prelude::*};

    fn with_sample<const PIXEL_COUNT: usize>(pixbuf: &mut [Rgb<u8>], rect: &Rectangle<LinearSpace>, mut shader: impl FnMut(Coordinates<LinearSpace>, &mut Rgb<u8>)) {
        for (coords, pix) in pixbuf.sample(rect) {
            assert!(coords.x < PIXEL_COUNT, "{:?} is outside the {} pixel buffer while sampling {rect:?}", coords, PIXEL_COUNT);
            shader(coords, pix);
        }
    }

    fn test_gradient<const PIXEL_COUNT: usize>(rect: &Rectangle<LinearSpace>, expected: usize) {
        let mut num_sampled = 0;

        // The buffer starts with increasingly red pixels
        let mut pixbuf: [_; PIXEL_COUNT] = array::from_fn(|n| { Rgb::new(n as u8, 0, 0) });

        let buf_start = rect.left();
        let buf_end = min(pixbuf.len(), buf_start + rect.width());

        // Then, set the blue value to the selection coordinates
        with_sample::<PIXEL_COUNT>(&mut pixbuf, rect, |coords: Coordinates<LinearSpace>, pix| {
            assert_eq!(pix, &Rgb::new(coords.x as u8, 0, 0));
            pix.b = coords.x as u8;
            num_sampled += 1;
        });
        assert_eq!(num_sampled, expected, "Expected to sample {expected} pixels but only found {num_sampled} while sampling {rect:?}");

        for (idx, pix) in pixbuf[..].into_iter().enumerate() {
            if idx < buf_start || idx >= buf_end {
                // Pixels outside the selection should be untouched
                assert_eq!(pix, &Rgb::new(idx as u8, 0, 0), "Pixel {idx}/{PIXEL_COUNT} was unexpectedly written with {pix:?} while sampling {rect:?}");
            } else {
                // And pixels inside the selection should be updated
                assert_eq!(pix, &Rgb::new(idx as u8, 0, idx as u8), "Pixel {idx}/{PIXEL_COUNT} has incorrect color {pix:?} while sampling {rect:?}");
            }
        }
    }

    #[test]
    fn test_full_range_samples() {
        let everything = Rectangle::everything();
        test_gradient::<0>(&everything, 0);
        test_gradient::<255>(&everything, 255);
        test_gradient::<1024>(&everything, 1024);
    }

    #[test]
    fn test_top_left_samples() {
        for width in 0..128 {
            let rect = Rectangle::new(Coordinates::new(0, 0), Coordinates::new(width, width));
            test_gradient::<0>(&rect, 0);
            test_gradient::<255>(&rect, width);
            test_gradient::<1024>(&rect, width);
        }
    }

    #[test]
    fn test_square_samples() {
        for width in 0..128 {
            let rect = Rectangle::new(Coordinates::new(width, width), Coordinates::new(width * 2, width * 2));
            test_gradient::<0>(&rect, 0);
            test_gradient::<255>(&rect, width);
            test_gradient::<1024>(&rect, width);
        }
    }
}