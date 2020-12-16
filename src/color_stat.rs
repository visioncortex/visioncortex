use crate::{Color, ColorF64, ColorI32, SimpleStatBuilder};

/// Simple statistics of color samples
#[derive(Default)]
pub struct ColorStat {
	pub mean: ColorI32,
	pub deviation: ColorF64,
}

/// Compute simple statistics from color samples
#[derive(Default)]
pub struct ColorStatBuilder {
	r: SimpleStatBuilder,
	g: SimpleStatBuilder,
	b: SimpleStatBuilder,
}

impl ColorStatBuilder {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn add(&mut self, color: Color) {
		self.r.add(color.r as i32);
		self.g.add(color.g as i32);
		self.b.add(color.b as i32);
	}

	pub fn build(&self) -> ColorStat {
		let rs = self.r.build();
		let gs = self.g.build();
		let bs = self.b.build();
		ColorStat {
			mean: ColorI32 {
				r: rs.mean as i32,
				g: gs.mean as i32,
				b: bs.mean as i32,
			},
			deviation: ColorF64 {
				r: rs.deviation,
				g: gs.deviation,
				b: bs.deviation,
			}
		}
	}
}