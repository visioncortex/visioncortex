use std::fmt::{Debug};

/// Matrix operations adapted from https://github.com/sloisel/numeric
#[derive(Clone, PartialEq)]
pub struct Matrix<const I: usize, const J: usize> {
    pub m: [[f64; J]; I],
}

impl<const I: usize, const J: usize> Default for Matrix<I, J> {
    fn default() -> Self {
        Self {
            m: [[0.; J]; I],
        }
    }
}

impl<const I: usize, const J: usize> Matrix<I, J> {
    pub fn new(m: [[f64; J]; I]) -> Self {
        Self { m }
    }

    pub fn dim(&self) -> [usize; 2] {
        return [I, J];
    }

    pub fn transpose(&self) -> Matrix<J, I> {
        let mut m = Matrix::default();
        for i in 0..I {
            for j in 0..J {
                m.m[j][i] = self.m[i][j];
            }
        }
        m
    }
}

/// Only for square matrix
impl<const I: usize> Matrix<I, I> {
    pub fn identity() -> Self {
        let mut m = Matrix::default();
        for i in 0..I {
            m.m[i][i] = 1.0;
        }
        m
    }

    pub fn inv(&self) -> Option<Self> {
        let mut mx = self.clone();
        let mx = &mut mx.m;
        let mut ret = Self::identity();
        let ii = &mut ret.m;
        for j in 0..I {
            let mut i0 = 0;
            let mut v0 = -1.0;
            for i in j..I {
                let k = (mx[i][j]).abs();
                if k > v0 {
                    i0 = i;
                    v0 = k;
                }
            }
            mx.swap(i0, j);
            ii.swap(i0, j);
            let x = mx[j][j];
            if x == 0.0 {
                return None;
            }
            for k in j..I {
                mx[j][k] /= x; 
            }
            for k in (0..I).rev() {
                ii[j][k] /= x;
            }
            for i in (0..I).rev() {
                if i != j {
                    let x = mx[i][j];
                    for k in j+1..I {
                        mx[i][k] -= mx[j][k]*x;
                    }
                    let mut k = I as i32 - 1;
                    while k > 0 {
                        ii[i][k as usize] -= ii[j][k as usize]*x;
                        k -= 1;
                        ii[i][k as usize] -= ii[j][k as usize]*x;
                        k -= 1;
                    }
                    if k == 0 {
                        ii[i][0] -= ii[j][0]*x;
                    }
                }
            }
        }
        Some(ret)
    }
}

impl<const I: usize, const J: usize> Matrix<I, J> {
    /// Multiply two matrices
    pub fn dot_mm_small<const K: usize>(&self, y: &Matrix<J, K>) -> Matrix<I, K> {
        let y = &y.m;
        let mut ret = Matrix::default();
        for i in (0..I).rev() {
            let mut foo = [0.0; K];
            let bar = &self.m[i];
            for k in (0..K).rev() {
                let mut woo = bar[J-1]*y[J-1][k];
                let mut j = J as i32 - 2;
                while j >= 1 {
                    let i0 = j-1;
                    woo += bar[j as usize]*y[j as usize][k] + bar[i0 as usize]*y[i0 as usize][k];
                    j -= 2;
                }
                if j == 0 {
                    woo += bar[0]*y[0][k];
                }
                foo[k] = woo;
            }
            ret.m[i] = foo;
        }
        ret
    }

    /// Multiply a matrix to a vector
    pub fn dot_mv(&self, y: &[f64; J]) -> [f64; I] {
        let mut ret = [0.0; I];
        for i in (0..I).rev() {
            ret[i] = Self::dot_vv(&self.m[i], y);
        }
        ret
    }

    pub fn dot_vv<const K: usize>(x: &[f64; K], y: &[f64; K]) -> f64 {
        let mut ret = x[K-1]*y[K-1];
        let mut i = K as i32 - 2;
        while i >= 1 {
            let i1 = i-1;
            ret += x[i as usize]*y[i as usize] + x[i1 as usize]*y[i1 as usize];
            i -= 2;
        }
        if i == 0 {
            ret += x[0]*y[0];
        }
        ret
    }

    pub fn eq(&self, y: &Self, epsilon: f64) -> bool {
        let x = &self.m;
        let y = &y.m;
        for i in 0..I {
            for j in 0..J {
                if (x[i][j] - y[i][j]).abs() > epsilon {
                    return false;
                }
            }
        }
        true
    }

    pub fn scale(&mut self, s: f64) {
        let x = &mut self.m;
        for i in 0..I {
            for j in 0..J {
                x[i][j] *= s;
            }
        }
    }
}

impl<const I: usize, const J: usize> Debug for Matrix<I, J> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Matrix([")?;
        for i in 0..I {
            writeln!(f, "    {:?},", self.m[i])?;
        }
        write!(f, "])")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_matrix_display() {
        assert_eq!(
            &format!("{:?}", Matrix::<3, 3>::identity()),
            r#"Matrix([
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
])"#
        );
    }

    #[test]
    fn test_matrix_inverse() {
        let m1 = Matrix::new([
            [1., 0., 0.],
            [3., 5., 0.],
            [2., 1., 8.],
        ]);
        let mut m2 = Matrix::new([
            [40.,  0., 0.],
            [-24., 8., 0.],
            [-7., -1., 5.],
        ]);
        m2.scale(1./40.);
        assert!(m1.inv().unwrap().eq(&m2, 1e-7));

        let m1 = Matrix::new([
            [2., 0., 5.],
            [3., 1., 3.],
            [9., 5., 9.],
        ]);
        let mut m2 = Matrix::new([
            [-6.,  25., -5.],
            [ 0., -27.,  9.],
            [ 6., -10.,  2.],
        ]);
        m2.scale(1./18.);
        assert!(m1.inv().unwrap().eq(&m2, 1e-7));
    }
}