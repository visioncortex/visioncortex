use std::fmt::Debug;

use crate::PointF64;

use super::Orientation;

/// Common properties/methods for any polygons.
///
/// To be composed into any structs that represents a polygon
#[derive(Clone, Default)]
pub struct PolygonProps {
    pub points: Vec<PointF64>,
    pub is_hole: bool,
}

impl Debug for PolygonProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut formatted = vec![(if self.is_hole {"HOLE"} else {"SOLID"}).to_string()];

        for point in self.points.iter() {
            formatted.push(format!("({}, {})", point.x, point.y));
        }

        f.write_str(&("<".to_owned() + &formatted.join(", ") + ">\n"))
    }
}

pub trait PolygonInterface {
    fn props(&self) -> &PolygonProps;
    fn props_mut(&mut self) -> &mut PolygonProps;

    fn num_points(&self) -> usize {
        self.props().num_points()
    }

    fn get_point(&self, i: usize) -> PointF64 {
        self.props().points[i]
    }

    fn set_point(&mut self, i: usize, p: PointF64) {
        self.props_mut().points[i] = p;
    }

    fn is_hole(&self) -> bool {
        self.props().is_hole
    }
}

#[derive(Clone, Debug, Default)]
// The most basic and generic polygon class
pub struct Polygon {
    props: PolygonProps,
}

impl PolygonInterface for Polygon {
    fn props(&self) -> &PolygonProps {
        &self.props
    }

    fn props_mut(&mut self) -> &mut PolygonProps {
        &mut self.props
    }
}

impl Polygon {
    pub fn from_points_and_is_hole(points: Vec<PointF64>, is_hole: bool) -> Self {
        Self {
            props: PolygonProps::from_points_and_is_hole(points, is_hole)
        }
    }

    /// Create a triangle with the default props
    pub fn triangle(p1: PointF64, p2: PointF64, p3: PointF64) -> Self {
        let mut triangle = Self::default();
        triangle.props.points = vec![p1, p2, p3];
        triangle
    }

    pub fn is_valid(&self) -> bool {
        self.props.is_valid()
    }
}

impl PolygonProps {
    pub fn from_points_and_is_hole(points: Vec<PointF64>, is_hole: bool) -> Self {
        Self {
            points,
            is_hole
        }
    }

    pub fn num_points(&self) -> usize {
        self.points.len()
    }

    pub fn get_point_safe(&self, i: usize) -> Option<PointF64> {
        if i < self.num_points() {
            Some(self.points[i])
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.points.clear()
    }

    /// Invert the order of vertices
    pub fn invert(&mut self) {
        self.points.reverse()
    }

    pub fn get_orientation(&self) -> Orientation {
        let mut area = 0.0;
        let len = self.num_points();
        for curr in 0..len {
            let next = (curr+1) % len;
            let (curr_pt, next_pt) = (self.points[curr], self.points[next]);
            area += curr_pt.x * next_pt.y - curr_pt.y * next_pt.x;
        }

        if area > 0.0 {
            Orientation::CounterClockwise
        } else if area < 0.0 {
            Orientation::Clockwise
        } else {
            Orientation::None
        }
    }

    pub fn set_orientation(&mut self, orientation: Orientation) {
        let poly_orientation = self.get_orientation();
        if poly_orientation != Orientation::None && poly_orientation != orientation {
            self.invert();
        }
    }

    /// A valid PolygonProps should have at least 3 points
    pub fn is_valid(&self) -> bool {
        self.num_points() >= 3
    }

    pub fn dump(&self, decimal: bool) -> String {
        let mut dump = vec![self.num_points().to_string(), (self.is_hole as i32).to_string()];
        for p in self.points.iter() {
            dump.push(format!("{} {}",
                if decimal {p.x.to_string()} else {(p.x as i32).to_string()},
                if decimal {p.y.to_string()} else {(p.y as i32).to_string()}
            ));
        }
        dump.join("\n")
    }
}