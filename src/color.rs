pub trait ColorType {
    type ValueType;

    fn channel(&self, c: usize) -> Option<Self::ValueType>;
}

/// RGBA; each channel is 8 bit unsigned
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Color names
pub enum ColorName {
    Black,
    White,
    Red,
}

/// RGB; each channel is 32 bit signed
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct ColorI32 {
    pub r: i32,
    pub g: i32,
    pub b: i32,
}

/// RGB; each channel is 64 bit float
#[derive(Copy, Clone, Default, PartialEq, Debug)]
pub struct ColorF64 {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

/// RGBA; each channel is 32 bit unsigned
#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct ColorSum {
    pub r: u32,
    pub g: u32,
    pub b: u32,
    pub a: u32,
    pub counter: u32,
}

/// HSV; each channel is 64 bit float
#[derive(Copy, Clone, PartialEq)]
pub struct ColorHsv {
    pub h: f64,
    pub s: f64,
    pub v: f64,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self::new_rgba(r, g, b, 255)
    }

    pub fn new_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn color(name: &ColorName) -> Self {
        match name {
            ColorName::Black => Self::new(0, 0, 0),
            ColorName::White => Self::new(255, 255, 255),
            ColorName::Red => Self::new(255, 0, 0),
        }
    }

    pub fn get_palette_color(i: usize) -> Self {
        match i % 8 {
            // https://codepen.io/chorijan/pen/azVzPO
            0 => Self::new(216, 51, 74),   // Ruby
            1 => Self::new(255, 232, 96),  // Lemon
            2 => Self::new(160, 212, 104), // Grass
            3 => Self::new(72, 207, 173),  // Mint
            4 => Self::new(79, 193, 233),  // Aqua
            5 => Self::new(93, 156, 236),  // Jeans
            6 => Self::new(128, 103, 183), // Plum
            7 => Self::new(172, 146, 236), // Lavender
            _ => panic!("%"),
        }
    }

    pub fn to_color_string(&self) -> String {
        format!(
            "rgba({},{},{},{})",
            self.r,
            self.g,
            self.b,
            self.a as f64 / 255.0
        )
    }

    pub fn to_hex_string(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    pub fn to_color_i32(&self) -> ColorI32 {
        ColorI32::new(self)
    }

    #[allow(
        clippy::many_single_char_names,
        clippy::float_cmp
    )]
    pub fn to_hsv(&self) -> ColorHsv {
        // Adapted from
        // https://github.com/bgrins/TinyColor
        // Brian Grinstead, MIT License

        // Converts an RGB color value to HSV
        // *Assumes:* [ r, g, b ] in [0, 255]
        // *Returns:* [ h, s, v ] in [0, 1]

        let r = self.r as f64 / 255.0;
        let g = self.g as f64 / 255.0;
        let b = self.b as f64 / 255.0;

        let max = maxf64(r, maxf64(g, b));
        let min = minf64(r, minf64(g, b));
        let mut h;
        let s;
        let v = max;
        let d = max - min;
        s = if max == 0.0 { 0.0 } else { d / max };

        if max == min {
            h = 0.0; // achromatic
        } else {
            h = match max {
                k if (k == r) => (g - b) / d + (if g < b { 6.0 } else { 0.0 }),
                k if (k == g) => (b - r) / d + 2.0,
                k if (k == b) => (r - g) / d + 4.0,
                _ => unreachable!(),
            };
            h /= 6.0;
        }
        return ColorHsv::new(h, s, v);

        fn maxf64(a: f64, b: f64) -> f64 {
            if a > b {
                a
            } else {
                b
            }
        }

        fn minf64(a: f64, b: f64) -> f64 {
            if a < b {
                a
            } else {
                b
            }
        }
    }
}

impl ColorType for Color {
    type ValueType = u8;

    fn channel(&self, c: usize) -> Option<Self::ValueType> {
        match c {
            0 => Some(self.r),
            1 => Some(self.g),
            2 => Some(self.b),
            3 => Some(self.a),
            _ => None,
        }
    }
}

impl ColorI32 {
    pub fn new(color: &Color) -> Self {
        Self {
            r: color.r as i32,
            g: color.g as i32,
            b: color.b as i32,
        }
    }

    pub fn add(&self, other: &Self) -> Self {
        Self {
            r: self.r + other.r,
            g: self.g + other.g,
            b: self.b + other.b,
        }
    }

    pub fn diff(&self, other: &Self) -> Self {
        Self {
            r: self.r - other.r,
            g: self.g - other.g,
            b: self.b - other.b,
        }
    }

    pub fn to_color(&self) -> Color {
        assert!(0 <= self.r && self.r < 256);
        assert!(0 <= self.g && self.g < 256);
        assert!(0 <= self.b && self.b < 256);
        Color::new(self.r as u8, self.g as u8, self.b as u8)
    }
}

impl ColorF64 {
    pub fn new(color: &ColorI32) -> Self {
        Self {
            r: color.r as f64,
            g: color.g as f64,
            b: color.b as f64,
        }
    }

    pub fn magnitude(&self) -> f64 {
        (self.r * self.r + self.g * self.g + self.b * self.b).sqrt()
    }
}

impl ColorHsv {
    pub fn new(h: f64, s: f64, v: f64) -> Self {
        Self { h, s, v }
    }
}

impl ColorSum {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add(&mut self, color: &Color) {
        self.r += color.r as u32;
        self.g += color.g as u32;
        self.b += color.b as u32;
        self.a += color.a as u32;
        self.counter += 1;
    }

    pub fn merge(&mut self, color: &ColorSum) {
        self.r += color.r;
        self.g += color.g;
        self.b += color.b;
        self.a += color.a;
        self.counter += color.counter;
    }

    pub fn average(&self) -> Color {
        Color::new_rgba(
            (self.r / self.counter) as u8,
            (self.g / self.counter) as u8,
            (self.b / self.counter) as u8,
            (self.a / self.counter) as u8,
        )
    }

    pub fn clear(&mut self) {
        self.r = 0;
        self.g = 0;
        self.b = 0;
        self.a = 0;
        self.counter = 0;
    }
}
