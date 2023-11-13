use crate::{PointF64, PointI32, Spline};
use std::cmp::Ordering;

/// Thanks https://spencermortensen.com/articles/bezier-circle/ for the magic constants
/// P_0 = (0, a), P_1 = (b, c), P_2 = (c, b), P_3 = (a, 0)
/// We took the diff from 1.0 because we work at the corner, not the center
const ARC_A: f64 = 1.00005519 - 1.0;
const ARC_B: f64 = 1.0 - 0.55342686;
const ARC_C: f64 = 1.0 - 0.99873585;

/// note: the vector p_0 -> p_3 must be at 45 degrees
/// ap: the corner; to decide wich side to take from p_0 -> p_3
/// the center of the circular arc lies on the opposite side of ap
pub fn circular_arc(p_0: PointI32, ap: PointI32, p_3: PointI32) -> Spline {
    let dx = sign_of(ap.x, p_0.x);
    let dy = sign_of(ap.y, p_0.y);
    let dxx = sign_of(p_3.x, ap.x);
    let dyy = sign_of(p_3.y, ap.y);
    let mut p_0 = p_0.to_point_f64();
    let ap = ap.to_point_f64();
    let mut p_3 = p_3.to_point_f64();
    let r = (p_3.x - p_0.x).abs();
    let a = r * ARC_A;
    let b = r * ARC_B;
    let c = r * ARC_C;
    // 2 sides of the mirror
    match (dx, dy) {
        // <->
        (1 | -1, 0) => {
            p_0.y -= mul(dyy, a);
            p_3.x += mul(dx, a);
            let mut spline = Spline::new(p_0);
            spline.add(
                PointF64::new(ap.x - mul(dx, b), ap.y + mul(dyy, c)),
                PointF64::new(ap.x - mul(dx, c), ap.y + mul(dyy, b)),
                p_3,
            );
            spline
        }
        // /\
        // \/
        (0, 1 | -1) => {
            p_0.x -= mul(dxx, a);
            p_3.y += mul(dy, a);
            let mut spline = Spline::new(p_0);
            spline.add(
                PointF64::new(ap.x + mul(dxx, c), ap.y - mul(dy, b)),
                PointF64::new(ap.x + mul(dxx, b), ap.y - mul(dy, c)),
                p_3,
            );
            spline
        }
        _ => panic!("What is ({dx},{dy})?"),
    }
}

pub fn approximate_circle_with_spline(left_top: PointI32, diameter: i32) -> Spline {
    let r = diameter / 2;
    let mut a = left_top;
    a.x += r;
    let mut b = left_top;
    b.x += diameter;
    let mut c = b;
    c.y += r;
    let mut spline = circular_arc(a, b, c);
    b.y += diameter;
    a.y += diameter;
    spline.points.extend(&circular_arc(c, b, a).points[1..]);
    b.x = left_top.x;
    c.x = left_top.x;
    spline.points.extend(&circular_arc(a, b, c).points[1..]);
    b = left_top;
    a.y = left_top.y;
    spline.points.extend(&circular_arc(c, b, a).points[1..]);
    spline
}

#[inline]
fn sign_of<T>(a: T, b: T) -> i32
    where T: std::cmp::PartialOrd,
{
    match a.partial_cmp(&b).unwrap() {
        Ordering::Equal => 0,
        Ordering::Greater => 1,
        Ordering::Less => -1,
    }
}

#[inline]
fn mul<T>(s: i32, v: T) -> T
    where T: std::ops::Neg<Output = T> + Default,
{
    match s {
        1 => v,
        -1 => v.neg(),
        0 => Default::default(),
        _ => panic!("What s = {s}?"),
    }
}
