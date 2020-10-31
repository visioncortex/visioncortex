use std::f64::consts::PI;

use crate::{PointF64, PointI32};

/// assume origin is top left corner, signed_area > 0 imply clockwise
pub(crate) fn signed_area(p1: PointI32, p2: PointI32, p3: PointI32) -> i32 {
    (p2.x - p1.x) * (p3.y - p1.y) - (p3.x - p1.x) * (p2.y - p1.y)
}

/// Given lines p1p2 and p3p4, returns their intersection.
/// If the two lines coincide, returns the mid-pt of p2 and p3.
/// If the two lines are parallel, panicks.
/// https://github.com/tyt2y3/vaser-unity/blob/master/Assets/Vaser/Vec2Ext.cs#L107 (Intersect)
pub(crate) fn find_intersection(p1: &PointF64, p2: &PointF64, p3: &PointF64, p4: &PointF64) -> PointF64 {

    const EPSILON: f64 = 1e-7;
    
    let (denom, numera, numerb);
    denom  = (p4.y-p3.y) * (p2.x-p1.x) - (p4.x-p3.x) * (p2.y-p1.y);
    numera = (p4.x-p3.x) * (p1.y-p3.y) - (p4.y-p3.y) * (p1.x-p3.x);
    numerb = (p2.x-p1.x) * (p1.y-p3.y) - (p2.y-p1.y) * (p1.x-p3.x);

    if denom <= EPSILON && numera <= EPSILON && numerb <= EPSILON {
        // The two lines coincide
        return find_mid_point(p2, p3);
    }

    if denom <= EPSILON {
        panic!("The two lines are parallel!");
    }

    let mua = numera/denom;

    PointF64 {x: p1.x + mua * (p2.x-p1.x), y: p1.y + mua * (p2.y-p1.y)}
}

pub(crate) fn find_mid_point(p1: &PointF64, p2: &PointF64) -> PointF64 {
    let x = (p1.x + p2.x) / 2.0;
    let y = (p1.y + p2.y) / 2.0;
    PointF64 {x, y}
}

pub(crate) fn norm(p: &PointI32) -> f64 {
    ((p.x*p.x + p.y*p.y) as f64).sqrt()
}

pub(crate) fn norm_f64(p: &PointF64) -> f64 {
    (p.x*p.x + p.y*p.y).sqrt()
}

pub(crate) fn normalize(p: &PointI32) -> PointF64 {
    let norm = norm(p);
    PointF64::new(p.x as f64 / norm, p.y as f64 / norm)
}

pub(crate) fn normalize_f64(p: &PointF64) -> PointF64 {
    let norm = norm_f64(p);
    PointF64::new(p.x / norm, p.y / norm)
}

pub(crate) fn angle(p: &PointF64) -> f64 {
    let mut ag = p.x.acos();
    if p.y < 0.0 {
        ag = -ag;
    }
    ag
}

/// Given angles in (-pi,pi], find the signed angle difference
/// Positive in clockwise direction, 0-degree axis is the positive x axis
pub(crate) fn signed_angle_difference(from: &f64, to: &f64) -> f64 {
    let v1 = *from;
    let mut v2 = *to;
    if v1>v2 {
        v2 += 2.0*PI;
    }

    let diff = v2-v1;
    if diff > PI {
        diff-2.0*PI
    } else {
        diff
    }
}