use crate::{BinaryImage, BoundingRect};

/// For sampling and resizing binary images
pub struct Sampler {
    pub image: BinaryImage,
}

impl Sampler {
    pub fn new(image: &BinaryImage) -> Sampler {
        let size = std::cmp::max(image.width, image.height);
        Self::new_with_size(image, size)
    }

    pub fn new_with_size(image: &BinaryImage, sampler_size: usize) -> Sampler {
        Self::new_with_size_crop(image, sampler_size, Default::default())
    }

    pub fn new_with_size_crop(
        image: &BinaryImage,
        sampler_size: usize,
        crop: BoundingRect,
    ) -> Sampler {
        let new_image;
        assert_eq!(crop.width(), crop.height());
        if crop.is_empty() && image.width == image.height && image.width == sampler_size {
            new_image = image.clone();
        } else if !crop.is_empty()
            && crop.width() as usize == sampler_size
            && crop.height() as usize == sampler_size
        {
            new_image = image.crop_with_rect(crop);
        } else {
            new_image = Self::resample_square_image(&image, crop, sampler_size);
        }
        Sampler { image: new_image }
    }

    /// Resize an image of any size into a square image while keeping the aspect ratio of content.
    /// Would empty fill expanded area.
    pub fn resample_square_image(
        image: &BinaryImage,
        crop: BoundingRect,
        new_size: usize,
    ) -> BinaryImage {
        let mut new_image = BinaryImage::new_w_h(new_size, new_size);
        let new_size = new_size as i32;
        let crop = if !crop.is_empty() {
            crop
        } else {
            BoundingRect::new_x_y_w_h(0, 0, image.width as i32, image.height as i32)
        };
        let image_size = std::cmp::max(crop.width(), crop.height()) as i32;
        let ox = (image_size - crop.width() as i32) >> 1;
        let oy = (image_size - crop.height() as i32) >> 1;
        for y in 0..new_size {
            for x in 0..new_size {
                let xx = x * image_size / new_size - ox + crop.left;
                let yy = y * image_size / new_size - oy + crop.top;
                new_image.set_pixel(x as usize, y as usize, image.get_pixel_safe(xx, yy));
            }
        }
        new_image
    }

    pub fn resample_image(image: &BinaryImage, new_width: usize, new_height: usize) -> BinaryImage {
        Self::resample_image_with_crop(image, Default::default(), new_width, new_height)
    }

    pub fn resample_image_with_crop(
        image: &BinaryImage,
        crop: BoundingRect,
        new_width: usize,
        new_height: usize,
    ) -> BinaryImage {
        let mut new_image = BinaryImage::new_w_h(new_width, new_height);
        Self::resample_image_with_crop_to_image(
            image,
            crop,
            &mut new_image,
            BoundingRect::new_x_y_w_h(0, 0, new_width as i32, new_height as i32),
        );
        new_image
    }

    pub fn resample_image_with_crop_to_image(
        src: &BinaryImage,
        src_rect: BoundingRect,
        dst: &mut BinaryImage,
        dst_rect: BoundingRect,
    ) {
        Self::resample_image_with_crop_to_image_overlay(src, src_rect, dst, dst_rect, false);
    }

    pub fn resample_image_with_crop_to_image_overlay(
        src: &BinaryImage,
        src_rect: BoundingRect,
        dst: &mut BinaryImage,
        dst_rect: BoundingRect,
        overlay: bool,
    ) {
        let src_rect = if !src_rect.is_empty() {
            src_rect
        } else {
            BoundingRect::new_x_y_w_h(0, 0, src.width as i32, src.height as i32)
        };
        for y in 0..dst_rect.height() {
            for x in 0..dst_rect.width() {
                let s = 1;
                let xx =
                    s * x as i32 * src_rect.width() / dst_rect.width() as i32
                    + s * src_rect.left;
                let yy =
                    s * y as i32 * src_rect.height() / dst_rect.height() as i32
                    + s * src_rect.top;
                let pixel = src.get_pixel_safe(xx / s, yy / s);
                if !overlay || pixel {
                    // overlay: set pixel only if pixel is true
                    // otherwise: set pixel anyway
                    dst.set_pixel(
                        (dst_rect.left + x) as usize,
                        (dst_rect.top + y) as usize,
                        pixel,
                    );
                }
            }
        }
    }
}

impl Sampler {
    pub fn size(&self) -> usize {
        self.image.width
    }

    pub fn bounding_rect(&self) -> BoundingRect {
        self.image.bounding_rect()
    }

    pub fn sample(&self, left: usize, top: usize, right: usize, bottom: usize) -> usize {
        let mut count = 0;
        for y in top..bottom {
            for x in left..right {
                if self.image.get_pixel(x, y) {
                    count += 1;
                }
            }
        }
        count
    }
}

#[allow(dead_code)]
fn is_pow_of_four(n: usize) -> bool {
    (1 << (2 * pow_of_four(n))) == n
}

fn pow_of_four(mut n: usize) -> usize {
    let mut pow_of_4 = 0;
    while n > 3 {
        n >>= 2;
        pow_of_4 += 1;
    }
    pow_of_4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sampler_2() {
        let size = 2;
        let mut image = BinaryImage::new_w_h(size, size);
        image.set_pixel(0, 0, true);
        image.set_pixel(1, 1, true);
        let sampler = Sampler::new_with_size(&image, size);
        assert_eq!(sampler.sample(0, 0, 1, 1), 1);
        assert_eq!(sampler.sample(0, 1, 1, 2), 0);
        assert_eq!(sampler.sample(1, 0, 2, 1), 0);
        assert_eq!(sampler.sample(1, 1, 2, 2), 1);
    }

    #[test]
    /// cropping a 4x4 image to a 2x2 image same as above
    fn sampler_crop() {
        let size = 4;
        let mut image = BinaryImage::new_w_h(size,size);
        image.set_pixel(1,1,true);
        image.set_pixel(2,2,true);
        let sampler = Sampler::new_with_size_crop(&image, 2, BoundingRect {
            left: 1, top: 1, right: 3, bottom: 3
        });
        assert_eq!(sampler.image.width, 2);
        assert_eq!(sampler.size(), 2);
        assert_eq!(sampler.sample(0, 0, 1, 1), 1);
        assert_eq!(sampler.sample(0, 1, 1, 2), 0);
        assert_eq!(sampler.sample(1, 0, 2, 1), 0);
        assert_eq!(sampler.sample(1, 1, 2, 2), 1);
    }

    #[test]
    fn sampler_4() {
        let size = 4;
        let mut image = BinaryImage::new_w_h(size, size);
        image.set_pixel(0, 0, true);
        image.set_pixel(1, 1, true);
        let sampler = Sampler::new_with_size(&image, size);
        assert_eq!(sampler.sample(0, 0, size, size), 2); // whole
        assert_eq!(sampler.sample(0, 0, size / 2, size / 2), 2);
        assert_eq!(sampler.sample(size / 2, size / 2, size, size), 0);
        assert_eq!(sampler.sample(0,0,1,1), 1);
        assert_eq!(sampler.sample(1,1,size/2,size/2), 1);
        let rect = sampler.bounding_rect();
        assert_eq!(rect, BoundingRect { left: 0, right: 2, top: 0, bottom: 2 });
        assert_eq!(rect.width(), 2);
        assert_eq!(rect.height(), 2);
    }

    #[test]
    fn sampler_upsize() {
        let mut image = BinaryImage::new_w_h(4, 4);
        image.set_pixel(1, 1, true);
        println!("image:\n{}", image.to_string());
        let sampler = Sampler::new_with_size(&image, 8);
        assert_eq!(sampler.image.width, 8);
        println!("upsized:\n{}", sampler.image.to_string());
        assert_eq!(sampler.sample(2, 2, 4, 4), 4);
        assert_eq!(sampler.sample(0, 0, 8, 8), 4);
        assert_eq!(sampler.sample(4, 4, 8, 8), 0);
    }

    #[test]
    fn sampler_crop_upsize() {
        let mut image = BinaryImage::new_w_h(8,8);
        image.set_pixel(2,2,true);
        println!("image:\n{}", image.to_string());
        let sampler = Sampler::new_with_size_crop(&image, 8, BoundingRect {
            left: 1, top: 1, right: 5, bottom: 5
        });
        assert_eq!(sampler.image.width, 8);
        println!("cropped & upsized:\n{}", sampler.image.to_string());
        assert_eq!(sampler.sample(2, 2, 4, 4), 4);
        assert_eq!(sampler.sample(0, 0, 8, 8), 4);
        assert_eq!(sampler.sample(4, 4, 8, 8), 0);
    }

    #[test]
    fn sampler_upsize_non_exact() {
        let mut image = BinaryImage::new_w_h(6, 6);
        image.set_pixel(1, 1, true);
        let sampler = Sampler::new_with_size(&image, 8);
        assert_eq!(sampler.sample(0, 0, 8, 8), 1);
    }

    #[test]
    fn resample_image_2x2_to_4x2() {
        let image = BinaryImage::from_string(&(
            "*-\n".to_owned()+
            "-*\n"));
        assert_eq!(
            Sampler::resample_image(&image, 4, 2).to_string(),
            BinaryImage::from_string(&(
                "**--\n".to_owned()+
                "--**\n")).to_string()
        );
    }

    #[test]
    fn resample_image_2x2_to_4x2_crop() {
        let image = BinaryImage::from_string(&(
            "--\n".to_owned()+
            "*-\n"+
            "-*\n"));
        let mut new_image = BinaryImage::new_w_h(4, 4);
        Sampler::resample_image_with_crop_to_image(
            &image, BoundingRect::new_x_y_w_h(0, 1, 2, 2),
            &mut new_image, BoundingRect::new_x_y_w_h(0, 1, 4, 2),
        );
        assert_eq!(
            new_image.to_string(),
            BinaryImage::from_string(&(
                "----\n".to_owned()+
                "**--\n"+
                "--**\n"+
                "----\n")).to_string()
        );
    }

    #[test]
    fn resample_image_2x2_to_3x2() {
        let image = BinaryImage::from_string(&(
            "*-\n".to_owned()+
            "-*\n"));
        assert_eq!(
            Sampler::resample_image(&image, 3, 2).to_string(),
            BinaryImage::from_string(&(
                "**-\n".to_owned()+
                "--*\n")).to_string()
        )
    }

    #[test]
    fn resample_image_3x3_to_2x2() {
        let mut image = BinaryImage::new_w_h(3, 3);
        image.set_pixel(1, 1, true);
        image.set_pixel(2, 1, true);
        image.set_pixel(1, 2, true);
        image.set_pixel(2, 2, true);
        let new_image = Sampler::resample_image(&image, 2, 2);
        assert_eq!(new_image.width, 2);
        assert_eq!(new_image.height, 2);
        assert_eq!(new_image.get_pixel(0, 0), false);
        assert_eq!(new_image.get_pixel(0, 1), false);
        assert_eq!(new_image.get_pixel(1, 0), false);
        assert_eq!(new_image.get_pixel(1, 1), true);
    }
}
