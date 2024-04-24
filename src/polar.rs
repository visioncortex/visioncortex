use crate::Point2;
use num_traits::Float;

/// Polar coordinate in 2D space
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Polar2<F: Float> {
    pub a: F,
    pub r: F,
}

impl<F: Float> Polar2<F> {
    pub fn to_point(&self) -> Point2<F> {
        Point2 {
            x: self.r * self.a.cos(),
            y: self.r * self.a.sin(),
        }
    }
}

/// 2D Polar with `f64` component
pub type PolarF64 = Polar2<f64>;