use core::fmt::{Debug, Formatter};
use core::ops::{Mul, Sub, Add};
use num::{One, pow, integer::Roots};
use core::cmp::{min, max};

pub trait CoordinateOp: PartialOrd + PartialEq + Sub + Clone + Mul + Copy + One + Add + Eq + Ord + Debug where
Self: Sub<Output=Self> + Add<Output=Self> {
    const MIN: Self;
    const MAX: Self;
    fn distance(x1: Self, y1: Self, x2: Self, y2: Self) -> Self;
}

pub trait CoordinateSpace {
    type Data: CoordinateOp;
}

#[derive(PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub struct Coordinates<S: CoordinateSpace> {
    pub x: S::Data,
    pub y: S::Data,
}

impl<S: CoordinateSpace> Debug for Coordinates<S> where S::Data: Debug {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("@")
            .field(&self.x)
            .field(&self.y)
            .finish()
    }
}

impl CoordinateOp for u8 {
    const MIN: u8 = 0;
    const MAX: u8 = 255;

    fn distance(x1: Self, y1: Self, x2: Self, y2: Self) -> Self {
        let dx = (max(x1, x2) - min(x1, x2)) as u16;
        let dy = (max(y1, y2) - min(y1, y2)) as u16;
        (dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy))).sqrt() as u8
    }
}

impl CoordinateOp for u16 {
    const MIN: u16 = u16::MIN;
    const MAX: u16 = u16::MAX;

    fn distance(x1: Self, y1: Self, x2: Self, y2: Self) -> Self {
        (pow(x2 - x1, 2) + pow(y2 - y1, 2)).sqrt()
    }
}

impl CoordinateOp for usize {
    const MIN: usize = usize::MIN;
    const MAX: usize = usize::MAX;

    fn distance(x1: Self, y1: Self, x2: Self, y2: Self) -> Self {
        (pow(x2 - x1, 2) + pow(y2 - y1, 2)).sqrt()
    }
}

impl<S: CoordinateSpace> Coordinates<S> {
    pub const fn new(x: S::Data, y: S::Data) -> Self {
        Self {
            x,
            y
        }
    }
 
    pub fn rotated(&self, rotation: u8) -> Self {
        match rotation {
            1 => Self { x: self.y, y: self.x },
            2 => Self { x: S::Data::MAX - self.y, y: S::Data::MAX - self.x },
            3 => Self { x: S::Data::MAX - self.y, y: S::Data::MAX - self.y },
            _ => Self { x: self.x, y: self.y }
        }
    }

    pub const fn top_left() -> Self {
        Self::new(S::Data::MIN, S::Data::MIN)
    }

    pub const fn top_right() -> Self {
        Self::new(S::Data::MAX, S::Data::MIN)
    }

    pub const fn bottom_left() -> Self {
        Self::new(S::Data::MIN, S::Data::MAX)
    }

    pub const fn bottom_right() -> Self {
        Self::new(S::Data::MAX, S::Data::MAX)
    }

    pub fn distance_to(&self, other: &Self) -> S::Data {
        S::Data::distance(self.x, self.y, other.x, other.y)
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct Virtual {}
impl CoordinateSpace for Virtual {
    type Data = u8;
}

pub type VirtualCoordinates = Coordinates<Virtual>;

#[derive(PartialEq, Eq, Copy, Clone, Debug, PartialOrd)]
pub struct Rectangle<Space: CoordinateSpace> {
    pub top_left: Coordinates<Space>,
    pub bottom_right: Coordinates<Space>
}

impl<Space: CoordinateSpace> Rectangle<Space> {
    pub const fn new(top_left: Coordinates<Space>, bottom_right: Coordinates<Space>) -> Self {
        Self {
            top_left,
            bottom_right
        }
    }

    pub fn rotated(&self, rotation: u8) -> Self {
        let a = self.top_left.rotated(rotation);
        let b = self.bottom_right.rotated(rotation);

        Self {
            top_left: Coordinates::new(min(a.x, b.x), min(a.y, b.y)),
            bottom_right: Coordinates::new(max(a.x, b.x), max(a.y, b.y))
        }
    }

    pub const fn everything() -> Self {
        Self {
            top_left: Coordinates::<Space>::top_left(),
            bottom_right: Coordinates::<Space>::bottom_right()
        }
    }

    pub fn width(&self) -> Space::Data {
        self.bottom_right.x - self.top_left.x
    }

    pub fn height(&self) -> Space::Data {
        self.bottom_right.y - self.top_left.y
    }

    pub const fn left(&self) -> Space::Data {
        self.top_left.x
    }

    pub const fn top(&self) -> Space::Data {
        self.top_left.y
    }

    pub const fn right (&self) -> Space::Data {
        self.bottom_right.x
    }

    pub const fn bottom(&self) -> Space::Data {
        self.bottom_right.y
    }
}
