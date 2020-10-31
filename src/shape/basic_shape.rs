use crate::{BinaryImage, BoundingRect, PointI32};

/// A conceptual object represented by an image
#[derive(Clone)]
pub struct Shape {
    pub image: BinaryImage,
}

impl Shape {
    pub fn image_boundary(image: &BinaryImage) -> BinaryImage {
        Self::image_boundary_and_position_length(image).0
    }

    /// image boundary with position of top-left pixel and path length
    pub fn image_boundary_and_position_length(
        image: &BinaryImage,
    ) -> (BinaryImage, Option<PointI32>, u32) {
        let mut length = 0;
        let mut boundary = BinaryImage::new_w_h(image.width, image.height);
        let mut first = None;
        for y in 0..image.height as i32 {
            for x in 0..image.width as i32 {
                if   image.get_pixel(x as usize, y as usize) && (
                    !image.get_pixel_safe(x-1, y) ||
                    !image.get_pixel_safe(x+1, y) ||
                    !image.get_pixel_safe(x, y-1) ||
                    !image.get_pixel_safe(x, y+1) ) {
                    first = match first {
                        Some(first) => Some(first),
                        None => Some(PointI32 { x, y }),
                    };
                    boundary.set_pixel(x as usize, y as usize, true);
                    length += 1;
                }
            }
        }
        (boundary, first, length)
    }

    pub fn image_boundary_list(image: &BinaryImage) -> Vec<PointI32> {
        Self::image_boundary_list_transpose(image, false)
    }

    pub fn image_boundary_list_transpose(image: &BinaryImage, transpose: bool) -> Vec<PointI32> {
        let mut boundary = Vec::new();
        for xx in 0..image.width as i32 {
            for y in 0..image.height as i32 {
                for xxx in 0..image.width as i32 {
                    let x = if transpose { xx } else { xxx };
                    if   image.get_pixel(x as usize, y as usize) && (
                        !image.get_pixel_safe(x-1, y) ||
                        !image.get_pixel_safe(x+1, y) ||
                        !image.get_pixel_safe(x, y-1) ||
                        !image.get_pixel_safe(x, y+1) ) {
                        boundary.push(PointI32 { x, y });
                    }
                    if transpose {
                        break;
                    }
                }
            }
            if !transpose {
                break;
            }
        }
        boundary
    }

    pub fn rect(&self) -> BoundingRect {
        BoundingRect {
            left: 0,
            top: 0,
            right: self.image.width as i32,
            bottom: self.image.height as i32,
        }
    }

    pub fn circle(width: usize, height: usize) -> Self {
        let diameter = std::cmp::min(width, height) as i32;
        let radius = diameter / 2;
        let limit = radius + diameter % 2;
        let cx = width as i32 / 2;
        let cy = height as i32 / 2;
        let mut image = BinaryImage::new_w_h(width, height);
        for yy in -radius..radius+1 {
            for xx in -radius..radius+1 {
                if (((xx * xx + yy * yy) as f64).sqrt().round() as i32) < limit {
                    image.set_pixel((cx + xx) as usize, (cy + yy) as usize, true);
                }
            }
        }
        Self {
            image
        }
    }

    pub fn ellipse(width: usize, height: usize) -> Self {
        let rx2 = (width * width / 4) as f64;
        let ry2 = (height * height / 4) as f64;
        let cx = width as i32 / 2;
        let cy = height as i32 / 2;
        let mut image = BinaryImage::new_w_h(width, height);
        for yy in 0..height as i32 {
            for xx in 0..width as i32 {
                let xxx = (xx - cx) as f64;
                let yyy = (yy - cy) as f64;
                if ((xxx * xxx / rx2 + yyy * yyy / ry2) as f64).sqrt() < 1.0 {
                    image.set_pixel(xx as usize, yy as usize, true);
                }
            }
        }
        Self {
            image
        }
    }

    pub fn is_circle(&self) -> bool {
        if self.image.width <= 4 && self.image.height <= 4 {
            return false;
        }
        if std::cmp::max(self.image.width, self.image.height) - 
            std::cmp::min(self.image.width, self.image.height) >
            std::cmp::max(self.image.width, self.image.height) / 4 {
            return false;
        }
        let threshold = self.image.width * self.image.height / 4;
        let diff = self.image.diff(&Self::ellipse(self.image.width, self.image.height).image);
        let clusters = diff.to_clusters(false);
        let mut sum = 0;
        for cluster in clusters.iter() {
            sum += 1 + 3 * cluster.size() - 2 * cluster.boundary().len();
            if sum > threshold {
                return false;
            }
        }
        #[cfg(test)] { println!("sum={}", sum) }
        true
    }
}

impl From<BinaryImage> for Shape {
    fn from(image: BinaryImage) -> Self {
        Self { image }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_circle_3() {
        let image = Shape::circle(3, 3).image;
        assert_eq!(
            image.to_string(),
            "***\n".to_owned() +
            "***\n" +
            "***\n"
        );
    }

    #[test]
    fn shape_circle_5() {
        let image = Shape::circle(5, 5).image;
        assert_eq!(
            image.to_string(),
            "-***-\n".to_owned() +
            "*****\n" +
            "*****\n" +
            "*****\n" +
            "-***-\n"
        );
    }

    #[test]
    fn shape_circle_7() {
        let image = Shape::circle(7, 7).image;
        assert_eq!(
            image.to_string(),
            "--***--\n".to_owned() +
            "-*****-\n" +
            "*******\n" +
            "*******\n" +
            "*******\n" +
            "-*****-\n" +
            "--***--\n"
        );
    }

    #[test]
    fn shape_is_circle() {
        assert!(Shape::from(BinaryImage::from_string(&(
            "-***-\n".to_owned() +
            "*****\n" +
            "*****\n" +
            "*****\n" +
            "-***-\n"
        ))).is_circle());
    }

    #[test]
    fn shape_is_circle_2() {
        assert!(Shape::from(BinaryImage::from_string(&(
            "---*---\n".to_owned() +
            "-*****-\n" +
            "*******\n" +
            "*******\n" +
            "*******\n" +
            "-*****-\n" +
            "---*---\n"
        ))).is_circle());
    }

    #[test]
    fn shape_is_not_circle() {
        assert!(!Shape::from(BinaryImage::from_string(&(
            "*******\n".to_owned() +
            "*******\n" +
            "*******\n" +
            "*******\n" +
            "*******\n" +
            "*******\n" +
            "*******\n"
        ))).is_circle());
    }

    #[test]
    fn shape_is_not_circle_2() {
        assert!(!Shape::from(BinaryImage::from_string(&(
            "*****\n".to_owned() +
            "*****\n" +
            "*****\n" +
            "*****\n" +
            "*****\n"
        ))).is_circle());
    }

    #[test]
    fn shape_ellipse_5_5() {
        let image = Shape::ellipse(5, 5).image;
        assert_eq!(
            image.to_string(),
            "-***-\n".to_owned() +
            "*****\n" +
            "*****\n" +
            "*****\n" +
            "-***-\n"
        );
    }

    #[test]
    fn shape_ellipse_7_5() {
        let image = Shape::ellipse(7, 5).image;
        assert_eq!(
            image.to_string(),
            "--***--\n".to_owned() +
            "*******\n" +
            "*******\n" +
            "*******\n" +
            "--***--\n"
        );
    }
}
