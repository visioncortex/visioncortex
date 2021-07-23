use num_traits::Float;
use std::{fmt::Display, ops::*};

/// Generic point in 2D space
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point2<T> {
    pub x: T,
    pub y: T,
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
    pub fn translate(self, vector: Self) -> Self {
        self + vector
    }

    #[inline]
    pub fn norm(self) -> T {
        self.dot(self).sqrt()
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

/// 2D Point with `u8` component
pub type PointU8 = Point2<u8>;
/// 2D Point with `i32` component
pub type PointI32 = Point2<i32>;
/// 2D Point with `f32` component
pub type PointF32 = Point2<f32>;
/// 2D Point with `f64` component
pub type PointF64 = Point2<f64>;

impl PointI32 {
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
}