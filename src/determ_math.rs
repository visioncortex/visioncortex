//! Deterministic cross-platform transcendental math.
//!
//! The default platform libm (Microsoft CRT on Windows, Apple libsystem on
//! macOS, glibc/musl on Linux) ships different transcendental implementations
//! whose results disagree at ULP level on some inputs. IEEE 754 does not
//! guarantee bit-exact transcendentals.
//!
//! This module defines `DetermMath`, a trait that dispatches `sin`/`cos`/
//! `atan2`/`acos` calls through [`libm`] when the `deterministic-math`
//! feature is enabled, producing bit-identical output on every target.
//! With the feature off (default), it delegates to the standard library —
//! zero behavior change for existing users.
//!
//! `sqrt` is not exposed here because IEEE 754 mandates a correctly-rounded
//! `sqrt` on all modern platforms; `std::f*::sqrt` is already bit-identical.

pub trait DetermMath: Copy {
    fn determ_sin(self) -> Self;
    fn determ_cos(self) -> Self;
    fn determ_atan2(self, x: Self) -> Self;
    fn determ_acos(self) -> Self;
}

#[cfg(feature = "deterministic-math")]
impl DetermMath for f32 {
    #[inline] fn determ_sin(self) -> Self { libm::sinf(self) }
    #[inline] fn determ_cos(self) -> Self { libm::cosf(self) }
    #[inline] fn determ_atan2(self, x: Self) -> Self { libm::atan2f(self, x) }
    #[inline] fn determ_acos(self) -> Self { libm::acosf(self) }
}

#[cfg(not(feature = "deterministic-math"))]
impl DetermMath for f32 {
    #[inline] fn determ_sin(self) -> Self { self.sin() }
    #[inline] fn determ_cos(self) -> Self { self.cos() }
    #[inline] fn determ_atan2(self, x: Self) -> Self { self.atan2(x) }
    #[inline] fn determ_acos(self) -> Self { self.acos() }
}

#[cfg(feature = "deterministic-math")]
impl DetermMath for f64 {
    #[inline] fn determ_sin(self) -> Self { libm::sin(self) }
    #[inline] fn determ_cos(self) -> Self { libm::cos(self) }
    #[inline] fn determ_atan2(self, x: Self) -> Self { libm::atan2(self, x) }
    #[inline] fn determ_acos(self) -> Self { libm::acos(self) }
}

#[cfg(not(feature = "deterministic-math"))]
impl DetermMath for f64 {
    #[inline] fn determ_sin(self) -> Self { self.sin() }
    #[inline] fn determ_cos(self) -> Self { self.cos() }
    #[inline] fn determ_atan2(self, x: Self) -> Self { self.atan2(x) }
    #[inline] fn determ_acos(self) -> Self { self.acos() }
}

#[cfg(all(test, feature = "deterministic-math"))]
mod tests {
    use super::*;

    // Bit-identity lock tests — these assert the feature-gated path
    // produces exactly what libm produces. Run via:
    //   cargo test --features deterministic-math determ_math

    #[test]
    fn f64_sin_locked() {
        let r = 1.0_f64.determ_sin();
        assert_eq!(r.to_bits(), libm::sin(1.0).to_bits());
    }

    #[test]
    fn f64_cos_locked() {
        let r = 1.0_f64.determ_cos();
        assert_eq!(r.to_bits(), libm::cos(1.0).to_bits());
    }

    #[test]
    fn f64_atan2_locked() {
        let r = 1.0_f64.determ_atan2(2.0);
        assert_eq!(r.to_bits(), libm::atan2(1.0, 2.0).to_bits());
    }

    #[test]
    fn f64_acos_locked() {
        let r = 0.5_f64.determ_acos();
        assert_eq!(r.to_bits(), libm::acos(0.5).to_bits());
    }

    #[test]
    fn f32_sin_locked() {
        let r = 1.0_f32.determ_sin();
        assert_eq!(r.to_bits(), libm::sinf(1.0).to_bits());
    }

    #[test]
    fn f32_cos_locked() {
        let r = 1.0_f32.determ_cos();
        assert_eq!(r.to_bits(), libm::cosf(1.0).to_bits());
    }

    #[test]
    fn f32_atan2_locked() {
        let r = 1.0_f32.determ_atan2(2.0);
        assert_eq!(r.to_bits(), libm::atan2f(1.0, 2.0).to_bits());
    }

    #[test]
    fn f32_acos_locked() {
        let r = 0.5_f32.determ_acos();
        assert_eq!(r.to_bits(), libm::acosf(0.5).to_bits());
    }

    /// Targeted subnormal / boundary-value check. These are the classic
    /// inputs where Microsoft CRT and Apple libsystem have historically
    /// disagreed — locking against libm eliminates the drift source.
    #[test]
    fn f64_atan2_boundary_values_locked() {
        let cases = [
            (0.0_f64, 1.0_f64),
            (1.0, 0.0),
            (1.0, 1.0),
            (-1.0, 1.0),
            (1.0e-300, 1.0),
            (f64::MIN_POSITIVE, 1.0),
        ];
        for (y, x) in cases {
            let r = y.determ_atan2(x);
            assert_eq!(
                r.to_bits(),
                libm::atan2(y, x).to_bits(),
                "atan2({y}, {x})"
            );
        }
    }
}
