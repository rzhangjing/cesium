//! Quaternion extension functions - CesiumJS-specific algorithms not in glam.
//!
//! Maps to CesiumJS `Core/Quaternion.js` extension methods:
//! computeAxis, computeAngle, log, exp, computeInnerQuadrangle, squad, fastSlerp, fastSquad

use glam::{DQuat, DVec3};

use crate::math_utils;

/// Computes the normalized rotation axis of a quaternion.
/// Maps to `Quaternion.computeAxis`.
pub fn compute_axis(quaternion: DQuat) -> DVec3 {
    let w = quaternion.w;
    if (w - 1.0).abs() < math_utils::EPSILON6 || (w + 1.0).abs() < math_utils::EPSILON6 {
        return DVec3::new(1.0, 0.0, 0.0);
    }

    let scalar = 1.0 / (1.0 - w * w).sqrt();
    DVec3::new(
        quaternion.x * scalar,
        quaternion.y * scalar,
        quaternion.z * scalar,
    )
}

/// Computes the angle of rotation of the provided quaternion.
/// Maps to `Quaternion.computeAngle`.
pub fn compute_angle(quaternion: DQuat) -> f64 {
    if (quaternion.w - 1.0).abs() < math_utils::EPSILON6 {
        return 0.0;
    }
    2.0 * quaternion.w.acos()
}

/// The logarithmic quaternion function.
/// Maps to `Quaternion.log`.
/// Returns the Cartesian3 (vector part) of the logarithm.
pub fn quaternion_log(quaternion: DQuat) -> DVec3 {
    let theta = math_utils::acos_clamped(quaternion.w);
    let mut theta_over_sin_theta = 0.0;

    if theta != 0.0 {
        theta_over_sin_theta = theta / theta.sin();
    }

    DVec3::new(
        quaternion.x * theta_over_sin_theta,
        quaternion.y * theta_over_sin_theta,
        quaternion.z * theta_over_sin_theta,
    )
}

/// The exponential quaternion function.
/// Maps to `Quaternion.exp`.
/// Takes a Cartesian3 (pure imaginary quaternion) and returns a unit quaternion.
pub fn quaternion_exp(cartesian: DVec3) -> DQuat {
    let theta = cartesian.length();
    let mut sin_theta_over_theta = 0.0;

    if theta != 0.0 {
        sin_theta_over_theta = theta.sin() / theta;
    }

    DQuat::from_xyzw(
        cartesian.x * sin_theta_over_theta,
        cartesian.y * sin_theta_over_theta,
        cartesian.z * sin_theta_over_theta,
        theta.cos(),
    )
}

/// Computes an inner quadrangle point.
/// This will compute quaternions that ensure a squad curve is C¹.
/// Maps to `Quaternion.computeInnerQuadrangle`.
pub fn compute_inner_quadrangle(q0: DQuat, q1: DQuat, q2: DQuat) -> DQuat {
    let q_inv = q1.conjugate();

    let product1 = q_inv * q2;
    let cart0 = quaternion_log(product1);

    let product2 = q_inv * q0;
    let cart1 = quaternion_log(product2);

    let sum = cart0 + cart1;
    let negated = sum * (-0.25);
    let exp_result = quaternion_exp(negated);

    q1 * exp_result
}

/// Computes the linear interpolation or extrapolation at t using the provided quaternions.
/// Maps to `Quaternion.lerp`.
pub fn quaternion_lerp(start: DQuat, end: DQuat, t: f64) -> DQuat {
    let scaled_end = DQuat::from_xyzw(
        end.x * t,
        end.y * t,
        end.z * t,
        end.w * t,
    );
    let scaled_start = DQuat::from_xyzw(
        start.x * (1.0 - t),
        start.y * (1.0 - t),
        start.z * (1.0 - t),
        start.w * (1.0 - t),
    );
    DQuat::from_xyzw(
        scaled_start.x + scaled_end.x,
        scaled_start.y + scaled_end.y,
        scaled_start.z + scaled_end.z,
        scaled_start.w + scaled_end.w,
    )
}

/// Computes the spherical linear interpolation or extrapolation at t.
/// Maps to `Quaternion.slerp` (CesiumJS version with lerp fallback).
pub fn cesium_slerp(start: DQuat, end: DQuat, t: f64) -> DQuat {
    let mut dot = start.x * end.x + start.y * end.y + start.z * end.z + start.w * end.w;

    // The angle between start must be acute. Since q and -q represent
    // the same rotation, negate q to get the acute angle.
    let mut r = end;
    if dot < 0.0 {
        dot = -dot;
        r = DQuat::from_xyzw(-end.x, -end.y, -end.z, -end.w);
    }

    // dot > 0, as the dot product approaches 1, the angle between the
    // quaternions vanishes. use linear interpolation.
    if 1.0 - dot < math_utils::EPSILON6 {
        return quaternion_lerp(start, r, t);
    }

    let theta = dot.acos();
    let sin_theta = theta.sin();
    let s0 = ((1.0 - t) * theta).sin() / sin_theta;
    let s1 = (t * theta).sin() / sin_theta;

    DQuat::from_xyzw(
        start.x * s0 + r.x * s1,
        start.y * s0 + r.y * s1,
        start.z * s0 + r.z * s1,
        start.w * s0 + r.w * s1,
    )
}

/// Computes the spherical quadrangle interpolation between quaternions.
/// Maps to `Quaternion.squad`.
pub fn squad(q0: DQuat, q1: DQuat, s0: DQuat, s1: DQuat, t: f64) -> DQuat {
    let slerp0 = cesium_slerp(q0, q1, t);
    let slerp1 = cesium_slerp(s0, s1, t);
    cesium_slerp(slerp0, slerp1, 2.0 * t * (1.0 - t))
}

// Constants for fastSlerp polynomial approximation
const OPMU: f64 = 1.90110745351730037;

/// Precomputed u and v arrays for fastSlerp.
fn fast_slerp_coefficients() -> ([f64; 8], [f64; 8]) {
    let mut u = [0.0f64; 8];
    let mut v = [0.0f64; 8];

    for i in 0..7 {
        let s = i as f64 + 1.0;
        let t = 2.0 * s + 1.0;
        u[i] = 1.0 / (s * t);
        v[i] = s / t;
    }

    u[7] = OPMU / (8.0 * 17.0);
    v[7] = (OPMU * 8.0) / 17.0;

    (u, v)
}

/// Computes the spherical linear interpolation or extrapolation at t.
/// This implementation is faster than slerp, but is only accurate up to 10⁻⁶.
/// Maps to `Quaternion.fastSlerp`.
pub fn fast_slerp(start: DQuat, end: DQuat, t: f64) -> DQuat {
    let (u, v) = fast_slerp_coefficients();

    let mut x = start.x * end.x + start.y * end.y + start.z * end.z + start.w * end.w;

    let sign;
    if x >= 0.0 {
        sign = 1.0;
    } else {
        sign = -1.0;
        x = -x;
    }

    let xm1 = x - 1.0;
    let d = 1.0 - t;
    let sqr_t = t * t;
    let sqr_d = d * d;

    let mut b_t = [0.0f64; 8];
    let mut b_d = [0.0f64; 8];

    for i in 0..8 {
        b_t[i] = (u[i] * sqr_t - v[i]) * xm1;
        b_d[i] = (u[i] * sqr_d - v[i]) * xm1;
    }

    let c_t = sign
        * t
        * (1.0
            + b_t[0]
                * (1.0
                    + b_t[1]
                        * (1.0
                            + b_t[2]
                                * (1.0
                                    + b_t[3]
                                        * (1.0
                                            + b_t[4]
                                                * (1.0
                                                    + b_t[5]
                                                        * (1.0 + b_t[6] * (1.0 + b_t[7]))))))));
    let c_d = d
        * (1.0
            + b_d[0]
                * (1.0
                    + b_d[1]
                        * (1.0
                            + b_d[2]
                                * (1.0
                                    + b_d[3]
                                        * (1.0
                                            + b_d[4]
                                                * (1.0
                                                    + b_d[5]
                                                        * (1.0 + b_d[6] * (1.0 + b_d[7]))))))));

    DQuat::from_xyzw(
        start.x * c_d + end.x * c_t,
        start.y * c_d + end.y * c_t,
        start.z * c_d + end.z * c_t,
        start.w * c_d + end.w * c_t,
    )
}

/// Computes the spherical quadrangle interpolation between quaternions.
/// An implementation that is faster than squad, but less accurate.
/// Maps to `Quaternion.fastSquad`.
pub fn fast_squad(q0: DQuat, q1: DQuat, s0: DQuat, s1: DQuat, t: f64) -> DQuat {
    let slerp0 = fast_slerp(q0, q1, t);
    let slerp1 = fast_slerp(s0, s1, t);
    fast_slerp(slerp0, slerp1, 2.0 * t * (1.0 - t))
}
