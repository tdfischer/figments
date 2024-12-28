use core::cmp::{max, min};
use core::fmt::{Formatter, Debug};

use crate::liber8tion::interpolate::scale8;

use super::buffers::Pixbuf;
use super::geometry::*;
use super::render::PixelView;

pub trait CoordinateView<'a>: Debug {
    type Space: CoordinateSpace;
    fn next(&mut self) -> Option<(Coordinates<Virtual>, Coordinates<Self::Space>)>;
}

pub trait Select<'a> {
    type Space: CoordinateSpace;
    type View: CoordinateView<'a>;
    fn select(&'a self, rect: &Rectangle<Virtual>) -> Self::View;
}

#[derive(Debug)]
pub struct LinearCoordView {
    max_x: u8,
    idx: usize,
}

pub struct LinearSpace {}
impl CoordinateSpace for LinearSpace {
    type Data = usize;
}

pub type LinearCoords = Coordinates<LinearSpace>;

impl CoordinateView<'_> for LinearCoordView {
    type Space = LinearSpace;
    fn next(&mut self) -> Option<(VirtualCoordinates, LinearCoords)> {
        if self.idx as u8 == self.max_x {
            None
        } else {
            let virt = VirtualCoordinates::new(self.idx as u8, 0); // FIXME: scale8
            let phys = LinearCoords::new(
                self.idx,
                0
            );
            self.idx += 1;
            Some((virt, phys))
        }
    }
}

#[derive(Default)]
pub struct LinearPixelMapping {
}

impl<'a> Select<'a> for LinearPixelMapping {
    type Space = LinearSpace;
    type View = LinearCoordView;
    fn select(&'a self, rect: &Rectangle<Virtual>) -> Self::View {
        LinearCoordView {
            max_x: rect.bottom_right.x,
            idx: 0,
        }
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stride {
    pub length: u8,
    pub x: u8,
    pub y: u8,
    pub reverse: bool,
    pub physical_idx: usize
}

impl Stride {
    pub const fn pixel_idx_for_offset(&self, offset: u8) -> usize {
        if self.reverse {
            self.physical_idx + (self.length + self.y - 1 - offset) as usize
        } else {
            self.physical_idx + offset as usize
        }
    }
}

#[derive(Debug)]
pub struct StrideMapping<const STRIDE_NUM: usize = 24> {
    pub strides: [Stride; STRIDE_NUM],
    pub pixel_count: usize,
    pub size: Rectangle<StrideSpace>,
    rotation: u8
}

impl<const STRIDE_NUM: usize> Default for StrideMapping<STRIDE_NUM> {
    fn default() -> Self {
        Self::from_json(&[
            (0, 0, 255, false)
        ])
    }
}

impl<const STRIDE_NUM: usize> StrideMapping<STRIDE_NUM> {
    pub fn from_json(stride_json: &[(u8, u8, u8, bool)]) -> Self {
        let mut strides = [Stride::default(); STRIDE_NUM];
        let stride_count = stride_json.len();
        let mut physical_idx = 0;
        let mut size: Option<Rectangle<StrideSpace>> = None;
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
            physical_idx += length as usize;
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
            rotation: 2
        }
    }
}

impl<'a> Select<'a> for StrideMapping {
    type Space = StrideSpace;
    type View = StrideView<'a>;
    fn select(&'a self, rect: &Rectangle<Virtual>) -> Self::View {
        StrideView::new(self, &rect.rotated(self.rotation))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StrideSpace {}
impl CoordinateSpace for StrideSpace {
    type Data = u8;
}
pub type StrideCoords = Coordinates<StrideSpace>;

pub struct StrideView<'a> {
    pub map: &'a StrideMapping,
    range: Rectangle<StrideSpace>,
    cur: StrideCoords,
    step_size: VirtualCoordinates
}

impl Debug for StrideView<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StrideView")
        .field("range", &self.range)
        .field("step", &self.step_size)
        .field("cur", &self.cur).finish()
    }
}

impl<'a> StrideView<'a> {
    fn new(map: &'a StrideMapping, rect: &Rectangle<Virtual>) -> Self {
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
        debug_assert!(
            range.bottom_right.x <= map.size.width() &&
            range.bottom_right.y <= map.size.height(),
            "the range for this view is out of bounds range={:?} rect={:?}, map_size={:?}",
            range,
            rect,
            (map.size.width(), map.size.height())
        );
        let step_size = VirtualCoordinates::new(
            u8::MAX / core::cmp::max(1, range.width()),
            u8::MAX / core::cmp::max(1, range.height())
        );
        debug_assert_ne!(step_size.x, 0);
        debug_assert_ne!(step_size.y, 0);
        Self {
            map,
            range,
            step_size,
            cur: range.top_left
        }
    }
}

impl<'a> CoordinateView<'a> for StrideView<'a> {
    type Space = StrideSpace;
    fn next(&mut self) -> Option<(VirtualCoordinates, StrideCoords)> {
        // Keep scanning until we reach the far right of the range
        while self.cur.x <= self.range.bottom_right.x {
            debug_assert!((self.cur.x as usize) < self.map.strides.len(), "stride out of bounds {:?}", self);
            let cur_stride: &Stride = &self.map.strides[self.cur.x as usize];

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
            debug_assert!(self.cur.y < cur_stride.y + cur_stride.length, "coords={:?} out of bounds for stride={:?} view={:?}", self.cur, cur_stride, self);

            // Move to the next coord and return
            let physical_coords = self.cur;
            self.cur.y += 1;

            let virtual_coords = VirtualCoordinates::new(
                physical_coords.x.saturating_mul(self.step_size.x),
                physical_coords.y.saturating_mul(self.step_size.y)
            );

            return Some((virtual_coords,  physical_coords));
        }

        None
    }
}

pub struct StrideSampler<'a, P: Pixbuf> {
    pixbuf: &'a mut P,
    selection: StrideView<'a>
}

impl<'a, P: Pixbuf> StrideSampler<'a, P> {
    pub fn new(pixbuf: &'a mut P, selection: StrideView<'a>) -> Self {
        StrideSampler {
            pixbuf,
            selection
        }
    }
}

impl<P: Pixbuf> PixelView for StrideSampler<'_, P> {
    type Pixel = P::Pixel;
    fn next(&mut self) -> Option<(Coordinates<Virtual>, &mut Self::Pixel)> {
        if let Some((virt, coords)) = self.selection.next() {
            let idx = self.selection.map.strides[coords.x as usize].pixel_idx_for_offset(coords.y);
            Some((virt, &mut self.pixbuf[idx]))
        } else {
            None
        }
    }
}