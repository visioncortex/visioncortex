    use std::f64::consts::PI;

    use crate::{PathF64, PathI32, PointF64, PointI32};
    use flo_curves::{Coord2, bezier, BezierCurveFactory};

    pub struct SubdivideSmooth;

    impl SubdivideSmooth {

        /// Takes a path forming a polygon, returns a vector of bool
        /// representing its corners. (angle in radians bigger than threshold)
        /// Note that the length of output is 1 less that of the original path,
        /// because the last point of the original path is always equal to the first point
        /// (closed path)
        pub fn find_corners(path: &PathI32, threshold: f64) -> Vec<bool> {
            
            let path = &path.path[0..(path.path.len()-1)];
            let len = path.len();
            if len == 0 {
                return vec![];
            }

            let mut corners: Vec<bool> = vec![false; len];
            for i in 0..len {
                let prev = if i==0 {len-1} else {i-1};
                let next = (i+1) % len;

                let v1: PointI32 = path[i]-path[prev];
                let v2: PointI32 = path[next]-path[i];

                let angle_v1: f64 = angle(&normalize(&v1));
                let angle_v2: f64 = angle(&normalize(&v2));

                let angle_diff = signed_angle_difference(&angle_v1, &angle_v2).abs();

                if angle_diff >= threshold {
                    corners[i] = true;
                }
            }
            
            corners
        }

        /// Takes a smoothed path, returns a vector of bool
        /// representing its splice points (angle displacement in radians bigger than threshold)
        /// Note that the length of output is 1 less that of the original path,
        /// because the last point of the original path is always equal to the first point
        /// (closed path)
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

                let angle_v1: f64 = angle(&normalize_f64(&v1));
                let angle_v2: f64 = angle(&normalize_f64(&v2));

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

        /// Subdivide the path iteratively
        pub fn subdivide_iterative(path: &PathI32, corners_in: &[bool], length_threshold: f64, max_iterations: usize) -> PathF64 {
            let mut path = path.to_pathf64();
            let mut corners: Vec<bool> = vec![];
            corners.extend_from_slice(corners_in);
            for _i in 0..max_iterations {
                let result = subdivide_keep_corners(&path, &corners, length_threshold);
                path = result.0;
                corners = result.1;
                if result.2 { // Can terminate early
                    break;
                }
            }
            path
        }

        pub fn fit_points_with_bezier(points: &[PointF64]) -> [PointF64; 4] {
            let mut coords:Vec<Coord2> = vec![];
        
            for i in points.iter() {
                coords.push(Coord2(i.x as f64, i.y as f64));
            }
        
            let opt = bezier::Curve::fit_from_points(&coords, 10.0);
            match opt {
                None => [PointF64::default(),PointF64::default(),PointF64::default(),PointF64::default()],
                Some(curves) => {
        
                    if curves.is_empty() {
                        return [PointF64::default(),PointF64::default(),PointF64::default(),PointF64::default()];
                    }
                    let curve = curves[0];
                    let p1 = points[0];
                    let p4 = points[points.len()-1];
        
                    let (cp1, cp2) = curve.control_points;
                    let p2 = PointF64 {x: cp1.0, y: cp1.1};
                    let p3 = PointF64 {x: cp2.0, y: cp2.1};
        
                    retract_handles(&p1, &p2, &p3, &p4)
                }
            }
        }
    }

    fn norm(p: &PointI32) -> f64 {
        ((p.x*p.x + p.y*p.y) as f64).sqrt()
    }

    fn norm_f64(p: &PointF64) -> f64 {
        (p.x*p.x + p.y*p.y).sqrt()
    }

    fn normalize(p: &PointI32) -> PointF64 {
        let norm = norm(p);
        PointF64::new(p.x as f64 / norm, p.y as f64 / norm)
    }

    fn normalize_f64(p: &PointF64) -> PointF64 {
        let norm = norm_f64(p);
        PointF64::new(p.x / norm, p.y / norm)
    }

    fn angle(p: &PointF64) -> f64 {
        let mut ag = p.x.acos();
        if p.y < 0.0 {
            ag = -ag;
        }
        ag
    }

    /// Given angles in (-pi,pi], find the signed angle difference
    /// Positive in clockwise direction, 0-degree axis is the positive x axis
    fn signed_angle_difference(from: &f64, to: &f64) -> f64 {
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

    /// Takes a path forming a polygon and a slice of bool representing corner positions.
    /// Use the 4-point (or 3-point) scheme to subdivide, but keeping all corners.
    /// Also returns a bool to indicate if all resulted segments < length_threshold
    fn subdivide_keep_corners(path: &PathF64, corners: &[bool], length_threshold: f64) -> (PathF64, Vec<bool>, bool) {

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
            let length_curr = norm_f64(&(path[i] - path[j]));
            if length_curr <= length_threshold {
                continue;
            }

            let mut prev = if i==0 {len-1} else {i-1};
            let mut next = (j+1)%len;

            // Check ratio of adjacent segments
            let length_prev = norm_f64(&(path[prev] - path[i]));
            let length_next = norm_f64(&(path[next] - path[j]));
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
                let new_point = find_new_point(&path[i], &path[j], &path[prev], &path[next]);
                new_path.push(new_point);
                new_corners.push(false); // new point will never be corner
                // If any of the new segments is still bigger than the length threshold, further iterations will be needed
                if norm_f64(&(path[i] - new_point)) > length_threshold || norm_f64(&(path[j] - new_point)) > length_threshold {
                    can_terminate_iteration = false;
                }
            }
        }

        // Close path
        new_path.push(new_path[0]);

        (PathF64 { path: new_path }, new_corners, can_terminate_iteration)
    }

    /// Finds mid-points between (p_i and p_j) and (p_1 and p_2),
    /// then returns the new point constructed by the 4-point scheme
    fn find_new_point(p_i: &PointF64, p_j: &PointF64, p_1: &PointF64, p_2: &PointF64) -> PointF64 {

        let mid_out = find_mid_point(p_i, p_j);
        let mid_in = find_mid_point(p_1, p_2);

        let vector_out: PointF64 = mid_out - mid_in;
        // Using the ratio 1:8
        let new_magnitude = norm_f64(&vector_out) / 8.0;
        if new_magnitude < 1e-5 {
            // mid_out == mid_in in this case
            return mid_out;
        }
        let unit_vector = normalize_f64(&vector_out);
        let frac_vector = PointF64 {x: unit_vector.x * new_magnitude, y: unit_vector.y * new_magnitude};

        // Point out from mid_out
        mid_out + frac_vector
    }

    fn find_mid_point(p1: &PointF64, p2: &PointF64) -> PointF64 {
        let x = (p1.x + p2.x) / 2.0;
        let y = (p1.y + p2.y) / 2.0;
        PointF64 {x, y}
    }

    fn retract_handles(a: &PointF64, b: &PointF64, c: &PointF64, d: &PointF64) -> [PointF64; 4] {
        let da: PointF64 = *a-*d;
        let ab: PointF64 = *b-*a;
        // signed angle DAB
        let dab = signed_angle_difference(&angle(&normalize_f64(&da)), &angle(&normalize_f64(&ab)));

        let bc: PointF64 = *c-*b;
        // signed angle ABC
        let abc = signed_angle_difference(&angle(&normalize_f64(&ab)), &angle(&normalize_f64(&bc)));

        // They intersect
        if dab.is_sign_positive() != abc.is_sign_positive() {
            let intersection = find_intersection(a, b, c, d);
            [*a, intersection, intersection, *d]
        } else {
            [*a, *b, *c, *d]
        }
    }

    /// Given lines p1p2 and p3p4, returns their intersection.
    /// If the two lines coincide, returns the mid-pt of p2 and p3.
    /// If the two lines are parallel, panicks.
    /// https://github.com/tyt2y3/vaser-unity/blob/master/Assets/Vaser/Vec2Ext.cs#L107 (Intersect)
    fn find_intersection(p1: &PointF64, p2: &PointF64, p3: &PointF64, p4: &PointF64) -> PointF64 {

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