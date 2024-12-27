use super::buffers::Pixbuf;
use super::geometry::*;

use crate::lib8::interpolate::scale8;
use super::render::PixelView;

use core::cmp::{max, min};
use core::fmt::{Formatter, Debug};

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

impl<'a> CoordinateView<'a> for LinearCoordView {
    type Space = LinearSpace;
    fn next(&mut self) -> Option<(VirtualCoordinates, LinearCoords)> {
        if self.idx as u8 == self.max_x {
            None
        } else {
            let virt = VirtualCoordinates::new(self.idx as u8, 0); // FIXME: scale8
            let phys = LinearCoords::new(
                self.idx as usize,
                0
            );
            self.idx += 1;
            return Some((virt, phys))
        }
    }
}

pub struct LinearPixelMapping {
}

impl LinearPixelMapping {
    pub fn new() -> Self {
        Self {}
    }
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

impl<const STRIDE_NUM: usize> StrideMapping<STRIDE_NUM> {
    pub fn new() -> Self {
        Self::from_json(&[
            (0, 0, 255, false)
        ])
    }

    pub fn new_fairylights() -> Self {
        Self::from_json(&[
            (0, 0, 50, false)
        ])
    }

    pub fn new_cyberplague() -> Self {
        Self::from_json(&[
            (0, 6, 6, false),
            (1, 6, 6, true),
            (2, 6, 6, false),
            (3, 4, 9, true),
            (4, 4, 14, false),
            (5, 0, 17, true),
            (6, 2, 12, false),
            (7, 0, 18, true),
            (8, 4, 14, false),
            (9, 5, 9, true),
            (10, 4, 7, false),
            (11, 5, 6, true),
            (12, 5, 6, false)
        ])
    }

    pub fn new_jar() -> Self {
        Self::from_json(&[
            (0, 0, 17, false),
            (1, 0, 17, false),
            (2, 0, 17, false),
            (3, 0, 17, false),
            (4, 0, 16, false),
            (5, 0, 17, false),
            (6, 0, 17, false),
            (7, 0, 17, false),
            (8, 0, 17, false),
            (9, 0, 17, false),
            (10, 0, 17, false),
            (11, 0, 17, false),
            (12, 0, 18, false),
            (13, 0, 17, false),
            (14, 0, 18, false),
            (15, 0, 17, false),
            (16, 0, 17, false),
            (17, 0, 17, false)
        ])
    }

    pub fn new_panel() -> Self {
        Self::from_json(&[
            (0, 0, 16, false),
            (1, 0, 16, true),
            (2, 0, 16, false),
            (3, 0, 16, true),
            (4, 0, 16, false),
            (5, 0, 16, true),
            (6, 0, 16, false),
            (7, 0, 16, true),
            (8, 0, 16, false),
            (9, 0, 16, true),
            (10, 0, 16, false),
            (11, 0, 16, true),
            (12, 0, 16, false),
            (13, 0, 16, true),
            (14, 0, 16, false),
            (15, 0, 16, true),
        ])
    }

    pub fn new_albus() -> Self {
        Self::from_json(&[
            (0, 0, 29, false),
            (1, 0, 20, false),
            (2, 0, 22, false),
            (3, 0, 19, false),
            (4, 0, 12, false),
            (5, 0, 14, false),
            (6, 0, 16, false),
            (7, 0, 19, false),
        ])
    }

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
            log::info!("stride={:?} size={:?}", strides[stride_idx], size);
        }
        let s = size.take().unwrap();
        log::info!("size={:?}", s);

        Self {
            strides,
            pixel_count: physical_idx,
            size: s,
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

impl<'a> Debug for StrideView<'a> {
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
                scale8(map.size.width() as u8, rect.top_left.x) + map.size.left() as u8,
                scale8(map.size.height() as u8, rect.top_left.y) + map.size.top() as u8
            ),
            Coordinates::new(
                scale8(map.size.width() as u8, rect.bottom_right.x) + map.size.left() as u8,
                scale8(map.size.height() as u8, rect.bottom_right.y) + map.size.top() as u8
            )
        );
        //log::info!("rect={:?} map.size={:?} range={:?}", rect, map.size, range);
        debug_assert!(
            range.bottom_right.x <= map.size.width() as u8 &&
            range.bottom_right.y <= map.size.height() as u8,
            "the range for this view is out of bounds range={:?} rect={:?}, map_size={:?}",
            range,
            rect,
            (map.size.width(), map.size.height())
        );
        let step_size = VirtualCoordinates::new(
            u8::MAX / core::cmp::max(1, range.width()),
            u8::MAX / core::cmp::max(1, range.height())
            //scale8(255, std::cmp::max(1, range.bottom_right.x - range.top_left.x)),
            //scale8(255, std::cmp::max(1, range.bottom_right.y - range.top_left.y))
        );
        debug_assert_ne!(step_size.x, 0);
        debug_assert_ne!(step_size.y, 0);
        return Self {
            map,
            range,
            step_size,
            cur: range.top_left
        };
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
            debug_assert!(self.cur.y <= cur_stride.y + cur_stride.length - 1, "coords={:?} out of bounds for stride={:?} view={:?}", self.cur, cur_stride, self);

            // Move to the next coord and return
            let physical_coords = self.cur;
            self.cur.y += 1;

            let virtual_coords = VirtualCoordinates::new(
                (physical_coords.x as u8).saturating_mul(self.step_size.x),
                (physical_coords.y as u8).saturating_mul(self.step_size.y)
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

impl<'a, P: Pixbuf> PixelView for StrideSampler<'a, P> {
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