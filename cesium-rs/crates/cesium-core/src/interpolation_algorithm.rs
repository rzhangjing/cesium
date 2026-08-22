//! Ported from `packages/engine/Source/Core/InterpolationAlgorithm.js`.

/// The interface for interpolation algorithms.
pub trait InterpolationAlgorithm {
    /// Gets the name of this interpolation algorithm.
    fn algorithm_type() -> &'static str;

    /// Given the desired degree, returns the number of data points required.
    fn get_required_data_points(degree: usize) -> usize;

    /// Performs zero order interpolation.
    fn interpolate_order_zero(
        x: f64,
        x_table: &[f64],
        y_table: &[f64],
        y_stride: usize,
        result: &mut Vec<f64>,
    );

    /// Performs higher order interpolation.
    fn interpolate(
        x: f64,
        x_table: &[f64],
        y_table: &[f64],
        y_stride: usize,
        input_order: usize,
        output_order: usize,
        result: &mut Vec<f64>,
    );
}
