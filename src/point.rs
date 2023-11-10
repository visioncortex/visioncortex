use flo_curves::{Coordinate, Coordinate2D};
use num_traits::Float;
use std::{cmp::PartialOrd, convert::{From, Into}, fmt::Display, ops::*};

/// Generic point in 2D space
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point2<T> {
    pub x: T,
    pub y: T,
}

pub trait PointType: Default + Copy {
    fn from<P: PointType>(p: &P) -> Self;

    fn to<T: PointType>(&self) -> T {
        T::from(self)
    }

    fn to_point_f64(&self) -> PointF64;

    fn to_point_i32(&self) -> PointI32;
}

pub trait ToSvgString {
    fn to_svg_string(&self, precision: Option<u32>) -> String;
}

impl<T> ToSvgString for Point2<T> 
where
    T: Copy + NumberFormat
{
    fn to_svg_string(&self, precision: Option<u32>) -> String {
        format!("{},{}", Self::number_format(self.x, precision), Self::number_format(self.y, precision))
    }
}

pub trait NumberFormat: Display {
    fn number_format(num: Self, precision: Option<u32>) -> String;
}

impl NumberFormat for i32 {
    fn number_format(num: Self, _precision: Option<u32>) -> String {
        format!("{}", num)
    }
}

impl NumberFormat for f64 {
    fn number_format(num: Self, precision: Option<u32>) -> String {
        match precision {
            None => format!("{}", num),
            Some(0) => format!("{1:.0$}", 0, num),
            Some(p) => {
                let mut string: String = format!("{1:.0$}", p as usize, num);
                string = string.trim_end_matches('0').trim_end_matches('.').to_owned();
                string
            },
        }
    }
}

impl<T> Point2<T>
where
    T: NumberFormat,
{
    #[inline]
    pub(crate) fn number_format(num: T, precision: Option<u32>) -> String {
        NumberFormat::number_format(num, precision)
    }
}

impl<T> Point2<T> {
    #[inline]
    pub const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl<T> Point2<T>
where
    T: Add<Output = T> + Mul<Output = T>,
{
    #[inline]
    pub fn dot(self, v: Self) -> T {
        self.x * v.x + self.y * v.y
    }
}

impl<T> Point2<T>
where
    T: Add<Output = T>
{
    #[inline]
    pub fn translate(self, vector: Self) -> Self {
        self + vector
    }
}

impl<T> Point2<T>
where
    T: Add<Output = T> + Copy + Neg<Output = T> + Sub<Output = T>,
{
    #[inline]
    /// Assumes a coordinate system with origin at the top-left. The behavior
    /// is undefined otherwise.
    pub fn rotate_90deg(&self, origin: Self, clockwise: bool) -> Self {
        let o = origin;

        if !clockwise {
            Self {
                x: (self.y - o.y) + o.x,
                y: -(self.x - o.x) + o.y,
            }
        } else {
            Self {
                x: -(self.y - o.y) + o.x,
                y: (self.x - o.x) + o.y,
            }
        }
    }
}

impl<T> Point2<T>
where
    T: Float,
{
    #[inline]
    pub fn rotate(&self, origin: Self, angle: T) -> Self {
        let o = origin;
        let a = angle;
        Self {
            x: a.cos() * (self.x - o.x) - a.sin() * (self.y - o.y) + o.x,
            y: a.sin() * (self.x - o.x) + a.cos() * (self.y - o.y) + o.y,
        }
    }

    #[inline]
    /// The L2-norm
    pub fn norm(self) -> T {
        self.dot(self).sqrt()
    }

    #[inline]
    /// The euclidean distance
    pub fn distance_to(&self, other: Point2<T>) -> T {
        (*self - other).norm()
    }
}

impl<T> Point2<T>
where
    T: Default + Float,
{
    #[inline]
    pub fn get_normalized(&self) -> Self {
        let norm = self.norm();
        if norm != T::zero() {
            *self / norm
        } else {
            Self::default()
        }
    }

}

impl<T> Neg for Point2<T>
where
    T: Neg<Output = T>,
{
    type Output = Self;
    #[inline]
    fn neg(self) -> Self::Output {
        Self {
            x: self.x.neg(),
            y: self.y.neg(),
        }
    }
}

impl<T> Add for Point2<T>
where
    T: Add<Output = T>,
{
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x.add(other.x),
            y: self.y.add(other.y),
        }
    }
}

impl<T> AddAssign for Point2<T>
where
    T: AddAssign,
{   #[inline]
    fn add_assign(&mut self, other: Self) {
        self.x.add_assign(other.x);
        self.y.add_assign(other.y);
    }
}

impl<T> Sub for Point2<T>
where
    T: Sub<Output = T>,
{
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x.sub(other.x),
            y: self.y.sub(other.y),
        }
    }
}

impl<T> SubAssign for Point2<T>
where
    T: SubAssign,
{
    #[inline]
    fn sub_assign(&mut self, other: Self) {
        self.x.sub_assign(other.x);
        self.y.sub_assign(other.y);
    }
}

impl<T, F> Mul<F> for Point2<T>
where
    T: Mul<F, Output = T>,
    F: Float,
{
    type Output = Self;

    fn mul(self, rhs: F) -> Self::Output {
        Self {
            x: self.x.mul(rhs),
            y: self.y.mul(rhs),
        }
    }
}

impl<T, F> MulAssign<F> for Point2<T>
where
    T: MulAssign<F>,
    F: Float,
{
    fn mul_assign(&mut self, rhs: F) {
        self.x.mul_assign(rhs);
        self.y.mul_assign(rhs);
    }
}

impl<T, F> Div<F> for Point2<T>
where
    T: Div<F, Output = T>,
    F: Float,
{
    type Output = Self;

    #[inline]
    fn div(self, rhs: F) -> Self::Output {
        Self {
            x: self.x.div(rhs),
            y: self.y.div(rhs),
        }
    }
}

impl<T, F> DivAssign<F> for Point2<T>
where
    T: DivAssign<F>,
    F: Float,
{
    #[inline]
    fn div_assign(&mut self, rhs: F) {
        self.x.div_assign(rhs);
        self.y.div_assign(rhs);
    }
}

impl<F> Coordinate2D for Point2<F>
where
    F: Copy + Into<f64>,
{
    fn x(&self) -> f64 {
        self.x.into()
    }

    fn y(&self) -> f64 {
        self.y.into()
    }
}

impl<F> Coordinate for Point2<F>
where
    F: Add<Output = F> + Copy + Default + Float + From<f64> + Into<f64> + Mul<f64, Output = F> + PartialEq + Sub<Output = F>,
{
    #[inline]
    fn from_components(components: &[f64]) -> Self {
        Self::new(components[0].into(), components[1].into())
    }

    #[inline]
    fn origin() -> Self {
        Self::default()
    }

    #[inline]
    fn len() -> usize {
        2
    }

    #[inline]
    fn get(&self, index: usize) -> f64 {
        match index {
            0 => self.x.into(),
            1 => self.y.into(),
            _ => panic!("Point2 only has two components")
        }
    }

    fn from_biggest_components(p1: Self, p2: Self) -> Self {
        Self::new(
            f64::from_biggest_components(p1.x.into(), p2.x.into()).into(),
            f64::from_biggest_components(p1.y.into(), p2.y.into()).into(),
        )
    }

    fn from_smallest_components(p1: Self, p2: Self) -> Self {
        Self::new(
            f64::from_smallest_components(p1.x.into(), p2.x.into()).into(),
            f64::from_smallest_components(p1.y.into(), p2.y.into()).into(),
        )
    }
}

/// 2D Point with `u8` component
pub type PointU8 = Point2<u8>;
/// 2D Point with `usize` component
pub type PointUsize = Point2<usize>;
/// 2D Point with `i32` component
pub type PointI32 = Point2<i32>;
/// 2D Point with `f32` component
pub type PointF32 = Point2<f32>;
/// 2D Point with `f64` component
pub type PointF64 = Point2<f64>;

impl PointI32 {
    pub fn to_point_usize(&self) -> PointUsize {
        PointUsize {x: self.x as usize, y: self.y as usize}
    }

    pub fn to_point_f64(&self) -> PointF64 {
        PointF64 { x: self.x as f64, y: self.y as f64 }
    }
}

impl PointF64 {
    pub fn to_point_i32(&self) -> PointI32 {
        PointI32 { x: self.x as i32, y: self.y as i32 }
    }

    pub fn to_point_f32(&self) -> PointF32 {
        PointF32 { x: self.x as f32, y: self.y as f32 }
    }
}

impl PointType for PointI32 {
    fn from<P: PointType>(p: &P) -> Self {
        p.to_point_i32()
    }

    #[inline]
    fn to_point_f64(&self) -> PointF64 {
        self.to_point_f64()
    }

    #[inline]
    fn to_point_i32(&self) -> PointI32 {
        *self
    }
}

impl PointType for PointF64 {
    fn from<P: PointType>(p: &P) -> Self {
        p.to_point_f64()
    }

    #[inline]
    fn to_point_f64(&self) -> PointF64 {
        *self
    }

    #[inline]
    fn to_point_i32(&self) -> PointI32 {
        self.to_point_i32()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// rotate counter clockwise by 90 degrees
    fn pointf64_rotate() {
        let p = PointF64 { x: 1.0, y: 0.0 };
        let r = p.rotate(PointF64 { x: 0.0, y: 0.0 }, std::f64::consts::PI / 2.0);
        // should be close to PointF64 { x: 0.0, y: 1.0 }
        assert!(-0.000000001 < r.x && r.x < 0.000000001);
        assert!(1.0 - 0.000000001 < r.y && r.y < 1.0 + 0.000000001);
    }

    #[test]
    fn test_round_i32() {
        let z = PointI32 { x: 0, y: 2 };
        assert_eq!(z.to_svg_string(None), "0,2");
        assert_eq!(z.to_svg_string(Some(5)), "0,2");

        let r = PointI32 { x: 1, y: 2 };
        assert_eq!(r.to_svg_string(None), "1,2");
        assert_eq!(r.to_svg_string(Some(5)), "1,2");
    }

    #[test]
    fn test_round_f64() {
        let z = PointF64 { x: 0.0, y: 0.1 };
        assert_eq!(z.to_svg_string(Some(0)), "0,0");
        assert_eq!(z.to_svg_string(Some(1)), "0,0.1");
        assert_eq!(z.to_svg_string(Some(2)), "0,0.1");
        assert_eq!(z.to_svg_string(None), "0,0.1");

        let p = PointF64 { x: 1.21786434, y: 2.98252586 };
        assert_eq!(p.to_svg_string(Some(0)), "1,3");
        assert_eq!(p.to_svg_string(Some(1)), "1.2,3");
        assert_eq!(p.to_svg_string(Some(2)), "1.22,2.98");
        assert_eq!(p.to_svg_string(Some(3)), "1.218,2.983");
        assert_eq!(p.to_svg_string(Some(4)), "1.2179,2.9825");
        assert_eq!(p.to_svg_string(Some(5)), "1.21786,2.98253");
        assert_eq!(p.to_svg_string(Some(6)), "1.217864,2.982526");
        assert_eq!(p.to_svg_string(Some(7)), "1.2178643,2.9825259");
        assert_eq!(p.to_svg_string(None), "1.21786434,2.98252586");
    }

    #[test]
    /// rotate clockwise by 90 degrees
    fn pointi32_rotate() {
        let p = PointI32 { x: 1, y: 0 };
        let r = p.rotate_90deg(PointI32::default(), true);
        assert_eq!(PointI32::new(0, 1), r);
    }
}