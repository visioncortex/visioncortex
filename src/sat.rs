use crate::{ColorImage, PointI32};

/// A data structure to efficiently compute summed pixel values over regions in an image (repeatedly).
pub struct SummedAreaTable {
    pub sums: Vec<u32>,
    pub width: usize,
    pub height: usize,
}

impl SummedAreaTable {
    /// Creates an SAT of the same size of image, where each entry (x,y) is the sum of pixel
    /// values of the block of pixels with bottom right corner at (x,y) in image.
    ///
    /// This construction takes 1 pass through the pixels in image.
    pub fn from_color_image(image: &ColorImage) -> Self {
        let (width, height) = (image.width, image.height);

        let mut sums = vec![0; width * height];
        let get_sum = |x: i32, y: i32, sums: &Vec<u32>| {
            if x >= 0 && y >= 0 {
                sums[(y * width as i32 + x) as usize]
            } else {
                0
            }
        };

        // Closure to get pixel intensity from image
        let get_val = |x: usize, y: usize| {
            let c = image.get_pixel(x, y);
            (c.r as u32 + c.g as u32 + c.b as u32) / 3
        };

        // Fill the sums starting from the top-left corner
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                let up_left = get_sum(x-1, y-1, &sums);
                let up = get_sum(x, y-1, &sums);
                let left = get_sum(x-1, y, &sums);
                let curr = get_val(x as usize, y as usize);
                sums[(y * width as i32 + x) as usize] = up + left + curr - up_left;
            }
        }

        Self {
            sums,
            width,
            height
        }
    }

    /// Returns the entry in the SAT.
    ///
    /// If the input point is out of boundary, this function returns 0.
    ///
    /// This is only to facilitate the implementation of other functions; avoid calling this function directly.
    pub fn get_bot_right_sum(&self, x: i32, y: i32) -> u32 {
        if x >= 0 && y >= 0 && x < self.width as i32 && y < self.height as i32 {
            self.sums[(y * self.width as i32 + x) as usize]
        } else {
            0
        }
    }

    fn correct_top_left_bot_right(top_left: &PointI32, bot_right: &PointI32) -> bool {
        top_left.x <= bot_right.x && top_left.y <= bot_right.y
    }

    /// Computes the sum of pixel values in the specified region in O(1) time.
    pub fn get_region_sum_top_left_bot_right(&self, top_left: PointI32, bot_right: PointI32) -> u32 {
        if !Self::correct_top_left_bot_right(&top_left, &bot_right) {
            panic!("Top left and bottom right points are invalid.")
        }
        let left_region = self.get_bot_right_sum(top_left.x-1, bot_right.y);
        let up_region = self.get_bot_right_sum(bot_right.x, top_left.y-1);
        let overlap = self.get_bot_right_sum(top_left.x-1, top_left.y-1);
        let total = self.get_bot_right_sum(bot_right.x, bot_right.y);

        total + overlap - left_region - up_region
    }

    /// Computes the sum of pixel values in the specified region in O(1) time.
    pub fn get_region_sum_x_y_w_h(&self, x: usize, y: usize, w: usize, h: usize) -> u32 {
        let top_left = PointI32::new(x as i32, y as i32);
        let bot_right = PointI32::new((x+w-1) as i32, (y+h-1) as i32);
        self.get_region_sum_top_left_bot_right(top_left, bot_right)
    }

    /// Computes the mean of pixel values in the specified region in O(1) time.
    pub fn get_region_mean_top_left_bot_right(&self, top_left: PointI32, bot_right: PointI32) -> f64 {
        let w = bot_right.x - top_left.x + 1;
        let h = bot_right.y - top_left.y + 1;
        self.get_region_mean_x_y_w_h(top_left.x as usize, top_left.y as usize, w as usize, h as usize)
    }

    /// Computes the mean of pixel values in the specified region in O(1) time.
    pub fn get_region_mean_x_y_w_h(&self, x: usize, y: usize, w: usize, h: usize) -> f64 {
        self.get_region_sum_x_y_w_h(x, y, w, h) as f64 / (w*h) as f64
    }
}

#[cfg(test)]
mod tests {
    use crate::Color;

    use super::*;

    fn create_color_image_helper(width: usize, height: usize, pixels: Vec<u8>) -> ColorImage {
        let mut image = ColorImage::new_w_h(width, height);
        for (i, &val) in pixels.iter().enumerate() {
            image.set_pixel_at(i, &Color::new(val, val, val));
        }
        image
    }

    #[test]
    fn sat_construct() {
        // Example from wikipedia
        let pixels = vec![
            31, 2, 4, 33, 5, 36,
            12, 26, 9, 10, 29, 25,
            13, 17, 21, 22, 20, 18,
            24, 23, 15, 16, 14, 19,
            30, 8, 28, 27, 11, 7,
            1, 35, 34, 3, 32, 6,
        ];
        let image = create_color_image_helper(6, 6, pixels);
        let sat = SummedAreaTable::from_color_image(&image);
        assert_eq!(sat.get_bot_right_sum(0, 0), 31);
        assert_eq!(sat.get_bot_right_sum(1, 1), 71);
        assert_eq!(sat.get_bot_right_sum(1, 2), 101);
        assert_eq!(sat.get_bot_right_sum(5, 0), 111);
        assert_eq!(sat.get_bot_right_sum(0, 5), 111);
        assert_eq!(sat.get_bot_right_sum(5, 5), 666);
        assert_eq!(sat.get_bot_right_sum(4, 4), 450);
        assert_eq!(sat.get_bot_right_sum(1, 4), 186);
        assert_eq!(sat.get_bot_right_sum(4, 2), 254);
    }

    #[test]
    fn sat_region_sum() {
        // Example from wikipedia
        let pixels = vec![
            31, 2, 4, 33, 5, 36,
            12, 26, 9, 10, 29, 25,
            13, 17, 21, 22, 20, 18,
            24, 23, 15, 16, 14, 19,
            30, 8, 28, 27, 11, 7,
            1, 35, 34, 3, 32, 6,
        ];
        let image = create_color_image_helper(6, 6, pixels);
        let sat = SummedAreaTable::from_color_image(&image);
        assert_eq!(sat.get_region_sum_top_left_bot_right(PointI32::new(2, 3), PointI32::new(4, 4)), 111);
        assert_eq!(sat.get_region_sum_x_y_w_h(2, 3, 3, 2), 111);
        assert_eq!(sat.get_region_sum_x_y_w_h(0, 0, 6, 6), 666);
        assert_eq!(sat.get_region_sum_x_y_w_h(0, 0, 1, 6), 111);
        assert_eq!(sat.get_region_sum_x_y_w_h(0, 0, 6, 1), 111);
        assert_eq!(sat.get_region_sum_x_y_w_h(2, 4, 3, 2), 135);
        assert_eq!(sat.get_region_sum_x_y_w_h(1, 2, 3, 4), 249);
    }

    #[test]
    fn sat_region_mean() {
        // Example from wikipedia
        let pixels = vec![
            31, 2, 4, 33, 5, 36,
            12, 26, 9, 10, 29, 25,
            13, 17, 21, 22, 20, 18,
            24, 23, 15, 16, 14, 19,
            30, 8, 28, 27, 11, 7,
            1, 35, 34, 3, 32, 6,
        ];
        let image = create_color_image_helper(6, 6, pixels);
        let sat = SummedAreaTable::from_color_image(&image);
        assert!(sat.get_region_mean_top_left_bot_right(PointI32::new(2, 3), PointI32::new(4, 4)) - (111.0 / 6.0) < 1e-6);
        assert!(sat.get_region_mean_x_y_w_h(2, 3, 3, 2) - (111.0 / 6.0) < 1e-6);
        assert!(sat.get_region_mean_x_y_w_h(0, 0, 6, 6) - (666.0 / 36.0) < 1e-6);
        assert!(sat.get_region_mean_x_y_w_h(0, 0, 1, 6) - (111.0 / 6.0) < 1e-6);
        assert!(sat.get_region_mean_x_y_w_h(0, 0, 6, 1) - (111.0 / 6.0) < 1e-6);
        assert!(sat.get_region_mean_x_y_w_h(2, 4, 3, 2) - (135.0 / 6.0) < 1e-6);
        assert!(sat.get_region_mean_x_y_w_h(1, 2, 3, 4) - (249.0 / 12.0) < 1e-6);
    }
}