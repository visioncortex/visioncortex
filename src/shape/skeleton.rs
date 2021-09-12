use crate::{BinaryImage, MonoImage, MonoImageItem, SampleStat, SampleStatBuilder, Shape};

/// The skeleton of a binary image (aka medial axis)
pub struct Skeleton {
    pub image: BinaryImage,
    pub stat: SampleStat,
}

impl Shape {
    pub fn to_skeleton(&self) -> Skeleton {
        self.image.to_skeleton()
    }
}

impl BinaryImage {

    /// An extremely fast implementation of skeletonization
    #[allow(clippy::many_single_char_names)]
    pub fn to_skeleton(&self) -> Skeleton {
        if  self.width > MonoImageItem::MAX as usize ||
            self.height > MonoImageItem::MAX as usize {
            panic!(
                "image too large: width={}, height={}",
                self.width, self.height
            );
        }
        let boundary = Shape::image_boundary(self);
        let mut centerx = BinaryImage::new_w_h(self.width, self.height);
        let mut centery = BinaryImage::new_w_h(self.width, self.height);
        let mut centerxy = BinaryImage::new_w_h(self.width, self.height);
        let mut centeryx = BinaryImage::new_w_h(self.width, self.height);
        let mut spanx = MonoImage::new_w_h(self.width, self.height);
        let mut spany = MonoImage::new_w_h(self.width, self.height);
        let mut spanxy = MonoImage::new_w_h(self.width, self.height);
        let mut spanyx = MonoImage::new_w_h(self.width, self.height);
        let mut result = BinaryImage::new_w_h(self.width, self.height);

        // span width for each horizontal scan line
        for y in 0..self.height {
            let mut u = false;
            let mut span = 0;
            for x in 0..self.width + 1 {
                let mut new_span = false;
                if x < self.width {
                    let v = self.get_pixel(x, y);
                    if !u && v {
                        // 0 -> 1
                        span = 0;
                    } else if u && !v {
                        // 1 -> 0
                        new_span = true;
                    }
                    if v {
                        span += 1;
                    }
                    u = v;
                } else if u {
                    new_span = true;
                }
                if new_span {
                    for b in x - span..x {
                        if b == x - span / 2 - 1 {
                            centerx.set_pixel(b, y, true);
                        }
                        spanx.set_pixel(b, y, span as MonoImageItem);
                    }
                }
            }
        }

        // span height for each vertical scan line
        for x in 0..self.width {
            let mut u = false;
            let mut span = 0;
            for y in 0..self.height + 1 {
                let mut new_span = false;
                if y < self.height {
                    let v = self.get_pixel(x, y);
                    if !u && v {
                        // 0 -> 1
                        span = 0;
                    } else if u && !v {
                        // 1 -> 0
                        new_span = true;
                    }
                    if v {
                        span += 1;
                    }
                    u = v;
                } else if u {
                    new_span = true;
                }
                if new_span {
                    for b in y - span..y {
                        if b == y - span / 2 - 1 {
                            centery.set_pixel(x, b, true);
                        }
                        spany.set_pixel(x, b, span as MonoImageItem);
                    }
                }
            }
        }

        // diagonal scanline go towards north east
        let mut x: i32 = 0;
        let mut y: i32 = 0;
        let mut xx = 0;
        let mut yy = 0;
        while x < self.width as i32 && y < self.height as i32 {
            let mut u = false;
            let mut span = 0;
            while x <= self.width as i32 && y >= -1 {
                let mut new_span = false;
                if x < self.width as i32 && y >= 0 {
                    let v = self.get_pixel(x as usize, y as usize);
                    if !u && v {
                        // 0 -> 1
                        span = 0;
                    } else if u && !v {
                        // 1 -> 0
                        new_span = true;
                    }
                    if v {
                        span += 1;
                    }
                    u = v;
                } else if u {
                    new_span = true;
                }
                if new_span {
                    let mut bx = x - span;
                    let mut by = y + span;
                    while bx < x && by > y {
                        if bx == x - span / 2 - 1 {
                            centerxy.set_pixel(bx as usize, by as usize, true);
                        }
                        spanxy.set_pixel(bx as usize, by as usize, span as MonoImageItem);
                        bx += 1;
                        by -= 1;
                    }
                }
                x += 1;
                y -= 1;
            }
            if yy < self.height - 1 {
                yy += 1;
            } else {
                xx += 1;
            }
            x = xx as i32;
            y = yy as i32;
        }

        // diagonal scanline go towards south east
        let mut x: i32 = 0;
        let mut y: i32 = 0;
        let mut xx = self.width - 1;
        let mut yy = 0;
        while x < self.width as i32 && y < self.height as i32 {
            let mut u = false;
            let mut span = 0;
            while x <= self.width as i32 && y <= self.height as i32 {
                let mut new_span = false;
                if x < self.width as i32 && y < self.height as i32 {
                    let v = self.get_pixel(x as usize, y as usize);
                    if !u && v {
                        // 0 -> 1
                        span = 0;
                    } else if u && !v {
                        // 1 -> 0
                        new_span = true;
                    }
                    if v {
                        span += 1;
                    }
                    u = v;
                } else if u {
                    new_span = true;
                }
                if new_span {
                    let mut bx = x - span;
                    let mut by = y - span;
                    while bx < x && by < y {
                        if bx == x - span / 2 - 1 {
                            centeryx.set_pixel(bx as usize, by as usize, true);
                        }
                        spanyx.set_pixel(bx as usize, by as usize, span as MonoImageItem);
                        bx += 1;
                        by += 1;
                    }
                }
                x += 1;
                y += 1;
            }
            if xx > 0 {
                xx -= 1;
            } else {
                yy += 1;
            }
            x = xx as i32;
            y = yy as i32;
        }

        // filter speckle
        for y in 0..self.height {
            for x in 0..self.width {
                let u = centerx.get_pixel(x, y);
                let v = centery.get_pixel(x, y);
                let w = centerxy.get_pixel(x, y);
                let z = centeryx.get_pixel(x, y);
                if u || v || w || z {
                    let mut on = false;
                    let sx = spanx.get_pixel(x, y);
                    let sy = spany.get_pixel(x, y);
                    let sxy = spanxy.get_pixel(x, y);
                    let syx = spanyx.get_pixel(x, y);
                    if u && sx <= sy && sx <= sxy && sx <= syx {
                        on = true;
                    }
                    if v && sy < sx && sy <= sxy && sy <= syx {
                        on = true;
                    }
                    if w && sxy < sx && sxy < sy && sxy <= syx {
                        on = true;
                    }
                    if z && syx < sx && syx < sy && syx < sxy {
                        on = true;
                    }
                    if on {
                        let min = std::cmp::min(std::cmp::min(sx, sy), std::cmp::min(sxy, syx));
                        if boundary.get_pixel(x, y) && min > 1 {
                            on = false;
                        }
                    }
                    if on {
                        result.set_pixel(x, y, true);
                    }
                }
            }
        }

        // final aggregation
        let mut stat = SampleStatBuilder::new();
        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                if result.get_pixel(x as usize, y as usize) {
                    if  result.get_pixel_safe(x-1, y) ||
                        result.get_pixel_safe(x+1, y) ||
                        result.get_pixel_safe(x, y-1) ||
                        result.get_pixel_safe(x, y+1) ||
                        result.get_pixel_safe(x-1, y-1) ||
                        result.get_pixel_safe(x-1, y+1) ||
                        result.get_pixel_safe(x+1, y-1) ||
                        result.get_pixel_safe(x+1, y+1) {
                        let mut dd = 0;
                        let x = x as usize;
                        let y = y as usize;
                        let sx = spanx.get_pixel(x, y);
                        let sy = spany.get_pixel(x, y);
                        let sxy = spanxy.get_pixel(x, y);
                        let syx = spanyx.get_pixel(x, y);
                        if centerx.get_pixel(x, y) && sx <= sy && sx <= sxy && sx <= syx {
                            dd = sx;
                        }
                        if centery.get_pixel(x, y) && sy < sx && sy <= sxy && sy <= syx {
                            dd = sy;
                        }
                        if centerxy.get_pixel(x, y) && sxy < sx && sxy < sy && sxy <= syx {
                            dd = sxy;
                        }
                        if centeryx.get_pixel(x, y) && syx < sx && syx < sy && syx < sxy {
                            dd = syx;
                        }
                        if dd > 0 {
                            stat.add(dd as i32);
                        }
                    } else {
                        result.set_pixel(x as usize, y as usize, false);
                    }
                }
            }
        }

        Skeleton {
            image: result,
            stat: stat.build(),
        }
    }
}