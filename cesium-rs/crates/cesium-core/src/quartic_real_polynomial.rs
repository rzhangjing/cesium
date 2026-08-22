//! Ported from `packages/engine/Source/Core/QuarticRealPolynomial.js`.

use crate::cubic_real_polynomial::CubicRealPolynomial;
use crate::math::CesiumMath;
use crate::quadratic_real_polynomial::QuadraticRealPolynomial;

/// Functions for 4th order polynomial functions of one variable with only real coefficients.
pub struct QuarticRealPolynomial;

impl QuarticRealPolynomial {
    /// Provides the discriminant of the quartic equation.
    pub fn compute_discriminant(a: f64, b: f64, c: f64, d: f64, e: f64) -> f64 {
        let a2 = a * a;
        let a3 = a2 * a;
        let b2 = b * b;
        let b3 = b2 * b;
        let c2 = c * c;
        let c3 = c2 * c;
        let d2 = d * d;
        let d3 = d2 * d;
        let e2 = e * e;
        let e3 = e2 * e;

        b2 * c2 * d2 - 4.0 * b3 * d3 - 4.0 * a * c3 * d2
            + 18.0 * a * b * c * d3 - 27.0 * a2 * d2 * d2 + 256.0 * a3 * e3
            + e * (18.0 * b3 * c * d - 4.0 * b2 * c3 + 16.0 * a * c2 * c2
                   - 80.0 * a * b * c2 * d - 6.0 * a * b2 * d2
                   + 144.0 * a2 * c * d2)
            + e2 * (144.0 * a * b2 * c - 27.0 * b2 * b2 - 128.0 * a2 * c2
                    - 192.0 * a2 * b * d)
    }

    /// Provides the real valued roots of the quartic polynomial.
    pub fn compute_real_roots(a: f64, b: f64, c: f64, d: f64, e: f64) -> Vec<f64> {
        if a.abs() < CesiumMath::EPSILON15 {
            return CubicRealPolynomial::compute_real_roots(b, c, d, e);
        }

        let a3 = b / a;
        let a2 = c / a;
        let a1 = d / a;
        let a0 = e / a;

        let mut k = if a3 < 0.0 { 1 } else { 0 };
        k += if a2 < 0.0 { k + 1 } else { k };
        k += if a1 < 0.0 { k + 1 } else { k };
        k += if a0 < 0.0 { k + 1 } else { k };

        match k {
            0 | 3..=4 | 6..=7 | 9..=10 | 12..=15 => original(a3, a2, a1, a0),
            1 | 2 | 5 | 8 | 11 => neumark(a3, a2, a1, a0),
            _ => vec![],
        }
    }
}

fn original(a3: f64, a2: f64, a1: f64, a0: f64) -> Vec<f64> {
    let a3_squared = a3 * a3;

    let p = a2 - 3.0 * a3_squared / 8.0;
    let q = a1 - a2 * a3 / 2.0 + a3_squared * a3 / 8.0;
    let r = a0 - a1 * a3 / 4.0 + a2 * a3_squared / 16.0
        - 3.0 * a3_squared * a3_squared / 256.0;

    let cubic_roots = CubicRealPolynomial::compute_real_roots(
        1.0,
        2.0 * p,
        p * p - 4.0 * r,
        -q * q,
    );

    if !cubic_roots.is_empty() {
        let temp = -a3 / 4.0;
        let h_squared = cubic_roots[cubic_roots.len() - 1];

        if h_squared.abs() < CesiumMath::EPSILON14 {
            let roots = QuadraticRealPolynomial::compute_real_roots(1.0, p, r);
            if roots.len() == 2 {
                let root0 = roots[0];
                let root1 = roots[1];
                if root0 >= 0.0 && root1 >= 0.0 {
                    let y0 = root0.sqrt();
                    let y1 = root1.sqrt();
                    return vec![temp - y1, temp - y0, temp + y0, temp + y1];
                } else if root0 >= 0.0 && root1 < 0.0 {
                    let y = root0.sqrt();
                    return vec![temp - y, temp + y];
                } else if root0 < 0.0 && root1 >= 0.0 {
                    let y = root1.sqrt();
                    return vec![temp - y, temp + y];
                }
            }
            return vec![];
        } else if h_squared > 0.0 {
            let h = h_squared.sqrt();
            let m = (p + h_squared - q / h) / 2.0;
            let n = (p + h_squared + q / h) / 2.0;

            let mut roots1 = QuadraticRealPolynomial::compute_real_roots(1.0, h, m);
            let mut roots2 = QuadraticRealPolynomial::compute_real_roots(1.0, -h, n);

            if !roots1.is_empty() {
                roots1[0] += temp;
                roots1[1] += temp;

                if !roots2.is_empty() {
                    roots2[0] += temp;
                    roots2[1] += temp;

                    if roots1[1] <= roots2[0] {
                        return vec![roots1[0], roots1[1], roots2[0], roots2[1]];
                    } else if roots2[1] <= roots1[0] {
                        return vec![roots2[0], roots2[1], roots1[0], roots1[1]];
                    } else if roots1[0] >= roots2[0] && roots1[1] <= roots2[1] {
                        return vec![roots2[0], roots1[0], roots1[1], roots2[1]];
                    } else if roots2[0] >= roots1[0] && roots2[1] <= roots1[1] {
                        return vec![roots1[0], roots2[0], roots2[1], roots1[1]];
                    } else if roots1[0] > roots2[0] && roots1[0] < roots2[1] {
                        return vec![roots2[0], roots1[0], roots2[1], roots1[1]];
                    }
                    return vec![roots1[0], roots2[0], roots1[1], roots2[1]];
                }
                return roots1;
            }

            if !roots2.is_empty() {
                roots2[0] += temp;
                roots2[1] += temp;
                return roots2;
            }
        }
    }
    vec![]
}

fn neumark(a3: f64, a2: f64, a1: f64, a0: f64) -> Vec<f64> {
    let a1_squared = a1 * a1;
    let a2_squared = a2 * a2;
    let a3_squared = a3 * a3;

    let p = -2.0 * a2;
    let q = a1 * a3 + a2_squared - 4.0 * a0;
    let r = a3_squared * a0 - a1 * a2 * a3 + a1_squared;

    let cubic_roots = CubicRealPolynomial::compute_real_roots(1.0, p, q, r);

    if !cubic_roots.is_empty() {
        let y = cubic_roots[0];
        let temp = a2 - y;
        let temp_squared = temp * temp;

        let g1 = a3 / 2.0;
        let h1 = temp / 2.0;

        let m = temp_squared - 4.0 * a0;
        let m_error = temp_squared + 4.0 * a0.abs();
        let n = a3_squared - 4.0 * y;
        let n_error = a3_squared + 4.0 * y.abs();

        let (g2, h2);
        if y < 0.0 || m * n_error < n * m_error {
            let sqrt_n = n.sqrt();
            g2 = sqrt_n / 2.0;
            h2 = if sqrt_n == 0.0 { 0.0 } else { (a3 * h1 - a1) / sqrt_n };
        } else {
            let sqrt_m = m.sqrt();
            g2 = if sqrt_m == 0.0 { 0.0 } else { (a3 * h1 - a1) / sqrt_m };
            h2 = sqrt_m / 2.0;
        }

        let (big_g, little_g);
        if g1 == 0.0 && g2 == 0.0 {
            big_g = 0.0;
            little_g = 0.0;
        } else if CesiumMath::sign(g1) == CesiumMath::sign(g2) {
            big_g = g1 + g2;
            little_g = y / big_g;
        } else {
            little_g = g1 - g2;
            big_g = y / little_g;
        }

        let (big_h, little_h);
        if h1 == 0.0 && h2 == 0.0 {
            big_h = 0.0;
            little_h = 0.0;
        } else if CesiumMath::sign(h1) == CesiumMath::sign(h2) {
            big_h = h1 + h2;
            little_h = a0 / big_h;
        } else {
            little_h = h1 - h2;
            big_h = a0 / little_h;
        }

        let roots1 = QuadraticRealPolynomial::compute_real_roots(1.0, big_g, big_h);
        let roots2 = QuadraticRealPolynomial::compute_real_roots(1.0, little_g, little_h);

        if !roots1.is_empty() {
            if !roots2.is_empty() {
                if roots1[1] <= roots2[0] {
                    return vec![roots1[0], roots1[1], roots2[0], roots2[1]];
                } else if roots2[1] <= roots1[0] {
                    return vec![roots2[0], roots2[1], roots1[0], roots1[1]];
                } else if roots1[0] >= roots2[0] && roots1[1] <= roots2[1] {
                    return vec![roots2[0], roots1[0], roots1[1], roots2[1]];
                } else if roots2[0] >= roots1[0] && roots2[1] <= roots1[1] {
                    return vec![roots1[0], roots2[0], roots2[1], roots1[1]];
                } else if roots1[0] > roots2[0] && roots1[0] < roots2[1] {
                    return vec![roots2[0], roots1[0], roots2[1], roots1[1]];
                }
                return vec![roots1[0], roots2[0], roots1[1], roots2[1]];
            }
            return roots1;
        }
        if !roots2.is_empty() {
            return roots2;
        }
    }
    vec![]
}
