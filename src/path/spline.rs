use std::{cmp::Ordering};
use crate::{BinaryImage, PointI32, PointF64};
use super::{PathI32, PathSimplify, SubdivideSmooth};

#[derive(Default)]
pub struct Spline {
    pub path: Vec<PointF64>, // Assumed close path: head = tail
    pub splice_points: Vec<bool> // len is path.len()-1 because of above
}

impl Spline {

    pub fn new_empty() -> Self {
        Self {
            path: vec![],
            splice_points: vec![]
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

    pub fn image_to_path(image: &BinaryImage, clockwise: bool, corner_threshold: f64, length_threshold: f64, max_iterations: usize, splice_threshold: f64) -> Spline {
            
        let path = PathI32::image_to_path_baseline(image, clockwise);
        let path = PathSimplify::remove_staircase(&path, clockwise);
        let path = PathSimplify::simplify(&path);
        let corners = SubdivideSmooth::find_corners(&path, corner_threshold);
        let path = SubdivideSmooth::subdivide_iterative(&path, &corners, length_threshold, max_iterations);
        let splice_points = SubdivideSmooth::find_splice_points(&path, splice_threshold);
        Spline {
            path: path.path,
            splice_points
        }
            
    }

    pub fn to_svg_path(&self, close: bool, offset: &PointI32) -> String {
        
        let o = &PointF64 {x: offset.x as f64, y: offset.y as f64};

        let path = &self.path;
        let len = path.len();
        if len<=1 {
            return String::from("");
        }
        if len==2 {
            return format!("M{} {}, L{} {}",
                path[0].x+o.x, path[0].y+o.y,
                path[1].x+o.x, path[1].y+o.y
            );
        }
        let splice_points = &self.splice_points;

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

        let mut result: Vec<String> = vec![format!("M{} {} ", path[cut_points[0]].x + o.x, path[cut_points[0]].y + o.y)];
        for i in 0..num_cut_points {
            let j = (i+1)%num_cut_points;

            let current = cut_points[i];
            let next = cut_points[j];
            let subpath = self.get_circular_subpath(current, next);
            result.push(Self::find_bezier(&subpath, o));
        }

        if close {
            result.push(String::from("Z"));
        }

        result.concat()
    }

    fn get_circular_subpath(&self, from: usize, to: usize) -> Vec<PointF64> {

        let path = &self.path[0..self.path.len()-1];
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

    fn find_bezier(points: &[PointF64], offset: &PointF64) -> String {
        
        let bezier_points: [PointF64; 4] = SubdivideSmooth::fit_points_with_bezier(points);

        let o = offset;
        format!("C{} {}, {} {}, {} {} ", 
            bezier_points[1].x + o.x, bezier_points[1].y + o.y,
            bezier_points[2].x + o.x, bezier_points[2].y + o.y,
            bezier_points[3].x + o.x, bezier_points[3].y + o.y
        )

    }
}
