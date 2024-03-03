use std::{cmp::Ordering};
use crate::{BinaryImage, PathF64, PointF64, PathSimplifyMode};
use super::{PathI32, smooth::SubdivideSmooth};

#[derive(Debug, Default, Clone)]
/// Series of connecting 2D Bezier Curves
pub struct Spline {
    /// 1+3*(num_curves) points, where the first curve is represented by the first 4 points and each subsequent curve is represented by the last point in the previous curve plus 3 points
    /// Points are of PointF64 type.
    pub points: Vec<PointF64>,
}

impl Spline {

    /// Creates an empty spline defined by a starting point
    pub fn new(point: PointF64) -> Self {
        Self {
            points: vec![point],
        }
    }

    /// Adds a curve to the end of the spline. Takes 3 points that are the second to fourth control points of the bezier curve. Note that the first control point is taken from the last point of the previous curve.
    pub fn add(&mut self, point2: PointF64, point3: PointF64, point4: PointF64) {
        self.points.push(point2);
        self.points.push(point3);
        self.points.push(point4);
    }

    /// Returns an iterator on the vector of points on the spline
    pub fn iter(&self) -> std::slice::Iter<PointF64> {
        self.points.iter()
    }

    pub fn get_control_points(&self) -> Vec<&[PointF64]> {
        self.points.iter().as_slice().windows(4).step_by(3).collect()
    }

    /// Returns the number of points on the spline
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Returns the number of curves on the spline
    pub fn num_curves(&self) -> usize {
        if !self.points.is_empty() {(self.points.len()-1)/3} else {0}
    }

    /// Returns true if the spline contains no curve, false otherwise
    /// A curve is defined by 4 points, so a non-empty spline should contain at least 4 points.
    pub fn is_empty(&self) -> bool {
        self.points.len() <= 3
    }

    /// Applies an offset to all points on the spline
    pub fn offset(&mut self, offset: &PointF64) {
        for path in self.points.iter_mut() {
            path.x += offset.x;
            path.y += offset.y;
        }
    }

    /// Returns a spline created from image.
    /// The following steps are performed:
    /// 1. Convert pixels into path
    /// 2. Simplify the path into polygon
    /// 3. Smoothen the polygon and approximate it with a curve-fitter
    /// 
    /// Corner/Splice thresholds are specified in radians.
    /// Length threshold is specified in pixels (length unit in path coordinate system).
    pub fn from_image(
        image: &BinaryImage, clockwise: bool, corner_threshold: f64, outset_ratio: f64,
        segment_length: f64, max_iterations: usize, splice_threshold: f64
    ) -> Self {
        let path = PathI32::image_to_path(image, clockwise, PathSimplifyMode::Polygon);
        let path = path.smooth(corner_threshold, outset_ratio, segment_length, max_iterations);
        Self::from_path_f64(&path, splice_threshold)
    }

    /// Returns a spline by curve-fitting a path.
    /// 
    /// Splice threshold is specified in radians.
    pub fn from_path_f64(path: &PathF64, splice_threshold: f64) -> Self {
        // First locate all the splice points
        let splice_points = SubdivideSmooth::find_splice_points(&path, splice_threshold);
        let path = &path.path[0..path.len()-1];
        let len = path.len();
        if len<=1 {
            return Self::new(PointF64 {x:0.0,y:0.0});
        }
        if len==2 {
            let mut result = Self::new(path[0]);
            result.add(path[1], path[1], path[1]);
            return result;
        }

        // This vector stores the indices
        let mut cut_points: Vec<usize> = splice_points.iter()
            .enumerate()
            .filter(|(_, &cut)| {cut})
            .map(|(i, _)| {i})
            .collect();

        if cut_points.is_empty() {
            cut_points.push(0);
        }
        if cut_points.len() == 1 {
            cut_points.push((cut_points[0]+len/2)%len);
        }
        let num_cut_points = cut_points.len();

        let mut result = Self::new(PointF64 {x:0.0,y:0.0}); // Dummy initialization
        for i in 0..num_cut_points {
            let j = (i+1)%num_cut_points;

            let current = cut_points[i];
            let next = cut_points[j];
            let subpath = Self::get_circular_subpath(path, current, next);
            let bezier_points = SubdivideSmooth::fit_points_with_bezier(&subpath);

            // Only the first curve need to add the first point
            if i==0 {
                result = Self::new(bezier_points[0]);
            }
            // Subsequent curves take their first point from previous curve's last point
            result.add(bezier_points[1], bezier_points[2], bezier_points[3]);
        }

        result
    }

    /// Converts spline to svg path. Panic if the length of spline is not valid (not 1+3n for some integer n)
    pub fn to_svg_string(&self, close: bool, offset: &PointF64, precision: Option<u32>) -> String {

        let o = offset;

        if self.is_empty() {
            return String::from("");
        }

        if (self.len() - 1) % 3 != 0 {
            panic!("Invalid spline! Length must be 1+3n.");
        }

        let points = &self.points;
        let len = points.len();
        let mut result: Vec<String> = vec![format!("M{} {} ", PointF64::number_format(points[0].x + o.x, precision), PointF64::number_format(points[0].y + o.y, precision))];

        let mut i = 1;
        while i < len {
            result.push(
                format!("C{} {} {} {} {} {} ",
                PointF64::number_format(points[i].x + o.x, precision), PointF64::number_format(points[i].y + o.y, precision),
                PointF64::number_format(points[i+1].x + o.x, precision), PointF64::number_format(points[i+1].y + o.y, precision),
                PointF64::number_format(points[i+2].x + o.x, precision), PointF64::number_format(points[i+2].y + o.y, precision))
            );
            i += 3;
        }

        if close {
            result.push(String::from("Z "));
        }

        result.concat()
    }

    fn get_circular_subpath(path: &[PointF64], from: usize, to: usize) -> Vec<PointF64> {

        let len = path.len();
        let mut subpath: Vec<PointF64> = vec![];
    
        match from.cmp(&to) {
            Ordering::Less => {
                subpath.extend_from_slice(&path[from..=to]);
            },
            Ordering::Greater => {
                subpath.extend_from_slice(&path[from..len]);
                subpath.extend_from_slice(&path[0..=to]);
            },
            Ordering:: Equal => {}
        }
        
        subpath
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spline_to_svg() {
        let spline = Spline {
            points: vec![
                PointF64 { x: 2.22, y: 2.67 },
                PointF64 { x: 3.50, y: 3.48 },
                PointF64 { x: 4.19, y: 4.72 },
                PointF64 { x: 5.68, y: 5.26 },
            ]
        };
        assert_eq!(
            spline.to_svg_string(false, &PointF64 { x: 0.0, y: 0.0 }, None),
            "M2.22 2.67 C3.5 3.48 4.19 4.72 5.68 5.26 ".to_owned()
        );
        assert_eq!(
            spline.to_svg_string(false, &PointF64 { x: 0.0, y: 0.0 }, Some(1)),
            "M2.2 2.7 C3.5 3.5 4.2 4.7 5.7 5.3 ".to_owned()
        );
        assert_eq!(
            spline.to_svg_string(false, &PointF64 { x: 0.0, y: 0.0 }, Some(0)),
            "M2 3 C4 3 4 5 6 5 ".to_owned()
        );
    }
}