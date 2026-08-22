//! Ported from `packages/engine/Source/Core/QuadraticRealPolynomial.js`.

use crate::math::CesiumMath;

/// Functions for 2nd order polynomial functions of one variable with only real coefficients.
pub struct QuadraticRealPolynomial;

impl QuadraticRealPolynomial {
    /// Provides the discriminant of the quadratic equation.
    pub fn compute_discriminant(a: f64, b: f64, c: f64) -> f64 {
        b * b - 4.0 * a * c
    }

    /// Provides the real valued roots of the quadratic polynomial.
    pub fn compute_real_roots(a: f64, b: f64, c: f64) -> Vec<f64> {
        if a == 0.0 {
            if b == 0.0 {
                return vec![];
            }
            return vec![-c / b];
        } else if b == 0.0 {
            if c == 0.0 {
                return vec![0.0, 0.0];
            }
            let c_mag = c.abs();
            let a_mag = a.abs();
            if c_mag < a_mag && c_mag / a_mag < CesiumMath::EPSILON14 {
                return vec![0.0, 0.0];
            } else if c_mag > a_mag && a_mag / c_mag < CesiumMath::EPSILON14 {
                return vec![];
            }
            let ratio = -c / a;
            if ratio < 0.0 {
                return vec![];
            }
            let root = ratio.sqrt();
            return vec![-root, root];
        } else if c == 0.0 {
            let ratio = -b / a;
            if ratio < 0.0 {
                return vec![ratio, 0.0];
            }
            return vec![0.0, ratio];
        }

        let b2 = b * b;
        let four_ac = 4.0 * a * c;
        let radicand = add_with_cancellation_check(b2, -four_ac, CesiumMath::EPSILON14);

        if radicand < 0.0 {
            return vec![];
        }

        let q = -0.5 * add_with_cancellation_check(
            b,
            CesiumMath::sign(b) * radicand.sqrt(),
            CesiumMath::EPSILON14,
        );

        if b > 0.0 {
            vec![q / a, c / q]
        } else {
            vec![c / q, q / a]
        }
    }
}

fn add_with_cancellation_check(left: f64, right: f64, tolerance: f64) -> f64 {
    let difference = left + right;
    if CesiumMath::sign(left) != CesiumMath::sign(right)
        && (difference / left.abs().max(right.abs())).abs() < tolerance
    {
        return 0.0;
    }
    difference
}
