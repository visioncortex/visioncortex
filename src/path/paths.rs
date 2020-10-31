use std::ops::{Add, AddAssign};

use crate::{BinaryImage, PointF64, PointI32, Shape, ToSvgString};
use super::{PathSimplify, PathSimplifyMode, PathWalker, smooth::SubdivideSmooth};

#[derive(Default)]
/// Path of generic points in 2D space
pub struct Path<T> {
    /// T can be PointI32/PointF64, etc. (see src/point.rs).
    pub path: Vec<T>,
}

impl<T> Path<T>
{
    /// Creates a new 2D Path with no points
    pub fn new() -> Self {
        Self {
            path: vec![]
        }
    }

    /// Adds a point to the end of the path
    pub fn add(&mut self, point: T) {
        self.path.push(point);
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
    pub fn to_svg_path(&self, close: bool, offset: &T) -> String {
        let o = offset;
        [
            self.path
                .iter()
                .take(1)
                .map(|p| format!("M{} ", (*p+*o).to_svg_string()))
                .collect::<String>(),
            self.path
                .iter()
                .skip(1)
                .map(|p| format!("L{} ", (*p+*o).to_svg_string()))
                .collect::<String>(),
            if close {
                "Z ".to_owned()
            } else {
                "".to_owned()
            },
        ]
        .concat()
    }
}

/// Path of 2D PointI32
pub type PathI32 = Path<PointI32>;
/// Path of 2D PointF64
pub type PathF64 = Path<PointF64>;

impl PathI32 {

    /// Returns a copy of self after Path Simplification:
    /// 
    /// First remove staircases then simplify by limiting penalties.
    pub fn simplify(&self, clockwise: bool) -> Self {
        let path = PathSimplify::remove_staircase(self, clockwise);
        PathSimplify::limit_penalties(&path)
    }

    /// Returns a copy of self after Path Smoothing, preserving corners.
    /// 
    /// Corner threshold is specified in radians.
    /// Length threshold is specified in pixels (length unit in path coordinate system).
    pub fn smooth(&self, corner_threshold: f64, length_threshold: f64, max_iterations: usize) -> PathF64 {
        // First locate all corners
        let mut corners = SubdivideSmooth::find_corners(self, corner_threshold);
        let mut path = self.to_path_f64();
        for _i in 0..max_iterations {
            let result = SubdivideSmooth::subdivide_keep_corners(&path, &corners, length_threshold);
            path = result.0;
            corners = result.1;
            if result.2 { // Can terminate early
                break;
            }
        }
        path
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