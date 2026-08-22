//! Ported from `packages/engine/Source/Core/mergeSort.js`.

/// Performs a merge sort on the provided array using the given comparator.
pub fn merge_sort<T: Clone>(array: &mut [T], comparator: impl Fn(&T, &T) -> i32) {
    let len = array.len();
    if len < 2 {
        return;
    }
    let mut scratch = array.to_vec();
    merge_sort_impl(array, &mut scratch, 0, len as isize - 1, &comparator);
}

fn merge_sort_impl<T: Clone>(
    array: &mut [T],
    scratch: &mut [T],
    left: isize,
    right: isize,
    comparator: &dyn Fn(&T, &T) -> i32,
) {
    if left >= right {
        return;
    }
    let mid = (left + right) / 2;
    merge_sort_impl(array, scratch, left, mid, comparator);
    merge_sort_impl(array, scratch, mid + 1, right, comparator);

    // merge
    let mut i = left;
    let mut j = mid + 1;
    let mut k = left;

    while i <= mid && j <= right {
        if comparator(&array[i as usize], &array[j as usize]) <= 0 {
            scratch[k as usize] = array[i as usize].clone();
            i += 1;
        } else {
            scratch[k as usize] = array[j as usize].clone();
            j += 1;
        }
        k += 1;
    }

    while i <= mid {
        scratch[k as usize] = array[i as usize].clone();
        i += 1;
        k += 1;
    }

    while j <= right {
        scratch[k as usize] = array[j as usize].clone();
        j += 1;
        k += 1;
    }

    for idx in left..=right {
        array[idx as usize] = scratch[idx as usize].clone();
    }
}
