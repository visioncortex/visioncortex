use crate::{BinaryImage, BitVec, Sampler, Shape, SpiralWalker};

#[derive(Default)]
pub struct ShapeEncoding {
    pub seq: Vec<BitVec>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ShapeEncodingString {
    Hex(String),
    Bit(String),
}

impl ShapeEncoding {
    pub fn encode_as_shape_encoding(sampler: &Sampler) -> ShapeEncoding {
        let mut seq = Vec::<BitVec>::new();
        let size = sampler.size();
        let layers = ShapeEncoding::layer_from_area(size * size);
        let mut square = size;
        let mut number = 1; // number of samples to take at each layer
        #[cfg(test)]
        let print = false;
        #[cfg(test)]
        {
            if print {
                println!("size={}, layers={}", size, layers);
            }
        }
        for _l in 0..layers {
            let mut bits = BitVec::new();
            let grid_size = 1 << (ShapeEncoding::pow_of_two(number) >> 1); // binary sqrt
            let half_value = 1 << (std::cmp::max(ShapeEncoding::pow_of_two(square) << 1, 1) - 1); // square*square / 2
            let layer_string_length = std::cmp::max(ShapeEncoding::pow_of_two(square) << 1, 1);
            #[cfg(test)]
            {
                if print {
                    println!(
                        "layer={}, square={}, layer_string_length={}",
                        _l, square, layer_string_length
                    );
                }
            }
            for (x, y) in SpiralWalker::new(grid_size) {
                let x = x as usize;
                let y = y as usize;
                let mut value =
                    sampler.sample(x * square, y * square, (x + 1) * square, (y + 1) * square);
                // To cramp 5 options into 4 slots,
                // We have to discard 1 bit.
                // So for value > V/2, we minus 1.
                if value > half_value {
                    value -= 1;
                }
                #[cfg(test)]
                {
                    if print {
                        print!(
                            "  ({}, {}, {}, {}), value={}, bits=",
                            x * square,
                            y * square,
                            (x + 1) * square,
                            (y + 1) * square,
                            value
                        );
                    }
                }
                let mut cursor = layer_string_length as i32 - 1;
                while cursor >= 0 {
                    let bit = (value >> cursor) & 1;
                    bits.push(bit == 1);
                    cursor -= 1;
                    #[cfg(test)]
                    {
                        if print {
                            print!("{}", bit);
                        }
                    }
                }
            }
            seq.push(bits);
            square >>= 1;
            number <<= 2;
        }
        ShapeEncoding { seq }
    }

    pub fn encode_binary_image(image: &BinaryImage) -> ShapeEncodingString {
        Self::encode_binary_image_as_hex_encoding_string_and_size(image).0
    }

    pub fn encode_binary_image_as_hex_encoding_string_and_size(
        image: &BinaryImage,
    ) -> (ShapeEncodingString, usize) {
        let (encoding, encoder_size) = Self::encode_binary_image_as_shape_encoding_and_size(image);
        (encoding.hexstring(), encoder_size)
    }

    pub fn encode_binary_image_as_shape_encoding_and_size(
        image: &BinaryImage,
    ) -> (ShapeEncoding, usize) {
        let size = std::cmp::max(image.width, image.height) as usize;
        let encoder_size = 1 << ShapeEncoding::next_pow_of_four(size * size);
        let sampler = Sampler::new_with_size(image, encoder_size);
        let encoding = Self::encode_as_shape_encoding(&sampler);
        (encoding, encoder_size)
    }

    /// decode shape from hexstring
    pub fn decode_from(shape_string: &ShapeEncodingString) -> Shape {
        match shape_string {
            ShapeEncodingString::Hex(hexstring) => Shape {
                image: Self::bits_to_binary_image(&ShapeEncoding::hexstring_to_bits(&hexstring)),
            },
            _ => panic!(),
        }
    }

    pub fn bits_to_binary_image(bits: &BitVec) -> BinaryImage {
        Self::bits_to_binary_image_with_limit(bits, bits.len())
    }

    pub fn bits_to_binary_image_with_limit(bits: &BitVec, limit: usize) -> BinaryImage {
        let size = ShapeEncoding::size_from_length(limit);
        let area = size * size;
        let mut image = BinaryImage::new_w_h(size, size);
        let offset = limit - area;
        for (i, (x, y)) in SpiralWalker::new(size).enumerate() {
            image.set_pixel(x as usize, y as usize, bits[i + offset]);
        }
        image
    }

    pub fn bits(&self) -> BitVec {
        let area = self.area();
        let mut bits = BitVec::with_capacity(Self::length_from_area(area));
        if self.seq.len() == 1 {
            return self.seq[0].clone();
        }
        let mut level = 2; // starts from 1/2
        #[cfg(test)]
        let print = false;
        while level <= area {
            #[cfg(test)]
            let mut indent_str = "".to_owned();
            #[cfg(test)]
            {
                if print {
                    println!("level={}", level);
                    let indent = Self::pow_of_two(level);
                    indent_str = (0..indent).map(|_| "  ").collect::<String>();
                }
            }
            for i in 0..self.seq.len() {
                let exp = 1 << (i << 1); // 4^i
                let mul = if i == 0 { 2 } else { exp };
                let offset = Self::pow_of_two(level / mul);
                let stride = self.seq[i].len() / exp;
                let il = mul * (1 << offset);
                //#[cfg(test)] { if print { println!("                i={}, mul={}, offset={}, stride={}", i, mul, offset, stride); } }
                if level == il && offset < stride {
                    if i < self.seq.len() - 1 {
                        let iexp = 1 << ((i + 1) << 1);
                        let iil = iexp * (1 << Self::pow_of_two(level / iexp));
                        if il >= iil {
                            // skip if overlap with next level
                            continue;
                        }
                    }
                    for j in 0..exp {
                        let index = j * stride + offset;
                        #[cfg(test)]
                        {
                            if print {
                                println!("  {}:{}{}", i, indent_str, index);
                            }
                        }
                        bits.push(self.seq[i][index]);
                    }
                }
            }
            level <<= 1; // double each deeper level
        }
        bits
    }

    pub fn bitstring(&self) -> ShapeEncodingString {
        ShapeEncodingString::Bit(Self::bits_to_bitstring(&self.bits()))
    }

    pub fn hexstring(&self) -> ShapeEncodingString {
        let bits = self.bits();
        let mut string = Self::bits_to_hexstring(&bits);
        if 0 < bits.len() % 8 && bits.len() % 8 <= 4 {
            string.truncate(std::cmp::max(1, string.len() - 1));
        }
        ShapeEncodingString::Hex(string)
    }

    /// because hexstring may contain trailing 0s, trimming the last 4 bits would yield better prefix match
    pub fn hexstring_trim(&self) -> ShapeEncodingString {
        let bits = self.bits();
        let mut string = Self::bits_to_hexstring(&bits);
        if bits.len() % 8 != 0 {
            string.truncate(std::cmp::max(1, string.len() - 2));
        }
        ShapeEncodingString::Hex(string)
    }

    pub fn hexstring_to_bits(string: &str) -> BitVec {
        let mut hexstring = string.to_string();
        if hexstring.len() % 2 != 0 {
            hexstring.push_str("0");
        }
        let bytes: Vec<u8> = (0..hexstring.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hexstring[i..i + 2], 16).unwrap())
            .collect();
        let mut bits = BitVec::from_bytes(&bytes);
        if bytes.len() == 1 && (bytes[0] == 0 || bytes[0] == 0b10000000) {
            bits.truncate(1);
            return bits;
        }
        bits.truncate(Self::hexstring_to_bits_length(bits.len()));
        bits
    }

    pub fn hexstring_to_bits_length(hexstring_len: usize) -> usize {
        match hexstring_len {
            8 => 5,
            32 => 25,
            112 => 105,
            432 => 425,
            1712 => 1705,
            6832 => 6825,
            27312 => 27305,
            109232 => 109225,
            n => unimplemented!("n={}", n),
        }
    }

    pub fn bitstring_to_bits(string: &str) -> BitVec {
        let mut bitstring = string.to_string();
        while bitstring.len() % 8 != 0 {
            bitstring.push_str("0");
        }
        let bytes: Vec<u8> = (0..bitstring.len())
            .step_by(8)
            .map(|i| u8::from_str_radix(&bitstring[i..i + 8], 2).unwrap())
            .collect();
        let mut bits = BitVec::from_bytes(&bytes);
        bits.truncate(string.len());
        bits
    }

    pub fn bits_to_bitstring(bits: &BitVec) -> String {
        let len = bits.len();
        let mut string = bits
            .to_bytes()
            .iter()
            .map(|b| format!("{:08b}", b))
            .collect::<String>();
        string.truncate(len);
        string
    }

    pub fn bits_to_hexstring(bits: &BitVec) -> String {
        bits.to_bytes()
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<String>()
    }

    pub fn pow_of_two(mut n: usize) -> usize {
        let mut pow_of_2 = 0;
        while n > 1 {
            n >>= 1;
            pow_of_2 += 1;
        }
        pow_of_2
    }

    pub fn pow_of_four(mut n: usize) -> usize {
        let mut pow_of_4 = 0;
        while n > 3 {
            n >>= 2;
            pow_of_4 += 1;
        }
        pow_of_4
    }

    pub fn is_pow_of_four(n: usize) -> bool {
        (1 << (2 * Self::pow_of_four(n))) == n
    }

    pub fn next_pow_of_four(n: usize) -> usize {
        let mut size = Self::pow_of_four(n);
        if !Self::is_pow_of_four(n) {
            size += 1;
        }
        size
    }

    fn get_length_from_area(area: usize) -> usize {
        fn rev(area: usize) -> usize {
            if area <= 4 {
                area + area / 4
            } else {
                area + area / 4 + rev(area / 4)
            }
        }
        rev(area)
    }

    pub fn layer_from_area(n: usize) -> usize {
        Self::pow_of_four(n) + 1
    }

    pub fn length_from_area(n: usize) -> usize {
        match n {
            /* 1*1 */ 1 => 1,
            /* 2*2 */ 4 => 5,
            /* 4*4 */ 16 => 25,
            /* 8*8 */ 64 => 105,
            /* 16*16 */ 256 => 425,
            /* 32*32 */ 1024 => 1705,
            /* 64*64 */ 4096 => 6825,
            /* 128*128 */ 16384 => 27305,
            /* 256*256 */ 65536 => 109225,
            _ => Self::get_length_from_area(n),
        }
    }

    pub fn length_from_size(n: usize) -> usize {
        Self::length_from_area(n * n)
    }

    pub fn area_from_length(length: usize) -> usize {
        1 << Self::pow_of_two(length)
    }

    pub fn size_from_length(length: usize) -> usize {
        1 << (Self::pow_of_two(length) >> 1)
    }

    /// side of the square of the image
    /// must be power of 2
    pub fn size(&self) -> usize {
        1 << (self.seq.len() - 1)
    }

    /// area of the image
    /// equals to size*size
    pub fn area(&self) -> usize {
        1 << ((self.seq.len() - 1) << 1)
    }

    /// length of the shape encoding
    /// guaranteed to be smaller than (not equal to) 2*size^2
    pub fn length(&self) -> usize {
        Self::length_from_area(self.area())
    }

    pub fn shape_encoding_diff(me: &ShapeEncodingString, other: &ShapeEncodingString) -> u64 {
        let (a, b) = match (me, other) {
            (ShapeEncodingString::Hex(aa), ShapeEncodingString::Hex(bb)) => (
                ShapeEncoding::hexstring_to_bits(aa),
                ShapeEncoding::hexstring_to_bits(bb),
            ),
            (ShapeEncodingString::Bit(aa), ShapeEncodingString::Bit(bb)) => (
                ShapeEncoding::bitstring_to_bits(aa),
                ShapeEncoding::bitstring_to_bits(bb),
            ),
            _ => panic!("different encoding type"),
        };
        Self::shape_encoding_diff_bits(&a, &b)
    }

    /// Almost like humming distance between the two bit strings,
    /// but more significant bits have higher weights
    pub fn shape_encoding_diff_bits(a: &BitVec, b: &BitVec) -> u64 {
        let mut l = 1;
        let mut diff: u64 = 0;
        while l <= 65536 {
            let ll = ShapeEncoding::length_from_area(l);
            let lls = Self::pow_of_two(l);
            if a.len() <= ll - l && b.len() <= ll - l {
                let remainder = Self::remainder((lls >> 1) as u64);
                diff += remainder;
                #[cfg(test)]
                println!("diff += {}", remainder);
                break;
            }
            let mut cc: u64 = 0;
            for i in 0..l {
                let ii = ll - l + i;
                let aa = if ii < a.len() { a[ii] } else { ii % 4 == 0 };
                let bb = if ii < b.len() { b[ii] } else { ii % 4 == 1 };
                if aa != bb {
                    cc += 1;
                }
            }
            cc = std::cmp::min(cc, std::cmp::max(l as u64 - 1, 1));
            let shift = match l {
                1 => 8,
                4 => 8,
                16 => 8,
                64 => 8,
                256 => 8,
                1024 => 6,
                4096 => 4,
                16384 => 2,
                65536 => 0,
                _ => panic!("impossible"),
            };
            diff += cc << shift;
            #[cfg(test)]
            println!("{} << {}", cc, shift);
            l <<= 2; // times 4
        }
        diff
    }

    #[allow(clippy::absurd_extreme_comparisons, clippy::identity_op)]
    fn remainder(i: u64) -> u64 {
        let mut n = 0;
        if i <= 0 {
            n += (1 << 0) << 8;
        }
        if i <= 1 {
            n += (1 << 1) << 8;
        }
        if i <= 2 {
            n += (1 << 3) << 8;
        }
        if i <= 3 {
            n += (1 << 5) << 8;
        }
        if i <= 4 {
            n += (1 << 7) << 8;
        }
        if i <= 5 {
            n += (1 << 7) << 6;
        }
        if i <= 6 {
            n += (1 << 7) << 4;
        }
        if i <= 7 {
            n += (1 << 7) << 2;
        }
        if i <= 8 {
            n += (1 << 7) << 0;
        }
        n
    }
}

impl ShapeEncodingString {
    pub fn unwrap(&self) -> &String {
        match self {
            Self::Hex(string) => string,
            Self::Bit(string) => string,
        }
    }

    pub fn diff(&self, other: &Self) -> u64 {
        ShapeEncoding::shape_encoding_diff(self, other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pow_of_two() {
        assert_eq!(ShapeEncoding::pow_of_two(0), 0);
        assert_eq!(ShapeEncoding::pow_of_two(1), 0);
        assert_eq!(ShapeEncoding::pow_of_two(2), 1);
        assert_eq!(ShapeEncoding::pow_of_two(4), 2);
        assert_eq!(ShapeEncoding::pow_of_two(8), 3);
        assert_eq!(ShapeEncoding::pow_of_two(16), 4);
    }

    #[test]
    fn pow_of_four() {
        assert_eq!(ShapeEncoding::pow_of_four(0), 0);
        assert_eq!(ShapeEncoding::pow_of_four(1), 0);
        assert_eq!(ShapeEncoding::pow_of_four(4), 1);
        assert_eq!(ShapeEncoding::pow_of_four(8), 1);
        assert_eq!(ShapeEncoding::pow_of_four(16), 2);
    }

    #[test]
    fn next_pow_of_four() {
        assert_eq!(ShapeEncoding::next_pow_of_four(0), 1);
        assert_eq!(ShapeEncoding::next_pow_of_four(4), 1);
        assert_eq!(ShapeEncoding::next_pow_of_four(6), 2);
        assert_eq!(ShapeEncoding::next_pow_of_four(8), 2);
        assert_eq!(ShapeEncoding::next_pow_of_four(12), 2);
        assert_eq!(ShapeEncoding::next_pow_of_four(16), 2);
    }

    #[test]
    fn area_and_length() {
        for i in 0..10 {
            let a = 1 << (i << 1);
            println!("{} => {},", a, ShapeEncoding::get_length_from_area(a));
            assert_eq!(
                a,
                ShapeEncoding::area_from_length(ShapeEncoding::get_length_from_area(a))
            );
        }
    }

    #[test]
    fn length_from_area() {
        for i in 0..9 {
            let area = 4_i32.pow(i) as usize;
            let len = ShapeEncoding::length_from_area(area);
            assert_eq!(len, ShapeEncoding::get_length_from_area(area));
            println!("{} => {},", area, len);
        }
    }

    #[test]
    /* size=2, layers=2
    layer=0, square=2, layer_string_length=2
      (0, 0, 2, 2), value=2, bits=10
    layer=1, square=1, layer_string_length=1
      (0, 0, 1, 1), value=1, bits=1
      (1, 0, 2, 1), value=0, bits=0
      (0, 1, 1, 2), value=0, bits=0
      (1, 1, 2, 2), value=1, bits=1
    */
    fn encode_2x2() {
        let size = 2;
        let mut image = BinaryImage::new_w_h(size, size);
        image.set_pixel(0, 0, true);
        image.set_pixel(1, 1, true);
        let sampler = Sampler::new_with_size(&image, size);
        let encoding = ShapeEncoding::encode_as_shape_encoding(&sampler);
        assert_eq!(encoding.size(), size);
        assert_eq!(encoding.length(), 5);
        assert_eq!(encoding.seq.len(), 2);
        let layer0 = &encoding.seq[0];
        assert_eq!(layer0.len(), 2);
        // 10
        assert_eq!(layer0[0], true);
        assert_eq!(layer0[1], false);
        let layer1 = &encoding.seq[1];
        assert_eq!(layer1.len(), 4);
        // 1 0 0 1 >> spiral >> 1 0 1 0
        assert_eq!(layer1[0], true);
        assert_eq!(layer1[1], false);
        assert_eq!(layer1[2], true);
        assert_eq!(layer1[3], false);
    }

    #[test]
    /// 2 and 3 are also encoded as 10
    fn encode_2x2_3() {
        let size = 2;
        let mut image = BinaryImage::new_w_h(size, size);
        image.set_pixel(0, 0, true);
        image.set_pixel(0, 1, true);
        image.set_pixel(1, 1, true);
        let sampler = Sampler::new_with_size(&image, size);
        let encoding = ShapeEncoding::encode_as_shape_encoding(&sampler);
        let layer0 = &encoding.seq[0];
        assert_eq!(layer0.len(), 2);
        // 10
        assert_eq!(layer0[0], true);
        assert_eq!(layer0[1], false);
    }

    #[test]
    /// 4 is encoded as 11
    fn encode_2x2_4() {
        let size = 2;
        let mut image = BinaryImage::new_w_h(size, size);
        image.set_pixel(0, 0, true);
        image.set_pixel(0, 1, true);
        image.set_pixel(1, 0, true);
        image.set_pixel(1, 1, true);
        let sampler = Sampler::new_with_size(&image, size);
        let encoding = ShapeEncoding::encode_as_shape_encoding(&sampler);
        let layer0 = &encoding.seq[0];
        assert_eq!(layer0.len(), 2);
        // 11
        assert_eq!(layer0[0], true);
        assert_eq!(layer0[1], true);
    }

    #[test]
    /* size=4, layers=3
    layer=0, square=4, layer_string_length=4
      (0, 0, 4, 4), value=4, bits=0100
    layer=1, square=2, layer_string_length=2
      (0, 0, 2, 2), value=2, bits=10
      (2, 0, 4, 2), value=0, bits=00
      (0, 2, 2, 4), value=0, bits=00
      (2, 2, 4, 4), value=2, bits=10
    layer=2, square=1, layer_string_length=1
      (0, 0, 1, 1), value=1, bits=1
      (1, 0, 2, 1), value=0, bits=0
      (2, 0, 3, 1), value=0, bits=0
      (3, 0, 4, 1), value=0, bits=0
      (0, 1, 1, 2), value=0, bits=0
      (1, 1, 2, 2), value=1, bits=1
      (2, 1, 3, 2), value=0, bits=0
      (3, 1, 4, 2), value=0, bits=0
      (0, 2, 1, 3), value=0, bits=0
      (1, 2, 2, 3), value=0, bits=0
      (2, 2, 3, 3), value=1, bits=1
      (3, 2, 4, 3), value=0, bits=0
      (0, 3, 1, 4), value=0, bits=0
      (1, 3, 2, 4), value=0, bits=0
      (2, 3, 3, 4), value=0, bits=0
      (3, 3, 4, 4), value=1, bits=1
    */
    fn encode_4x4() {
        let size = 4;
        let mut image = BinaryImage::new_w_h(size, size);
        image.set_pixel(0, 0, true);
        image.set_pixel(1, 1, true);
        image.set_pixel(2, 2, true);
        image.set_pixel(3, 3, true);
        let sampler = Sampler::new_with_size(&image, size);
        let encoding = ShapeEncoding::encode_as_shape_encoding(&sampler);
        assert_eq!(encoding.size(), size);
        assert_eq!(encoding.length(), 25);
        assert_eq!(encoding.seq.len(), 3);
        let layer0 = &encoding.seq[0];
        assert_eq!(layer0.len(), 4);
        // 0100
        assert_eq!(layer0[0], false);
        assert_eq!(layer0[1], true);
        assert_eq!(layer0[2], false);
        assert_eq!(layer0[3], false);
        let layer1 = &encoding.seq[1];
        assert_eq!(layer1.len(), 8);
        // 10 00 00 10 >> spiral walk >> 10 00 10 00
        assert_eq!(layer1[0], true);
        assert_eq!(layer1[1], false);
        assert_eq!(layer1[2], false);
        assert_eq!(layer1[3], false);
        assert_eq!(layer1[4], true);
        assert_eq!(layer1[5], false);
        assert_eq!(layer1[6], false);
        assert_eq!(layer1[7], false);
        let l2 = &encoding.seq[2];
        assert_eq!(l2.len(), 16);
        // 1000 0100 0010 0001 >>spiral walk>> 1010 0010 0000 1000
        assert_eq!(l2[0], true);
        assert_eq!(l2[1], false);
        assert_eq!(l2[2], true);
        assert_eq!(l2[3], false);
        assert_eq!(l2[4], false);
        assert_eq!(l2[5], false);
        assert_eq!(l2[6], true);
        assert_eq!(l2[7], false);
        assert_eq!(l2[8], false);
        assert_eq!(l2[9], false);
        assert_eq!(l2[10], false);
        assert_eq!(l2[11], false);
        assert_eq!(l2[12], true);
        assert_eq!(l2[13], false);
        assert_eq!(l2[14], false);
        assert_eq!(l2[15], false);
    }

    #[test]
    fn bitstring_2x2() {
        let size = 2;
        let mut image = BinaryImage::new_w_h(size, size);
        image.set_pixel(0, 0, true);
        image.set_pixel(1, 1, true);
        let sampler = Sampler::new_with_size(&image, size);
        let encoding = ShapeEncoding::encode_as_shape_encoding(&sampler);
        let bits = encoding.bits();
        assert_eq!(bits.len(), 5);
        //let ans = vec![1, 1,0,0,1];
        let ans = vec![1, 1, 0, 1, 0]; // spiral walk
        for i in 0..ans.len() {
            assert_eq!(ans[i] == 1, bits[i]);
        }
    }

    #[test]
    fn bitstring_4x4() {
        let size = 4;
        let mut image = BinaryImage::new_w_h(size, size);
        image.set_pixel(0, 0, true);
        image.set_pixel(1, 1, true);
        image.set_pixel(2, 2, true);
        image.set_pixel(3, 3, true);
        let sampler = Sampler::new_with_size(&image, size);
        let encoding = ShapeEncoding::encode_as_shape_encoding(&sampler);
        let bits = encoding.bits();
        assert_eq!(bits.len(), 25);
        //let ans = vec![0, 1,0,0,1, 0,0,0,0, 1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1];
        // 1000 0100 0010 0001 >>spiral walk>> 1010 0010 0000 1000
        let ans = vec![
            0, 1, 0, 1, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0,
        ]; // spiral walk
        for i in 0..ans.len() {
            assert_eq!(ans[i] == 1, bits[i]);
            print!("{}", if bits[i] { 1 } else { 0 });
        }
        assert_eq!(
            encoding.hexstring(),
            ShapeEncodingString::Hex("5051040".to_owned())
        );
        //    0100
        //        10 00 00 10
        //                   1000 0100 0010 0001
        //
        // -> 0 1001 0000 1000 0100 0010 0001
    }

    #[test]
    /// here we see that the leading bits of the encoding do not change after upscaling
    fn bitstring_upscale() {
        let mut image = BinaryImage::new_w_h(2, 2);
        image.set_pixel(0, 0, true);
        image.set_pixel(1, 1, true);
        {
            let sampler = Sampler::new_with_size(&image, 2);
            let encoding = ShapeEncoding::encode_as_shape_encoding(&sampler);
            println!("{}", encoding.bitstring().unwrap());
            assert_eq!(encoding.bits().len(), 5);
            assert_eq!(
                encoding.bitstring(),
                ShapeEncodingString::Bit("11010".to_owned())
            );
            assert_eq!(
                encoding.hexstring(),
                ShapeEncodingString::Hex("D0".to_owned())
            );
            assert_eq!(
                encoding.hexstring_trim(),
                ShapeEncodingString::Hex("D".to_owned())
            );
        }
        {
            let sampler = Sampler::new_with_size(&image, 4);
            let encoding = ShapeEncoding::encode_as_shape_encoding(&sampler);
            println!("{}", encoding.bitstring().unwrap());
            assert_eq!(encoding.bits().len(), 25);
            // 1 1001 1001 1100 1100 0011 0011 >> 1 1010 1010 1010 0111 0001 1100
            assert_eq!(
                encoding.bitstring(),
                ShapeEncodingString::Bit("1101010101010011100011100".to_owned())
            );
            assert_eq!(
                encoding.hexstring(),
                ShapeEncodingString::Hex("D5538E0".to_owned())
            );
            assert_eq!(
                encoding.hexstring_trim(),
                ShapeEncodingString::Hex("D5538E".to_owned())
            );
        }
        {
            let sampler = Sampler::new_with_size(&image, 8);
            let encoding = ShapeEncoding::encode_as_shape_encoding(&sampler);
            assert_eq!(encoding.bits().len(), 105);
            println!("{}", encoding.bitstring().unwrap());
            let mut bits = encoding.bits();
            bits.truncate(25);
            assert_eq!(
                ShapeEncoding::bits_to_bitstring(&bits),
                "1101010101010011100011100"
            );
        }
    }

    #[test]
    fn encoding_1x1() {
        let encoding = ShapeEncoding {
            seq: vec![BitVec::from_elem(1, true)],
        };
        assert_eq!(
            encoding.bitstring(),
            ShapeEncodingString::Bit("1".to_owned())
        );
        assert_eq!(
            encoding.hexstring(),
            ShapeEncodingString::Hex("8".to_owned())
        );
    }

    #[test]
    fn hexstring_to_bits() {
        let bits = ShapeEncoding::hexstring_to_bits(&"CCE6198".to_owned());
        assert_eq!(bits.len(), 25);
        assert_eq!(
            ShapeEncoding::bits_to_bitstring(&bits),
            "1100110011100110000110011"
        );

        let bits = ShapeEncoding::hexstring_to_bits(&"A8".to_owned());
        assert_eq!(bits.len(), 5);
        assert_eq!(ShapeEncoding::bits_to_bitstring(&bits), "10101");
    }

    #[test]
    fn bitstring_to_bits() {
        let bits = ShapeEncoding::bitstring_to_bits(&"01010".to_owned());
        assert_eq!(bits.len(), 5);
        let mut mbits = BitVec::new();
        mbits.push(false);
        mbits.push(true);
        mbits.push(false);
        mbits.push(true);
        mbits.push(false);
        assert_eq!(bits, mbits);
        let bitstring = "1100010110101111".to_owned();
        let bits = ShapeEncoding::bitstring_to_bits(&bitstring);
        assert_eq!(bits.len(), bitstring.len());
        assert_eq!(ShapeEncoding::bits_to_bitstring(&bits), bitstring);
    }

    #[test]
    fn encode_decode() {
        let size = 4;
        let mut image = BinaryImage::new_w_h(size, size);
        image.set_pixel(0, 0, true);
        image.set_pixel(1, 1, true);
        let sampler = Sampler::new_with_size(&image, size);
        let hexstring = ShapeEncoding::encode_as_shape_encoding(&sampler).hexstring();
        let decoded = ShapeEncoding::decode_from(&hexstring).image;
        assert_eq!(decoded.pixels, image.pixels);
        assert_eq!(decoded.get_pixel(0, 0), true);
        assert_eq!(decoded.get_pixel(1, 1), true);
        assert_eq!(decoded.get_pixel(2, 2), false);
        assert_eq!(decoded.get_pixel(3, 3), false);
    }

    #[test]
    fn binary_image_crop() {
        let mut image_a = BinaryImage::new_w_h(4, 4);
        image_a.set_pixel(0, 0, true);
        image_a.set_pixel(1, 1, true);
        let mut image_b = BinaryImage::new_w_h(2, 2);
        image_b.set_pixel(0, 0, true);
        image_b.set_pixel(1, 1, true);
        let cropped = image_a.crop();
        assert_eq!(cropped.pixels, image_b.pixels);
    }

    #[test]
    fn popcount() {
        assert_eq!(BinaryImage::popcount(1), 1);
        assert_eq!(BinaryImage::popcount(0b111000), 3);
        assert_eq!(BinaryImage::popcount(0b111000111), 6);
        assert_eq!(BinaryImage::popcount(0b111000111000111), 9);
    }

    #[test]
    fn encoding_diff() {
        assert_eq!(
            ShapeEncodingString::Bit("1".to_owned())
                .diff(&ShapeEncodingString::Bit("1".to_owned())),
            ShapeEncoding::remainder(1)
        );
        assert_eq!(
            ShapeEncodingString::Bit("10010".to_owned())
                .diff(&ShapeEncodingString::Bit("10001".to_owned())),
            (2 << 8) + ShapeEncoding::remainder(2)
        );
        assert_eq!(
            ShapeEncodingString::Bit("10010".to_owned())
                .diff(&ShapeEncodingString::Bit("00001".to_owned())),
            (1 << 8) + (2 << 8) + ShapeEncoding::remainder(2)
        );
        assert_eq!(
            ShapeEncodingString::Bit("1101010101010011100011100".to_owned()).diff(
                &ShapeEncodingString::Bit("1111010101011001100110101".to_owned())
            ),
            (1 << 8) + (5 << 8) + ShapeEncoding::remainder(3)
        );
    }
}
