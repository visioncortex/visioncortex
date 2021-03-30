use std::fmt;
use std::fmt::Write;

pub use bit_vec::BitVec;

use crate::{BoundingRect, Color, ColorName, ColorType, Field, PointF32, PointF64, PointI32};

/// Image with 1 bit per pixel
#[derive(Debug, Clone, Default)]
pub struct BinaryImage {
    pub pixels: BitVec,
    pub width: usize,
    pub height: usize,
}

/// Generalization of 2D array of pixels with any Item
#[derive(Clone, Default)]
pub struct ScalerField<T> {
    field: Field<T>,
}

/// Component of `MonoImage`
pub type MonoImageItem = u16;
/// Image with grayscale values
pub type MonoImage = ScalerField<MonoImageItem>;

/// Image with 4 bytes per pixel
#[derive(Clone, Default)]
pub struct ColorImage {
    pub pixels: Vec<u8>,
    pub width: usize,
    pub height: usize,
}

/// Iterate over each pixel of ColorImage
pub struct ColorImageIter<'a> {
    im: &'a ColorImage,
    curr: usize,
    stop: usize,
}

impl BinaryImage {
    pub fn new_w_h(width: usize, height: usize) -> BinaryImage {
        BinaryImage {
            pixels: BitVec::from_elem(width * height, false),
            width,
            height,
        }
    }

    pub fn get_pixel_at(&self, p: PointI32) -> bool {
        self.get_pixel(p.x as usize, p.y as usize)
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> bool {
        let i = y * self.width + x;
        self.pixels.get(i).unwrap()
    }

    pub fn get_pixel_at_safe(&self, p: PointI32) -> bool {
        self.get_pixel_safe(p.x, p.y)
    }

    pub fn get_pixel_safe(&self, x: i32, y: i32) -> bool {
        if  x >= 0 && x < self.width as i32 &&
            y >= 0 && y < self.height as i32 {
            return self.get_pixel(x as usize, y as usize);
        }
        false
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, v: bool) {
        let i = y * self.width + x;
        self.pixels.set(i, v);
    }

    pub fn set_pixel_at(&mut self, p: PointI32, v: bool) {
        self.set_pixel(p.x as usize, p.y as usize, v);
    }

    pub fn set_pixel_index(&mut self, i: usize, v: bool) {
        self.pixels.set(i, v);
    }

    pub fn set_pixel_safe(&mut self, x: i32, y: i32, v: bool) -> bool {
        if  x >= 0 && x < self.width as i32 &&
            y >= 0 && y < self.height as i32 {
            self.set_pixel(x as usize, y as usize, v);
            return true;
        }
        false
    }

    pub fn set_pixel_at_safe(&mut self, p: PointI32, v: bool) {
        self.set_pixel_safe(p.x, p.y, v);
    }

    pub fn bounding_rect(&self) -> BoundingRect {
        let mut rect = BoundingRect::default();
        for y in 0..self.height {
            for x in 0..self.width {
                if self.get_pixel(x, y) {
                    rect.add_x_y(x as i32, y as i32);
                }
            }
        }
        rect
    }

    pub fn area(&self) -> u64 {
        self.pixels.iter().filter(|x| *x).count() as u64
    }

    /// crop image to fit content
    pub fn crop(&self) -> BinaryImage {
        self.crop_with_rect(self.bounding_rect())
    }

    /// crop a specific area from image
    pub fn crop_with_rect(&self, rect: BoundingRect) -> BinaryImage {
        let mut image = BinaryImage::new_w_h(rect.width() as usize, rect.height() as usize);
        for y in rect.top..rect.bottom {
            for x in rect.left..rect.right {
                if self.get_pixel(x as usize, y as usize) {
                    image.set_pixel(
                        x as usize - rect.left as usize,
                        y as usize - rect.top as usize,
                        true,
                    );
                }
            }
        }
        image
    }

    /// expand the image while center original image so that there will be more space surrounding
    pub fn uncrop(&self, new_width: usize, new_height: usize) -> BinaryImage {
        let xx = (new_width - self.width) >> 1;
        let yy = (new_height - self.height) >> 1;
        let mut new_image = BinaryImage::new_w_h(new_width, new_height);
        for y in 0..self.height {
            for x in 0..self.width {
                if self.get_pixel(x, y) {
                    new_image.set_pixel(x + xx, y + yy, true);
                }
            }
        }
        new_image
    }

    pub fn from_string(string: &str) -> Self {
        let mut width = 0;
        let mut height = 0;
        for line in string.lines() {
            if height == 0 {
                width = line.len();
            }
            height += 1;
        }
        let mut image = Self::new_w_h(width, height);
        for (y, line) in string.lines().enumerate() {
            for (x, c)   in line.chars().enumerate() {
                image.set_pixel(x, y, c == '*');
            }
        }
        image
    }

    pub fn rotate(&self, angle: f64) -> BinaryImage {
        let rotated_width = (self.width as f64 * angle.cos().abs() + self.height as f64 * angle.sin().abs()).round() as usize;
        let rotated_height = (self.width as f64 * angle.sin().abs() + self.height as f64 * angle.cos().abs()).round() as usize;
        let mut rotated_image = BinaryImage::new_w_h(rotated_width, rotated_height);
        let origin = PointF64::new(rotated_width as f64 / 2.0, rotated_height as f64 / 2.0);
        let offset = PointF64::new(
            (rotated_width as i32 - self.width as i32) as f64 / 2.0,
            (rotated_height as i32 - self.height as i32) as f64 / 2.0
        );
        for y in 0..rotated_image.height {
            for x in 0..rotated_image.width {
                let rotated = PointF64::new(x as f64, y as f64).rotate(origin, -angle).translate(-offset);
                rotated_image.set_pixel(
                    x, y,
                    self.get_pixel_safe(rotated.x.round() as i32, rotated.y.round() as i32)
                );
            }
        }
        rotated_image
    }

    /// Paste the content of `src` into `self`, with `offset` with respective to the upper-left corner.
    pub fn paste_from(&mut self, src: &BinaryImage, offset: PointI32) {
        for y in 0..src.height {
            for x in 0..src.width {
                if src.get_pixel(x, y) {
                    self.set_pixel(
                        x + offset.x as usize,
                        y + offset.y as usize,
                        true
                    );
                }
            }
        }
    }

    pub fn to_color_image(&self) -> ColorImage {
        let mut image = ColorImage::new_w_h(self.width, self.height);
        let black = Color::color(&ColorName::Black);
        let white = Color::color(&ColorName::White);
        for y in 0..self.height {
            for x in 0..self.width {
                image.set_pixel(x, y, if self.get_pixel(x, y) {
                    &black
                } else {
                    &white
                });
            }
        }
        image
    }
}

impl fmt::Display for BinaryImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for y in 0..self.height {
            for x in 0..self.width {
                f.write_char(if self.get_pixel(x, y) { '*' } else { '-' })?;
            }
            f.write_char('\n')?;
        }
        Ok(())
    }
}

impl<T> ScalerField<T> where T: Default {
    pub fn new_w_h(width: usize, height: usize) -> Self {
        Self {
            field: Field::with_default(width, height),
        }
    }
}

impl<T> ScalerField<T> where T: Clone {
    pub fn get_pixel(&self, x: usize, y: usize) -> T {
        self.field.get(self.field.index_at(x, y)).unwrap()
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, v: T) {
        self.field.replace(self.field.index_at(x, y), v);
    }
}

impl Iterator for ColorImageIter<'_> {
    type Item = Color;

    fn next(&mut self) -> Option<Color> {
        if self.curr < self.stop {
            let res = self.im.get_pixel_at(self.curr);
            self.curr += 1;
            Some(res)
        } else {
            None
        }
    }
}

impl ColorImage {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_w_h(width: usize, height: usize) -> Self {
        Self {
            pixels: vec![0; width * height * 4],
            width,
            height,
        }
    }

    pub fn iter(&self) -> ColorImageIter {
        ColorImageIter {
            im: self,
            curr: 0,
            stop: self.width * self.height,
        }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> Color {
        let index = y * self.width + x;
        self.get_pixel_at(index)
    }

    pub fn get_pixel_at_point_safe(&self, p: PointI32) -> Option<Color> {
        self.get_pixel_safe(p.x, p.y)
    }

    pub fn get_pixel_safe(&self, x: i32, y: i32) -> Option<Color> {
        if  x >= 0 && x < self.width as i32 &&
            y >= 0 && y < self.height as i32 {
            return Some(self.get_pixel(x as usize, y as usize));
        }
        None
    }

    pub fn get_pixel_at(&self, index: usize) -> Color {
        let index = index * 4;
        let r = self.pixels[index];
        let g = self.pixels[index + 1];
        let b = self.pixels[index + 2];
        let a = self.pixels[index + 3];

        Color::new_rgba(r, g, b, a)
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: &Color) {
        let index = y * self.width + x;
        self.set_pixel_at(index, color);
    }

    pub fn set_pixel_at(&mut self, index: usize, color: &Color) {
        let index = index * 4;
        self.pixels[index] = color.r;
        self.pixels[index + 1] = color.g;
        self.pixels[index + 2] = color.b;
        self.pixels[index + 3] = color.a;
    }

    pub fn to_binary_image<F>(&self, f: F) -> BinaryImage
        where F: Fn(Color) -> bool {
        let mut image = BinaryImage::new_w_h(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                image.set_pixel(x, y, f(self.get_pixel(x, y)));
            }
        }
        image
    }

    pub fn sample_pixel_at(&self, p: PointF32) -> Color {
        bilinear_interpolate(self, p)
    }

    pub fn sample_pixel_at_safe(&self, p:PointF32) -> Option<Color> {
        bilinear_interpolate_safe(self, p)
    }
}

pub fn bilinear_interpolate_safe(im: &ColorImage, p: PointF32) -> Option<Color> {
    if p.x.is_sign_negative() || p.y.is_sign_negative() || p.x > (im.width - 1) as f32 || p.y > (im.height - 1) as f32 {
        None
    } else {
        Some(bilinear_interpolate(im, p))
    }
}

pub fn bilinear_interpolate(im: &ColorImage, p: PointF32) -> Color {
    let x_0 = p.x.floor() as usize;
    let x_1 = p.x.ceil() as usize;
    let y_0 = p.y.floor() as usize;
    let y_1 = p.y.ceil() as usize;
    let c_00 = im.get_pixel(x_0, y_0);
    let c_01 = im.get_pixel(x_0, y_1);
    let c_10 = im.get_pixel(x_1, y_0);
    let c_11 = im.get_pixel(x_1, y_1);

    let interpolate = |channel: usize| {
        let f_00 = c_00.channel(channel).unwrap() as f32;
        let f_01 = c_01.channel(channel).unwrap() as f32;
        let f_10 = c_10.channel(channel).unwrap() as f32;
        let f_11 = c_11.channel(channel).unwrap() as f32;
        let x = p.x - p.x.floor();
        let y = p.y - p.y.floor();

        (f_00 * (1.0 - x) * (1.0 - y) +
        f_10 * x * (1.0 - y) +
        f_01 * (1.0 - x) * y +
        f_11 * x * y) as u8
    };

    Color::new_rgba(
        interpolate(0),
        interpolate(1), 
        interpolate(2), 
        interpolate(3),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_image_crop() {
        let mut image = BinaryImage::new_w_h(4, 4);
        image.set_pixel(1, 1, true);
        image.set_pixel(2, 2, true);
        let crop = image.crop();
        assert_eq!(crop.width, 2);
        assert_eq!(crop.height, 2);
        assert_eq!(crop.get_pixel(0, 0), true);
        assert_eq!(crop.get_pixel(0, 1), false);
        assert_eq!(crop.get_pixel(1, 0), false);
        assert_eq!(crop.get_pixel(1, 1), true);
    }

    #[test]
    fn image_as_string() {
        let mut image = BinaryImage::new_w_h(2,2);
        image.set_pixel(0,0,true);
        image.set_pixel(1,1,true);
        assert_eq!(image.to_string(),
            "*-\n".to_owned()+
            "-*\n");
        let recover = BinaryImage::from_string(&image.to_string());
        assert_eq!(image.width, recover.width);
        assert_eq!(image.height, recover.height);
        for y in 0..image.height {
            for x in 0..image.width {
                assert_eq!(image.get_pixel(x, y), recover.get_pixel(x, y));
            }
        }
    }

    #[test]
    fn rotate_test() {
        assert_eq!(
            BinaryImage::from_string(&(
            "-----------*************---------\n".to_owned()+
            "---------*****************-------\n"+
            "-------*********************-----\n"+
            "-----************************----\n"+
            "----**************************---\n"+
            "---****************************--\n"+
            "--*****************************--\n"+
            "--******************************-\n"+
            "-*******************************-\n"+
            "-********************************\n"+
            "*********************************\n"+
            "*********************************\n"+
            "********************************-\n"+
            "********************************-\n"+
            "********************************-\n"+
            "*******************************--\n"+
            "-******************************--\n"+
            "-*****************************---\n"+
            "--***************************----\n"+
            "---*************************-----\n"+
            "----***********************------\n"+
            "-----*********************-------\n"+
            "-------*****************---------\n"+
            "---------************------------\n"
            )).rotate(1.3962634015954636).to_string(),
            "-----------------------------\n".to_owned()+
            "-----------------------------\n"+
            "-----------****-*------------\n"+
            "---------**********----------\n"+
            "-------*************---------\n"+
            "------***************--------\n"+
            "-----*****************-------\n"+
            "-----*******************-----\n"+
            "-----*******************-----\n"+
            "----*********************----\n"+
            "----*********************----\n"+
            "---***********************---\n"+
            "---************************--\n"+
            "--*************************--\n"+
            "---************************--\n"+
            "---************************--\n"+
            "---************************--\n"+
            "---************************--\n"+
            "---*************************-\n"+
            "---*************************-\n"+
            "----************************-\n"+
            "----************************-\n"+
            "----************************-\n"+
            "----************************-\n"+
            "----************************-\n"+
            "-----***********************-\n"+
            "------*********************--\n"+
            "------*********************--\n"+
            "-------*******************---\n"+
            "--------*****************----\n"+
            "---------****************----\n"+
            "-----------**************----\n"+
            "------------***********------\n"+
            "---------------******--------\n"+
            "------------------*----------\n"+
            "-----------------------------\n"+
            "-----------------------------\n"
        );
    }
}