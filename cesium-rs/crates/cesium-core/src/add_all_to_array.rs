//! Ported from packages/engine/Source/Core/addAllToArray.js

/// Adds all elements from the given source array to the given target array.
///
/// If the `source` is `None` or empty, then nothing will be done. Otherwise,
/// this has the same semantics as `for (const s of source) target.push(s);`.
///
/// Port of CesiumJS `addAllToArray(target, source)`.
///
/// # Example
/// ```
/// # use cesium_core::add_all_to_array::add_all_to_array;
/// let mut target = vec![0, 1, 2];
/// let source = vec![3, 4, 5];
/// add_all_to_array(&mut target, Some(&source));
/// assert_eq!(target, vec![0, 1, 2, 3, 4, 5]);
/// ```
pub fn add_all_to_array<T: Clone>(target: &mut Vec<T>, source: Option<&[T]>) {
    let Some(source) = source else {
        return;
    };
    let source_length = source.len();
    if source_length == 0 {
        return;
    }
    target.reserve(source_length);
    target.extend(source.iter().cloned());
}
