//! Ported from `packages/engine/Source/Core/arrayRemoveDuplicates.js`.

/// Removes adjacent duplicate values in an array of values.
///
/// Returns a new Vec with no adjacent duplicates, or the original if none found.
/// If `wrap_around` is true, also compares the last value against the first.
pub fn array_remove_duplicates<T: Clone>(
    values: &[T],
    equals_epsilon: impl Fn(&T, &T, f64) -> bool,
    wrap_around: bool,
    mut removed_indices: Option<&mut Vec<usize>>,
) -> Option<Vec<T>> {
    let length = values.len();
    if length < 2 {
        return None;
    }

    let epsilon = 1e-10; // EPSILON10
    let store_removed = removed_indices.is_some();

    let mut i: usize;
    let mut v0_idx = 0;
    let mut cleaned: Option<Vec<T>> = None;
    let mut last_clean_index: usize = 0;
    let mut removed_index_lci: isize = -1;

    for idx in 1..length {
        i = idx;
        if equals_epsilon(&values[v0_idx], &values[i], epsilon) {
            if cleaned.is_none() {
                let c = values[..i].to_vec();
                last_clean_index = i - 1;
                removed_index_lci = 0;
                cleaned = Some(c);
            }
            if store_removed {
                if let Some(ref mut ri) = removed_indices {
                    ri.push(i);
                }
            }
        } else {
            if let Some(ref mut c) = cleaned {
                c.push(values[i].clone());
                last_clean_index = i;
                if store_removed {
                    if let Some(ref ri) = removed_indices {
                        removed_index_lci = ri.len() as isize;
                    }
                }
            }
            v0_idx = i;
        }
    }

    if wrap_around && equals_epsilon(&values[0], &values[length - 1], epsilon) {
        if store_removed {
            if let Some(ref mut ri) = removed_indices {
                if cleaned.is_some() {
                    ri.insert(removed_index_lci as usize, last_clean_index);
                } else {
                    ri.push(length - 1);
                }
            }
        }
        if let Some(ref mut c) = cleaned {
            c.pop();
        } else {
            cleaned = Some(values[..length - 1].to_vec());
        }
    }

    cleaned
}
