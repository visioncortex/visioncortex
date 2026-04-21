use crate::{DetermMath, Point2};
use num_traits::Float;

/// Polar coordinate in 2D space
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Polar2<F: Float> {
    /// angle
    pub a: F,
    /// radius
    pub r: F,
}

impl<F: Float + DetermMath> Polar2<F> {
    pub fn to_point(&self) -> Point2<F> {
        Point2 {
            x: self.r * self.a.determ_cos(),
            y: self.r * self.a.determ_sin(),
        }
    }
}

/// 2D Polar with `f64` component
pub type PolarF64 = Polar2<f64>;