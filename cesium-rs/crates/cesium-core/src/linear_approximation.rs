//! Ported from `packages/engine/Source/Core/LinearApproximation.js`.
//!
//! An interpolation algorithm for performing linear interpolation.

/// The type name for this interpolation algorithm.
pub const INTERPOLATION_TYPE: &str = "Linear";

/// Returns the number of data points required for linear interpolation (always 2).
pub fn get_required_data_points(_degree: u32) -> u32 {
    2
}

/// Interpolates values using linear approximation (order zero).
///
/// - `x`: the independent variable
/// - `x_table`: exactly 2 independent variable values
/// - `y_table`: dependent variable values
/// - `y_stride`: number of dependent values per independent value
/// - `result`: optional output buffer
pub fn interpolate_order_zero(
    x: f64,
    x_table: &[f64],
    y_table: &[f64],
    y_stride: usize,
    result: &mut [f64],
) {
    assert_eq!(x_table.len(), 2, "xTable must have exactly two elements");
    assert!(y_stride > 0, "yStride must be at least 1");

    let x0 = x_table[0];
    let x1 = x_table[1];
    assert!(x0 != x1, "xTable[0] and xTable[1] must not be equal");

    for i in 0..y_stride {
        let y0 = y_table[i];
        let y1 = y_table[i + y_stride];
        result[i] = ((y1 - y0) * x + x1 * y0 - x0 * y1) / (x1 - x0);
    }
}
