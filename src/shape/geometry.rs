use crate::{BinaryImage, BoundingRect, clusters::Cluster, CompoundPathElement, PathSimplifyMode, PointI32};
use super::rasterizer::rasterize_triangle;

/// A conceptual object represented by an image
#[derive(Clone)]
pub struct Shape {
    pub image: BinaryImage,
}

impl Shape {
    pub fn new(image: BinaryImage) -> Self {
        image.into()
    }

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

    /// Return coordinates of the points on the shape boundary
    ///
    /// transpose = false: x first, then y
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

    pub fn rounded_rect(rect: BoundingRect, radius: i32) -> Self {
        let width  = rect.width()  as usize;
        let height = rect.height() as usize;
        let w = width  as f64;
        let h = height as f64;
        let r = (radius as f64).max(0.0).min(w.min(h) / 2.0);

        let mut image = BinaryImage::new_w_h(width, height);
        for yy in 0..height {
            for xx in 0..width {
                let x = xx as f64 + 0.5;
                let y = yy as f64 + 0.5;
                let nx = x.max(r).min(w - r);
                let ny = y.max(r).min(h - r);
                let dx = x - nx;
                let dy = y - ny;
                if dx * dx + dy * dy <= r * r {
                    image.set_pixel(xx, yy, true);
                }
            }
        }
        Self { image }
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

    pub fn is_isosceles_triangle(&self) -> bool {
        if self.image.width < 3 && self.image.height < 3 {
            return false;
        }
        let area = self.image.width * self.image.height;
        let threshold = area / 3;
        let mut reference = BinaryImage::new_w_h(self.image.width, self.image.height);
        rasterize_triangle(&[
            PointI32::new(0, self.image.height as i32),
            PointI32::new(self.image.width as i32 / 2, 0),
            PointI32::new(self.image.width as i32, self.image.height as i32),
        ], &mut reference);
        let diff = self.image.diff(&reference);
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

    #[test]
    fn shape_is_isosceles_triangle() {
        assert!(!Shape::from(BinaryImage::from_string(&(
            "***\n".to_owned() +
            "***\n" +
            "***\n"
        ))).is_isosceles_triangle());
        assert!(Shape::from(BinaryImage::from_string(&(
            "-*-\n".to_owned() +
            "***\n" +
            "***\n"
        ))).is_isosceles_triangle());
        let shape = Shape::from(BinaryImage::from_string(&(
            "--*--\n".to_owned() +
            "-***-\n" +
            "-***-\n" +
            "*****\n"
        )));
        assert!(shape.is_isosceles_triangle());
        assert!(!shape.is_circle());
        let shape = Shape::from(BinaryImage::from_string(&(
            "-***-\n".to_owned() +
            "-***-\n" +
            "*****\n" +
            "*****\n"
        )));
        assert!(!shape.is_isosceles_triangle());
        assert!(!shape.is_circle());
        assert!(!shape.is_quadrilateral());
    }

    #[test]
    fn rounded_rect_r0_is_full_rect() {
        let rect = BoundingRect::new_x_y_w_h(0, 0, 6, 4);
        let shape = Shape::rounded_rect(rect, 0);
        // r=0 => plain filled rectangle, all pixels set
        assert!(shape.image.pixels.iter().all(|p| p));
        assert_eq!(shape.image.width, 6);
        assert_eq!(shape.image.height, 4);
    }

    #[test]
    fn rounded_rect_degenerate_circle() {
        // 10x10 with r=5: corners should be empty, centre filled
        let rect = BoundingRect::new_x_y_w_h(0, 0, 10, 10);
        let rr = Shape::rounded_rect(rect, 5);
        // corners cut
        assert!(!rr.image.get_pixel(0, 0));
        assert!(!rr.image.get_pixel(9, 0));
        assert!(!rr.image.get_pixel(0, 9));
        assert!(!rr.image.get_pixel(9, 9));
        // centre filled
        assert!(rr.image.get_pixel(5, 5));
        // left/right midpoints on boundary
        assert!(rr.image.get_pixel(0, 4));
        assert!(rr.image.get_pixel(9, 5));
    }

    #[test]
    fn rounded_rect_radius_clamped() {
        // Huge radius should clamp to min(w,h)/2 == 5, same as r=5
        let rect = BoundingRect::new_x_y_w_h(0, 0, 10, 10);
        let clamped = Shape::rounded_rect(rect, 9999);
        let explicit = Shape::rounded_rect(rect, 5);
        assert_eq!(
            clamped.image.pixels.iter().collect::<Vec<_>>(),
            explicit.image.pixels.iter().collect::<Vec<_>>()
        );
    }

    #[test]
    fn rounded_rect_corners_cut() {
        // 6x4 with r=2: top-left corner pixel (0,0) should be empty
        let rect = BoundingRect::new_x_y_w_h(0, 0, 6, 4);
        let shape = Shape::rounded_rect(rect, 2);
        assert!(!shape.image.get_pixel(0, 0));
        assert!(!shape.image.get_pixel(5, 0));
        assert!(!shape.image.get_pixel(0, 3));
        assert!(!shape.image.get_pixel(5, 3));
        // centre pixel should be filled
        assert!(shape.image.get_pixel(3, 2));
    }

    #[test]
    fn rounded_rect_string_format() {
        let rect = BoundingRect::new_x_y_w_h(0, 0, 11, 9);
        let shape = Shape::rounded_rect(rect, 4);
        assert_eq!(
            shape.image.to_string(),
            "--*******--\n".to_owned() +
            "-*********-\n" +
            "***********\n" +
            "***********\n" +
            "***********\n" +
            "***********\n" +
            "***********\n" +
            "-*********-\n" +
            "--*******--\n"
        );

        let rect = BoundingRect::new_x_y_w_h(0, 0, 20, 9);
        let shape = Shape::rounded_rect(rect, 4);
        assert_eq!(
            shape.image.to_string(),
            "--****************--\n".to_owned() +
            "-******************-\n" +
            "********************\n" +
            "********************\n" +
            "********************\n" +
            "********************\n" +
            "********************\n" +
            "-******************-\n" +
            "--****************--\n"
        );
    }
}
