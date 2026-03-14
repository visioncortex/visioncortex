//! Functions to compute and manipulate bounding rectangles

use std::cmp::min;
use crate::{PointI32, PointF64, disjoint_sets};

/// Any object that has a bounding rect
pub trait Bound {
    fn bound(&self) -> BoundingRect;
    fn overlaps<B: Bound>(&self, other: &B) -> bool {
        self.bound().hit(other.bound())
    }
}

/// The rectangle that bounds an object
#[derive(Copy, Clone, PartialEq, Default, Eq, Debug)]
pub struct BoundingRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct BoundingRectF64 {
    pub left_top: PointF64,
    pub right_bottom: PointF64,
}

/// Statistics over a collection of objects with `Bound` trait
#[derive(Debug)]
pub struct BoundStat {
    pub average_area: i32,
    pub average_width: i32,
    pub average_height: i32,
    pub min_width: i32,
    pub min_height: i32,
}

impl BoundStat {
    pub fn calculate<B: Bound>(bs: &[B]) -> Self {
        let mut sum_area   = 0;
        let mut sum_width  = 0;
        let mut sum_height = 0;
        let mut min_width  = i32::MAX;
        let mut min_height = i32::MAX;

        for b in bs.iter() {
            let b      = b.bound();
            let width  = b.width();
            let height = b.height();

            sum_area   += width * height;
            sum_width  += width;
            sum_height += height;
            min_width   = min(min_width, width);
            min_height  = min(min_height, height);
        }

        let n = bs.len() as i32;

        Self {
            average_area:   sum_area / n,
            average_width:  sum_width / n,
            average_height: sum_height / n,
            min_width,
            min_height,
        }
    }
}

impl BoundingRect {
    // assume top-left origin
    pub fn new_x_y_w_h(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            left: x,
            top: y,
            right: x + w,
            bottom: y + h,
        }
    }

    pub fn width(self) -> i32 {
        self.right - self.left
    }

    pub fn height(self) -> i32 {
        self.bottom - self.top
    }

    pub fn area(&self) -> i32 {
        self.width() * self.height()
    }

    pub fn is_empty(self) -> bool {
        self.width() == 0 && self.height() == 0
    }

    pub fn center(self) -> PointI32 {
        PointI32 {
            x: (self.left + self.right) >> 1,
            y: (self.top + self.bottom) >> 1,
        }
    }

    #[inline]
    pub fn left_top(&self) -> PointI32 {
        PointI32::new(self.left, self.top)
    }

    #[inline]
    pub fn top_left(&self) -> PointI32 {
        PointI32::new(self.left, self.top)
    }

    #[inline]
    pub fn top_right(&self) -> PointI32 {
        PointI32::new(self.right, self.top)
    }

    #[inline]
    pub fn bottom_left(&self) -> PointI32 {
        PointI32::new(self.left, self.bottom)
    }

    #[inline]
    pub fn right_bottom(&self) -> PointI32 {
        PointI32::new(self.right, self.bottom)
    }

    #[inline]
    pub fn bottom_right(&self) -> PointI32 {
        PointI32::new(self.right, self.bottom)
    }

    /// Calculates the squared distance betweeen the center of two `BoundingRect`s.
    pub fn sq_dist(self, other: Self) -> i32 {
        let diff = self.center() - other.center();
        diff.dot(diff)
    }

    pub fn aspect_ratio(self) -> f64 {
        std::cmp::max(self.width(), self.height()) as f64
            / std::cmp::min(self.width(), self.height()) as f64
    }

    pub fn aspect_ratio_doubled(self) -> i32 {
        2 * std::cmp::max(self.width(), self.height()) / std::cmp::min(self.width(), self.height())
    }

    pub fn add_x_y(&mut self, x: i32, y: i32) {
        if self.is_empty() {
            self.left = x;
            self.right = x + 1;
            self.top = y;
            self.bottom = y + 1;
            return;
        }
        if x < self.left {
            self.left = x;
        } else if x + 1 > self.right {
            self.right = x + 1;
        }
        if y < self.top {
            self.top = y;
        } else if y + 1 > self.bottom {
            self.bottom = y + 1;
        }
    }

    pub fn merge(&mut self, other: Self) {
        if other.is_empty() {
            return;
        }
        if self.is_empty() {
            self.left = other.left;
            self.right = other.right;
            self.top = other.top;
            self.bottom = other.bottom;
            return;
        }
        self.left = std::cmp::min(self.left, other.left);
        self.right = std::cmp::max(self.right, other.right);
        self.top = std::cmp::min(self.top, other.top);
        self.bottom = std::cmp::max(self.bottom, other.bottom);
    }

    pub fn clear(&mut self) {
        self.left = 0;
        self.right = 0;
        self.top = 0;
        self.bottom = 0;
    }

    /// Returns true if the two rectangles overlap in any way — including when
    /// one is completely contained within the other, and when they only touch
    /// at an edge or corner. Use [`BoundingRect::intersect`] to exclude containment.
    ///
    /// ## Examples
    /// ```text
    /// Partial overlap → true     Containment → true     Touching edge → true   Disjoint → false
    /// ┌────┐                     ┌──────────┐           ┌────┬────┐            ┌────┐  ┌────┐
    /// │  ┌─┼──┐                  │  ┌────┐  │           │    │    │            │    │  │    │
    /// │  │ │  │                  │  │    │  │           │    │    │            │    │  │    │
    /// └──┼─┘  │                  │  └────┘  │           └────┴────┘            └────┘  └────┘
    ///    └────┘                  └──────────┘
    /// ```
    pub fn hit(self, other: Self) -> bool {
        let r1 = self;
        let r2 = other;
        !(r2.left > r1.right ||
          r2.right < r1.left ||
          r2.top > r1.bottom ||
          r2.bottom < r1.top )
    }

    /// Returns true if the **edges** of the two rectangles cross each other —
    /// i.e. they partially overlap, but neither is fully contained within the other.
    ///
    /// This differs from [`hit`] in:
    /// - Returns `false` when one rect is completely inside the other (no edge crossing).
    ///
    /// Touching at a shared edge or corner counts as intersecting (non-strict inequalities),
    /// consistent with [`hit`].
    ///
    /// ## Examples
    /// ```text
    /// Partial overlap → true     Containment → false    Disjoint → false
    /// ┌────┐                     ┌──────────┐           ┌────┐  ┌────┐
    /// │  ┌─┼──┐                  │  ┌────┐  │           │    │  │    │
    /// │  │ │  │                  │  │    │  │           │    │  │    │
    /// └──┼─┘  │                  │  └────┘  │           └────┘  └────┘
    ///    └────┘                  └──────────┘
    /// ```
    pub fn intersect(self, other: Self) -> bool {
        let r1 = self;
        let r2 = other;

        let overlap =
            !(r2.left > r1.right  || r2.right  < r1.left ||
              r2.top  > r1.bottom || r2.bottom < r1.top);

        let r1_contains_r2 =
            r1.left <= r2.left && r1.right  >= r2.right &&
            r1.top  <= r2.top  && r1.bottom >= r2.bottom;

        let r2_contains_r1 =
            r2.left <= r1.left && r2.right  >= r1.right &&
            r2.top  <= r1.top  && r2.bottom >= r1.bottom;

        overlap && !r1_contains_r2 && !r2_contains_r1
    }

    /// Returns true if `self` fully contains `other`.
    ///
    /// Touching edges count as containment — a rect contains itself.
    ///
    /// This is asymmetric: `a.contains(b)` does **not** imply `b.contains(a)`.
    ///
    /// ## Examples
    /// ```text
    /// Contains → true    Shared edge → true   Not contained → false
    /// ┌──────────┐       ┌──────────┐         ┌────┐
    /// │  ┌────┐  │       ┌──────┐   │         │  ┌─┼──┐
    /// │  │    │  │       │      │   │         │  │ │  │
    /// │  └────┘  │       └──────┘   │         └──┼─┘  │
    /// └──────────┘       └──────────┘            └────┘
    /// ```
    pub fn contains(self, other: Self) -> bool {
        self.left   <= other.left   &&
        self.right  >= other.right  &&
        self.top    <= other.top    &&
        self.bottom >= other.bottom
    }

    pub fn clip(&mut self, other: Self) {
        if self.left < other.left {
            self.left = other.left;
        }
        if self.top < other.top {
            self.top = other.top;
        }
        if self.right > other.right {
            self.right = other.right;
        }
        if self.bottom > other.bottom {
            self.bottom = other.bottom;
        }
    }

    pub fn squared(self) -> Self {
        let size = std::cmp::max(self.width(), self.height());
        Self::new_x_y_w_h(
            self.left - ((size - self.width()) >> 1),
            self.top - ((size - self.height()) >> 1),
            size,
            size,
        )
    }

    pub fn translate(&mut self, p: PointI32) {
        self.left += p.x;
        self.top += p.y;
        self.right += p.x;
        self.bottom += p.y;
    }

    pub fn expand_xy(&self, expand_x: i32, expand_y: i32) -> Self {
        expand(*self, expand_x, expand_y)
    }

    /// Tolerance means:
    ///     1. Extend each boundary on both sides by `tolerance` units along its direction.
    ///     2. `true` is returned iff `p` lies on either one of the extended boundaries.
    /// 
    /// A point `p` lying on boundary "strictly" means this function returns true with `p`
    /// and `tolerance` set as 0.
    pub fn have_point_on_boundary(&self, p: PointI32, tolerance: usize) -> bool {
        let t = tolerance as i32;
        // On left or right bounds
        (p.x == self.left || p.x == self.right) && (self.top-t <= p.y && p.y <= self.bottom+t)
            ||
        // On top or bottom bounds
        (p.y == self.top || p.y == self.bottom) && (self.left-t <= p.x && p.x <= self.right+t)
    }

    pub fn have_point_inside(&self, p: PointI32) -> bool {
        (self.left < p.x && p.x < self.right)
        &&
        (self.top < p.y && p.y < self.bottom)
    }

    /// For definition of `boundary_tolerance`, see BoundingRect::have_point_on_boundary().
    pub fn have_point_on_boundary_or_inside(&self, p: PointI32, boundary_tolerance: usize) -> bool {
        self.have_point_on_boundary(p, boundary_tolerance) || self.have_point_inside(p)
    }

    /// Given a point on the boundary, return the closest point inside the
    /// bounding rect. The behavior is undefined unless 'p' is a point on
    /// boundary (strictly) and the area of this rect is larger than 1.
    pub fn get_closest_point_inside(&self, p: PointI32) -> PointI32 {
        assert!(self.have_point_on_boundary(p, 0));
        assert!(self.width() * self.height() > 1);

        p + PointI32::new(
            if p.x == self.left {1}
            else if p.x == self.right {-1}
            else {0},
            if p.y == self.top {1}
            else if p.y == self.bottom {-1}
            else {0},
        )
    }

    /// Given a point on the boundary, return the closest point outside the
    /// bounding rect. Note that if 'p' is a corner, there are three closest
    /// points, but the diagonal one is always returned. The behavior is
    /// undefined unless 'p' is a point on boundary (strictly).
    pub fn get_closest_point_outside(&self, p: PointI32) -> PointI32 {
        assert!(self.have_point_on_boundary(p, 0));

        p + PointI32::new(
            if p.x == self.left {-1}
            else if p.x == self.right {1}
            else {0},
            if p.y == self.top {-1}
            else if p.y == self.bottom {1}
            else {0},
        )
    }

    /// Starting from 'p', copy the boundary points into a new Vec following
    /// the orientation specified by 'clockwise' and return it. The behavior
    /// is undefined unless 'p' is a point on boundary (strictly).
    pub fn get_boundary_points_from(&self, p: PointI32, clockwise: bool) -> Vec<PointI32> {
        assert!(self.have_point_on_boundary(p, 0));

        let mut boundary_points = vec![p];
        // Evaluate the next point to be pushed
        let mut offset = if p.x == self.left {
            PointI32::new(0, -1)
        } else if p.y == self.top {
            PointI32::new(1, 0)
        } else if p.x == self.right {
            PointI32::new(0, 1)
        } else {
            PointI32::new(-1, 0)
        };
        if !clockwise { offset = -offset; }
        let mut curr = p + offset;
        if !self.have_point_on_boundary(curr, 0) {
            curr = curr.rotate_90deg(p, clockwise);
        }

        let mut prev = p;

        let four_neighbors_offsets = [
            PointI32::new(1, 0),
            PointI32::new(-1, 0),
            PointI32::new(0, 1),
            PointI32::new(0, -1),
        ];
        
        while curr != p {
            boundary_points.push(curr);
            let temp_curr = curr;
            for offset in four_neighbors_offsets.iter() {
                let next = curr + *offset;
                if next != prev && self.have_point_on_boundary(next, 0) {
                    curr = next;
                    break;
                }
            }
            // curr must have changed
            assert_ne!(curr, boundary_points.last().copied().unwrap());
            prev = temp_curr;
        }

        boundary_points
    }
}

impl Default for BoundingRectF64 {
    fn default() -> Self {
        Self {
            left_top: PointF64::new(f64::MAX, f64::MAX),
            right_bottom: PointF64::new(f64::MIN, f64::MIN),
        }
    }
}

impl BoundingRectF64 {
    pub fn new(left_top: PointF64, right_bottom: PointF64) -> Self {
        Self { left_top, right_bottom }
    }

    pub fn new_x_y_w_h(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self {
            left_top: PointF64::new(x, y),
            right_bottom: PointF64::new(x + w, y + h),
        }
    }

    pub fn new_xy_wh(xy: PointF64, wh: PointF64) -> Self {
        Self {
            left_top: xy,
            right_bottom: xy + wh,
        }
    }

    pub fn is_empty(self) -> bool {
        self.left_top.x == f64::MAX &&
        self.left_top.y == f64::MAX &&
        self.right_bottom.x == f64::MIN &&
        self.right_bottom.y == f64::MIN
    }

    pub fn right_top(&self) -> PointF64 {
        PointF64::new(self.right_bottom.x, self.left_top.y)
    }

    pub fn left_bottom(&self) -> PointF64 {
        PointF64::new(self.left_top.x, self.right_bottom.y)
    }

    pub fn width(self) -> f64 {
        self.right_bottom.x - self.left_top.x
    }

    pub fn height(self) -> f64 {
        self.right_bottom.y - self.left_top.y
    }

    pub fn merge(&mut self, other: Self) {
        if other.is_empty() {
            return;
        }
        if self.is_empty() {
            self.left_top = other.left_top;
            self.right_bottom = other.right_bottom;
            return;
        }
        self.left_top.x = self.left_top.x.min(other.left_top.x);
        self.left_top.y = self.left_top.y.min(other.left_top.y);
        self.right_bottom.x = self.right_bottom.x.max(other.right_bottom.x);
        self.right_bottom.y = self.right_bottom.y.max(other.right_bottom.y);
    }

    pub fn add_point(&mut self, p: PointF64) {
        self.left_top.x = self.left_top.x.min(p.x);
        self.left_top.y = self.left_top.y.min(p.y);
        self.right_bottom.x = self.right_bottom.x.max(p.x);
        self.right_bottom.y = self.right_bottom.y.max(p.y);
    }

    pub fn to_rect(&self) -> BoundingRect {
        BoundingRect {
            left: self.left_top.x.floor() as i32,
            top: self.left_top.y.floor() as i32,
            right: self.right_bottom.x.ceil() as i32,
            bottom: self.right_bottom.y.ceil() as i32,
        }
    }
}

impl Bound for BoundingRect {
    fn bound(&self) -> BoundingRect {
        *self
    }
}

impl Bound for BoundingRectF64 {
    fn bound(&self) -> BoundingRect {
        self.to_rect()
    }
}

pub fn average_width<B: Bound>(bs: &[B]) -> i32 {
    let sum: i32 = bs
        .iter()
        .map(|b| b.bound().width())
        .sum();

    sum / (bs.len() as i32)
}

pub fn average_height<B: Bound>(bs: &[B]) -> i32 {
    let sum: i32 = bs
        .iter()
        .map(|b| b.bound().height())
        .sum();

    sum / (bs.len() as i32)
}

pub fn enclosing_bound<B: Bound>(bs: &[B]) -> BoundingRect {
    let mut enclosing = BoundingRect::default();

    for b in bs.iter() {
        enclosing.merge(b.bound());
    }

    enclosing
}

pub fn merge_expand<B: Bound>(items: Vec<B>, expand_x: i32, expand_y: i32) -> Vec<Vec<B>> {
    disjoint_sets::group_by_cached_key(
        items,
        |item| {
            expand(item.bound(), expand_x, expand_y)
        },
        |a, b| a.overlaps(b),
    )
}

pub fn expand(b: BoundingRect, expand_x: i32, expand_y: i32) -> BoundingRect {
    BoundingRect::new_x_y_w_h(
        b.left - expand_x,
        b.top - expand_y,
        b.width() + 2 * expand_x,
        b.height() + 2 * expand_y
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounding_rect_1x1() {
        let mut rect = BoundingRect::default();
        rect.add_x_y(0, 0);
        assert_eq!(rect.left, 0);
        assert_eq!(rect.top, 0);
        assert_eq!(rect.right, 1);
        assert_eq!(rect.bottom, 1);
        assert_eq!(rect.width(), 1);
        assert_eq!(rect.height(), 1);
    }

    #[test]
    fn bounding_rect_2x2() {
        let mut rect = BoundingRect::default();
        rect.add_x_y(1, 1);
        rect.add_x_y(2, 2);
        assert_eq!(rect.left, 1);
        assert_eq!(rect.top, 1);
        assert_eq!(rect.right, 3);
        assert_eq!(rect.bottom, 3);
        assert_eq!(rect.width(), 2);
        assert_eq!(rect.height(), 2);
    }

    #[test]
    fn bounding_rect_aspect_ratio_doubled() {
        let mut rect = BoundingRect::default();
        rect.add_x_y(0, 0);
        rect.add_x_y(1, 0);
        assert_eq!(rect.aspect_ratio_doubled(), 4);
    }

    #[test]
    fn bounding_rect_clip() {
        let mut rect = BoundingRect::default();
        rect.add_x_y(1, 1);
        rect.add_x_y(4, 4);
        rect.clip(BoundingRect::new_x_y_w_h(0, 0, 3, 3));
        assert_eq!(rect, BoundingRect::new_x_y_w_h(1, 1, 2, 2));
    }

    #[test]
    fn enclosing_bound_test() {
        let mut a = BoundingRect::default();
        a.add_x_y(1, 1);
        let mut b = BoundingRect::default();
        b.add_x_y(2, 2);
        assert_eq!(
            enclosing_bound(&[a, b]),
            BoundingRect { left: 1, top: 1, right: 3, bottom: 3 }
        );
    }

    #[test]
    fn merge_expand_noop() {
        let mut a = BoundingRect::default();
        a.add_x_y(1, 1);
        let mut b = BoundingRect::default();
        b.add_x_y(3, 3);
        assert_eq!(
            merge_expand(vec![a, b], 0, 0),
            [[b],[a]]
        );
    }

    #[test]
    fn merge_expand_merged() {
        let mut a = BoundingRect::default();
        a.add_x_y(1, 1);
        let mut b = BoundingRect::default();
        b.add_x_y(3, 3);
        assert_eq!(
            merge_expand(vec![a, b], 1, 1),
            [[b,a]]
        );
    }

    #[test]
    fn merge_horizontal() {
        let mut a = BoundingRect::default();
        a.add_x_y(1, 1);
        let mut b = BoundingRect::default();
        b.add_x_y(3, 1);
        assert_eq!(
            merge_expand(vec![a, b], 1, 0),
            [[b,a]]
        );
    }

    #[test]
    fn merge_horizontal_noop() {
        let mut a = BoundingRect::default();
        a.add_x_y(1, 1);
        let mut b = BoundingRect::default();
        b.add_x_y(1, 3);
        assert_eq!(
            merge_expand(vec![a, b], 1, 0),
            [[b],[a]]
        );
    }

    #[test]
    fn merge_vertical() {
        let mut a = BoundingRect::default();
        a.add_x_y(1, 1);
        let mut b = BoundingRect::default();
        b.add_x_y(1, 3);
        assert_eq!(
            merge_expand(vec![a, b], 0, 1),
            [[b,a]]
        );
    }

    #[test]
    fn merge_vertical_noop() {
        let mut a = BoundingRect::default();
        a.add_x_y(1, 1);
        let mut b = BoundingRect::default();
        b.add_x_y(3, 1);
        assert_eq!(
            merge_expand(vec![a, b], 0, 1),
            [[b],[a]]
        );
    }

    #[test]
    fn point_on_boundary() {
        // GIVEN a generic bounding rect and its corners
        let rect = BoundingRect::new_x_y_w_h(0, 0, 5, 6);
        let top_left = PointI32::new(rect.left, rect.top);
        let top_right = PointI32::new(rect.right, rect.top);
        let bottom_left = PointI32::new(rect.left, rect.bottom);
        let bottom_right = PointI32::new(rect.right, rect.bottom);

        // WHEN the tolerance for boundary check is the strictest
        let t = 0;

        // THEN its corners are on its boundary
        assert!(rect.have_point_on_boundary(top_left, t));
        assert!(rect.have_point_on_boundary(top_right, t));
        assert!(rect.have_point_on_boundary(bottom_left, t));
        assert!(rect.have_point_on_boundary(bottom_right, t));

        // THEN points inside are not on its boundary
        assert!(!rect.have_point_on_boundary(top_left.translate(PointI32::new(1, 1)), t));
        assert!(!rect.have_point_on_boundary(top_right.translate(PointI32::new(-1, 1)), t));

        // THEN points outside are not on its boundary
        assert!(!rect.have_point_on_boundary(bottom_left.translate(PointI32::new(-1, 1)), t));
        assert!(!rect.have_point_on_boundary(bottom_right.translate(PointI32::new(-1, -1)), t));
    }

    #[test]
    fn point_near_boundary() {
        // GIVEN a generic bounding rect and its corners
        let rect = BoundingRect::new_x_y_w_h(0, 0, 5, 6);
        let top_left = PointI32::new(rect.left, rect.top);
        let top_right = PointI32::new(rect.right, rect.top);
        let bottom_left = PointI32::new(rect.left, rect.bottom);
        let bottom_right = PointI32::new(rect.right, rect.bottom);

        // GIVEN points on its boundary
        let p1 = PointI32::new(rect.left, rect.top + 2);
        let p2 = PointI32::new(rect.left + 3, rect.bottom);

        // THEN the nearest points of those points should be correctly identified
        assert_eq!(top_left + PointI32::new(1, 1), rect.get_closest_point_inside(top_left));
        assert_eq!(top_right + PointI32::new(-1, 1), rect.get_closest_point_inside(top_right));
        assert_eq!(bottom_left + PointI32::new(1, -1), rect.get_closest_point_inside(bottom_left));
        assert_eq!(bottom_right + PointI32::new(-1, -1), rect.get_closest_point_inside(bottom_right));
        assert_eq!(p1 + PointI32::new(1, 0), rect.get_closest_point_inside(p1));
        assert_eq!(p2 + PointI32::new(0, -1), rect.get_closest_point_inside(p2));

        assert_eq!(top_left - PointI32::new(1, 1), rect.get_closest_point_outside(top_left));
        assert_eq!(top_right - PointI32::new(-1, 1), rect.get_closest_point_outside(top_right));
        assert_eq!(bottom_left - PointI32::new(1, -1), rect.get_closest_point_outside(bottom_left));
        assert_eq!(bottom_right - PointI32::new(-1, -1), rect.get_closest_point_outside(bottom_right));
        assert_eq!(p1 - PointI32::new(1, 0), rect.get_closest_point_outside(p1));
        assert_eq!(p2 - PointI32::new(0, -1), rect.get_closest_point_outside(p2));
    }

    #[test]
    fn get_vec_of_boundary_points() {
        // GIVEN a generic bounding rect and some of its corners
        let rect = BoundingRect::new_x_y_w_h(0, 0, 5, 6);
        let top_left = PointI32::new(rect.left, rect.top);
        let bottom_right = PointI32::new(rect.right, rect.bottom);

        // GIVEN points on its boundary
        let p1 = PointI32::new(rect.left, rect.top + 2);
        let p2 = PointI32::new(rect.left + 3, rect.bottom);

        // THEN the vecs of boundary points should be correctly extracted
        let len = ((rect.width() + rect.height()) * 2) as usize;

        let boundary_points = rect.get_boundary_points_from(top_left, true);
        assert_eq!(len, boundary_points.len());
        assert_eq!(top_left, boundary_points[0]);
        assert_eq!(top_left + PointI32::new(1, 0), boundary_points[1]);
        assert_eq!(top_left + PointI32::new(0, 1), boundary_points[len-1]);

        let boundary_points = rect.get_boundary_points_from(bottom_right, false);
        assert_eq!(len, boundary_points.len());
        assert_eq!(bottom_right, boundary_points[0]);
        assert_eq!(bottom_right + PointI32::new(0, -1), boundary_points[1]);
        assert_eq!(bottom_right + PointI32::new(-1, 0), boundary_points[len-1]);

        let boundary_points = rect.get_boundary_points_from(p1, true);
        assert_eq!(len, boundary_points.len());
        assert_eq!(p1, boundary_points[0]);
        assert_eq!(p1 + PointI32::new(0, -1), boundary_points[1]);
        assert_eq!(p1 + PointI32::new(0, 1), boundary_points[len-1]);

        let boundary_points = rect.get_boundary_points_from(p2, false);
        assert_eq!(len, boundary_points.len());
        assert_eq!(p2, boundary_points[0]);
        assert_eq!(p2 + PointI32::new(1, 0), boundary_points[1]);
        assert_eq!(p2 + PointI32::new(-1, 0), boundary_points[len-1]);
    }

    // Helpers: build a rect from (left, top, right, bottom) directly.
    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> BoundingRect {
        BoundingRect { left, top, right, bottom }
    }

    #[test]
    fn intersect_partial_overlap() {
        // Classic diagonal overlap — edges clearly cross
        let a = rect(0, 0, 10, 10);
        let b = rect(5, 5, 15, 15);
        assert!(a.intersect(b));
        assert!(b.intersect(a)); // symmetric
    }

    #[test]
    fn intersect_cross_pattern() {
        // a is tall, b is wide — each contains the other on one axis, partial on the other.
        // Forms a plus/cross shape; edges clearly cross on all four sides.
        let a = rect(2, 0, 8,  10);
        let b = rect(0, 3, 10,  7);
        assert!(a.intersect(b));
        assert!(b.intersect(a));
    }

    #[test]
    fn intersect_shared_edge() {
        // Rects share exactly one edge — counts as intersecting (non-strict)
        let a = rect(0, 0, 10, 10);
        let b = rect(10, 0, 20, 10);
        assert!(a.intersect(b));
        assert!(b.intersect(a));
    }

    #[test]
    fn intersect_shared_corner() {
        // Rects touch at a single corner point
        let a = rect(0, 0, 10, 10);
        let b = rect(10, 10, 20, 20);
        assert!(a.intersect(b));
        assert!(b.intersect(a));
    }

    #[test]
    fn intersect_disjoint() {
        // Completely separate — no overlap at all
        let a = rect(0, 0, 10, 10);
        let b = rect(20, 20, 30, 30);
        assert!(!a.intersect(b));
        assert!(!b.intersect(a));
    }

    #[test]
    fn intersect_containment_excluded() {
        // b is fully inside a — no edge crossing
        let a = rect(0, 0, 20, 20);
        let b = rect(5, 5, 15, 15);
        assert!(!a.intersect(b));
        assert!(!b.intersect(a));
    }

    #[test]
    fn intersect_identical_rects() {
        // Identical rects: each contains the other — no edge crossing
        let a = rect(0, 0, 10, 10);
        assert!(!a.intersect(a));
    }

    #[test]
    fn intersect_contained_on_one_axis_partial_on_other() {
        // b is wider than a (contains a on x) but taller on y (partial overlap)
        // Edges DO cross: b's top/bottom edges slice through a
        let a = rect(2,  0, 8,  10);
        let b = rect(0,  5, 10, 15);
        assert!(a.intersect(b));
        assert!(b.intersect(a));
    }

    #[test]
    fn contains_strict() {
        let outer = BoundingRect::new_x_y_w_h(0, 0, 10, 10);
        let inner = BoundingRect::new_x_y_w_h(2, 2,  6,  6);
        assert!( outer.contains(inner));
        assert!(!inner.contains(outer));
    }

    #[test]
    fn contains_touching_boundary() {
        // inner shares the top-left corner and is smaller — still fully contained
        let outer = BoundingRect::new_x_y_w_h(0, 0, 4, 4);
        let inner = BoundingRect::new_x_y_w_h(0, 0, 2, 2);
        assert!(outer.contains(inner));
        assert!(!inner.contains(outer));
    }

    #[test]
    fn contains_self() {
        let r = BoundingRect::new_x_y_w_h(3, 3, 5, 5);
        assert!(r.contains(r));
    }

    #[test]
    fn contains_partial_overlap_is_false() {
        let a = BoundingRect::new_x_y_w_h(0, 0, 5, 5);
        let b = BoundingRect::new_x_y_w_h(3, 3, 5, 5);
        assert!(!a.contains(b));
        assert!(!b.contains(a));
    }

    #[test]
    fn contains_touching_bottom_right_boundary() {
        // inner shares outer's bottom-right corner — still fully contained
        let outer = BoundingRect::new_x_y_w_h(0, 0, 4, 4);
        let inner = BoundingRect::new_x_y_w_h(2, 2, 2, 2);
        assert!(outer.contains(inner));
        assert!(!inner.contains(outer));
    }

    #[test]
    fn contains_touching_externally_is_false() {
        // outer's bottom-right corner == inner's top-left corner
        let outer = BoundingRect::new_x_y_w_h(0, 0, 4, 4);
        let inner = BoundingRect::new_x_y_w_h(4, 4, 2, 2);
        assert!(!outer.contains(inner));
    }

    #[test]
    fn contains_disjoint_is_false() {
        let a = BoundingRect::new_x_y_w_h(0, 0, 4, 4);
        let b = BoundingRect::new_x_y_w_h(5, 5, 4, 4);
        assert!(!a.contains(b));
        assert!(!b.contains(a));
    }

    #[test]
    fn hit_partial_overlap() {
        let a = BoundingRect::new_x_y_w_h(0, 0, 5, 5);
        let b = BoundingRect::new_x_y_w_h(3, 3, 5, 5);
        assert!(a.hit(b));
        assert!(b.hit(a));
    }

    #[test]
    fn hit_containment() {
        let outer = BoundingRect::new_x_y_w_h(0, 0, 10, 10);
        let inner = BoundingRect::new_x_y_w_h(2, 2,  4,  4);
        assert!(outer.hit(inner));
        assert!(inner.hit(outer));
    }

    #[test]
    fn hit_touching_edge() {
        // right edge of a == left edge of b
        let a = BoundingRect::new_x_y_w_h(0, 0, 4, 4);
        let b = BoundingRect::new_x_y_w_h(4, 0, 4, 4);
        assert!(a.hit(b));
        assert!(b.hit(a));
    }

    #[test]
    fn hit_touching_corner() {
        let a = BoundingRect::new_x_y_w_h(0, 0, 4, 4);
        let b = BoundingRect::new_x_y_w_h(4, 4, 4, 4);
        assert!(a.hit(b));
        assert!(b.hit(a));
    }

    #[test]
    fn hit_disjoint() {
        let a = BoundingRect::new_x_y_w_h(0, 0, 4, 4);
        let b = BoundingRect::new_x_y_w_h(5, 5, 4, 4);
        assert!(!a.hit(b));
        assert!(!b.hit(a));
    }

    #[test]
    fn hit_self() {
        let r = BoundingRect::new_x_y_w_h(3, 3, 5, 5);
        assert!(r.hit(r));
    }
}
