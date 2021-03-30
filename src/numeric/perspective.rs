use crate::PointF64;

use super::Numeric;

/// A perspective transform can easily be used to map one 2D quadrilateral to another, 
/// given the corner coordinates for the source and destination quadrilaterals.
///
/// Adapted from https://github.com/jlouthan/perspective-transform
pub struct PerspectiveTransform {
    src_pts: Vec<f64>,
    dst_pts: Vec<f64>,
    coeffs: Vec<f64>,
    coeffs_inv: Vec<f64>,
}

impl PerspectiveTransform {

    pub fn from_point_f64(src_pts: &[PointF64], dst_pts: &[PointF64]) -> Self {
        let mut src_f64 = vec![];
        let mut dst_f64 = vec![];

        for point in src_pts.into_iter() {
            src_f64.push(point.x);
            src_f64.push(point.y);
        }

        for point in dst_pts.into_iter() {
            dst_f64.push(point.x);
            dst_f64.push(point.y);
        }

        Self::new(src_f64, dst_f64)
    }

    pub fn new(src_pts: Vec<f64>, dst_pts: Vec<f64>) -> PerspectiveTransform {
        let coeffs = Self::get_normalization_coefficients(&src_pts, &dst_pts, false);
        let coeffs_inv = Self::get_normalization_coefficients(&src_pts, &dst_pts, true);
        PerspectiveTransform {
            src_pts,
            dst_pts,
            coeffs,
            coeffs_inv,
        }
    }

    pub fn default() -> PerspectiveTransform {
        PerspectiveTransform {
            src_pts: Vec::new(),
            dst_pts: Vec::new(),
            coeffs: Vec::new(),
            coeffs_inv: Vec::new(),
        }
    }

    fn get_normalization_coefficients(src_pts_in: &Vec<f64>, dst_pts_in: &Vec<f64>, is_inverse: bool) -> Vec<f64> {
        let (src_pts, dst_pts);
        if is_inverse {
            src_pts = dst_pts_in;
            dst_pts = src_pts_in;
        } else {
            src_pts = src_pts_in;
            dst_pts = dst_pts_in;
        }
        let r1 = vec![src_pts[0], src_pts[1], 1.0, 0.0, 0.0, 0.0, -1.0*dst_pts[0]*src_pts[0], -1.0*dst_pts[0]*src_pts[1]];
        let r2 = vec![0.0, 0.0, 0.0, src_pts[0], src_pts[1], 1.0, -1.0*dst_pts[1]*src_pts[0], -1.0*dst_pts[1]*src_pts[1]];
        let r3 = vec![src_pts[2], src_pts[3], 1.0, 0.0, 0.0, 0.0, -1.0*dst_pts[2]*src_pts[2], -1.0*dst_pts[2]*src_pts[3]];
        let r4 = vec![0.0, 0.0, 0.0, src_pts[2], src_pts[3], 1.0, -1.0*dst_pts[3]*src_pts[2], -1.0*dst_pts[3]*src_pts[3]];
        let r5 = vec![src_pts[4], src_pts[5], 1.0, 0.0, 0.0, 0.0, -1.0*dst_pts[4]*src_pts[4], -1.0*dst_pts[4]*src_pts[5]];
        let r6 = vec![0.0, 0.0, 0.0, src_pts[4], src_pts[5], 1.0, -1.0*dst_pts[5]*src_pts[4], -1.0*dst_pts[5]*src_pts[5]];
        let r7 = vec![src_pts[6], src_pts[7], 1.0, 0.0, 0.0, 0.0, -1.0*dst_pts[6]*src_pts[6], -1.0*dst_pts[6]*src_pts[7]];
        let r8 = vec![0.0, 0.0, 0.0, src_pts[6], src_pts[7], 1.0, -1.0*dst_pts[7]*src_pts[6], -1.0*dst_pts[7]*src_pts[7]];

        let mat_a = vec![r1, r2, r3, r4, r5, r6, r7, r8];
        let mat_b = dst_pts.clone();
        let mat_c;

        if let Some(mat) = Numeric::inv(&Numeric::dot_mm_small(&Numeric::transpose(&mat_a), &mat_a)) {
            mat_c = mat;
        } else {
            return vec![1.0,0.0,0.0,0.0, 1.0,0.0,0.0,0.0];
        }

        let mat_d = Numeric::dot_mm_small(&mat_c, &Numeric::transpose(&mat_a));
        let mut mat_x = Numeric::dot_mv(&mat_d, &mat_b);
        for i in 0..mat_x.len() {
            mat_x[i] = round(mat_x[i]);
        }
        mat_x.push(1.0);

        return mat_x;

        fn round(num: f64) -> f64 {
            (num*10000000000.0).round()/10000000000.0
        }
    }

    pub fn transform(&self, point: PointF64) -> PointF64 {
        let (x, y) = (point.x, point.y);
        PointF64 {
            x: (self.coeffs[0]*x + self.coeffs[1]*y + self.coeffs[2]) / (self.coeffs[6]*x + self.coeffs[7]*y + 1.0),
            y: (self.coeffs[3]*x + self.coeffs[4]*y + self.coeffs[5]) / (self.coeffs[6]*x + self.coeffs[7]*y + 1.0)
        }
    }

    pub fn transform_inverse(&self, point: PointF64) -> PointF64 {
        let (x, y) = (point.x, point.y);
        PointF64 {
            x: (self.coeffs_inv[0]*x + self.coeffs_inv[1]*y + self.coeffs_inv[2]) / (self.coeffs_inv[6]*x + self.coeffs_inv[7]*y + 1.0),
            y: (self.coeffs_inv[3]*x + self.coeffs_inv[4]*y + self.coeffs_inv[5]) / (self.coeffs_inv[6]*x + self.coeffs_inv[7]*y + 1.0)
        }
    }

    pub fn print_coeffs(&self) -> String {
        format!("{:?}", self.coeffs)
    }
}