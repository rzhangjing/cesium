//! Array utilities - faithful port of CesiumJS `Core/arrayRemoveDuplicates.js`.

use crate::math_utils::EPSILON10;

/// Removes adjacent duplicate values in an array of values.
///
/// Maps to CesiumJS `arrayRemoveDuplicates`.
///
/// # Arguments
/// * `values` - The array of values.
/// * `equals_epsilon` - Function to compare values with an epsilon: `fn(&T, &T, f64) -> bool`.
/// * `wrap_around` - Compare the last value against the first. If equal, the last is removed.
///
/// # Returns
/// A tuple of `(cleaned_values, removed_indices)`.
/// - If no duplicates found, returns the original values unchanged and empty removed_indices.
/// - If duplicates found, returns a new Vec with duplicates removed and their original indices.
pub fn array_remove_duplicates<T: Clone>(
    values: &[T],
    equals_epsilon: fn(&T, &T, f64) -> bool,
    wrap_around: bool,
) -> (Vec<T>, Vec<usize>) {
    let mut removed_indices: Vec<usize> = Vec::new();

    let length = values.len();
    if length < 2 {
        return (values.to_vec(), removed_indices);
    }

    let mut cleaned_values: Option<Vec<T>> = None;
    let mut last_clean_index: usize = 0;
    let mut removed_index_lci: usize = 0;

    let mut v0_idx = 0usize;

    for i in 1..length {
        if equals_epsilon(&values[v0_idx], &values[i], EPSILON10) {
            if cleaned_values.is_none() {
                cleaned_values = Some(values[0..i].to_vec());
                last_clean_index = i - 1;
                removed_index_lci = 0;
            }
            removed_indices.push(i);
        } else {
            if let Some(ref mut cv) = cleaned_values {
                cv.push(values[i].clone());
                last_clean_index = i;
                removed_index_lci = removed_indices.len();
            }
            v0_idx = i;
        }
    }

    if wrap_around && equals_epsilon(&values[0], &values[length - 1], EPSILON10) {
        if let Some(ref mut cv) = cleaned_values {
            // Insert lastCleanIndex into removedIndices at the proper sorted position
            removed_indices.insert(removed_index_lci, last_clean_index);
            cv.truncate(cv.len() - 1);
        } else {
            removed_indices.push(length - 1);
            cleaned_values = Some(values[0..length - 1].to_vec());
        }
    }

    match cleaned_values {
        Some(cv) => (cv, removed_indices),
        None => (values.to_vec(), removed_indices),
    }
}

/// Convenience: returns true if no duplicates were removed (original array unchanged).
pub fn array_remove_duplicates_in_place<T: Clone>(
    values: &[T],
    equals_epsilon: fn(&T, &T, f64) -> bool,
    wrap_around: bool,
    removed_indices: &mut Vec<usize>,
) -> Vec<T> {
    let (result, removed) = array_remove_duplicates(values, equals_epsilon, wrap_around);
    *removed_indices = removed;
    result
}
