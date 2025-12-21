use crate::{Color, ColorImage, SpiralWalker};

pub struct ImageEmbedding {
    pub values: Vec<u8>,
}

impl ImageEmbedding {
    pub fn ideal_image_size(_w: usize, _h: usize) -> (usize, usize) {
        (128, 128)
    }

    /// Compute image embedding.
    ///
    /// Only accepts image of ideal_image_size.
    pub fn encode(mut image: ColorImage) -> Self {
        assert_eq!(image.width, 128);
        assert_eq!(image.height, 128);

        // 0| 1| 2| 3|  4|  5|  6|   7
        // 1| 2| 4| 8| 16| 32| 64| 128

        // image pyramid
        let mut images = Vec::new();
        let (mut w, mut h) = (image.width, image.height);
        while w != 1 && h != 1 {
            images.push(std::mem::take(&mut image));
            let parent = images.last().unwrap();
            w /= 2; h /= 2;
            image = ColorImage::new_w_h(w, h);
            for y in 0..h {
                for x in 0..w {
                    let xx = x * 2; let yy = y * 2;
                    image.set_pixel(x, y, &Color::ave_4(
                        parent.get_pixel(xx  , yy  ),
                        parent.get_pixel(xx  , yy+1),
                        parent.get_pixel(xx+1, yy  ),
                        parent.get_pixel(xx+1, yy+1),
                    ));
                }
            }
            #[cfg(test)]
            println!("\n{}", image.to_binary_image(|c| c.r != 0));
        }
        images.push(std::mem::take(&mut image));

        let mut values = Vec::new();

        for (i, image) in images.into_iter().rev().enumerate() {
            debug_assert_eq!(i, power_of_two(image.width));
            for (x, y) in SpiralWalker::new(image.width) {
                values.extend(&image.get_pixel(x as usize, y as usize).rgb_u8());
            }
        }

        debug_assert_eq!(
            values.len(), 3 * (1*1 + 2*2 + 4*4 + 8*8 + 16*16 + 32*32 + 64*64 + 128*128)
        );

        Self { values }
    }

    /// Reconstruct original input image
    pub fn decode(&self) -> ColorImage {
        let (w, h) = ImageEmbedding::ideal_image_size(0, 0);
        let idx = self.values.len() - w * h * 3;
        let mut image = ColorImage::new_w_h(w, h);
        for (i, (x, y)) in SpiralWalker::new(w).enumerate() {
            image.set_pixel(x as usize, y as usize, &Color::new(
                self.values[idx + i * 3 + 0],
                self.values[idx + i * 3 + 1],
                self.values[idx + i * 3 + 2],
            ));
        }
        image
    }

    /// Get the 0th layer as color
    pub fn value_0th(&self) -> Color {
        Color::new(
            self.values[0],
            self.values[1],
            self.values[2],
        )
    }

    pub fn values_1st(&self) -> [u8; 12] {
        let idx = 3;
        let mut values = [0; 12];
        for i in 0..4 {
            values[i * 3 + 0] = self.values[idx + i * 3 + 0];
            values[i * 3 + 1] = self.values[idx + i * 3 + 1];
            values[i * 3 + 2] = self.values[idx + i * 3 + 2];
        }
        values
    }

    pub fn values_2nd(&self) -> &'_[u8] {
        &self.values[15 .. (3 * (1*1 + 2*2 + 4*4))]
    }

    pub fn load(values: Vec<u8>) -> ImageEmbedding {
        ImageEmbedding { values }
    }
}

fn power_of_two(n: usize) -> usize {
    // n must be power of 2
    n.trailing_zeros() as usize
    // slightly better: (usize::BITS - 1) - n.leading_zeros()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_embedding_1() {
        let (w, h) = ImageEmbedding::ideal_image_size(0, 0);
        let mut image = ColorImage::new_w_h(w, h);
        let red = Color::new(255, 0, 0);
        image.set_pixel(55, 55, &red);
        for x in 56..64 {
            for y in 56..64 {
                image.set_pixel(x, y, &red);
            }
        }
        let embedding = ImageEmbedding::encode(image).values;
        // panic!("{embedding:?}");

        let red_fraction = 255u32 * 65 / 128 / 128; // = 1
        assert_eq!(embedding[0] as u32, red_fraction);
        assert_eq!(embedding[1] as u32, 0);
        assert_eq!(embedding[2] as u32, 0);

        // layer 1
        let red_fraction = 255u32 * 65 / 64 / 64; // = 4
        assert_eq!(embedding[3] as u32, red_fraction);
        assert_eq!(embedding[4] as u32, 0);
        assert_eq!(embedding[5] as u32, 0);
        assert_eq!(embedding[6] as u32, 0);
        assert_eq!(embedding[7] as u32, 0);
        assert_eq!(embedding[8] as u32, 0);

        // layer 2
        let red_fraction = 255u32 * 65 / 32 / 32; // = 16
        assert_eq!(embedding[15] as u32, red_fraction);
        assert_eq!(embedding[16] as u32, 0);
        assert_eq!(embedding[17] as u32, 0);
        assert_eq!(embedding[18] as u32, 0);
        assert_eq!(embedding[19] as u32, 0);
        assert_eq!(embedding[20] as u32, 0);
    }
}
