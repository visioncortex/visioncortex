use std::fmt::{Debug, Write};
use std::ops::{Add, AddAssign, Index, IndexMut, Mul, Range, RangeFrom, RangeInclusive, Sub};

use crate::{BinaryImage, Point2, PointF64, PointI32, Shape, ToSvgString};
use super::{PathSimplify, PathSimplifyMode, PathWalker, smooth::SubdivideSmooth, reduce::reduce};

#[derive(Clone, Debug, Default)]
/// Path of generic points in 2D space
pub struct Path<T> {
    /// T can be PointI32/PointF64, etc. (see src/point.rs).
    pub path: Vec<T>,
}

/// Path of 2D PointI32
pub type PathI32 = Path<PointI32>;
/// Path of 2D PointF64
pub type PathF64 = Path<PointF64>;

impl<T> Path<T>
{
    /// Creates a new 2D Path with no points
    pub fn new() -> Self {
        Self {
            path: vec![]
        }
    }

    /// Creates a 2D Path with 'points' as its points
    pub fn from_points(points: Vec<T>) -> Self {
        Self {
            path: points
        }
    }

    /// Adds a point to the end of the path
    pub fn add(&mut self, point: T) {
        self.path.push(point);
    }

    /// Removes the last point from the path and returns it, or None if it is empty.
    pub fn pop(&mut self) -> Option<T> {
        self.path.pop()
    }

    /// Returns an iterator on the vector of points in the path
    pub fn iter(&self) -> std::slice::Iter<T> {
        self.path.iter()
    }

    /// Returns the number of points in the path
    pub fn len(&self) -> usize {
        self.path.len()
    }

    /// Returns true if the path is empty, false otherwise
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T> Index<usize> for Path<T>
{
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.path[index]
    }
}

impl<T> IndexMut<usize> for Path<T>
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.path[index]
    }
}

impl<T> Index<Range<usize>> for Path<T>
{
    type Output = [T];

    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self.path[index]
    }
}

impl<T> Index<RangeInclusive<usize>> for Path<T>
{
    type Output = [T];

    fn index(&self, index: RangeInclusive<usize>) -> &Self::Output {
        &self.path[index]
    }
}

impl<T> Index<RangeFrom<usize>> for Path<T>
{
    type Output = [T];

    fn index(&self, index: RangeFrom<usize>) -> &Self::Output {
        &self.path[index]
    }
}

impl<T> Path<T>
where
    T: Clone + PartialEq
{
    /// Convert a closed path to an open path.
    /// A clone of 'self' is returned untouched if 'self' is empty or open.
    pub fn to_open(&self) -> Self {
        if self.is_empty() {
            return self.clone();
        }
        
        let len = self.len();
        if self.path[0] != self.path[len-1] {
            self.clone()
        } else {
            Self::from_points(self.path[0..(len-1)].to_vec())
        }
    }

    /// Convert an unclosed path to a closed path.
    /// A clone of 'self' is returned untouched if 'self' is empty or closed.
    pub fn to_closed(&self) -> Self {
        if self.is_empty() {
            return self.clone();
        }

        let len = self.len();
        if self.path[0] == self.path[len-1] {
            self.clone()
        } else {
            let mut points = self.path.clone();
            points.push(self.path[0].clone());
            Self::from_points(points)
        }
    }
}

impl<T> Path<T>
where
    T: AddAssign + Copy
{
    /// Applies an offset to all points in the path
    pub fn offset(&mut self, o: &T) {
        for point in self.path.iter_mut() {
            point.add_assign(*o);
        }
    }
}

impl<T> Path<T>
where
    T: ToSvgString + Copy + Add<Output = T>
{
    /// Generates a string representation of the path in SVG format.
    /// 
    /// Takes a bool to indicate whether the end should be wrapped back to start.
    /// 
    /// An offset is specified to apply an offset to the display points (useful when displaying on canvas elements).
    /// 
    /// If `close` is true, assume the last point of the path repeats the first point
    pub fn to_svg_string(&self, close: bool, offset: &T, precision: Option<u32>) -> String {
        let o = *offset;
        let mut string = String::new();

        self.path
            .iter()
            .take(1)
            .for_each(|p| write!(&mut string, "M{} ", (*p+o).to_svg_string(precision)).unwrap());

        self.path
            .iter()
            .skip(1)
            .take(self.path.len() - if close { 2 } else { 1 })
            .for_each(|p| write!(&mut string, "L{} ", (*p+o).to_svg_string(precision)).unwrap());

        if close {
            write!(&mut string, "Z ").unwrap();
        }

        string
    }
}

impl<T> Path<Point2<T>>
where T: Add<Output = T> + Sub<Output = T> + Mul<Output = T> +
    std::cmp::PartialEq + std::cmp::PartialOrd + Copy + Into<f64> {

    /// Path is a closed path (shape), but the reduce algorithm only reduces open paths.
    /// We divide the path into four sections, spliced at the extreme points (max-x max-y min-x min-y),
    /// and reduce each section individually.
    /// Thus the most simplified path consists of at least 4 points.
    /// This function assumes the last point of the path repeats the first point.
    pub fn reduce(&self, tolerance: f64) -> Option<Self> {
        if !self.path.is_empty() {
            assert!(self.path[0] == self.path[self.path.len() - 1]);
        }
        let mut corners = [(0, self.path[0]); 4];
        for (i, p) in self.path.iter().enumerate() {
            if i == self.path.len() - 1 {
                break;
            }
            if p.x < corners[0].1.x { corners[0] = (i, *p); }
            if p.y <= corners[1].1.y { corners[1] = (i, *p); }
            if p.x >= corners[2].1.x { corners[2] = (i, *p); }
            if p.y >= corners[3].1.y { corners[3] = (i, *p); }
        }
        let abs = |i: T| -> f64 { let i: f64 = i.into(); if i < 0.0 { -i } else { i } };
        if  abs(corners[0].1.x - corners[2].1.x) < tolerance &&
            abs(corners[1].1.y - corners[3].1.y) < tolerance {
            return None;
        }
        corners.sort_by_key(|c| c.0);
        let mut sections = [
            &self.path[corners[0].0..=corners[1].0],
            &self.path[corners[1].0..=corners[2].0],
            &self.path[corners[2].0..=corners[3].0],
            &[],
        ];
        let mut last = self.path[corners[3].0..self.path.len()-1].to_vec();
        last.append(&mut self.path[0..=corners[0].0].to_vec());
        sections[3] = &last.as_slice();
        let mut combined = Vec::new();
        for (i, path) in sections.iter().enumerate() {
            let mut reduced = reduce::<T>(path, tolerance);
            if i != 3 {
                reduced.pop();
            }
            combined.append(&mut reduced);
        }
        if combined.len() <= 3 {
            return None
        }
        Some(Self {
            path: combined
        })
    }

}

impl PathI32 {
    /// Returns a copy of self after Path Smoothing, preserving corners.
    /// 
    /// `corner_threshold` is specified in radians.
    /// `outset_ratio` is a real number >= 1.0.
    /// `segment_length` is specified in pixels (length unit in path coordinate system).
    pub fn smooth(
        &self, corner_threshold: f64, outset_ratio: f64, segment_length: f64, max_iterations: usize
    ) -> PathF64 {
        assert!(max_iterations > 0);
        let mut corners = SubdivideSmooth::find_corners(self, corner_threshold);
        let mut path = self.to_path_f64();
        for _i in 0..max_iterations {
            let result = SubdivideSmooth::subdivide_keep_corners(&path, &corners, outset_ratio, segment_length);
            path = result.0;
            corners = result.1;
            if result.2 { // Can terminate early
                break;
            }
        }
        path
    }
}

impl PathF64 {
    pub fn smooth(
        &self, corner_threshold: f64, outset_ratio: f64, segment_length: f64, max_iterations: usize
    ) -> PathF64 {
        assert!(max_iterations > 0);
        let mut corners = SubdivideSmooth::find_corners(self, corner_threshold);
        let mut path = PathF64::new();
        for _i in 0..max_iterations {
            let result = SubdivideSmooth::subdivide_keep_corners(self, &corners, outset_ratio, segment_length);
            path = result.0;
            corners = result.1;
            if result.2 { // Can terminate early
                break;
            }
        }
        path
    }
}

impl PathI32 {

    /// Returns a copy of self after Path Simplification:
    /// 
    /// First remove staircases then simplify by limiting penalties.
    pub fn simplify(&self, clockwise: bool) -> Self {
        let path = PathSimplify::remove_staircase(self, clockwise);
        PathSimplify::limit_penalties(&path)
    }

    /// Converts outline of pixel cluster to path with Path Walker. 
    /// Takes a bool representing the clockwiseness of traversal (useful in svg representation to represent holes).
    /// Takes an enum PathSimplifyMode which indicates the required operation:
    /// 
    /// - Polygon - Walk path and simplify it
    /// - Otherwise - Walk path only
    pub fn image_to_path(image: &BinaryImage, clockwise: bool, mode: PathSimplifyMode) -> PathI32 {
        match mode {
            PathSimplifyMode::Polygon => {
                let path = Self::image_to_path_baseline(image, clockwise);
                path.simplify(clockwise)
            },
            // Otherwise
            PathSimplifyMode::None | PathSimplifyMode::Spline => {
                Self::image_to_path_baseline(image, clockwise)
            },
        }
    }

    /// Returns a copy of self converted to PathF64
    pub fn to_path_f64(&self) -> PathF64 {
        PathF64 {
            path: self.path.iter().map(|p| {PointF64{x:p.x as f64, y:p.y as f64}}).collect()
        }
    }

    fn image_to_path_baseline(image: &BinaryImage, clockwise: bool) -> PathI32 {
        let (_boundary, start, _length) = Shape::image_boundary_and_position_length(&image);
        let mut path = Vec::new();
        if let Some(start) = start {
            let walker = PathWalker::new(&image, start, clockwise);
            path.extend(walker);
        }
        PathI32 { path }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_svg_string() {
        let mut path = PathI32::new();
        path.add(PointI32 { x: 0, y: 0 });
        path.add(PointI32 { x: 1, y: 0 });
        path.add(PointI32 { x: 1, y: 1 });
        assert_eq!("M0,0 L1,0 L1,1 ", path.to_svg_string(false, &PointI32::default(), None));
    }

    #[test]
    fn test_to_svg_string_offset() {
        let mut path = PathI32::new();
        path.add(PointI32 { x: 0, y: 0 });
        path.add(PointI32 { x: 1, y: 0 });
        path.add(PointI32 { x: 1, y: 1 });
        assert_eq!("M1,1 L2,1 L2,2 ", path.to_svg_string(false, &PointI32 { x: 1, y: 1 }, None));
    }

    #[test]
    fn test_to_svg_string_closed() {
        let mut path = PathI32::new();
        path.add(PointI32 { x: 0, y: 0 });
        path.add(PointI32 { x: 1, y: 0 });
        path.add(PointI32 { x: 1, y: 1 });
        path.add(PointI32 { x: 0, y: 0 });
        assert_eq!("M0,0 L1,0 L1,1 Z ", path.to_svg_string(true, &PointI32::default(), None));
    }

    #[test]
    fn test_reduce_noop() {
        let path = Path {
            path: vec![
                PointI32 { x: 0, y: 0 },
                PointI32 { x: 1, y: 0 },
                PointI32 { x: 1, y: 1 },
                PointI32 { x: 0, y: 1 },
                PointI32 { x: 0, y: 0 },
            ]
        };
        assert_eq!(path.reduce(0.5).unwrap().path, path.path);
    }

    #[test]
    fn test_reduce_empty() {
        let path = Path {
            path: vec![
                PointI32 { x: 0, y: 0 },
                PointI32 { x: 1, y: 0 },
                PointI32 { x: 1, y: 1 },
                PointI32 { x: 0, y: 1 },
                PointI32 { x: 0, y: 0 },
            ]
        };
        assert!(path.reduce(2.0).is_none());
    }

    #[test]
    fn test_reduce_noop_2() {
        let path = Path {
            path: vec![
                PointI32 { x: 0, y: 0 },
                PointI32 { x: 1, y: 0 },
                PointI32 { x: 10, y: 0 },
                PointI32 { x: 10, y: 9 },
                PointI32 { x: 10, y: 10 },
                PointI32 { x: 0, y: 10 },
                PointI32 { x: 0, y: 9 },
                PointI32 { x: 0, y: 0 },
            ]
        };
        assert_eq!(path.reduce(0.5).unwrap().path, vec![
            PointI32 { x: 0, y: 0 },
            PointI32 { x: 10, y: 0 },
            PointI32 { x: 10, y: 10 },
            PointI32 { x: 0, y: 10 },
            PointI32 { x: 0, y: 0 },
        ]);
    }

    #[test]
    fn test_reduce() {
        let path = Path {
            path: vec![
                PointI32 { x: 0, y: 0 },
                PointI32 { x: 1, y: 0 },
                PointI32 { x: 10, y: 0 },
                PointI32 { x: 10, y: 9 },
                PointI32 { x: 10, y: 10 },
                PointI32 { x: 0, y: 10 },
                PointI32 { x: 0, y: 9 },
                PointI32 { x: 0, y: 0 },
            ]
        };
        assert_eq!(path.reduce(1.0).unwrap().path, vec![
            PointI32 { x: 0, y: 0 },
            PointI32 { x: 10, y: 0 },
            PointI32 { x: 10, y: 10 },
            PointI32 { x: 0, y: 10 },
            PointI32 { x: 0, y: 0 },
        ]);
    }

    #[test]
    fn test_reduce_shuffle() {
        let path = Path {
            path: vec![
                PointI32 { x: 0, y: 0 },
                PointI32 { x: 1, y: 0 },
                PointI32 { x: 10, y: 0 },
                PointI32 { x: 10, y: 10 },
                PointI32 { x: 9, y: 9 },
                PointI32 { x: 0, y: 9 },
                PointI32 { x: 0, y: 10 },
                PointI32 { x: 0, y: 0 },
            ]
        };
        assert_eq!(path.reduce(1.0).unwrap().path, vec![
            PointI32 { x: 0, y: 0 },
            PointI32 { x: 10, y: 0 },
            PointI32 { x: 10, y: 10 },
            PointI32 { x: 0, y: 10 },
            PointI32 { x: 0, y: 0 },
        ]);
    }

    #[test]
    fn test_reduce_diamond_noop() {
        let path = Path {
            path: vec![
                PointI32 { x: 0, y: 0 },
                PointI32 { x: 1, y: 1 },
                PointI32 { x: 0, y: 2 },
                PointI32 { x: -1, y: 1 },
                PointI32 { x: 0, y: 0 },
            ]
        };
        assert_eq!(path.reduce(0.5).unwrap().path, path.path);
    }

    #[test]
    fn test_reduce_diamond() {
        let path = Path {
            path: vec![
                PointI32 { x: 0, y: 0 },
                PointI32 { x: 10, y: 10 },
                PointI32 { x: 9, y: 9 },
                PointI32 { x: 0, y: 20 },
                PointI32 { x: 0, y: 19 },
                PointI32 { x: -10, y: 10 },
                PointI32 { x: -10, y: 9 },
                PointI32 { x: 0, y: 0 },
            ]
        };
        assert_eq!(path.reduce(2.0).unwrap().path, vec![
            PointI32 { x: 0, y: 0 },
            PointI32 { x: 10, y: 10 },
            PointI32 { x: 0, y: 20 },
            PointI32 { x: -10, y: 10 },
            PointI32 { x: 0, y: 0 },
        ]);
    }

    #[test]
    fn test_reduce_triangle_noop() {
        let path = Path {
            path: vec![
                PointI32 { x: 0, y: 0 },
                PointI32 { x: 1, y: 1 },
                PointI32 { x: 0, y: 1 },
                PointI32 { x: 0, y: 0 },
            ]
        };
        assert_eq!(path.reduce(0.5).unwrap().path, path.path);
    }

    #[test]
    fn test_reduce_triangle_degenerate() {
        let path = Path {
            path: vec![
                PointI32 { x: 0, y: 0 },
                PointI32 { x: 10, y: 10 },
                PointI32 { x: 0, y: 1 },
                PointI32 { x: 0, y: 0 },
            ]
        };
        assert!(path.reduce(2.0).is_none());
    }

    #[test]
    fn test_path_to_svg_precision_i32() {
        let path = Path {
            path: vec![
                PointI32 { x: 0, y: 0 },
                PointI32 { x: 2, y: 3 },
                PointI32 { x: 4, y: 5 },
            ]
        };
        assert_eq!(
            path.to_svg_string(false, &PointI32 { x: 0, y: 0 }, None),
            "M0,0 L2,3 L4,5 ".to_owned()
        );
        assert_eq!(
            path.to_svg_string(false, &PointI32 { x: 0, y: 0 }, Some(2)),
            "M0,0 L2,3 L4,5 ".to_owned()
        );
    }

    #[test]
    fn test_path_to_svg_precision_f64() {
        let path = Path {
            path: vec![
                PointF64 { x: 2.22, y: 2.67 },
                PointF64 { x: 3.50, y: 3.48 },
                PointF64 { x: 0.0, y: 0.0 },
            ]
        };
        assert_eq!(
            path.to_svg_string(false, &PointF64 { x: 0.0, y: 0.0 }, None),
            "M2.22,2.67 L3.5,3.48 L0,0 ".to_owned()
        );
        assert_eq!(
            path.to_svg_string(false, &PointF64 { x: 0.0, y: 0.0 }, Some(1)),
            "M2.2,2.7 L3.5,3.5 L0,0 ".to_owned()
        );
        assert_eq!(
            path.to_svg_string(false, &PointF64 { x: 0.0, y: 0.0 }, Some(0)),
            "M2,3 L4,3 L0,0 ".to_owned()
        );
    }
}