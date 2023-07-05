use crate::{Path, PathF64, PointF64, Point2};
use flo_curves::{Coord2, bezier, BezierCurveFactory};

/// Handles Path Smoothing
pub(crate) struct SubdivideSmooth;

use super::util::{angle, try_find_intersection, find_mid_point, norm, normalize, signed_angle_difference};

impl SubdivideSmooth {

    /// Takes a path forming a polygon, returns a vector of bool representing its corners 
    /// (angle in radians bigger than or equal to threshold).
    /// 
    /// Note that the length of output is 1 less than that of the original path,
    /// because the last point of the original path is always equal to the first point for paths of walked polygons (closed path)
    pub fn find_corners<T>(path: &Path<Point2<T>>, threshold: f64) -> Vec<bool>
    where T: std::ops::Add<Output = T> + std::ops::Sub<Output = T> + std::ops::Mul<Output = T> + Copy + Into<f64> {

        let path = &path.path[0..(path.path.len()-1)];
        let len = path.len();
        if len == 0 {
            return vec![];
        }

        let mut corners: Vec<bool> = vec![false; len];
        for i in 0..len {
            let prev = if i==0 {len-1} else {i-1};
            let next = (i+1) % len;

            let v1: Point2<T> = path[i]-path[prev];
            let v2: Point2<T> = path[next]-path[i];

            let angle_v1: f64 = angle(&normalize(&v1));
            let angle_v2: f64 = angle(&normalize(&v2));

            let angle_diff = signed_angle_difference(&angle_v1, &angle_v2).abs();

            if angle_diff >= threshold {
                corners[i] = true;
            }
        }

        corners
    }

    /// Takes a smoothed path forming a polygon, returns a vector of bool
    /// representing its splice points (angle displacement in radians bigger than threshold).
    /// 
    /// Note that the length of output is 1 less than that of the original path,
    /// because the last point of the original path is always equal to the first point for paths of walked polygons (closed path).
    pub fn find_splice_points(path: &PathF64, threshold: f64) -> Vec<bool> {

        let path = &path.path[0..(path.path.len()-1)];
        let len = path.len();
        if len == 0 {
            return vec![];
        }

        let mut splice_points: Vec<bool> = vec![false; len];
        let mut is_angle_increasing = false;
        let mut angle_disp = 0.0;
        for i in 0..len {
            let prev = if i==0 {len-1} else {i-1};
            let next = (i+1) % len;

            let v1: PointF64 = path[i]-path[prev];
            let v2: PointF64 = path[next]-path[i];

            let angle_v1: f64 = angle(&normalize(&v1));
            let angle_v2: f64 = angle(&normalize(&v2));

            let angle_diff = signed_angle_difference(&angle_v1, &angle_v2);
            let is_currently_increasing = angle_diff.is_sign_positive();

            // Test if this point is a point of inflection
            if i==0 {
                is_angle_increasing = is_currently_increasing;
            } else if is_angle_increasing != is_currently_increasing {
                // This point is a point of inflection
                splice_points[i] = true;
                is_angle_increasing = is_currently_increasing;
            }

            // Accumulate the angle of this point to see if a turn has finished here
            angle_disp += angle_diff;
            if angle_disp.abs() >= threshold {
                splice_points[i] = true;
            }

            // If this point is a splice point, reset the displacement
            if splice_points[i] {
                angle_disp = 0.0;
            }
        }

        splice_points
    }

    /// Takes a splice of points, returns 4 control points representing the approximating Bezier curve using a curve-fitter.
    pub fn fit_points_with_bezier(points: &[PointF64]) -> [PointF64; 4] {
            
        let opt = bezier::Curve::fit_from_points(points, 10.0);
        match opt {
            None => [PointF64::default(),PointF64::default(),PointF64::default(),PointF64::default()],
            Some(curves) => {
    
                if curves.is_empty() {
                    return [PointF64::default(),PointF64::default(),PointF64::default(),PointF64::default()];
                }
                let curve = curves[0];
                let p1 = points[0];
                let p4 = points[points.len()-1];
    
                let (p2, p3) = curve.control_points;
    
                Self::retract_handles(&p1, &p2, &p3, &p4)
            }
        }
    }

    /// Takes a path forming a polygon and a slice of bool representing corner positions.
    /// 
    /// Use the 4-point scheme to subdivide while keeping corners. 
    /// `outset_ratio` determines the relative amount to expand outward. 
    /// This function will not attempt to divide segments <= `segment_length`.
    /// 
    /// Returns a smoothed path, a Vec<bool> representing updated corner positions,
    /// and `true` when no further subdivision is needed.
    pub fn subdivide_keep_corners(
        path: &PathF64, corners: &[bool], outset_ratio: f64, segment_length: f64
    ) -> (PathF64, Vec<bool>, bool) {

        let path = &path.path[0..(path.path.len()-1)];
        let len = path.len();

        let mut can_terminate_iteration = true;

        // Store new points in this new path
        let mut new_path: Vec<PointF64> = vec![];
        // Update corners
        let mut new_corners: Vec<bool> = vec![];

        for i in 0..len {
            new_path.push(PointF64 {x: path[i].x, y: path[i].y});
            if corners[i] {
                new_corners.push(true);
            } else {
                new_corners.push(false);
            }
            let j = (i+1)%len;

            // Apply threshold on length of current segment
            let length_curr = norm(&(path[i] - path[j]));
            if length_curr <= segment_length {
                continue;
            }

            let mut prev = if i==0 {len-1} else {i-1};
            let mut next = (j+1)%len;

            // Check ratio of adjacent segments
            let length_prev = norm(&(path[prev] - path[i]));
            let length_next = norm(&(path[next] - path[j]));
            if length_prev/length_curr >= 2.0 || length_next/length_curr >= 2.0 {
                continue;
            }

            // Switch to 3-point scheme to preserve corners
            if corners[i] {
                prev = i;
            }
            if corners[j] {
                next = j;
            }

            // Two corners are neighbors -> no need to smooth this segment further
            if prev==i && next==j {
                continue;
            } else {
                let new_point = Self::find_new_point_from_4_point_scheme(
                    &path[i], &path[j], &path[prev], &path[next], outset_ratio
                );
                new_path.push(new_point);
                new_corners.push(false); // new point will never be corner
                // If any of the new segments is still bigger than the length threshold, further iterations will be needed
                if norm(&(path[i] - new_point)) > segment_length || norm(&(path[j] - new_point)) > segment_length {
                    can_terminate_iteration = false;
                }
            }
        }

        // Close path
        new_path.push(new_path[0]);

        (PathF64::from_points(new_path), new_corners, can_terminate_iteration)
    }

    /// Finds mid-points between (p_i and p_j) and (p_1 and p_2), where p_i and p_j should be between p_1 and p_2,
    /// then returns the new point constructed by the 4-point scheme
    fn find_new_point_from_4_point_scheme(
        p_i: &PointF64, p_j: &PointF64, p_1: &PointF64, p_2: &PointF64, outset_ratio: f64
    ) -> PointF64 {
        let mid_out = find_mid_point(p_i, p_j);
        let mid_in = find_mid_point(p_1, p_2);

        let vector_out = mid_out - mid_in;
        let new_magnitude = vector_out.norm() / outset_ratio;
        if new_magnitude < f64::EPSILON {
            // mid_out == mid_in in this case
            return mid_out;
        }

        // Point out from mid_out
        mid_out + vector_out.get_normalized() * new_magnitude
    }

    fn retract_handles(a: &PointF64, b: &PointF64, c: &PointF64, d: &PointF64) -> [PointF64; 4] {
        let da: PointF64 = *a-*d;
        let ab: PointF64 = *b-*a;
        // signed angle DAB
        let dab = signed_angle_difference(&angle(&normalize(&da)), &angle(&normalize(&ab)));

        let bc: PointF64 = *c-*b;
        // signed angle ABC
        let abc = signed_angle_difference(&angle(&normalize(&ab)), &angle(&normalize(&bc)));

        // They intersect
        if dab.is_sign_positive() != abc.is_sign_positive() {
            if let Some(intersection) = try_find_intersection(a, b, c, d) {
                return [*a, intersection, intersection, *d];
            }
        }
        [*a, *b, *c, *d]
    }
}