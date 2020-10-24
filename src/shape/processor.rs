use crate::{BinaryImage, PointI32, Shape};

/// Morphologically transforms a shape into another
#[derive(Default)]
pub struct AverageShape {
    from: BinaryImage,
    to: BinaryImage,
    morph: BinaryImage,
    boundary: Vec<PointI32>,
    counter: u32,
    stop_at: u32,
}

impl AverageShape {
    pub fn average_shape_between(a: BinaryImage, b: BinaryImage) -> Option<BinaryImage> {
        let mut processor = Self::new();
        if !processor.init(a, b) {
            return None;
        }
        while !processor.tick() {}
        Some(processor.result())
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn init_circle_square(&mut self) {
        let size = 128;
        let ss: i32 = 50;
        let c = (size >> 1) as i32;

        let mut circle = BinaryImage::new_w_h(size, size);
        for yy in -ss..ss {
            for xx in -ss..ss {
                if (((xx * xx + yy * yy) as f64).sqrt() as i32) < ss {
                    circle.set_pixel((c + xx) as usize, (c + yy) as usize, true);
                }
            }
        }

        let mut square = BinaryImage::new_w_h(size, size);
        for yy in -ss..ss {
            for xx in -ss..ss {
                square.set_pixel((c + xx) as usize, (c + yy) as usize, true);
            }
        }

        self.init(circle, square);
    }

    /// caution: if init returned false, tick will never exit
    pub fn init(&mut self, from: BinaryImage, to: BinaryImage) -> bool {
        self.from = from.intersect(&to);
        self.to = from.union(&to);
        let from_clusters = self.from.to_clusters(false);
        let to_clusters = self.to.to_clusters(false);
        self.reset();
        from_clusters.len() == to_clusters.len()
    }

    pub fn reset(&mut self) {
        self.morph = self.from.clone();
        self.boundary = Shape::image_boundary_list(&self.morph);
        self.counter = 0;
    }

    pub fn tick(&mut self) -> bool {
        let mut diff = 0;
        let mut new_boundary = Vec::new();
        for p in self.boundary.iter() {
            let mut set = false;
            for i in 0..4 {
                let (xx, yy) = match i {
                    0 => (p.x, p.y - 1),
                    1 => (p.x + 1, p.y),
                    2 => (p.x, p.y + 1),
                    3 => (p.x - 1, p.y),
                    _ => panic!("impossible"),
                };
                if !self.morph.get_pixel_safe(xx, yy) && self.to.get_pixel_safe(xx, yy) {
                    self.morph.set_pixel_safe(xx, yy, true);
                    diff += 1;
                    set = true;
                    new_boundary.push(PointI32 { x: xx, y: yy });
                }
            }
            if !set {
                new_boundary.push(*p);
            }
        }
        self.boundary = new_boundary;
        self.counter += 1;
        if self.counter == self.stop_at {
            return true;
        }
        diff == 0
    }

    pub fn result(&mut self) -> BinaryImage {
        self.stop_at = self.counter >> 1;
        self.reset();
        while !self.tick() {}
        std::mem::take(&mut self.morph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn average_shape() {
        assert_eq!(
            AverageShape::average_shape_between(
                BinaryImage::from_string(&("***\n".to_owned() +
                                           "***\n" +
                                           "***\n")),
                BinaryImage::from_string(&("---\n".to_owned() +
                                           "-*-\n" +
                                           "---\n")),
            )
            .unwrap()
            .to_string(),
            "-*-\n".to_owned() +
            "***\n" +
            "-*-\n"
        );
    }
}
