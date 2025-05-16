use core::cmp::{max, min};
use core::ops::IndexMut;

use crate::geometry::*;
use crate::liber8tion::interpolate::scale8;
use crate::pixels::*;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
struct Stride {
    pub length: usize,
    pub x: usize,
    pub y: usize,
    pub reverse: bool,
    pub physical_idx: usize
}

impl Stride {
    pub const fn pixel_idx_for_offset(&self, offset: usize) -> usize {
        if self.reverse {
            self.physical_idx + self.length + self.y - 1 - offset
        } else {
            self.physical_idx + offset
        }
    }
}

/// A mapping between 2d [Virtual] coordinates and a 2d display composed of individual strips of pixels
#[derive(Debug)]
pub struct StrideMapping<const STRIDE_NUM: usize = 24> {
    strides: [Stride; STRIDE_NUM],

    /// The number of physical pixels in this map
    pub pixel_count: usize,

    /// The physical size of the display this map is configured for
    pub size: Rectangle<StrideSpace>,
}

impl<const STRIDE_NUM: usize> Default for StrideMapping<STRIDE_NUM> {
    fn default() -> Self {
        Self::from_json(&[
            (0, 0, 255, false)
        ])
    }
}

impl<const STRIDE_NUM: usize> StrideMapping<STRIDE_NUM> {
    /// Creates a new stride mapping from a sequence of (x, y, pixel_num, reversed)
    pub fn from_json(stride_json: &[(usize, usize, usize, bool)]) -> Self {
        let mut strides = [Stride::default(); STRIDE_NUM];
        let stride_count = stride_json.len();
        let mut physical_idx = 0;
        let mut size: Option<Rectangle<StrideSpace>> = None;
        assert!(stride_count <= STRIDE_NUM);
        for stride_idx in 0..stride_count {
            let json_data = stride_json[stride_idx];
            let x = json_data.0;
            let y = json_data.1;
            let length = json_data.2;
            let reverse = json_data.3;
            strides[stride_idx] = Stride {
                length,
                x,
                y,
                reverse,
                physical_idx
            };
            physical_idx += length;
            size = Some(match size.take() {
                None => Rectangle::new(
                    Coordinates::new(x, y),
                    Coordinates::new(x, y + length - 1),
                ),
                Some(s) => Rectangle::new(
                    Coordinates::new(
                        min(s.top_left.x, x),
                        min(s.top_left.y, y)
                    ),
                    Coordinates::new(
                        max(s.bottom_right.x, x),
                        max(s.bottom_right.y, y + length - 1)
                    )
                )
            });
        }

        Self {
            strides,
            pixel_count: physical_idx,
            size: size.unwrap(),
        }
    }
}

/// A [CoordinateSpace] where Y means which segment along a strip of LEDs, and X is which pixel within that segment
#[derive(Debug, Clone, Copy, Default)]
pub struct StrideSpace {}
impl CoordinateSpace for StrideSpace {
    type Data = usize;
}
/// Coordinates within the stride space
pub type StrideCoords = Coordinates<StrideSpace>;

/// A [CoordinateView] that maps [Virtual] coordinates to stride based coordinates
#[derive(Debug)]
pub struct StrideView<'a, P: HardwarePixel, PB: IndexMut<usize, Output = P>> {
    map: &'a StrideMapping,
    range: Rectangle<StrideSpace>,
    cur: StrideCoords,
    pixbuf: &'a mut PB,
}

impl<'a, P: HardwarePixel, PB: IndexMut<usize, Output = P>> StrideView<'a, P, PB> {

    /// Creates a new sampler that uses a [StrideMapping] to map 2d virtual coordinates to a 1d linear strip of pixels
    pub fn new(pixbuf: &'a mut PB, map: &'a StrideMapping, rect: &Rectangle<Virtual>) -> Self {
        // Zero-index shape of the pixel picking area
        let range: Rectangle<StrideSpace> = Rectangle::new(
            Coordinates::new(
                scale8(map.size.width(), rect.top_left.x) + map.size.left(),
                scale8(map.size.height(), rect.top_left.y) + map.size.top()
            ),
            Coordinates::new(
                scale8(map.size.width(), rect.bottom_right.x) + map.size.left(),
                scale8(map.size.height(), rect.bottom_right.y) + map.size.top()
            )
        );
        //log::info!("range={:?}", range);
        debug_assert!(
            range.bottom_right.x <= map.size.width() &&
            range.bottom_right.y <= map.size.height(),
            "the range for this view is out of bounds range={:?} rect={:?}, map_size={:?}",
            range,
            rect,
            (map.size.width(), map.size.height())
        );
        Self {
            map,
            range,
            cur: range.top_left,
            pixbuf
        }
    }
}

impl<'a, P: HardwarePixel + 'a, PB: IndexMut<usize, Output = P>> Iterator for StrideView<'a, P, PB> {
    type Item = (VirtualCoordinates, &'a mut P);

    fn next(&mut self) -> Option<Self::Item> {
        // Keep scanning until we reach the far right of the range
        while self.range.height() > 0 && self.cur.x <= self.range.bottom_right.x {
            //debug_assert!((self.cur.x as usize) < self.map.strides.len(), "stride out of bounds {:?}", self);
            let cur_stride: &Stride = &self.map.strides[self.cur.x];

            // Skip ahead to the top of the current stride if we are starting from higher above.
            if self.cur.y < cur_stride.y {
                self.cur.y = cur_stride.y;
            }

            // If we are at the bottom of our rectangle, or our stride, go to the next stride.
            if self.cur.y > self.range.bottom_right.y || self.cur.y > cur_stride.y + cur_stride.length - 1 {
                self.cur.x += 1;
                self.cur.y = self.range.top_left.y;
                continue;
            }

            // By now, we must be safely somewhere inside our current stride
            //debug_assert!(self.cur.y < cur_stride.y + cur_stride.length, "coords={:?} out of bounds for stride={:?} view={:?}", self.cur, cur_stride, self);

            // Move to the next coord and return
            let physical_coords = self.cur;
            self.cur.y += 1;

            /*let virtual_coords = VirtualCoordinates::new(
                physical_coords.x.saturating_mul(self.step_size.x),
                physical_coords.y.saturating_mul(self.step_size.y)
            );*/

            let x_pct = (physical_coords.x - self.range.left()) as f32 / self.range.width() as f32;
            let y_pct = (physical_coords.y - self.range.top()) as f32 / self.range.height() as f32;

            let virtual_coords = VirtualCoordinates::new(
                (255f32 * x_pct) as u8,
                (255f32 * y_pct) as u8
            );

            let idx = self.map.strides[physical_coords.x].pixel_idx_for_offset(physical_coords.y);

            let entry = unsafe {
                &mut *(&mut self.pixbuf[idx] as *mut P)
            };

            return Some((virtual_coords, entry));
        }

        None
    }
}