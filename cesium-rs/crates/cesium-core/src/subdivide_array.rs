//! Ported from `packages/engine/Source/Core/subdivideArray.js`.

/// Splits an array into a specified number of roughly equal sub-arrays.
pub fn subdivide_array<T: Clone>(array: &[T], num_sub_arrays: usize) -> Vec<Vec<T>> {
    if num_sub_arrays == 0 {
        return vec![];
    }

    let mut result: Vec<Vec<T>> = Vec::with_capacity(num_sub_arrays);
    for _ in 0..num_sub_arrays {
        result.push(Vec::new());
    }

    if array.is_empty() {
        return result;
    }

    let items_per_array = (array.len() as f64 / num_sub_arrays as f64).ceil() as usize;

    let mut index = 0;
    for sub_array in &mut result {
        let end = (index + items_per_array).min(array.len());
        for item in &array[index..end] {
            sub_array.push(item.clone());
        }
        index = end;
        if index >= array.len() {
            break;
        }
    }

    result
}
