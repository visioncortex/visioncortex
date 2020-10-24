use crate::{BinaryImage, PointI32};

pub struct PathWalker<'a> {
    image: &'a BinaryImage,
    start: PointI32,
    curr: PointI32,
    prev: PointI32,
    prev_prev: PointI32,
    length: u32,
    clockwise: bool,
    first: bool,
}

pub struct SpiralWalker {
    pub x: i32,
    pub y: i32,
    dir: i32,
    gx: i32,
    gy: i32,
    sq: i32,
    cc: i32,
    area: usize,
    step: usize,
}

impl<'a> PathWalker<'a> {
    pub fn new(image: &'a BinaryImage, start: PointI32, clockwise: bool) -> Self {
        Self {
            image,
            start,
            curr: start,
            prev: start,
            prev_prev: start,
            length: 0,
            clockwise,
            first: true,
        }
    }

    pub fn count_neighbours_for(&self, at: PointI32) -> u32 {
        (0..8)
            .step_by(2)
            .map(|i| Self::ahead_of(at, i))
            .filter(|&ahead| self.image.get_pixel_at_safe(ahead))
            .count() as u32
    }

    pub fn dir_vec(dir: u32) -> PointI32 {
        match dir {
            0 => PointI32 { x:  0, y: -1 },
            1 => PointI32 { x:  1, y: -1 },
            2 => PointI32 { x:  1, y:  0 },
            3 => PointI32 { x:  1, y:  1 },
            4 => PointI32 { x:  0, y:  1 },
            5 => PointI32 { x: -1, y:  1 },
            6 => PointI32 { x: -1, y:  0 },
            7 => PointI32 { x: -1, y: -1 },
            _ => panic!("bad dir {}", dir),
        }
    }

    pub fn side_vecs(dir: u32) -> (PointI32, PointI32) {
        match dir {
            0 => (PointI32 { x: -1, y: -1 }, PointI32 { x: 0, y: -1 }),
            2 => (PointI32 { x: 0, y: 0 }, PointI32 { x: 0, y: -1 }),
            4 => (PointI32 { x: -1, y: 0 }, PointI32 { x: 0, y: 0 }),
            6 => (PointI32 { x: -1, y: 0 }, PointI32 { x: -1, y: -1 }),
            _ => panic!("bad dir {}", dir),
        }
    }

    pub fn ahead_of(curr: PointI32, dir: u32) -> PointI32 {
        let vec = Self::dir_vec(dir);
        PointI32 {
            x: curr.x + vec.x,
            y: curr.y + vec.y,
        }
    }
}

impl Iterator for PathWalker<'_> {
    type Item = PointI32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.first {
            self.first = false;
            return Some(self.start);
        }
        if self.curr == self.start && self.length > 0 {
            return None;
        }
        let mut dir = -1;
        loop {
            let mut go = -1;
            let range = if self.clockwise {
                [0, 2, 4, 6]
            } else {
                [6, 4, 2, 0]
            };
            for &k in range.iter() {
                if Self::ahead_of(self.curr, k) != self.prev
                    && Self::ahead_of(self.curr, k) != self.prev_prev
                {
                    let (a, b) = Self::side_vecs(k);
                    if self.image.get_pixel_at_safe(self.curr + a)
                        != self.image.get_pixel_at_safe(self.curr + b)
                    {
                        go = k as i32;
                        break;
                    }
                }
            }
            
            if go != -1 {
                if dir != -1 && dir != go as i32 {
                    break;
                }
                dir = go;
                self.prev_prev = self.prev;
                self.prev = self.curr;
                self.curr = Self::ahead_of(self.curr, go as u32);
                self.length += 1;
            } else {
                panic!("no way to go?");
            }
            
        }
        if self.length > 1000000 {
            panic!("STUCK: shape should be broken down first without diagonally connected component");
        }
        Some(self.curr)
    }
}

impl SpiralWalker {
    pub fn new(size: usize) -> Self {
        Self {
            x: ((size >> 1) as i32 - 1),
            y: ((size >> 1) as i32 - 1),
            dir: 0,
            gx: 1,
            gy: 0,
            sq: 1,
            cc: 0,
            area: size * size,
            step: 0,
        }
    }
}

impl Iterator for SpiralWalker {
    type Item = (i32, i32);

    fn next(&mut self) -> Option<(i32, i32)> {
        if self.step >= self.area {
            return None;
        }
        if self.area == 1 {
            self.step += 1;
            return Some((0, 0));
        }
        let (xx, yy) = (self.x, self.y);
        self.x += self.gx;
        self.y += self.gy;
        self.cc += 1;
        if self.cc >= self.sq {
            self.cc = 0;
            self.dir += 1;
            if self.dir >= 4 {
                self.dir = 0;
            }
            if self.dir % 2 == 0 {
                self.sq += 1;
            }
            if self.dir == 0 {
                self.gx = 1;
                self.gy = 0;
            } else if self.dir == 1 {
                self.gx = 0;
                self.gy = 1;
            } else if self.dir == 2 {
                self.gx = -1;
                self.gy = 0;
            } else {
                self.gx = 0;
                self.gy = -1;
            }
        }
        self.step += 1;
        Some((xx, yy))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spiral_walker() {
        let walker = SpiralWalker::new(0);
        assert_eq!(
            Vec::<(i32, i32)>::new(),
            walker.collect::<Vec<(i32, i32)>>()
        );
        let walker = SpiralWalker::new(1);
        assert_eq!(vec![(0, 0)], walker.collect::<Vec<(i32, i32)>>());
        let walker = SpiralWalker::new(2);
        assert_eq!(
            vec![(0, 0), (1, 0), (1, 1), (0, 1)],
            walker.collect::<Vec<(i32, i32)>>()
        );
        let walker = SpiralWalker::new(4);
        assert_eq!(
            vec![
                (1, 1),
                (2, 1),
                (2, 2),
                (1, 2),
                (0, 2),
                (0, 1),
                (0, 0),
                (1, 0),
                (2, 0),
                (3, 0),
                (3, 1),
                (3, 2),
                (3, 3),
                (2, 3),
                (1, 3),
                (0, 3)
            ],
            walker.collect::<Vec<(i32, i32)>>()
        );
    }
}
