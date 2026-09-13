//! cesium-specs: Integration test suite ported from CesiumJS Specs.
//!
//! This crate provides test helper utilities and re-exports for the
//! integration test files under `tests/`.

/// Assert that two f64 values are approximately equal within epsilon.
///
/// Maps to CesiumJS `toEqualEpsilon(expected, epsilon)`.
#[macro_export]
macro_rules! assert_approx {
    ($a:expr, $b:expr, $eps:expr) => {
        let (a_val, b_val, eps_val) = ($a as f64, $b as f64, $eps as f64);
        assert!(
            (a_val - b_val).abs() <= eps_val,
            "assertion failed: |{} - {}| = {} > epsilon {}",
            a_val,
            b_val,
            (a_val - b_val).abs(),
            eps_val
        );
    };
}

/// Assert that two DVec3 values are approximately equal within epsilon.
///
/// Maps to CesiumJS `toEqualEpsilon` for Cartesian3.
#[macro_export]
macro_rules! assert_vec3_epsilon {
    ($a:expr, $b:expr, $eps:expr) => {
        let (a_vec, b_vec, eps_val) = ($a, $b, $eps as f64);
        let diff = a_vec - b_vec;
        assert!(
            diff.x.abs() <= eps_val && diff.y.abs() <= eps_val && diff.z.abs() <= eps_val,
            "assertion failed: DVec3 {:?} != {:?} within epsilon {}",
            a_vec,
            b_vec,
            eps_val
        );
    };
}

/// Assert that two DVec2 values are approximately equal within epsilon.
#[macro_export]
macro_rules! assert_vec2_epsilon {
    ($a:expr, $b:expr, $eps:expr) => {
        let (a_vec, b_vec, eps_val) = ($a, $b, $eps as f64);
        let diff = a_vec - b_vec;
        assert!(
            diff.x.abs() <= eps_val && diff.y.abs() <= eps_val,
            "assertion failed: DVec2 {:?} != {:?} within epsilon {}",
            a_vec,
            b_vec,
            eps_val
        );
    };
}

/// Assert that two DVec4 values are approximately equal within epsilon.
#[macro_export]
macro_rules! assert_vec4_epsilon {
    ($a:expr, $b:expr, $eps:expr) => {
        let (a_vec, b_vec, eps_val) = ($a, $b, $eps as f64);
        let diff = a_vec - b_vec;
        assert!(
            diff.x.abs() <= eps_val
                && diff.y.abs() <= eps_val
                && diff.z.abs() <= eps_val
                && diff.w.abs() <= eps_val,
            "assertion failed: DVec4 {:?} != {:?} within epsilon {}",
            a_vec,
            b_vec,
            eps_val
        );
    };
}

/// Assert that two DQuat values are approximately equal within epsilon.
#[macro_export]
macro_rules! assert_quat_epsilon {
    ($a:expr, $b:expr, $eps:expr) => {
        let (a_q, b_q, eps_val) = ($a, $b, $eps as f64);
        let dot = a_q.x * b_q.x + a_q.y * b_q.y + a_q.z * b_q.z + a_q.w * b_q.w;
        assert!(
            (dot.abs() - 1.0).abs() <= eps_val,
            "assertion failed: DQuat {:?} != {:?} within epsilon {}",
            a_q,
            b_q,
            eps_val
        );
    };
}

/// Assert that two DMat3 values are approximately equal within epsilon.
#[macro_export]
macro_rules! assert_mat3_epsilon {
    ($a:expr, $b:expr, $eps:expr) => {
        let (a_m, b_m, eps_val) = ($a, $b, $eps as f64);
        let diff = a_m - b_m;
        let max_diff = diff
            .to_cols_array()
            .iter()
            .fold(0.0f64, |mx, &v| mx.max(v.abs()));
        assert!(
            max_diff <= eps_val,
            "assertion failed: DMat3 max diff {} > epsilon {}",
            max_diff,
            eps_val
        );
    };
}

/// Assert that two DMat4 values are approximately equal within epsilon.
#[macro_export]
macro_rules! assert_mat4_epsilon {
    ($a:expr, $b:expr, $eps:expr) => {
        let (a_m, b_m, eps_val) = ($a, $b, $eps as f64);
        let diff = a_m - b_m;
        let max_diff = diff
            .to_cols_array()
            .iter()
            .fold(0.0f64, |mx, &v| mx.max(v.abs()));
        assert!(
            max_diff <= eps_val,
            "assertion failed: DMat4 max diff {} > epsilon {}",
            max_diff,
            eps_val
        );
    };
}

/// Common epsilon constants matching CesiumJS Math constants.
pub mod epsilon {
    pub const EPSILON1: f64 = 1e-1;
    pub const EPSILON2: f64 = 1e-2;
    pub const EPSILON3: f64 = 1e-3;
    pub const EPSILON4: f64 = 1e-4;
    pub const EPSILON5: f64 = 1e-5;
    pub const EPSILON6: f64 = 1e-6;
    pub const EPSILON7: f64 = 1e-7;
    pub const EPSILON8: f64 = 1e-8;
    pub const EPSILON9: f64 = 1e-9;
    pub const EPSILON10: f64 = 1e-10;
    pub const EPSILON11: f64 = 1e-11;
    pub const EPSILON12: f64 = 1e-12;
    pub const EPSILON13: f64 = 1e-13;
    pub const EPSILON14: f64 = 1e-14;
    pub const EPSILON15: f64 = 1e-15;
    pub const EPSILON16: f64 = 1e-16;
    pub const EPSILON17: f64 = 1e-17;
    pub const EPSILON18: f64 = 1e-18;
    pub const EPSILON19: f64 = 1e-19;
    pub const EPSILON20: f64 = 1e-20;
}

/// Common math constants matching CesiumJS Math.
pub mod math_consts {
    pub const PI: f64 = std::f64::consts::PI;
    pub const TWO_PI: f64 = std::f64::consts::TAU;
    pub const PI_OVER_TWO: f64 = std::f64::consts::FRAC_PI_2;
    pub const PI_OVER_THREE: f64 = std::f64::consts::PI / 3.0;
    pub const PI_OVER_FOUR: f64 = std::f64::consts::FRAC_PI_4;
    pub const PI_OVER_SIX: f64 = std::f64::consts::PI / 6.0;
    pub const RADIANS_PER_DEGREE: f64 = std::f64::consts::PI / 180.0;
    pub const DEGREES_PER_RADIAN: f64 = 180.0 / std::f64::consts::PI;
}

/// Helper to convert degrees to radians.
pub fn to_radians(degrees: f64) -> f64 {
    degrees * math_consts::RADIANS_PER_DEGREE
}

/// Helper to convert radians to degrees.
pub fn to_degrees(radians: f64) -> f64 {
    radians * math_consts::DEGREES_PER_RADIAN
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;

    #[test]
    fn test_assert_approx_macro() {
        assert_approx!(1.0, 1.0 + 1e-16, epsilon::EPSILON15);
    }

    #[test]
    fn test_assert_vec3_epsilon_macro() {
        let a = DVec3::new(1.0, 2.0, 3.0);
        let b = DVec3::new(1.0 + 1e-16, 2.0 - 1e-16, 3.0);
        assert_vec3_epsilon!(a, b, epsilon::EPSILON15);
    }

    #[test]
    fn test_to_radians() {
        assert_approx!(to_radians(180.0), math_consts::PI, epsilon::EPSILON15);
        assert_approx!(to_radians(90.0), math_consts::PI_OVER_TWO, epsilon::EPSILON15);
    }

    #[test]
    fn test_to_degrees() {
        assert_approx!(to_degrees(math_consts::PI), 180.0, epsilon::EPSILON13);
    }
}
