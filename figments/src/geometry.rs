//! 2D Geometry primitives such as coordinates, coordinate spaces, and rectangles
//! 
//! 
use core::fmt::Debug;
use core::ops::{Mul, Sub, Add};
use num::traits::SaturatingAdd;
use num::{One, pow, integer::Roots};
use core::cmp::{min, max};

/// Basic trait for operations on 2d coordinate components
pub trait CoordinateOp: PartialOrd + PartialEq + Sub + Clone + Mul + Copy + One + Add + Eq + Ord + Debug + Send + Sync + SaturatingAdd where
Self: Sub<Output=Self> + Add<Output=Self> {
    /// The smallest possible value within a coordinate space
    const MIN: Self;
    /// The largest possible value within a coordinate space
    const MAX: Self;
    /// Calculates the distance between two points
    fn distance(x1: Self, y1: Self, x2: Self, y2: Self) -> Self;
}

/// Trait for describing coordinate spaces
pub trait CoordinateSpace: 'static + Debug + Copy + Clone {
    /// The underlying data type used for this coordinate space
    type Data: CoordinateOp;
}

/// The fundamental 2d coordinate type
#[derive(PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Debug)]
pub struct Coordinates<S: CoordinateSpace> {
    /// X coordinate
    pub x: S::Data,
    /// Y coordinate
    pub y: S::Data,
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
    /// Creates a new coordinate
    pub const fn new(x: S::Data, y: S::Data) -> Self {
        Self {
            x,
            y
        }
    }
 
    /// Returns a new coordinate that has been rotated 90 degrees around the center of the [CoordinateSpace] a given number of times
    /// 
    /// For example,
    pub fn rotated(&self, rotation: u8) -> Self {
        match rotation % 4 {
            1 => Self { x: self.y, y: self.x },
            2 => Self { x: S::Data::MAX - self.y, y: S::Data::MAX - self.x },
            3 => Self { x: S::Data::MAX - self.y, y: S::Data::MAX - self.y },
            _ => Self { x: self.x, y: self.y }
        }
    }

    /// The most top left coordinate in the associated [CoordinateSpace]
    pub const fn top_left() -> Self {
        Self::new(S::Data::MIN, S::Data::MIN)
    }

    /// The most top right coordinate in the associated [CoordinateSpace]
    pub const fn top_right() -> Self {
        Self::new(S::Data::MAX, S::Data::MIN)
    }

    /// The most bottom left coordinate in the associated [CoordinateSpace]
    pub const fn bottom_left() -> Self {
        Self::new(S::Data::MIN, S::Data::MAX)
    }

    /// The most bottom right coordinate in the associated [CoordinateSpace]
    pub const fn bottom_right() -> Self {
        Self::new(S::Data::MAX, S::Data::MAX)
    }

    /// Calculates the distance from this point to another point
    pub fn distance_to(&self, other: &Self) -> S::Data {
        S::Data::distance(self.x, self.y, other.x, other.y)
    }
}

/// The standard virtual [CoordinateSpace], which ranges from (0, 0) to (255, 255).
#[derive(PartialEq, Debug, Copy, Clone, Default)]
pub struct Virtual {}
impl CoordinateSpace for Virtual {
    type Data = u8;
}

/// Type alias for a coordinate within the [Virtual] space
pub type VirtualCoordinates = Coordinates<Virtual>;

/// A 2d rectangle specified with two [Coordinates]
#[derive(PartialEq, Eq, Copy, Clone, Debug, PartialOrd)]
pub struct Rectangle<Space: CoordinateSpace> {
    /// Top left [Coordinates] of the rectangle
    pub top_left: Coordinates<Space>,
    /// Bottom right [Coordinates] of the rectangle
    pub bottom_right: Coordinates<Space>
}

impl<Space: CoordinateSpace> Rectangle<Space> {
    /// Creates a new rectangle using two [Coordinates]
    pub const fn new(top_left: Coordinates<Space>, bottom_right: Coordinates<Space>) -> Self {
        Self {
            top_left,
            bottom_right
        }
    }

    /// A shortcut for Rectangle::new without having to use Coordinates::new(x, y)
    pub const fn new_from_coordinates(left: Space::Data, top: Space::Data, right: Space::Data, bottom: Space::Data) -> Self {
        Self::new(Coordinates::new(left, top), Coordinates::new(right, bottom))
    }

    /// Returns a new rectangle that is rotated a number of 90 degree turns around the center of the [CoordinateSpace]
    pub fn rotated(&self, rotation: u8) -> Self {
        let a = self.top_left.rotated(rotation);
        let b = self.bottom_right.rotated(rotation);

        Self {
            top_left: Coordinates::new(min(a.x, b.x), min(a.y, b.y)),
            bottom_right: Coordinates::new(max(a.x, b.x), max(a.y, b.y))
        }
    }

    /// Creates a new rectangle that covers the entire [CoordinateSpace]
    pub const fn everything() -> Self {
        Self {
            top_left: Coordinates::<Space>::top_left(),
            bottom_right: Coordinates::<Space>::bottom_right()
        }
    }

    /// Calculates the width of the rectangle
    pub fn width(&self) -> Space::Data {
        self.bottom_right.x - self.top_left.x
    }

    /// Calculates the height of the rectangle
    pub fn height(&self) -> Space::Data {
        self.bottom_right.y - self.top_left.y
    }

    /// Returns the leftmost X coordinate of the rectangle
    pub const fn left(&self) -> Space::Data {
        self.top_left.x
    }

    /// Returns the topmost Y coordinate of the rectangle
    pub const fn top(&self) -> Space::Data {
        self.top_left.y
    }

    /// Returns the rightmost X coordinate of the rectangle
    pub const fn right (&self) -> Space::Data {
        self.bottom_right.x
    }

    /// Returns the bottommost Y coordinate of the rectangle
    pub const fn bottom(&self) -> Space::Data {
        self.bottom_right.y
    }
}
