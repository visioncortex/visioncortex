//! Path simplification algorithms adapted from https://github.com/mourner/simplify-js

use std::ops::*;
use crate::Point2;

type Float = f64;

/// square distance between 2 points
fn get_sq_dist<T>(p1: Point2<T>, p2: Point2<T>) -> Float
where T: Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Copy + Into<Float> {

    let dx = p1.x - p2.x;
    let dy = p1.y - p2.y;

    (dx * dx + dy * dy).into()
}

/// square distance from a point to a segment
fn get_sq_seg_dist<T>(p: Point2<T>, p1: Point2<T>, p2: Point2<T>) -> Float
where T: Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Copy + Into<Float> {

    let mut x = p1.x.into();
    let mut y = p1.y.into();
    let mut dx = p2.x.into() - x;
    let mut dy = p2.y.into() - y;

    if dx != 0.0 || dy != 0.0 {

        let t = ((p.x.into() - x) * dx + (p.y.into() - y) * dy) / (dx * dx + dy * dy);

        if t > 1.0 {
            x = p2.x.into();
            y = p2.y.into();
        } else if t > 0.0 {
            x += dx * t;
            y += dy * t;
        }
    }

    dx = p.x.into() - x;
    dy = p.y.into() - y;

    dx * dx + dy * dy
}

// rest of the code doesn't care about point format

/// basic distance-based simplification
pub fn simplify_radial_dist<T>(points: &[Point2<T>], sq_tolerance: Float) -> Vec<Point2<T>>
where T: Add<Output = T> + Sub<Output = T> + Mul<Output = T> + std::cmp::PartialEq + Copy + Into<Float> {

    if points.len() <= 2 { return points.to_vec(); }

    let mut prev_point = points[0];
    let mut new_points = vec![prev_point];

    for point in points.iter().skip(1) {
        if get_sq_dist(*point, prev_point) > sq_tolerance {
            new_points.push(*point);
            prev_point = *point;
        }
    }

    if prev_point != *points.last().unwrap() { new_points.push(*points.last().unwrap()); }

    new_points
}

fn simplify_dp_step<T>(points: &[Point2<T>], first: usize, last: usize, sq_tolerance: Float, simplified: &mut Vec<Point2<T>>)
where T: Add<Output = T> + Sub<Output = T> + Mul<Output = T> + std::cmp::PartialEq + Copy + Into<Float> {
    let mut max_sq_dist = sq_tolerance;
    let mut index = 0;

    for i in first+1..last {
        let sq_dist = get_sq_seg_dist(points[i], points[first], points[last]);

        if sq_dist > max_sq_dist {
            index = i;
            max_sq_dist = sq_dist;
        }
    }

    if max_sq_dist > sq_tolerance {
        if index - first > 1 { simplify_dp_step(points, first, index, sq_tolerance, simplified); }
        simplified.push(points[index]);
        if last - index > 1 { simplify_dp_step(points, index, last, sq_tolerance, simplified); }
    }
}

/// simplification using Ramer-Douglas-Peucker algorithm
pub fn simplify_douglas_peucker<T>(points: &[Point2<T>], sq_tolerance: Float) -> Vec<Point2<T>>
where T: Add<Output = T> + Sub<Output = T> + Mul<Output = T> + std::cmp::PartialEq + Copy + Into<Float> {
    let last = points.len() - 1;

    let mut simplified = vec![points[0]];
    simplify_dp_step(points, 0, last, sq_tolerance, &mut simplified);
    simplified.push(points[last]);

    simplified
}

/// both algorithms combined for awesome performance
///
/// this is the original implementation from mourner/simplify-js
#[allow(dead_code)]
fn simplify<T>(original: &[Point2<T>], tolerance: Float, highest_quality: bool) -> Vec<Point2<T>>
where T: Add<Output = T> + Sub<Output = T> + Mul<Output = T> + std::cmp::PartialEq + Copy + Into<Float> {

    if original.len() <= 2 { return original.to_vec(); }

    let sq_tolerance = tolerance * tolerance;

    let radial;
    let points = if highest_quality {
        original
    } else {
        radial = simplify_radial_dist(original, sq_tolerance);
        &radial
    };

    simplify_douglas_peucker(points, sq_tolerance)
}

/// Reduce points from a path
///
/// The larger the tolerance, the fewer points will be left in output path
pub fn reduce<T>(original: &[Point2<T>], tolerance: Float) -> Vec<Point2<T>>
where T: Add<Output = T> + Sub<Output = T> + Mul<Output = T> + std::cmp::PartialEq + Copy + Into<Float> {

    if original.len() <= 2 { return original.to_vec(); }
    if tolerance == 0.0 { return original.to_vec(); }

    let sq_tolerance = tolerance * tolerance;

    let radial = simplify_radial_dist(original, sq_tolerance * 0.5);

    simplify_douglas_peucker(&radial, sq_tolerance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PointI32, PointF64};

    #[test]
    fn simplify_i32() {

        let points = vec![
            PointI32 { x: 22455, y: 25015 }, PointI32 { x: 22691, y: 24419 }, PointI32 { x: 23331, y: 24145 }, PointI32 {x: 23498, y: 23606 },
            PointI32 { x: 24421, y: 23276 }, PointI32 { x: 26259, y: 21531 }, PointI32 { x: 26776, y: 21381 }, PointI32 {x: 27357, y: 20184 },
            PointI32 { x: 27312, y: 19216 }, PointI32 { x: 27762, y: 18903 }, PointI32 { x: 28036, y: 18141 }, PointI32 {x: 28651, y: 17774 },
            PointI32 { x: 29241, y: 15937 }, PointI32 { x: 29691, y: 15564 }, PointI32 { x: 31495, y: 15137 }, PointI32 {x: 31975, y: 14516 },
            PointI32 { x: 33033, y: 13757 }, PointI32 { x: 34148, y: 13996 }, PointI32 { x: 36998, y: 13789 }, PointI32 {x: 38739, y: 14251 },
            PointI32 { x: 39128, y: 13939 }, PointI32 { x: 40952, y: 14114 }, PointI32 { x: 41482, y: 13975 }, PointI32 {x: 42772, y: 12730 },
            PointI32 { x: 43960, y: 11974 }, PointI32 { x: 47493, y: 10787 }, PointI32 { x: 48651, y: 10675 }, PointI32 {x: 48920, y: 10945 },
            PointI32 { x: 49379, y: 10863 }, PointI32 { x: 50474, y: 11966 }, PointI32 { x: 51296, y: 12235 }, PointI32 {x: 51863, y: 12089 },
            PointI32 { x: 52409, y: 12688 }, PointI32 { x: 52957, y: 12786 }, PointI32 { x: 53421, y: 14093 }, PointI32 {x: 53927, y: 14724 },
            PointI32 { x: 56769, y: 14891 }, PointI32 { x: 57525, y: 15726 }, PointI32 { x: 58062, y: 15815 }, PointI32 {x: 60153, y: 15685 },
            PointI32 { x: 61774, y: 15986 }, PointI32 { x: 62200, y: 16704 }, PointI32 { x: 62955, y: 19460 }, PointI32 {x: 63890, y: 19561 },
            PointI32 { x: 64126, y: 20081 }, PointI32 { x: 65177, y: 20456 }, PointI32 { x: 67155, y: 22255 }, PointI32 {x: 68368, y: 21745 },
            PointI32 { x: 69525, y: 21915 }, PointI32 { x: 70064, y: 21798 }, PointI32 { x: 70312, y: 21436 }, PointI32 {x: 71226, y: 21587 },
            PointI32 { x: 72149, y: 21281 }, PointI32 { x: 72781, y: 21336 }, PointI32 { x: 72998, y: 20873 }, PointI32 {x: 73532, y: 20820 },
            PointI32 { x: 73994, y: 20477 }, PointI32 { x: 76998, y: 20842 }, PointI32 { x: 77960, y: 21687 }, PointI32 {x: 78420, y: 21816 },
            PointI32 { x: 80024, y: 21462 }, PointI32 { x: 81053, y: 21973 }, PointI32 { x: 81719, y: 22682 }, PointI32 {x: 82077, y: 23617 },
            PointI32 { x: 82723, y: 23616 }, PointI32 { x: 82989, y: 23989 }, PointI32 { x: 85100, y: 24894 }, PointI32 {x: 85988, y: 25549 },
            PointI32 { x: 86521, y: 26853 }, PointI32 { x: 85795, y: 28030 }, PointI32 { x: 86548, y: 29145 }, PointI32 {x: 86681, y: 29866 },
            PointI32 { x: 86468, y: 30271 }, PointI32 { x: 86779, y: 30617 }, PointI32 { x: 85987, y: 31137 }, PointI32 {x: 86008, y: 31435 },
            PointI32 { x: 85829, y: 31494 }, PointI32 { x: 85810, y: 32760 }, PointI32 { x: 85454, y: 33540 }, PointI32 {x: 86092, y: 34300 },
            PointI32 { x: 85643, y: 35015 }, PointI32 { x: 85142, y: 35296 }, PointI32 { x: 84984, y: 35959 }, PointI32 {x: 85456, y: 36553 },
            PointI32 { x: 84974, y: 37038 }, PointI32 { x: 84409, y: 37189 }, PointI32 { x: 84475, y: 38044 }, PointI32 {x: 84152, y: 38367 },
            PointI32 { x: 83957, y: 39040 }, PointI32 { x: 84559, y: 39905 }, PointI32 { x: 84840, y: 40755 }, PointI32 {x: 84371, y: 41130 },
            PointI32 { x: 84409, y: 41988 }, PointI32 { x: 83951, y: 43276 }, PointI32 { x: 84133, y: 44104 }, PointI32 {x: 84762, y: 44922 },
            PointI32 { x: 84716, y: 45844 }, PointI32 { x: 85138, y: 46279 }, PointI32 { x: 85397, y: 47115 }, PointI32 {x: 86636, y: 48077 }
        ];

        let simplified = vec![
            PointI32 { x: 22455, y: 25015 }, PointI32 { x: 26776, y: 21381 }, PointI32 { x: 29691, y: 15564 }, PointI32 { x: 33033, y: 13757 },
            PointI32 { x: 40952, y: 14114 }, PointI32 { x: 43960, y: 11974 }, PointI32 { x: 48651, y: 10675 }, PointI32 { x: 52957, y: 12786 },
            PointI32 { x: 53927, y: 14724 }, PointI32 { x: 61774, y: 15986 }, PointI32 { x: 62955, y: 19460 }, PointI32 { x: 67155, y: 22255 },
            PointI32 { x: 72781, y: 21336 }, PointI32 { x: 73994, y: 20477 }, PointI32 { x: 76998, y: 20842 }, PointI32 { x: 77960, y: 21687 },
            PointI32 { x: 80024, y: 21462 }, PointI32 { x: 82077, y: 23617 }, PointI32 { x: 85988, y: 25549 }, PointI32 { x: 86521, y: 26853 },
            PointI32 { x: 85795, y: 28030 }, PointI32 { x: 86779, y: 30617 }, PointI32 { x: 85987, y: 31137 }, PointI32 { x: 85454, y: 33540 },
            PointI32 { x: 86092, y: 34300 }, PointI32 { x: 84984, y: 35959 }, PointI32 { x: 85456, y: 36553 }, PointI32 { x: 84409, y: 37189 },
            PointI32 { x: 83957, y: 39040 }, PointI32 { x: 84840, y: 40755 }, PointI32 { x: 83951, y: 43276 }, PointI32 { x: 85397, y: 47115 },
            PointI32 { x: 86636, y: 48077 }
        ];

        assert_eq!(simplify(&points, 500.0, false), simplified);
    }

    #[test]
    fn simplify_f64() {

        let points = vec![
            PointF64 { x: 224.55, y: 250.15 }, PointF64 { x: 226.91, y: 244.19 }, PointF64 { x: 233.31, y: 241.45 }, PointF64 {x: 234.98, y: 236.06 },
            PointF64 { x: 244.21, y: 232.76 }, PointF64 { x: 262.59, y: 215.31 }, PointF64 { x: 267.76, y: 213.81 }, PointF64 {x: 273.57, y: 201.84 },
            PointF64 { x: 273.12, y: 192.16 }, PointF64 { x: 277.62, y: 189.03 }, PointF64 { x: 280.36, y: 181.41 }, PointF64 {x: 286.51, y: 177.74 },
            PointF64 { x: 292.41, y: 159.37 }, PointF64 { x: 296.91, y: 155.64 }, PointF64 { x: 314.95, y: 151.37 }, PointF64 {x: 319.75, y: 145.16 },
            PointF64 { x: 330.33, y: 137.57 }, PointF64 { x: 341.48, y: 139.96 }, PointF64 { x: 369.98, y: 137.89 }, PointF64 {x: 387.39, y: 142.51 },
            PointF64 { x: 391.28, y: 139.39 }, PointF64 { x: 409.52, y: 141.14 }, PointF64 { x: 414.82, y: 139.75 }, PointF64 {x: 427.72, y: 127.30 },
            PointF64 { x: 439.60, y: 119.74 }, PointF64 { x: 474.93, y: 107.87 }, PointF64 { x: 486.51, y: 106.75 }, PointF64 {x: 489.20, y: 109.45 },
            PointF64 { x: 493.79, y: 108.63 }, PointF64 { x: 504.74, y: 119.66 }, PointF64 { x: 512.96, y: 122.35 }, PointF64 {x: 518.63, y: 120.89 },
            PointF64 { x: 524.09, y: 126.88 }, PointF64 { x: 529.57, y: 127.86 }, PointF64 { x: 534.21, y: 140.93 }, PointF64 {x: 539.27, y: 147.24 },
            PointF64 { x: 567.69, y: 148.91 }, PointF64 { x: 575.25, y: 157.26 }, PointF64 { x: 580.62, y: 158.15 }, PointF64 {x: 601.53, y: 156.85 },
            PointF64 { x: 617.74, y: 159.86 }, PointF64 { x: 622.00, y: 167.04 }, PointF64 { x: 629.55, y: 194.60 }, PointF64 {x: 638.90, y: 195.61 },
            PointF64 { x: 641.26, y: 200.81 }, PointF64 { x: 651.77, y: 204.56 }, PointF64 { x: 671.55, y: 222.55 }, PointF64 {x: 683.68, y: 217.45 },
            PointF64 { x: 695.25, y: 219.15 }, PointF64 { x: 700.64, y: 217.98 }, PointF64 { x: 703.12, y: 214.36 }, PointF64 {x: 712.26, y: 215.87 },
            PointF64 { x: 721.49, y: 212.81 }, PointF64 { x: 727.81, y: 213.36 }, PointF64 { x: 729.98, y: 208.73 }, PointF64 {x: 735.32, y: 208.20 },
            PointF64 { x: 739.94, y: 204.77 }, PointF64 { x: 769.98, y: 208.42 }, PointF64 { x: 779.60, y: 216.87 }, PointF64 {x: 784.20, y: 218.16 },
            PointF64 { x: 800.24, y: 214.62 }, PointF64 { x: 810.53, y: 219.73 }, PointF64 { x: 817.19, y: 226.82 }, PointF64 {x: 820.77, y: 236.17 },
            PointF64 { x: 827.23, y: 236.16 }, PointF64 { x: 829.89, y: 239.89 }, PointF64 { x: 851.00, y: 248.94 }, PointF64 {x: 859.88, y: 255.49 },
            PointF64 { x: 865.21, y: 268.53 }, PointF64 { x: 857.95, y: 280.30 }, PointF64 { x: 865.48, y: 291.45 }, PointF64 {x: 866.81, y: 298.66 },
            PointF64 { x: 864.68, y: 302.71 }, PointF64 { x: 867.79, y: 306.17 }, PointF64 { x: 859.87, y: 311.37 }, PointF64 {x: 860.08, y: 314.35 },
            PointF64 { x: 858.29, y: 314.94 }, PointF64 { x: 858.10, y: 327.60 }, PointF64 { x: 854.54, y: 335.40 }, PointF64 {x: 860.92, y: 343.00 },
            PointF64 { x: 856.43, y: 350.15 }, PointF64 { x: 851.42, y: 352.96 }, PointF64 { x: 849.84, y: 359.59 }, PointF64 {x: 854.56, y: 365.53 },
            PointF64 { x: 849.74, y: 370.38 }, PointF64 { x: 844.09, y: 371.89 }, PointF64 { x: 844.75, y: 380.44 }, PointF64 {x: 841.52, y: 383.67 },
            PointF64 { x: 839.57, y: 390.40 }, PointF64 { x: 845.59, y: 399.05 }, PointF64 { x: 848.40, y: 407.55 }, PointF64 {x: 843.71, y: 411.30 },
            PointF64 { x: 844.09, y: 419.88 }, PointF64 { x: 839.51, y: 432.76 }, PointF64 { x: 841.33, y: 441.04 }, PointF64 {x: 847.62, y: 449.22 },
            PointF64 { x: 847.16, y: 458.44 }, PointF64 { x: 851.38, y: 462.79 }, PointF64 { x: 853.97, y: 471.15 }, PointF64 {x: 866.36, y: 480.77 }
        ];

        let simplified = vec![
            PointF64 { x: 224.55, y: 250.15 }, PointF64 { x: 267.76, y: 213.81 }, PointF64 { x: 296.91, y: 155.64 }, PointF64 { x: 330.33, y: 137.57 },
            PointF64 { x: 409.52, y: 141.14 }, PointF64 { x: 439.60, y: 119.74 }, PointF64 { x: 486.51, y: 106.75 }, PointF64 { x: 529.57, y: 127.86 },
            PointF64 { x: 539.27, y: 147.24 }, PointF64 { x: 617.74, y: 159.86 }, PointF64 { x: 629.55, y: 194.60 }, PointF64 { x: 671.55, y: 222.55 },
            PointF64 { x: 727.81, y: 213.36 }, PointF64 { x: 739.94, y: 204.77 }, PointF64 { x: 769.98, y: 208.42 }, PointF64 { x: 779.60, y: 216.87 },
            PointF64 { x: 800.24, y: 214.62 }, PointF64 { x: 820.77, y: 236.17 }, PointF64 { x: 859.88, y: 255.49 }, PointF64 { x: 865.21, y: 268.53 },
            PointF64 { x: 857.95, y: 280.30 }, PointF64 { x: 867.79, y: 306.17 }, PointF64 { x: 859.87, y: 311.37 }, PointF64 { x: 854.54, y: 335.40 },
            PointF64 { x: 860.92, y: 343.00 }, PointF64 { x: 849.84, y: 359.59 }, PointF64 { x: 854.56, y: 365.53 }, PointF64 { x: 844.09, y: 371.89 },
            PointF64 { x: 839.57, y: 390.40 }, PointF64 { x: 848.40, y: 407.55 }, PointF64 { x: 839.51, y: 432.76 }, PointF64 { x: 853.97, y: 471.15 },
            PointF64 { x: 866.36, y: 480.77 }
        ];

        assert_eq!(simplify(&points, 5.0, false), simplified);
    }

    #[test]
    fn simplify_one() {
        let points = vec![ PointI32 {x: 22455, y: 25015 } ];
        assert_eq!(simplify(&points, 5.0, false), points);
    }

    #[test]
    fn simplify_zero() {
        let points = Vec::<PointI32>::new();
        assert_eq!(simplify(&points, 5.0, false), points);
    }
}