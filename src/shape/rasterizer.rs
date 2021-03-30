use crate::{BinaryImage, PointI32};

/// Bresenham's line algorithm; returns an iterator of all points. 
/// Adapted from https://github.com/madbence/node-bresenham
pub fn bresenham(p0: PointI32, p1: PointI32) -> BresenhamIterator {
    let dx = p1.x - p0.x;
    let dy = p1.y - p0.y;
    let adx = dx.abs();
    let ady = dy.abs();
    let eps = 0;
    let sx = if dx > 0 { 1 } else { -1 };
    let sy = if dy > 0 { 1 } else { -1 };
    BresenhamIterator { x: p0.x, y: p0.y, sx, sy, eps, adx, ady, p: p1, horizontal: adx > ady }
}

pub struct BresenhamIterator {
    x: i32,
    y: i32,
    sx: i32,
    sy: i32,
    eps: i32,
    adx: i32,
    ady: i32,
    p: PointI32,
    horizontal: bool,
}

impl Iterator for BresenhamIterator {
    type Item = PointI32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.horizontal {
            if if self.sx < 0 { self.x >= self.p.x } else { self.x <= self.p.x } {
                let pp = PointI32::new(self.x, self.y);
                self.eps += self.ady;
                if (self.eps << 1) >= self.adx {
                    self.y += self.sy;
                    self.eps -= self.adx;
                }
                self.x += self.sx;
                return Some(pp);
            }
            None
        } else {
            if if self.sy < 0 { self.y >= self.p.y } else { self.y <= self.p.y } {
                let pp = PointI32::new(self.x, self.y);
                self.eps += self.adx;
                if (self.eps << 1) >= self.ady {
                    self.x += self.sx;
                    self.eps -= self.ady;
                }
                self.y += self.sy;
                return Some(pp);
            }
            None
        }
    }
}

/// Walk through all points of this triangle via iterator.
/// Adapted from https://github.com/rastapasta/points-in-triangle
pub fn walk_triangle(triangle: &[PointI32; 3]) -> TriangleRasterizer {
    // Get all points on the triangles' sides ...
    let mut points: Vec<PointI32> = 
        bresenham(triangle[1], triangle[2])
        .chain(&mut bresenham(triangle[0], triangle[2]))
        .chain(&mut bresenham(triangle[0], triangle[1]))
        .collect();

    // ... and sort them by y, x
    points.sort_by(|a, b| if a.y == b.y { a.x.cmp(&b.x) } else { a.y.cmp(&b.y) });

    TriangleRasterizer {
        points,
        counter: 0,
        span: None,
    }
}

/// Rasterizes triangle onto a [`BinaryImage`]
pub fn rasterize_triangle(triangle: &[PointI32; 3], image: &mut BinaryImage) {
    for p in walk_triangle(triangle) {
        image.set_pixel_safe(p.x, p.y, true);
    }
}

pub struct TriangleRasterizer {
    points: Vec<PointI32>,
    counter: usize,
    span: Option<SpanRasterizer>,
}

impl Iterator for TriangleRasterizer {
    type Item = PointI32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.span.is_some() {
            if let Some(p) = self.span.as_mut().unwrap().next() {
                return Some(p);
            } else {
                self.span = None;
                self.counter += 1;
                // exit the nested for loop
            }
        }
        if self.counter >= self.points.len() {
            return None;
        }
        let point = self.points[self.counter];
        if self.counter + 1 < self.points.len() {
            let next = self.points[self.counter + 1];

            if point.y == next.y {
                self.span = Some(SpanRasterizer {
                    x: point.x,
                    x1: next.x,
                    y: point.y,
                });
                // enter the nested for loop to yield the next point
                return self.next();
            }
        }
        self.counter += 1;
        Some(PointI32::new(point.x, point.y))
    }
}

struct SpanRasterizer {
    x: i32,
    x1: i32,
    y: i32,
}

impl Iterator for SpanRasterizer {
    type Item = PointI32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.x < self.x1 {
            let p = PointI32::new(self.x, self.y);
            self.x += 1;
            Some(p)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triangle_1() {
        assert_eq!(
            walk_triangle(&[PointI32::new(0, 0), PointI32::new(4, 4), PointI32::new(4, 0)])
                .collect::<Vec<_>>(),
            vec![
                PointI32::new(0, 0),
                PointI32::new(1, 0),
                PointI32::new(2, 0),
                PointI32::new(3, 0),
                PointI32::new(4, 0),
                PointI32::new(1, 1),
                PointI32::new(2, 1),
                PointI32::new(3, 1),
                PointI32::new(4, 1),
                PointI32::new(2, 2),
                PointI32::new(3, 2),
                PointI32::new(4, 2),
                PointI32::new(3, 3),
                PointI32::new(4, 3),
                PointI32::new(4, 4),
            ]
        );
    }

    #[test]
    fn test_triangle_2() {
        assert_eq!(
            walk_triangle(&[PointI32::new(1, 1), PointI32::new(8, 4), PointI32::new(4, 8)])
                .collect::<Vec<_>>(),
            vec![
                PointI32::new(1, 1),
                PointI32::new(2, 1),
                PointI32::new(1, 2),
                PointI32::new(2, 2),
                PointI32::new(3, 2),
                PointI32::new(4, 2),
                PointI32::new(2, 3),
                PointI32::new(3, 3),
                PointI32::new(4, 3),
                PointI32::new(5, 3),
                PointI32::new(6, 3),
                PointI32::new(2, 4),
                PointI32::new(3, 4),
                PointI32::new(4, 4),
                PointI32::new(5, 4),
                PointI32::new(6, 4),
                PointI32::new(7, 4),
                PointI32::new(8, 4),
                PointI32::new(3, 5),
                PointI32::new(4, 5),
                PointI32::new(5, 5),
                PointI32::new(6, 5),
                PointI32::new(7, 5),
                PointI32::new(3, 6),
                PointI32::new(4, 6),
                PointI32::new(5, 6),
                PointI32::new(6, 6),
                PointI32::new(4, 7),
                PointI32::new(5, 7),
                PointI32::new(4, 8),
            ]
        );
    }

    #[test]
    fn rasterize_triangle_1() {
        let mut image = BinaryImage::new_w_h(5, 5);
        rasterize_triangle(&[PointI32::new(0, 0), PointI32::new(4, 4), PointI32::new(4, 0)], &mut image);
        assert_eq!(image.to_string(),
            "*****\n".to_owned() +
            "-****\n" +
            "--***\n" +
            "---**\n" +
            "----*\n"
        );
    }

    #[test]
    fn rasterize_triangle_2() {
        let mut image = BinaryImage::new_w_h(5, 5);
        rasterize_triangle(&[PointI32::new(0, 0), PointI32::new(0, 4), PointI32::new(4, 4)], &mut image);
        assert_eq!(image.to_string(),
            "*----\n".to_owned() +
            "**---\n" +
            "***--\n" +
            "****-\n" +
            "*****\n"
        );
    }

    #[test]
    fn rasterize_triangle_3() {
        let mut image = BinaryImage::new_w_h(6, 11);
        rasterize_triangle(&[PointI32::new(0, 0), PointI32::new(5, 5), PointI32::new(0, 10)], &mut image);
        assert_eq!(image.to_string(),
            "*-----\n".to_owned() +
            "**----\n" +
            "***---\n" +
            "****--\n" +
            "*****-\n" +
            "******\n" +
            "*****-\n" +
            "****--\n" +
            "***---\n" +
            "**----\n" +
            "*-----\n"
        );
    }

}