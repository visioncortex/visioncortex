use crate::{BoundingRect, PointI32};

/// Transformation of coordinate in space
pub trait Transform {
    fn transform(&self, vec: &PointI32) -> PointI32;
    fn transform_rect(&self, rect: &BoundingRect) -> BoundingRect;
}

/// Equivalent to a Homothetic transform
#[derive(Default)]
pub struct RectangularTransform {
    pub a: BoundingRect,
    pub b: BoundingRect,
}

impl RectangularTransform {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_rect_rect(a: BoundingRect, b: BoundingRect) -> Self {
        Self {
            a,
            b,
        }
    }

    pub fn new_point_point(a: PointI32, b: PointI32) -> Self {
        Self {
            a: BoundingRect::new_x_y_w_h(a.x, a.y, 1, 1),
            b: BoundingRect::new_x_y_w_h(b.x, b.y, 1, 1),
        }
    }

    pub fn new_point(b: &PointI32) -> Self {
        Self {
            a: BoundingRect::new_x_y_w_h(0, 0, 1, 1),
            b: BoundingRect::new_x_y_w_h(b.x, b.y, 1, 1),
        }
    }
}

impl Transform for RectangularTransform {
    fn transform(&self, p: &PointI32) -> PointI32 {
        if self.a.is_empty() || self.b.is_empty() {
            return *p;
        }
        PointI32 {
            x: (p.x - self.a.left) * self.b.width() / self.a.width() + self.b.left,
            y: (p.y - self.a.top) * self.b.height() / self.a.height() + self.b.top,
        }
    }

    fn transform_rect(&self, r: &BoundingRect) -> BoundingRect {
        let a = self.transform(&PointI32 {
            x: r.left,
            y: r.top,
        });
        let b = self.transform(&PointI32 {
            x: r.right,
            y: r.bottom,
        });
        BoundingRect {
            left: a.x,
            top: a.y,
            right: b.x,
            bottom: b.y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangular_transform() {
        assert_eq!(
            RectangularTransform::new_rect_rect(
                BoundingRect::new_x_y_w_h(1, 1, 2, 2),
                BoundingRect::new_x_y_w_h(2, 2, 2, 2)
            )
            .transform(&PointI32 { x: 2, y: 2 }),
            PointI32 { x: 3, y: 3 }
        );

        assert_eq!(
            RectangularTransform::new_rect_rect(
                BoundingRect::new_x_y_w_h(1, 1, 2, 2),
                BoundingRect::new_x_y_w_h(2, 2, 4, 4)
            )
            .transform(&PointI32 { x: 2, y: 2 }),
            PointI32 { x: 4, y: 4 }
        );
    }
}
