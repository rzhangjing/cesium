//! Ported from `packages/engine/Source/Core/HermitePolynomialApproximation.js`.

use crate::math::CesiumMath;

/// An interpolation algorithm for performing Hermite interpolation.
pub struct HermitePolynomialApproximation;

impl HermitePolynomialApproximation {
    /// The type identifier string.
    pub const TYPE: &'static str = "Hermite";

    /// Given the desired degree, returns the number of data points required.
    pub fn get_required_data_points(degree: f64, input_order: Option<f64>) -> f64 {
        let input_order = input_order.unwrap_or(0.0);
        ((degree + 1.0) / (input_order + 1.0)).floor().max(2.0)
    }

    /// Interpolates values using Hermite Polynomial Approximation (order zero).
    pub fn interpolate_order_zero(
        x: f64,
        x_table: &[f64],
        y_table: &[f64],
        y_stride: usize,
        result: Option<&mut Vec<f64>>,
    ) -> Vec<f64> {
        let mut r = result.cloned().unwrap_or_else(|| vec![0.0; y_stride]);
        let length = x_table.len();

        // coefficients[s][i] is a Vec<f64>
        let mut coefficients: Vec<Vec<Vec<f64>>> = Vec::with_capacity(y_stride);
        for _s in 0..y_stride {
            r[_s] = 0.0;
            let mut l = Vec::with_capacity(length);
            for _j in 0..length {
                l.push(Vec::new());
            }
            coefficients.push(l);
        }

        let z_indices: Vec<usize> = (0..length).collect();
        let z_indices_length = z_indices.len();

        let mut highest_non_zero_coef = length as isize - 1;

        for s in 0..y_stride {
            for j in 0..z_indices_length {
                let index = z_indices[j] * y_stride + s;
                coefficients[s][0].push(y_table[index]);
            }

            for i in 1..z_indices_length {
                let mut non_zero_coefficients = false;
                for j in 0..z_indices_length - i {
                    let zj = x_table[z_indices[j]];
                    let zn = x_table[z_indices[j + i]];

                    let numerator;
                    if zn - zj <= 0.0 {
                        let index = z_indices[j] * y_stride + y_stride * i + s;
                        numerator = y_table[index];
                        coefficients[s][i].push(numerator / CesiumMath::factorial(i as f64));
                    } else {
                        numerator = coefficients[s][i - 1][j + 1] - coefficients[s][i - 1][j];
                        coefficients[s][i].push(numerator / (zn - zj));
                    }
                    non_zero_coefficients = non_zero_coefficients || numerator != 0.0;
                }

                if !non_zero_coefficients {
                    highest_non_zero_coef = (i as isize) - 1;
                }
            }
        }

        let mut d: usize = 0;
        loop {
            if d as isize > highest_non_zero_coef {
                break;
            }
            for i in d..=(highest_non_zero_coef as usize) {
                let temp_term = calculate_coefficient_term(x, &z_indices, x_table, d, i, &mut vec![]);
                for s in 0..y_stride {
                    let coeff = coefficients[s][i][0];
                    if s + d * y_stride < r.len() {
                        r[s + d * y_stride] += coeff * temp_term;
                    }
                }
            }
            d += 1;
        }

        r
    }

    /// Interpolates values using Hermite Polynomial Approximation with input/output order.
    pub fn interpolate(
        x: f64,
        x_table: &[f64],
        y_table: &[f64],
        y_stride: usize,
        input_order: usize,
        output_order: usize,
        result: Option<&mut Vec<f64>>,
    ) -> Vec<f64> {
        let result_length = y_stride * (output_order + 1);
        let mut r = result.cloned().unwrap_or_else(|| vec![0.0; result_length]);
        for item in r.iter_mut().take(result_length) {
            *item = 0.0;
        }

        let length = x_table.len();
        let z_indices_length = length * (input_order + 1);
        let mut z_indices = vec![0usize; z_indices_length];
        for i in 0..length {
            for j in 0..=input_order {
                z_indices[i * (input_order + 1) + j] = i;
            }
        }

        let mut coefficients: Vec<f64> = Vec::new();
        let highest_non_zero_coef = fill_coefficient_list(
            &mut coefficients,
            &z_indices,
            x_table,
            y_table,
            y_stride,
            input_order,
        );

        let tmp = (z_indices_length * (z_indices_length + 1)) / 2;
        let loop_stop = highest_non_zero_coef.min(output_order as isize);

        for d in 0..=loop_stop as usize {
            for i in d..=(highest_non_zero_coef as usize) {
                let temp_term =
                    calculate_coefficient_term(x, &z_indices, x_table, d, i, &mut vec![]);
                // JS: dimTwo = ((i * (1 - i)) / 2) + zIndicesLength * i;
                // the intermediate i * (1 - i) is non-positive in JS
                // numbers, so compute it signed before converting back.
                let i_signed = i as isize;
                let dim_two = (z_indices_length as isize * i_signed
                    + (i_signed * (1 - i_signed)) / 2)
                    as usize;

                for s in 0..y_stride {
                    let dim_one = s * tmp;
                    let idx = dim_one + dim_two;
                    if idx < coefficients.len() {
                        let coef = coefficients[idx];
                        if s + d * y_stride < r.len() {
                            r[s + d * y_stride] += coef * temp_term;
                        }
                    }
                }
            }
        }

        r
    }
}

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
            let reserved = reserved_indices.contains(&i);
            if !reserved {
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
        let reserved = reserved_indices.contains(&i);
        if !reserved {
            result *= x - x_table[z_indices[i]];
        }
    }
    result
}

fn fill_coefficient_list(
    coefficients: &mut Vec<f64>,
    z_indices: &[usize],
    x_table: &[f64],
    y_table: &[f64],
    y_stride: usize,
    input_order: usize,
) -> isize {
    let z_indices_length = z_indices.len();
    let tmp = (z_indices_length * (z_indices_length + 1)) / 2;
    let needed = y_stride * tmp;
    coefficients.resize(needed, 0.0);

    let mut highest_non_zero: isize = -1;

    for s in 0..y_stride {
        let dim_one = s * tmp;

        for j in 0..z_indices_length {
            let index = z_indices[j] * y_stride * (input_order + 1) + s;
            coefficients[dim_one + j] = y_table[index];
        }

        for i in 1..z_indices_length {
            // JS: coefIndexBase = ((i * (1 - i)) / 2) + zIndicesLength * i;
            let i_signed = i as isize;
            let coef_index_base = (z_indices_length as isize * i_signed
                + (i_signed * (1 - i_signed)) / 2)
                as usize;
            let mut non_zero_coefficients = false;
            let mut coef_index = 0;

            for j in 0..z_indices_length - i {
                let zj = x_table[z_indices[j]];
                let zn = x_table[z_indices[j + i]];

                let numerator;
                if zn - zj <= 0.0 {
                    let index = z_indices[j] * y_stride * (input_order + 1) + y_stride * i + s;
                    numerator = y_table[index];
                    let coefficient = numerator / CesiumMath::factorial(i as f64);
                    coefficients[dim_one + coef_index_base + coef_index] = coefficient;
                    coef_index += 1;
                } else {
                    // JS: dimTwoMinusOne = (((i - 1) * (2 - i)) / 2) +
                    //     zIndicesLength * (i - 1);
                    let im1 = (i - 1) as isize;
                    let dim_two_minus_one = (z_indices_length as isize * im1
                        + (im1 * (2 - i as isize)) / 2)
                        as usize;
                    numerator = coefficients[dim_one + dim_two_minus_one + j + 1]
                        - coefficients[dim_one + dim_two_minus_one + j];
                    let coefficient = numerator / (zn - zj);
                    coefficients[dim_one + coef_index_base + coef_index] = coefficient;
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
