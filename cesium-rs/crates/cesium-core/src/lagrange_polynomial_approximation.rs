//! Ported from `packages/engine/Source/Core/LagrangePolynomialApproximation.js`.

/// An interpolation algorithm for performing Lagrange interpolation.
pub struct LagrangePolynomialApproximation;

impl LagrangePolynomialApproximation {
    /// The type identifier string.
    pub const TYPE: &'static str = "Lagrange";

    /// Given the desired degree, returns the number of data points required.
    pub fn get_required_data_points(degree: f64) -> f64 {
        (degree + 1.0).max(2.0)
    }

    /// Interpolates values using Lagrange Polynomial Approximation.
    pub fn interpolate_order_zero(
        x: f64,
        x_table: &[f64],
        y_table: &[f64],
        y_stride: usize,
        result: Option<&mut Vec<f64>>,
    ) -> Vec<f64> {
        let mut r = result.cloned().unwrap_or_else(|| vec![0.0; y_stride]);
        let length = x_table.len();

        for item in r.iter_mut().take(y_stride) {
            *item = 0.0;
        }

        for i in 0..length {
            let mut coefficient = 1.0;
            for j in 0..length {
                if j != i {
                    let diff_x = x_table[i] - x_table[j];
                    coefficient *= (x - x_table[j]) / diff_x;
                }
            }
            for j in 0..y_stride {
                r[j] += coefficient * y_table[i * y_stride + j];
            }
        }

        r
    }
}
