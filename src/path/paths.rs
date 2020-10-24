use std::{f64::consts::*};
use crate::{BinaryImage, PointI32, PointF64, Shape};
use super::{PathSimplify, PathSimplifyMode, PathWalker, SubdivideSmooth};

#[derive(Default)]
pub struct PathI32 {
    pub path: Vec<PointI32>,
}

#[derive(Default)]
pub struct PathF64 {
    pub path: Vec<PointF64>,
}

impl PathI32 {
    pub fn new_empty() -> Self {
        Self {
            path: vec![]
        }
    }

    pub fn add(&mut self, point: PointI32) {
        self.path.push(point);
    }

    pub fn iter(&self) -> std::slice::Iter<PointI32> {
        self.path.iter()
    }

    pub fn len(&self) -> usize {
        self.path.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn offset(&mut self, offset: &PointI32) {
        for path in self.path.iter_mut() {
            path.x += offset.x;
            path.y += offset.y;
        }
    }

    pub fn image_to_path(image: &BinaryImage, clockwise: bool, mode: PathSimplifyMode) -> PathI32 {
        match mode {
            PathSimplifyMode::Polygon => {
                let path = Self::image_to_path_baseline(image, clockwise);
                let path = PathSimplify::remove_staircase(&path, clockwise);
                PathSimplify::simplify(&path)
            },
            PathSimplifyMode::None | PathSimplifyMode::Spline => {
                Self::image_to_path_baseline(image, clockwise)
            },
        }
    }

    pub fn image_to_path_baseline(image: &BinaryImage, clockwise: bool) -> PathI32 {
        let (_boundary, start, _length) = Shape::image_boundary_and_position_length(&image);
        let mut path = Vec::new();
        if let Some(start) = start {
            let walker = PathWalker::new(&image, start, clockwise);
            path.extend(walker);
        }
        PathI32 { path }
    }

    pub fn to_svg_path(&self, close: bool, offset: &PointI32) -> String {
        let o = offset;
        [
            self.path
                .iter()
                .take(1)
                .map(|p| format!("M{},{} ", p.x + o.x, p.y + o.y))
                .collect::<String>(),
            self.path
                .iter()
                .skip(1)
                .map(|p| format!("L{},{} ", p.x + o.x, p.y + o.y))
                .collect::<String>(),
            if close {
                "Z ".to_owned()
            } else {
                "".to_owned()
            },
        ]
        .concat()
    }

    pub fn to_pathf64(&self) -> PathF64 {
        PathF64 {
            path: self.path.iter().map(|p| {PointF64{x:p.x as f64, y:p.y as f64}}).collect()
        }
    }
}

impl PathF64 {

    pub fn new_empty() -> Self {
        Self {
            path: vec![]
        }
    }

    pub fn add(&mut self, point: PointF64) {
        self.path.push(point);
    }

    pub fn iter(&self) -> std::slice::Iter<PointF64> {
        self.path.iter()
    }

    pub fn len(&self) -> usize {
        self.path.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn offset(&mut self, offset: &PointF64) {
        for path in self.path.iter_mut() {
            path.x += offset.x;
            path.y += offset.y;
        }
    }

    pub fn image_to_path(image: &BinaryImage, clockwise: bool) -> PathF64 {
        let path = PathI32::image_to_path_baseline(image, clockwise);
        let path = PathSimplify::remove_staircase(&path, clockwise);
        let path = PathSimplify::simplify(&path);
        let corners = SubdivideSmooth::find_corners(&path, FRAC_PI_3);
        SubdivideSmooth::subdivide_iterative(&path, &corners, 4.0, 10)
    }

    pub fn to_svg_path(&self, close: bool, offset: &PointF64) -> String {
        let o = &PointF64 {x: offset.x, y: offset.y};
        [
            self.path
                .iter()
                .take(1)
                .map(|p| format!("M{},{} ", p.x + o.x, p.y + o.y))
                .collect::<String>(),
            self.path
                .iter()
                .skip(1)
                .map(|p| format!("L{},{} ", p.x + o.x, p.y + o.y))
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
