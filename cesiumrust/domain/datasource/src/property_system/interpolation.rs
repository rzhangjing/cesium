//! Interpolation algorithms for sampled properties.
//!
//! Maps to CesiumJS:
//! - `Core/LinearApproximation.js`
//! - `Core/LagrangePolynomialApproximation.js`
//! - `Core/HermitePolynomialApproximation.js`
//! - `DataSources/ExtrapolationType.js`
//!
//! All algorithms operate on packed `f64` tables, exactly like CesiumJS:
//! - `x_table`: independent variable values (times, in seconds), increasing order.
//! - `y_table`: dependent values; for `y_stride` components per sample the layout
//!   is `{p1, q1, w1, p2, q2, w2, ...}`.

use cesium_geospatial::math_utils::factorial;

/// Determines how an interpolated value is extrapolated when querying outside
/// the bounds of available data.
///
/// Maps to CesiumJS `DataSources/ExtrapolationType.js`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ExtrapolationType {
    /// No extrapolation occurs; values outside the sample range are undefined.
    #[default]
    None,
    /// The first or last value is used when outside the range of sample data.
    Hold,
    /// The value is extrapolated.
    Extrapolate,
}

impl ExtrapolationType {
    /// The numeric value used by CesiumJS (NONE=0, HOLD=1, EXTRAPOLATE=2).
    pub fn to_u32(self) -> u32 {
        match self {
            ExtrapolationType::None => 0,
            ExtrapolationType::Hold => 1,
            ExtrapolationType::Extrapolate => 2,
        }
    }

    /// Creates an extrapolation type from its CesiumJS numeric value.
    pub fn from_u32(value: u32) -> Self {
        match value {
            1 => ExtrapolationType::Hold,
            2 => ExtrapolationType::Extrapolate,
            _ => ExtrapolationType::None,
        }
    }
}

/// An algorithm for interpolating packed dependent-variable tables.
///
/// Maps to the CesiumJS `InterpolationAlgorithm` interface implemented by
/// `LinearApproximation`, `LagrangePolynomialApproximation` and
/// `HermitePolynomialApproximation`.
pub trait InterpolationAlgorithm: Send + Sync {
    /// The algorithm type name (`"Linear"`, `"Lagrange"`, `"Hermite"`).
    fn name(&self) -> &'static str;

    /// Given the desired degree, returns the number of data points required
    /// for interpolation.
    ///
    /// `input_order` is the order of the inputs (0 means just the data,
    /// 1 means the data and its derivative, etc).
    fn get_required_data_points(&self, degree: usize, input_order: usize) -> usize;

    /// Whether this algorithm implements `interpolate` (i.e. supports
    /// derivative inputs/outputs). Only Hermite does in CesiumJS.
    fn supports_derivatives(&self) -> bool {
        false
    }

    /// Interpolates values using the algorithm (order zero, no derivatives).
    ///
    /// Returns a `Vec` of `y_stride` interpolated component values.
    fn interpolate_order_zero(
        &self,
        x: f64,
        x_table: &[f64],
        y_table: &[f64],
        y_stride: usize,
    ) -> Vec<f64>;

    /// Interpolates values with derivative inputs and outputs.
    ///
    /// The default implementation (used by algorithms that do not define
    /// `interpolate` in CesiumJS) falls back to order-zero interpolation and
    /// zero-fills the derivative outputs.
    fn interpolate(
        &self,
        x: f64,
        x_table: &[f64],
        y_table: &[f64],
        y_stride: usize,
        _input_order: usize,
        output_order: usize,
    ) -> Vec<f64> {
        let mut result = self.interpolate_order_zero(x, x_table, y_table, y_stride);
        result.resize(y_stride * (output_order + 1), 0.0);
        result
    }
}

/// Linear interpolation.
///
/// Maps to CesiumJS `Core/LinearApproximation.js`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LinearApproximation;

impl InterpolationAlgorithm for LinearApproximation {
    fn name(&self) -> &'static str {
        "Linear"
    }

    /// Since linear interpolation can only generate a first degree polynomial,
    /// this always returns 2.
    fn get_required_data_points(&self, _degree: usize, _input_order: usize) -> usize {
        2
    }

    fn interpolate_order_zero(
        &self,
        x: f64,
        x_table: &[f64],
        y_table: &[f64],
        y_stride: usize,
    ) -> Vec<f64> {
        debug_assert_eq!(
            x_table.len(),
            2,
            "The xTable provided to the linear interpolator must have exactly two elements."
        );
        debug_assert!(
            y_stride > 0,
            "There must be at least 1 dependent variable for each independent variable."
        );

        let mut result = vec![0.0; y_stride];
        let x0 = x_table[0];
        let x1 = x_table[1];
        debug_assert_ne!(x0, x1, "Divide by zero error: xTable[0] and xTable[1] are equal");

        for i in 0..y_stride {
            let y0 = y_table[i];
            let y1 = y_table[i + y_stride];
            result[i] = ((y1 - y0) * x + x1 * y0 - x0 * y1) / (x1 - x0);
        }
        result
    }
}

/// Lagrange polynomial interpolation.
///
/// Maps to CesiumJS `Core/LagrangePolynomialApproximation.js`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LagrangePolynomialApproximation;

impl InterpolationAlgorithm for LagrangePolynomialApproximation {
    fn name(&self) -> &'static str {
        "Lagrange"
    }

    fn get_required_data_points(&self, degree: usize, _input_order: usize) -> usize {
        (degree + 1).max(2)
    }

    fn interpolate_order_zero(
        &self,
        x: f64,
        x_table: &[f64],
        y_table: &[f64],
        y_stride: usize,
    ) -> Vec<f64> {
        let mut result = vec![0.0; y_stride];
        let length = x_table.len();

        for i in 0..length {
            let mut coefficient = 1.0;
            for j in 0..length {
                if j != i {
                    let diff_x = x_table[i] - x_table[j];
                    coefficient *= (x - x_table[j]) / diff_x;
                }
            }
            for j in 0..y_stride {
                result[j] += coefficient * y_table[i * y_stride + j];
            }
        }
        result
    }
}

/// Hermite polynomial interpolation (divided differences with derivative
/// support).
///
/// Maps to CesiumJS `Core/HermitePolynomialApproximation.js`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HermitePolynomialApproximation;

impl InterpolationAlgorithm for HermitePolynomialApproximation {
    fn name(&self) -> &'static str {
        "Hermite"
    }

    fn get_required_data_points(&self, degree: usize, input_order: usize) -> usize {
        ((degree + 1) / (input_order + 1)).max(2)
    }

    fn supports_derivatives(&self) -> bool {
        true
    }

    fn interpolate_order_zero(
        &self,
        x: f64,
        x_table: &[f64],
        y_table: &[f64],
        y_stride: usize,
    ) -> Vec<f64> {
        let length = x_table.len();
        let mut result = vec![0.0; y_stride];
        if length == 0 || y_stride == 0 {
            return result;
        }

        // coefficients[s][i] holds the divided-difference table rows for
        // component s.
        let mut coefficients: Vec<Vec<Vec<f64>>> =
            (0..y_stride).map(|_| (0..length).map(|_| Vec::new()).collect()).collect();

        let z_indices: Vec<usize> = (0..length).collect();

        let mut highest_non_zero_coef = length - 1;
        for (s, coef_s) in coefficients.iter_mut().enumerate() {
            for &z in &z_indices {
                let index = z * y_stride + s;
                coef_s[0].push(y_table[index]);
            }

            for i in 1..length {
                let mut non_zero_coefficients = false;
                for j in 0..(length - i) {
                    let zj = x_table[z_indices[j]];
                    let zn = x_table[z_indices[j + i]];

                    let numerator;
                    if zn - zj <= 0.0 {
                        let index = z_indices[j] * y_stride + y_stride * i + s;
                        numerator = y_table[index];
                        coef_s[i].push(numerator / factorial(i as u32) as f64);
                    } else {
                        numerator = coef_s[i - 1][j + 1] - coef_s[i - 1][j];
                        coef_s[i].push(numerator / (zn - zj));
                    }
                    non_zero_coefficients = non_zero_coefficients || numerator != 0.0;
                }

                if !non_zero_coefficients {
                    highest_non_zero_coef = i - 1;
                }
            }
        }

        // In interpolateOrderZero the outer loop only ever runs with d = 0
        // (`for (d = 0, len = 0; d <= len; d++)` in CesiumJS).
        // The explicit index loop mirrors the CesiumJS divided-difference
        // accumulation and cannot be expressed as a plain iterator zip.
        #[allow(clippy::needless_range_loop)]
        for i in 0..=highest_non_zero_coef {
            let temp_term =
                calculate_coefficient_term(x, &z_indices, x_table, 0, i, &mut Vec::new());
            for (result_s, coef_s) in result.iter_mut().zip(coefficients.iter()) {
                *result_s += coef_s[i][0] * temp_term;
            }
        }

        result
    }

    fn interpolate(
        &self,
        x: f64,
        x_table: &[f64],
        y_table: &[f64],
        y_stride: usize,
        input_order: usize,
        output_order: usize,
    ) -> Vec<f64> {
        let result_length = y_stride * (output_order + 1);
        let mut result = vec![0.0; result_length];

        let length = x_table.len();
        // The zIndices array holds copies of the addresses of the xTable values
        // in the range we're looking at.
        let z_len = length * (input_order + 1);
        let mut z_indices = vec![0usize; z_len];
        for i in 0..length {
            for j in 0..(input_order + 1) {
                z_indices[i * (input_order + 1) + j] = i;
            }
        }

        let tmp = z_len * (z_len + 1) / 2;
        let mut coefficients = vec![0.0f64; y_stride * tmp];
        let highest_non_zero_coef = fill_coefficient_list(
            &mut coefficients,
            &z_indices,
            x_table,
            y_table,
            y_stride,
            input_order,
        );

        let loop_stop = highest_non_zero_coef.min(output_order as isize);
        if loop_stop < 0 {
            return result;
        }
        for d in 0..=loop_stop as usize {
            for i in d..=highest_non_zero_coef as usize {
                let temp_term = calculate_coefficient_term(
                    x,
                    &z_indices,
                    x_table,
                    d,
                    i,
                    &mut Vec::new(),
                );
                let dim_two = row_offset(i, z_len);
                for s in 0..y_stride {
                    let dim_one = s * tmp;
                    let coef = coefficients[dim_one + dim_two];
                    result[s + d * y_stride] += coef * temp_term;
                }
            }
        }

        result
    }
}

/// Offset of divided-difference row `i` inside the packed coefficient buffer.
///
/// Row `i` holds `z_len - i` entries; CesiumJS computes this as
/// `Math.floor((i * (1 - i)) / 2) + zIndicesLength * i`.
fn row_offset(i: usize, z_len: usize) -> usize {
    let signed = (i as isize) * (1 - i as isize) / 2 + (z_len * i) as isize;
    signed as usize
}

fn fill_coefficient_list(
    coefficients: &mut [f64],
    z_indices: &[usize],
    x_table: &[f64],
    y_table: &[f64],
    y_stride: usize,
    input_order: usize,
) -> isize {
    let mut highest_non_zero: isize = -1;
    let z_len = z_indices.len();
    let tmp = z_len * (z_len + 1) / 2;

    for s in 0..y_stride {
        let dim_one = s * tmp;

        for j in 0..z_len {
            let index = z_indices[j] * y_stride * (input_order + 1) + s;
            coefficients[dim_one + j] = y_table[index];
        }

        for i in 1..z_len {
            let mut coef_index = 0usize;
            let dim_two = row_offset(i, z_len);
            let mut non_zero_coefficients = false;

            for j in 0..(z_len - i) {
                let zj = x_table[z_indices[j]];
                let zn = x_table[z_indices[j + i]];

                let numerator;
                if zn - zj <= 0.0 {
                    let index = z_indices[j] * y_stride * (input_order + 1) + y_stride * i + s;
                    numerator = y_table[index];
                    coefficients[dim_one + dim_two + coef_index] =
                        numerator / factorial(i as u32) as f64;
                    coef_index += 1;
                } else {
                    let dim_two_minus_one = row_offset(i - 1, z_len);
                    numerator = coefficients[dim_one + dim_two_minus_one + j + 1]
                        - coefficients[dim_one + dim_two_minus_one + j];
                    coefficients[dim_one + dim_two + coef_index] = numerator / (zn - zj);
                    coef_index += 1;
                }
                non_zero_coefficients = non_zero_coefficients || numerator != 0.0;
            }

            if non_zero_coefficients {
                highest_non_zero = highest_non_zero.max(i as isize);
            }
        }
    }

    highest_non_zero
}

/// Computes one term of the Newton-form coefficient polynomial, or one of its
/// derivatives.
///
/// With `deriv_order == 0` this is the product of `(x - xTable[zIndices[i]])`
/// over all non-reserved `i < term_order`. With `deriv_order > 0` it is the
/// `deriv_order`-th derivative of that product, computed recursively as the sum
/// over all ways of reserving one factor at a time.
fn calculate_coefficient_term(
    x: f64,
    z_indices: &[usize],
    x_table: &[f64],
    deriv_order: usize,
    term_order: usize,
    reserved_indices: &mut Vec<usize>,
) -> f64 {
    if deriv_order > 0 {
        let mut result = 0.0;
        for i in 0..term_order {
            if !reserved_indices.contains(&i) {
                reserved_indices.push(i);
                result += calculate_coefficient_term(
                    x,
                    z_indices,
                    x_table,
                    deriv_order - 1,
                    term_order,
                    reserved_indices,
                );
                reserved_indices.pop();
            }
        }
        return result;
    }

    let mut result = 1.0;
    for i in 0..term_order {
        if !reserved_indices.contains(&i) {
            result *= x - x_table[z_indices[i]];
        }
    }
    result
}

/// The built-in interpolation algorithms, selectable by value.
///
/// This enum dispatches to [`LinearApproximation`],
/// [`LagrangePolynomialApproximation`] and [`HermitePolynomialApproximation`]
/// and is used by `SampledProperty` to store the selected algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InterpolationAlgorithmKind {
    #[default]
    Linear,
    Lagrange,
    Hermite,
}

impl InterpolationAlgorithmKind {
    /// Returns the algorithm object for this kind.
    pub fn algorithm(&self) -> &'static dyn InterpolationAlgorithm {
        match self {
            InterpolationAlgorithmKind::Linear => &LinearApproximation,
            InterpolationAlgorithmKind::Lagrange => &LagrangePolynomialApproximation,
            InterpolationAlgorithmKind::Hermite => &HermitePolynomialApproximation,
        }
    }

    /// Parses a CesiumJS algorithm type name (`"Linear"`, `"Lagrange"`,
    /// `"Hermite"`).
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Linear" => Some(InterpolationAlgorithmKind::Linear),
            "Lagrange" => Some(InterpolationAlgorithmKind::Lagrange),
            "Hermite" => Some(InterpolationAlgorithmKind::Hermite),
            _ => None,
        }
    }
}

impl InterpolationAlgorithm for InterpolationAlgorithmKind {
    fn name(&self) -> &'static str {
        self.algorithm().name()
    }

    fn get_required_data_points(&self, degree: usize, input_order: usize) -> usize {
        self.algorithm().get_required_data_points(degree, input_order)
    }

    fn supports_derivatives(&self) -> bool {
        self.algorithm().supports_derivatives()
    }

    fn interpolate_order_zero(
        &self,
        x: f64,
        x_table: &[f64],
        y_table: &[f64],
        y_stride: usize,
    ) -> Vec<f64> {
        self.algorithm()
            .interpolate_order_zero(x, x_table, y_table, y_stride)
    }

    fn interpolate(
        &self,
        x: f64,
        x_table: &[f64],
        y_table: &[f64],
        y_stride: usize,
        input_order: usize,
        output_order: usize,
    ) -> Vec<f64> {
        self.algorithm()
            .interpolate(x, x_table, y_table, y_stride, input_order, output_order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-12;

    #[test]
    fn test_extrapolation_type_roundtrip() {
        assert_eq!(ExtrapolationType::None.to_u32(), 0);
        assert_eq!(ExtrapolationType::Hold.to_u32(), 1);
        assert_eq!(ExtrapolationType::Extrapolate.to_u32(), 2);
        assert_eq!(ExtrapolationType::from_u32(0), ExtrapolationType::None);
        assert_eq!(ExtrapolationType::from_u32(1), ExtrapolationType::Hold);
        assert_eq!(ExtrapolationType::from_u32(2), ExtrapolationType::Extrapolate);
        assert_eq!(ExtrapolationType::default(), ExtrapolationType::None);
    }

    #[test]
    fn test_linear_required_data_points() {
        assert_eq!(LinearApproximation.get_required_data_points(1, 0), 2);
        assert_eq!(LinearApproximation.get_required_data_points(5, 0), 2);
        assert_eq!(LinearApproximation.get_required_data_points(9, 2), 2);
    }

    #[test]
    fn test_linear_interpolate_midpoint() {
        // y = 2x + 1 sampled at x = 0 and x = 10 (two components per sample).
        let x_table = [0.0, 10.0];
        let y_table = [1.0, -1.0, 21.0, 19.0];
        let result = LinearApproximation.interpolate_order_zero(5.0, &x_table, &y_table, 2);
        assert!((result[0] - 11.0).abs() < EPS);
        assert!((result[1] - 9.0).abs() < EPS);
    }

    #[test]
    fn test_linear_interpolate_at_nodes() {
        let x_table = [-4.0, 2.0];
        let y_table = [3.0, 7.0];
        let r0 = LinearApproximation.interpolate_order_zero(-4.0, &x_table, &y_table, 1);
        let r1 = LinearApproximation.interpolate_order_zero(2.0, &x_table, &y_table, 1);
        assert!((r0[0] - 3.0).abs() < EPS);
        assert!((r1[0] - 7.0).abs() < EPS);
    }

    #[test]
    fn test_linear_negative_x_extrapolates() {
        // xTable values are relative (seconds from last sample) and may be
        // negative; the formula must still hold.
        let x_table = [-10.0, 0.0];
        let y_table = [0.0, 100.0];
        let result = LinearApproximation.interpolate_order_zero(-5.0, &x_table, &y_table, 1);
        assert!((result[0] - 50.0).abs() < EPS);
    }

    #[test]
    fn test_lagrange_required_data_points() {
        assert_eq!(LagrangePolynomialApproximation.get_required_data_points(0, 0), 2);
        assert_eq!(LagrangePolynomialApproximation.get_required_data_points(1, 0), 2);
        assert_eq!(LagrangePolynomialApproximation.get_required_data_points(2, 0), 3);
        assert_eq!(LagrangePolynomialApproximation.get_required_data_points(7, 0), 8);
    }

    #[test]
    fn test_lagrange_quadratic_exact() {
        // y = x^2 - 2x + 3 sampled at x = -1, 0, 2.
        let x_table = [-1.0, 0.0, 2.0];
        let y_table = [6.0, 3.0, 3.0];
        for &x in &[0.5, 1.0, -0.5, 1.7] {
            let expected = x * x - 2.0 * x + 3.0;
            let result =
                LagrangePolynomialApproximation.interpolate_order_zero(x, &x_table, &y_table, 1);
            assert!(
                (result[0] - expected).abs() < EPS,
                "x={x}: got {}, expected {expected}",
                result[0]
            );
        }
    }

    #[test]
    fn test_lagrange_multi_component() {
        // Two components: p = x, q = x^3 at x = 0, 1, 2, 3.
        let x_table = [0.0, 1.0, 2.0, 3.0];
        let y_table = [0.0, 0.0, 1.0, 1.0, 2.0, 8.0, 3.0, 27.0];
        let result =
            LagrangePolynomialApproximation.interpolate_order_zero(1.5, &x_table, &y_table, 2);
        assert!((result[0] - 1.5).abs() < EPS);
        assert!((result[1] - 3.375).abs() < EPS);
    }

    #[test]
    fn test_hermite_required_data_points() {
        assert_eq!(HermitePolynomialApproximation.get_required_data_points(1, 0), 2);
        assert_eq!(HermitePolynomialApproximation.get_required_data_points(3, 0), 4);
        // With derivatives (inputOrder=1): (degree+1)/2 points.
        assert_eq!(HermitePolynomialApproximation.get_required_data_points(3, 1), 2);
        assert_eq!(HermitePolynomialApproximation.get_required_data_points(5, 1), 3);
        assert_eq!(HermitePolynomialApproximation.get_required_data_points(0, 0), 2);
    }

    #[test]
    fn test_hermite_order_zero_matches_lagrange() {
        // With distinct points and no derivatives, Hermite order zero is
        // Newton-form polynomial interpolation: exact for cubics with 4 points.
        let x_table = [-3.0, -1.0, 1.0, 2.0];
        let f = |x: f64| 2.0 * x * x * x - x * x + 4.0 * x - 7.0;
        let y_table: Vec<f64> = x_table.iter().map(|&x| f(x)).collect();
        for &x in &[-2.0, 0.0, 0.5, 1.5] {
            let result =
                HermitePolynomialApproximation.interpolate_order_zero(x, &x_table, &y_table, 1);
            assert!(
                (result[0] - f(x)).abs() < 1e-9,
                "x={x}: got {}, expected {}",
                result[0],
                f(x)
            );
        }
    }

    #[test]
    fn test_hermite_order_zero_constant_data() {
        // All-equal samples: highestNonZeroCoef collapses to 0.
        let x_table = [0.0, 1.0, 2.0];
        let y_table = [5.0, 5.0, 5.0];
        let result =
            HermitePolynomialApproximation.interpolate_order_zero(0.7, &x_table, &y_table, 1);
        assert!((result[0] - 5.0).abs() < EPS);
    }

    #[test]
    fn test_hermite_with_derivatives_cubic() {
        // Classic cubic Hermite: f(t) = t^3 on [0, 1].
        // f(0)=0, f'(0)=0, f(1)=1, f'(1)=3.
        // yTable layout per point: [value, derivative].
        let x_table = [0.0, 1.0];
        let y_table = [0.0, 0.0, 1.0, 3.0];
        let result =
            HermitePolynomialApproximation.interpolate(0.5, &x_table, &y_table, 1, 1, 1);
        // result[0] = f(0.5) = 0.125, result[1] = f'(0.5) = 0.75.
        assert!(
            (result[0] - 0.125).abs() < 1e-9,
            "value: got {}",
            result[0]
        );
        assert!(
            (result[1] - 0.75).abs() < 1e-9,
            "derivative: got {}",
            result[1]
        );
    }

    #[test]
    fn test_hermite_with_derivatives_at_nodes() {
        let x_table = [-1.0, 2.0];
        // f(x) = x^2: f(-1)=1, f'(-1)=-2, f(2)=4, f'(2)=4.
        let y_table = [1.0, -2.0, 4.0, 4.0];
        for &x in &[-1.0, 0.0, 2.0] {
            let result =
                HermitePolynomialApproximation.interpolate(x, &x_table, &y_table, 1, 1, 1);
            assert!((result[0] - x * x).abs() < 1e-9, "x={x}: got {}", result[0]);
            assert!((result[1] - 2.0 * x).abs() < 1e-9, "x={x}: got {}", result[1]);
        }
    }

    #[test]
    fn test_hermite_three_points_with_derivatives() {
        // f(x) = x^5 - x sampled at -1, 0, 1 with derivatives.
        let x_table = [-1.0, 0.0, 1.0];
        let f = |x: f64| x.powi(5) - x;
        let df = |x: f64| 5.0 * x.powi(4) - 1.0;
        let mut y_table = Vec::new();
        for &x in &x_table {
            y_table.push(f(x));
            y_table.push(df(x));
        }
        // 3 points * 2 values = 6 z-indices; degree up to 5 → exact for x^5.
        let result =
            HermitePolynomialApproximation.interpolate(0.5, &x_table, &y_table, 1, 1, 1);
        assert!((result[0] - f(0.5)).abs() < 1e-9, "got {}", result[0]);
        assert!((result[1] - df(0.5)).abs() < 1e-9, "got {}", result[1]);
    }

    #[test]
    fn test_kind_dispatch_and_names() {
        assert_eq!(InterpolationAlgorithmKind::Linear.name(), "Linear");
        assert_eq!(InterpolationAlgorithmKind::Lagrange.name(), "Lagrange");
        assert_eq!(InterpolationAlgorithmKind::Hermite.name(), "Hermite");
        assert!(!InterpolationAlgorithmKind::Linear.supports_derivatives());
        assert!(!InterpolationAlgorithmKind::Lagrange.supports_derivatives());
        assert!(InterpolationAlgorithmKind::Hermite.supports_derivatives());
        assert_eq!(
            InterpolationAlgorithmKind::from_name("Hermite"),
            Some(InterpolationAlgorithmKind::Hermite)
        );
        assert_eq!(InterpolationAlgorithmKind::from_name("Bogus"), None);
        assert_eq!(
            InterpolationAlgorithmKind::default(),
            InterpolationAlgorithmKind::Linear
        );
    }

    #[test]
    fn test_kind_interpolate_fallback_zero_fills_derivatives() {
        // Non-Hermite algorithms fall back to order-zero and zero-fill.
        let x_table = [0.0, 10.0];
        let y_table = [1.0, 21.0];
        let result = InterpolationAlgorithmKind::Linear.interpolate(5.0, &x_table, &y_table, 1, 0, 1);
        assert!((result[0] - 11.0).abs() < EPS);
        assert_eq!(result[1], 0.0);
    }
}
