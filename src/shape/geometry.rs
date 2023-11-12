use crate::{BinaryImage, BoundingRect, clusters::Cluster, CompoundPathElement, PathSimplifyMode, PointI32};
use super::rasterizer::rasterize_triangle;

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
        if std::cmp::max(self.image.width, self.image.height) - 
            std::cmp::min(self.image.width, self.image.height) >
            std::cmp::max(self.image.width, self.image.height) / 4 {
            return false;
        }
        self.is_ellipse()
    }

    pub fn is_ellipse(&self) -> bool {
        if self.image.width <= 4 && self.image.height <= 4 {
            return false;
        }
        let corners = 
            self.image.get_pixel(0, 0) as i32 +
            self.image.get_pixel(self.image.width - 1, 0) as i32 +
            self.image.get_pixel(self.image.width - 1, self.image.height - 1) as i32 +
            self.image.get_pixel(0, self.image.height - 1) as i32;
        if corners > 1 {
            return false;
        }
        let area = self.image.width * self.image.height;
        let threshold = area / 2;
        let diff = self.image.diff(&Self::ellipse(self.image.width, self.image.height).image);
        Self::clustered_diff(&diff, threshold)
    }

    fn clustered_diff(diff: &BinaryImage, threshold: usize) -> bool {
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

    pub fn is_quadrilateral(&self) -> bool {
        let mut paths = Cluster::image_to_compound_path(
            &PointI32::default(),
            &self.image,
            PathSimplifyMode::None,
            0.0,
            0.0,
            0,
            0.0
        );
        paths.paths.truncate(1);
        let paths = paths.reduce(std::cmp::min(self.image.width, self.image.height) as f64);
        // the path is reduced to a quadrilateral bound by the north most, east most, south most and west most point
        let mut reduced = BinaryImage::new_w_h(self.image.width, self.image.height);
        let path = &match &paths.paths[0] {
            CompoundPathElement::PathI32(path) => path,
            _ => unreachable!(),
        }.path;
        let p0 = PointI32::new(path[0].x-1, path[0].y);
        let p2 = PointI32::new(path[2].x, path[2].y-1);
        rasterize_triangle(&[p0, PointI32::new(path[1].x-1, path[1].y-1), p2], &mut reduced);
        rasterize_triangle(&[p0, p2, PointI32::new(path[3].x, path[1].y-1)], &mut reduced);
        // panic!("\n{}", reduced.to_string());
        let diff = self.image.diff(&reduced);
        let threshold = self.image.width * self.image.height / 6;
        Self::clustered_diff(&diff, threshold)
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
    fn shape_is_not_circle_0() {
        assert!(!Shape::from(BinaryImage::from_string(&(
            "*\n".to_owned()
        ))).is_circle());

        assert!(!Shape::from(BinaryImage::from_string(&(
            "**\n".to_owned() +
            "**\n"
        ))).is_circle());

        assert!(!Shape::from(BinaryImage::from_string(&(
            "***\n".to_owned() +
            "***\n" +
            "***\n"
        ))).is_circle());
    }

    #[test]
    fn shape_is_not_circle_1() {
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

    #[test]
    fn is_quadrilateral_test_1() {
        assert!(!Shape::from(BinaryImage::from_string(&(
            "--***--\n".to_owned() +
            "-*****-\n" +
            "*******\n" +
            "*******\n" +
            "*******\n" +
            "-*****-\n" +
            "--***--\n"
        ))).is_quadrilateral());
    }

    #[test]
    fn is_quadrilateral_test_2() {
        assert!(Shape::from(BinaryImage::from_string(&(
            "----*----\n".to_owned() +
            "---***---\n" +
            "--*****--\n" +
            "-*******-\n" +
            "*********\n" +
            "*********\n" +
            "*********\n" +
            "-*******-\n" +
            "--*****--\n" +
            "---***---\n" +
            "----*----\n"
        ))).is_quadrilateral());
    }

    #[test]
    fn is_quadrilateral_test_3() {
        assert!(!Shape::from(BinaryImage::from_string(&(
            "----*----\n".to_owned() +
            "--*****--\n" +
            "-*******-\n" +
            "-*******-\n" +
            "*********\n" +
            "*********\n" +
            "*********\n" +
            "-*******-\n" +
            "-*******-\n" +
            "--*****--\n" +
            "----*----\n"
        ))).is_quadrilateral());
    }
}
