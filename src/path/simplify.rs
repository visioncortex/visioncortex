use crate::{PathI32, PointI32};

use super::util::signed_area;

pub(crate) struct PathSimplify;

#[derive(Copy, Clone, Debug)]
pub enum PathSimplifyMode {
    None,
    Polygon,
    Spline,
}

#[derive(Copy, Clone)]
pub enum Dir {
    Up,
    Right,
    Down,
    Left
}

impl Default for PathSimplifyMode {
    fn default() -> Self {
        Self::None
    }
}

impl PathSimplify {

    /// Returns a copy of a path after removing 1-pixel staircases.
    /// 
    /// Clockwiseness of path must be indicated to perform outset
    pub fn remove_staircase(path: &PathI32, clockwise: bool) -> PathI32 {
        let path = &path.path;
        let len = path.len();

        let segment_length = |i: usize, j: usize| -> i32 {
            (path[i].x - path[j].x).abs() + (path[i].y - path[j].y).abs()
        };

        let mut result = PathI32::new();
        if len == 0 {
            return result;
        }
        for i in 0..len {
            let j = (i + 1) % len;
            let h = if i > 0 { i - 1 } else { len - 1 };
            let keep = if i == 0 || i == len - 1 {
                true
            } else if segment_length(i, h) == 1 || segment_length(i, j) == 1 {
                let area = signed_area(path[h], path[i], path[j]);
                area != 0 && (area > 0) == clockwise
            } else {
                true
            };
            if keep {
                result.add(path[i]);
            }
        }
        result
    }

    pub fn limit_penalties(path: &PathI32) -> PathI32 {
        let tolerance = 1.0;
        let path = &path.path;
        let len = path.len();
        let past_delta = |from: usize, to: usize| -> f64 {
            (from..to).skip(1).map(|i| {
                Self::evaluate_penalty(path[from], path[i], path[to])
            }).fold(0.0, |a, b| a.max(b)) // find max
        };

        let mut result = PathI32::new();
        if len == 0 {
            return result;
        }
        let mut last = 0;
        for i in 0..len {
            if i == 0 {
                result.add(path[i]);
            } else if i == last + 1 {
                continue;
            } else if past_delta(last, i) >= tolerance {
                last = i - 1;
                result.add(path[i-1]);
            }
            if i == len - 1 {
                result.add(path[i]);
            }
        }
        result
    }

    fn evaluate_penalty(a: PointI32, b: PointI32, c: PointI32) -> f64 {
        let sq = |x| { (x * x) as f64 };
        let l1 = (sq(a.x - b.x) + sq(a.y - b.y)).sqrt();
        let l2 = (sq(b.x - c.x) + sq(b.y - c.y)).sqrt();
        let l3 = (sq(c.x - a.x) + sq(c.y - a.y)).sqrt();
        let p = (l1 + l2 + l3) / 2.0;
        let area = (p * (p - l1) * (p - l2) * (p - l3)).sqrt();
        area * area / l3
    }
}