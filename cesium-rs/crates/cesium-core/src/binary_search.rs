//! Ported from packages/engine/Source/Core/binarySearch.js

use crate::check::defined;

/// Finds an item in a sorted array.
///
/// Port of CesiumJS `binarySearch(array, itemToFind, comparator)`. Returns
/// the index of `item_to_find` in the array, if it exists. If `item_to_find`
/// does not exist, the return value is a negative number which is the
/// bitwise complement (`!`) of the index before which the `item_to_find`
/// should be inserted in order to maintain the sorted order of the array.
///
/// The comparator returns a value comparable as JS numbers: negative if `a`
/// is less than `b`, positive if `a` is greater than `b`, or 0 if equal.
///
/// # Example
/// ```
/// # use cesium_core::binary_search::binary_search;
/// let numbers = [0.0, 2.0, 4.0, 6.0, 8.0];
/// let index = binary_search(&numbers, &6.0, |a: &f64, b: &f64| a - b); // 3
/// assert_eq!(index, 3);
/// ```
pub fn binary_search<T, U, F>(array: &[T], item_to_find: &U, mut comparator: F) -> i64
where
    F: FnMut(&T, &U) -> f64,
{
    // >>includeStart('debug', pragmas.debug)
    if cfg!(debug_assertions) {
        defined("array", Some(&array));
        defined("itemToFind", Some(&item_to_find));
        // "comparator" is guaranteed by the type system.
    }
    // >>includeEnd('debug')

    let mut low: i64 = 0;
    let mut high: i64 = array.len() as i64 - 1;

    while low <= high {
        let i = ((low + high) / 2) as usize; // ~~((low + high) / 2)
        let comparison = comparator(&array[i], item_to_find);
        if comparison < 0.0 {
            low = i as i64 + 1;
            continue;
        }
        if comparison > 0.0 {
            high = i as i64 - 1;
            continue;
        }
        return i as i64;
    }
    !(high + 1)
}
