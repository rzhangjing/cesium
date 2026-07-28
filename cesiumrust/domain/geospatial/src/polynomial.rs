//! Polynomial root-finding algorithms.
//! Maps to CesiumJS `Core/QuadraticRealPolynomial.js`, `Core/CubicRealPolynomial.js`,
//! `Core/QuarticRealPolynomial.js`

use crate::math_utils::{sign, EPSILON14, EPSILON15};

// --- Helper ---

/// Adds two values with cancellation check.
/// If left and right have opposite signs and the result is negligibly small relative
/// to the larger operand, returns 0.0.
fn add_with_cancellation_check(left: f64, right: f64, tolerance: f64) -> f64 {
    let difference = left + right;
    if sign(left) != sign(right)
        && (difference / f64::max(left.abs(), right.abs())).abs() < tolerance
    {
        return 0.0;
    }
    difference
}

// =============================================================================
// QuadraticRealPolynomial
// =============================================================================

/// Provides the discriminant of the quadratic equation: b² - 4ac.
/// Maps to `QuadraticRealPolynomial.computeDiscriminant`
pub fn quadratic_discriminant(a: f64, b: f64, c: f64) -> f64 {
    b * b - 4.0 * a * c
}

/// Provides the real valued roots of the quadratic polynomial ax² + bx + c = 0.
/// Returns roots in ascending order.
/// Maps to `QuadraticRealPolynomial.computeRealRoots`
pub fn quadratic_real_roots(a: f64, b: f64, c: f64) -> Vec<f64> {
    if a == 0.0 {
        if b == 0.0 {
            // Constant function: c = 0.
            return vec![];
        }
        // Linear function: b * x + c = 0.
        return vec![-c / b];
    } else if b == 0.0 {
        if c == 0.0 {
            // 2nd order monomial: a * x^2 = 0.
            return vec![0.0, 0.0];
        }

        let c_magnitude = c.abs();
        let a_magnitude = a.abs();

        if c_magnitude < a_magnitude && c_magnitude / a_magnitude < EPSILON14 {
            // c ~= 0.0 → a * x^2 = 0.
            return vec![0.0, 0.0];
        } else if c_magnitude > a_magnitude && a_magnitude / c_magnitude < EPSILON14 {
            // a ~= 0.0 → Constant function.
            return vec![];
        }

        // a * x^2 + c = 0
        let ratio = -c / a;
        if ratio < 0.0 {
            return vec![];
        }
        let root = ratio.sqrt();
        return vec![-root, root];
    } else if c == 0.0 {
        // a * x^2 + b * x = 0
        let ratio = -b / a;
        if ratio < 0.0 {
            return vec![ratio, 0.0];
        }
        return vec![0.0, ratio];
    }

    // a * x^2 + b * x + c = 0
    let b2 = b * b;
    let four_ac = 4.0 * a * c;
    let radicand = add_with_cancellation_check(b2, -four_ac, EPSILON14);

    if radicand < 0.0 {
        return vec![];
    }

    let q = -0.5 * add_with_cancellation_check(b, sign(b) * radicand.sqrt(), EPSILON14);
    if b > 0.0 {
        return vec![q / a, c / q];
    }
    vec![c / q, q / a]
}

// =============================================================================
// CubicRealPolynomial
// =============================================================================

/// Provides the discriminant of the cubic equation.
/// Maps to `CubicRealPolynomial.computeDiscriminant`
pub fn cubic_discriminant(a: f64, b: f64, c: f64, d: f64) -> f64 {
    let a2 = a * a;
    let b2 = b * b;
    let c2 = c * c;
    let d2 = d * d;

    18.0 * a * b * c * d + b2 * c2 - 27.0 * a2 * d2 - 4.0 * (a * c2 * c + b2 * b * d)
}

/// Internal cubic root solver (general case).
fn cubic_compute_real_roots_internal(a: f64, b: f64, c: f64, d: f64) -> Vec<f64> {
    let big_a = a;
    let big_b = b / 3.0;
    let big_c = c / 3.0;
    let big_d = d;

    let ac = big_a * big_c;
    let bd = big_b * big_d;
    let b2 = big_b * big_b;
    let c2 = big_c * big_c;
    let delta1 = big_a * big_c - b2;
    let delta2 = big_a * big_d - big_b * big_c;
    let delta3 = big_b * big_d - c2;

    let discriminant = 4.0 * delta1 * delta3 - delta2 * delta2;

    if discriminant < 0.0 {
        let a_bar: f64;
        let c_bar: f64;
        let d_bar: f64;

        if b2 * bd >= ac * c2 {
            a_bar = big_a;
            c_bar = delta1;
            d_bar = -2.0 * big_b * delta1 + big_a * delta2;
        } else {
            a_bar = big_d;
            c_bar = delta3;
            d_bar = -big_d * delta2 + 2.0 * big_c * delta3;
        }

        let s = if d_bar < 0.0 { -1.0 } else { 1.0 }; // Not Math.sign!
        let temp0 = -s * a_bar.abs() * (-discriminant).sqrt();
        let temp1 = -d_bar + temp0;

        let x = temp1 / 2.0;
        let p = if x < 0.0 {
            -(-x).powf(1.0 / 3.0)
        } else {
            x.powf(1.0 / 3.0)
        };
        let q = if temp1 == temp0 { -p } else { -c_bar / p };

        let temp = if c_bar <= 0.0 {
            p + q
        } else {
            -d_bar / (p * p + q * q + c_bar)
        };

        if b2 * bd >= ac * c2 {
            return vec![(temp - big_b) / big_a];
        }
        return vec![-big_d / (temp + big_c)];
    }

    let c_bar_a = delta1;
    let d_bar_a = -2.0 * big_b * delta1 + big_a * delta2;

    let c_bar_d = delta3;
    let d_bar_d = -big_d * delta2 + 2.0 * big_c * delta3;

    let square_root_of_discriminant = discriminant.sqrt();
    let half_square_root_of_3 = 3.0_f64.sqrt() / 2.0;

    let mut theta = (big_a * square_root_of_discriminant).atan2(-d_bar_a).abs() / 3.0;
    let mut temp = 2.0 * (-c_bar_a).sqrt();
    let mut cosine = theta.cos();
    let mut temp1 = temp * cosine;
    let mut temp3 = temp * (-cosine / 2.0 - half_square_root_of_3 * theta.sin());

    let numerator_large = if temp1 + temp3 > 2.0 * big_b {
        temp1 - big_b
    } else {
        temp3 - big_b
    };
    let denominator_large = big_a;

    let root1 = numerator_large / denominator_large;

    theta = (big_d * square_root_of_discriminant).atan2(-d_bar_d).abs() / 3.0;
    temp = 2.0 * (-c_bar_d).sqrt();
    cosine = theta.cos();
    temp1 = temp * cosine;
    temp3 = temp * (-cosine / 2.0 - half_square_root_of_3 * theta.sin());

    let numerator_small = -big_d;
    let denominator_small = if temp1 + temp3 < 2.0 * big_c {
        temp1 + big_c
    } else {
        temp3 + big_c
    };

    let root3 = numerator_small / denominator_small;

    let e = denominator_large * denominator_small;
    let f = -numerator_large * denominator_small - denominator_large * numerator_small;
    let g = numerator_large * numerator_small;

    let root2 = (big_c * f - big_b * g) / (-big_b * f + big_c * e);

    // Sort roots
    let mut roots = vec![root1, root2, root3];
    roots.sort_by(|a, b| a.partial_cmp(b).unwrap());
    roots
}

/// Provides the real valued roots of the cubic polynomial ax³ + bx² + cx + d = 0.
/// Returns roots in ascending order.
/// Maps to `CubicRealPolynomial.computeRealRoots`
pub fn cubic_real_roots(a: f64, b: f64, c: f64, d: f64) -> Vec<f64> {
    if a == 0.0 {
        // Quadratic: b * x^2 + c * x + d = 0.
        return quadratic_real_roots(b, c, d);
    } else if b == 0.0 {
        if c == 0.0 {
            if d == 0.0 {
                // 3rd order monomial: a * x^3 = 0.
                return vec![0.0, 0.0, 0.0];
            }
            // a * x^3 + d = 0
            let ratio = -d / a;
            let root = if ratio < 0.0 {
                -(-ratio).powf(1.0 / 3.0)
            } else {
                ratio.powf(1.0 / 3.0)
            };
            return vec![root, root, root];
        } else if d == 0.0 {
            // x * (a * x^2 + c) = 0.
            let roots = quadratic_real_roots(a, 0.0, c);
            if roots.is_empty() {
                return vec![0.0];
            }
            return vec![roots[0], 0.0, roots[1]];
        }
        // Deflated cubic: a * x^3 + c * x + d = 0.
        return cubic_compute_real_roots_internal(a, 0.0, c, d);
    } else if c == 0.0 {
        if d == 0.0 {
            // x^2 * (a * x + b) = 0.
            let ratio = -b / a;
            if ratio < 0.0 {
                return vec![ratio, 0.0, 0.0];
            }
            return vec![0.0, 0.0, ratio];
        }
        // a * x^3 + b * x^2 + d = 0.
        return cubic_compute_real_roots_internal(a, b, 0.0, d);
    } else if d == 0.0 {
        // x * (a * x^2 + b * x + c) = 0
        let roots = quadratic_real_roots(a, b, c);
        if roots.is_empty() {
            return vec![0.0];
        } else if roots.len() >= 2 && roots[1] <= 0.0 {
            return vec![roots[0], roots[1], 0.0];
        } else if roots[0] >= 0.0 {
            return vec![0.0, roots[0], roots[1]];
        }
        return vec![roots[0], 0.0, roots[1]];
    }

    cubic_compute_real_roots_internal(a, b, c, d)
}

// =============================================================================
// QuarticRealPolynomial
// =============================================================================

/// Provides the discriminant of the quartic equation.
/// Maps to `QuarticRealPolynomial.computeDiscriminant`
pub fn quartic_discriminant(a: f64, b: f64, c: f64, d: f64, e: f64) -> f64 {
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

    b2 * c2 * d2 - 4.0 * b3 * d3 - 4.0 * a * c3 * d2 + 18.0 * a * b * c * d3
        - 27.0 * a2 * d2 * d2
        + 256.0 * a3 * e3
        + e * (18.0 * b3 * c * d - 4.0 * b2 * c3 + 16.0 * a * c2 * c2
            - 80.0 * a * b * c2 * d
            - 6.0 * a * b2 * d2
            + 144.0 * a2 * c * d2)
        + e2 * (144.0 * a * b2 * c - 27.0 * b2 * b2 - 128.0 * a2 * c2 - 192.0 * a2 * b * d)
}

/// Merges two sorted root arrays into a single sorted array.
fn merge_roots(roots1: &mut Vec<f64>, roots2: &mut Vec<f64>) -> Vec<f64> {
    if roots1.is_empty() {
        return roots2.clone();
    }
    if roots2.is_empty() {
        return roots1.clone();
    }

    if roots1[roots1.len() - 1] <= roots2[0] {
        let mut r = roots1.clone();
        r.extend_from_slice(roots2);
        return r;
    } else if roots2[roots2.len() - 1] <= roots1[0] {
        let mut r = roots2.clone();
        r.extend_from_slice(roots1);
        return r;
    } else if roots1[0] >= roots2[0] && roots1[roots1.len() - 1] <= roots2[roots2.len() - 1] {
        // roots1 nested inside roots2
        return vec![roots2[0], roots1[0], roots1[1], roots2[1]];
    } else if roots2[0] >= roots1[0] && roots2[roots2.len() - 1] <= roots1[roots1.len() - 1] {
        // roots2 nested inside roots1
        return vec![roots1[0], roots2[0], roots2[1], roots1[1]];
    } else if roots1[0] > roots2[0] && roots1[0] < roots2[roots2.len() - 1] {
        return vec![roots2[0], roots1[0], roots2[1], roots1[1]];
    }
    vec![roots1[0], roots2[0], roots1[1], roots2[1]]
}

/// Original quartic solver (Ferrari's method variant).
fn quartic_original(a3: f64, a2: f64, a1: f64, a0: f64) -> Vec<f64> {
    let a3_squared = a3 * a3;

    let p = a2 - (3.0 * a3_squared) / 8.0;
    let q = a1 - (a2 * a3) / 2.0 + (a3_squared * a3) / 8.0;
    let r = a0 - (a1 * a3) / 4.0 + (a2 * a3_squared) / 16.0
        - (3.0 * a3_squared * a3_squared) / 256.0;

    // Find roots of: h^6 + 2p h^4 + (p^2 - 4r) h^2 - q^2 = 0
    let cubic_roots = cubic_real_roots(1.0, 2.0 * p, p * p - 4.0 * r, -q * q);

    if !cubic_roots.is_empty() {
        let temp = -a3 / 4.0;
        let h_squared = cubic_roots[cubic_roots.len() - 1];

        if h_squared.abs() < EPSILON14 {
            // y^4 + p y^2 + r = 0
            let roots = quadratic_real_roots(1.0, p, r);
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

            let mut roots1 = quadratic_real_roots(1.0, h, m);
            let mut roots2 = quadratic_real_roots(1.0, -h, n);

            if !roots1.is_empty() {
                for r in roots1.iter_mut() {
                    *r += temp;
                }
                if !roots2.is_empty() {
                    for r in roots2.iter_mut() {
                        *r += temp;
                    }
                    return merge_roots(&mut roots1, &mut roots2);
                }
                return roots1;
            }
            if !roots2.is_empty() {
                for r in roots2.iter_mut() {
                    *r += temp;
                }
                return roots2;
            }
            return vec![];
        }
    }
    vec![]
}

/// Neumark's quartic solver.
fn quartic_neumark(a3: f64, a2: f64, a1: f64, a0: f64) -> Vec<f64> {
    let a1_squared = a1 * a1;
    let a2_squared = a2 * a2;
    let a3_squared = a3 * a3;

    let p = -2.0 * a2;
    let q = a1 * a3 + a2_squared - 4.0 * a0;
    let r = a3_squared * a0 - a1 * a2 * a3 + a1_squared;

    let cubic_roots = cubic_real_roots(1.0, p, q, r);

    if !cubic_roots.is_empty() {
        // Use the most positive root
        let y = cubic_roots[0];

        let temp = a2 - y;
        let temp_squared = temp * temp;

        let g1 = a3 / 2.0;
        let h1 = temp / 2.0;

        let m = temp_squared - 4.0 * a0;
        let m_error = temp_squared + 4.0 * a0.abs();

        let n = a3_squared - 4.0 * y;
        let n_error = a3_squared + 4.0 * y.abs();

        let g2: f64;
        let h2: f64;

        if y < 0.0 || m * n_error < n * m_error {
            let square_root_of_n = n.sqrt();
            g2 = square_root_of_n / 2.0;
            h2 = if square_root_of_n == 0.0 {
                0.0
            } else {
                (a3 * h1 - a1) / square_root_of_n
            };
        } else {
            let square_root_of_m = m.sqrt();
            g2 = if square_root_of_m == 0.0 {
                0.0
            } else {
                (a3 * h1 - a1) / square_root_of_m
            };
            h2 = square_root_of_m / 2.0;
        }

        let (big_g, small_g) = if g1 == 0.0 && g2 == 0.0 {
            (0.0, 0.0)
        } else if sign(g1) == sign(g2) {
            let big_g = g1 + g2;
            (big_g, y / big_g)
        } else {
            let small_g = g1 - g2;
            (y / small_g, small_g)
        };

        let (big_h, small_h) = if h1 == 0.0 && h2 == 0.0 {
            (0.0, 0.0)
        } else if sign(h1) == sign(h2) {
            let big_h = h1 + h2;
            (big_h, a0 / big_h)
        } else {
            let small_h = h1 - h2;
            (a0 / small_h, small_h)
        };

        let mut roots1 = quadratic_real_roots(1.0, big_g, big_h);
        let mut roots2 = quadratic_real_roots(1.0, small_g, small_h);

        if !roots1.is_empty() {
            if !roots2.is_empty() {
                return merge_roots(&mut roots1, &mut roots2);
            }
            return roots1;
        }
        if !roots2.is_empty() {
            return roots2;
        }
    }
    vec![]
}

/// Provides the real valued roots of the quartic polynomial ax⁴ + bx³ + cx² + dx + e = 0.
/// Returns roots in ascending order.
/// Maps to `QuarticRealPolynomial.computeRealRoots`
pub fn quartic_real_roots(a: f64, b: f64, c: f64, d: f64, e: f64) -> Vec<f64> {
    if a.abs() < EPSILON15 {
        return cubic_real_roots(b, c, d, e);
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
        0 => quartic_original(a3, a2, a1, a0),
        1 | 2 => quartic_neumark(a3, a2, a1, a0),
        3 | 4 => quartic_original(a3, a2, a1, a0),
        5 => quartic_neumark(a3, a2, a1, a0),
        6 | 7 => quartic_original(a3, a2, a1, a0),
        8 => quartic_neumark(a3, a2, a1, a0),
        9 | 10 => quartic_original(a3, a2, a1, a0),
        11 => quartic_neumark(a3, a2, a1, a0),
        12 | 13 | 14 | 15 => quartic_original(a3, a2, a1, a0),
        _ => vec![],
    }
}
