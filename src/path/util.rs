use std::f64::{NAN, consts::{PI}};

use crate::{Point2, PointF64, PointI32};

/// assume origin is top left corner, signed_area > 0 imply clockwise
pub(super) fn signed_area(p1: PointI32, p2: PointI32, p3: PointI32) -> i32 {
    (p2.x - p1.x) * (p3.y - p1.y) - (p3.x - p1.x) * (p2.y - p1.y)
}

#[derive(Debug, PartialEq)]
pub struct Intersection {
    /// The relative location between (p1, p2). 0 means p1, 1 means p2.
    pub mua: f64,
    /// The relative location between (p3, p4). 0 means p3, 1 means p4.
    pub mub: f64,
}

/// Given lines (p1, p2) and (p3, p4), returns their intersection.
/// If the two lines coincide, returns the mid-point of (p1, p2).
/// If the two lines are parallel, return None.
///
/// Adapted from https://github.com/tyt2y3/vaserenderer/blob/master/csharp/Assets/Vaser/Vec2Ext.cs#L107
///
/// Which in turn originates from http://paulbourke.net/geometry/lineline2d/
pub fn find_intersection(p1: &PointF64, p2: &PointF64, p3: &PointF64, p4: &PointF64)
    -> Option<(PointF64, Intersection)>
{
    let (denom, numera, numerb);
    denom  = (p4.y-p3.y) * (p2.x-p1.x) - (p4.x-p3.x) * (p2.y-p1.y);
    numera = (p4.x-p3.x) * (p1.y-p3.y) - (p4.y-p3.y) * (p1.x-p3.x);
    numerb = (p2.x-p1.x) * (p1.y-p3.y) - (p2.y-p1.y) * (p1.x-p3.x);

    if negligible(denom) && negligible(numera) && negligible(numerb) {
        // the two lines coincide
        return Some((find_mid_point(p1, p2), Intersection { mua: NAN, mub: NAN }));
    }

    if negligible(denom) {
        // the two line are parallel
        return None;
    }

    let mua = numera / denom;
    let mub = numerb / denom;

    Some((
        PointF64 {
            x: p1.x + mua * (p2.x - p1.x),
            y: p1.y + mua * (p2.y - p1.y),
        },
        Intersection { mua, mub }
    ))
}

impl Intersection {
    /// if the intersection lie outside of either path
    pub fn outside(&self) -> bool {
        const E: f64 = 1e-3;
        let s = self;
        (s.mua < 0.-E || s.mua > 1.+E) || (s.mub < 0.-E || s.mub > 1.+E)
    }

    /// if the intersection lie inside of both paths
    #[inline]
    pub fn inside(&self) -> bool {
        !self.outside()
    }

    pub fn coincide(&self) -> bool {
        self.mua.is_nan() && self.mub.is_nan()
    }
}

#[inline]
fn negligible(v: f64) -> bool {
    const EPSILON: f64 = 1e-7;    
    -EPSILON < v && v < EPSILON
}

pub(super) fn find_mid_point(p1: &PointF64, p2: &PointF64) -> PointF64 {
    let x = (p1.x + p2.x) / 2.0;
    let y = (p1.y + p2.y) / 2.0;
    PointF64 {x, y}
}

pub(super) fn norm<T>(p: &Point2<T>) -> f64
where T: std::ops::Add<Output = T> + std::ops::Mul<Output = T> + Copy + Into<f64> {
    let n: f64 = (p.x*p.x + p.y*p.y).into();
    n.sqrt()
}

pub(super) fn normalize<T>(p: &Point2<T>) -> PointF64
where T: std::ops::Add<Output = T> + std::ops::Mul<Output = T> + Copy + Into<f64> {
    let norm = norm(p);
    let (px, py): (f64, f64) = (p.x.into(), p.y.into());
    PointF64::new(px / norm, py / norm)
}

pub(super) fn angle(p: &PointF64) -> f64 {
    if p.y.is_sign_negative() {
        -p.x.acos()
    } else {
        p.x.acos()
    }
}

/// Given angles in (-pi,pi], find the signed angle difference
/// Positive in clockwise direction, 0-degree axis is the positive x axis
pub(super) fn signed_angle_difference(from: &f64, to: &f64) -> f64 {
    let v1 = *from;
    let mut v2 = *to;
    if v1 > v2 {
        v2 += 2.0 * PI;
    }

    let diff = v2 - v1;
    if diff > PI {
        diff - 2.0 * PI
    } else {
        diff
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_find_intersection_1() {
        assert_eq!(find_intersection(
            &PointF64::new(0.,0.), &PointF64::new(2.,0.),
            &PointF64::new(1.,-1.), &PointF64::new(1.,1.),
        ), Some((PointF64::new(1.,0.), Intersection { mua: 0.5, mub: 0.5 })));
    }

    #[test]
    fn test_find_intersection_2() {
        assert_eq!(find_intersection(
            &PointF64::new(0.,0.), &PointF64::new(2.,2.),
            &PointF64::new(0.,2.), &PointF64::new(2.,0.),
        ), Some((PointF64::new(1.,1.), Intersection { mua: 0.5, mub: 0.5 })));
    }

    #[test]
    fn test_find_intersection_3() {
        assert_eq!(find_intersection(
            &PointF64::new(0.,0.), &PointF64::new(1.,0.),
            &PointF64::new(1.,0.), &PointF64::new(1.,1.),
        ), Some((PointF64::new(1.,0.), Intersection { mua: 1., mub: 0. })));
    }

    #[test]
    fn test_find_intersection_4() {
        assert_eq!(find_intersection(
            &PointF64::new(0.,0.), &PointF64::new(2.,0.),
            &PointF64::new(1.,0.), &PointF64::new(1.,1.),
        ), Some((PointF64::new(1.,0.), Intersection { mua: 0.5, mub: 0. })));
    }
}