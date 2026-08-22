//! Ported from `packages/engine/Source/Core/Packable.js`.

/// Static interface for types which can store their values as packed
/// elements in an array.
pub trait Packable {
    /// The number of elements used to pack the object into an array.
    fn packed_length() -> usize;

    /// Stores the provided instance into the provided array.
    fn pack(&self, array: &mut [f64], starting_index: usize) -> usize;

    /// Retrieves an instance from a packed array.
    fn unpack(array: &[f64], starting_index: usize) -> Self
    where
        Self: Sized;
}
