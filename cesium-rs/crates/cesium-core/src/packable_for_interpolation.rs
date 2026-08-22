//! Ported from `packages/engine/Source/Core/PackableForInterpolation.js`.

/// Static interface for `Packable` types which are interpolated in a
/// different representation than their packed value.
pub trait PackableForInterpolation {
    /// The number of elements used to store the object into an array
    /// in its interpolatable form.
    fn packed_interpolation_length() -> usize;

    /// Converts a packed array into a form suitable for interpolation.
    fn convert_packed_array_for_interpolation(
        packed_array: &[f64],
        starting_index: usize,
        last_index: usize,
        result: &mut [f64],
    );

    /// Retrieves an instance from a packed array converted for interpolation.
    fn unpack_interpolation_result(
        array: &[f64],
        source_array: &[f64],
        starting_index: usize,
        last_index: usize,
    ) -> Self
    where
        Self: Sized;
}
