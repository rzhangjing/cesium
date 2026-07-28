//! Core utility functions.
//! Maps to CesiumJS `Core/binarySearch.js`, `Core/barycentricCoordinates.js`,
//! `Core/pointInsideTriangle.js`, `Core/subdivideArray.js`

use crate::math_utils::EPSILON14;
use glam::DVec3;

/// Finds an item in a sorted array using binary search.
///
/// Returns the index of `item_to_find` if it exists.
/// If not found, returns a negative number which is the bitwise complement (!)
/// of the index before which the item should be inserted.
///
/// Maps to `binarySearch(array, itemToFind, comparator)`
pub fn binary_search<T, F>(array: &[T], item_to_find: &T, comparator: F) -> i64
where
    F: Fn(&T, &T) -> i64,
{
    let mut low: i64 = 0;
    let mut high: i64 = array.len() as i64 - 1;

    while low <= high {
        let i = ((low + high) / 2) as usize;
        let comparison = comparator(&array[i], item_to_find);
        if comparison < 0 {
            low = i as i64 + 1;
        } else if comparison > 0 {
            high = i as i64 - 1;
        } else {
            return i as i64;
        }
    }
    !(high + 1)
}

/// Computes the barycentric coordinates for a point with respect to a triangle (3D).
///
/// Returns Some(DVec3) where x, y, z are the barycentric coordinates corresponding
/// to p0, p1, p2 respectively. Returns None if the triangle is degenerate.
///
/// Maps to `barycentricCoordinates(point, p0, p1, p2)`
pub fn barycentric_coordinates(
    point: DVec3,
    p0: DVec3,
    p1: DVec3,
    p2: DVec3,
) -> Option<DVec3> {
    // Check if point equals any vertex
    if point.abs_diff_eq(p0, EPSILON14) {
        return Some(DVec3::X);
    }
    if point.abs_diff_eq(p1, EPSILON14) {
        return Some(DVec3::Y);
    }
    if point.abs_diff_eq(p2, EPSILON14) {
        return Some(DVec3::Z);
    }

    let v0 = p1 - p0;
    let v1 = p2 - p0;
    let v2 = point - p0;

    let dot00 = v0.dot(v0);
    let dot01 = v0.dot(v1);
    let dot02 = v0.dot(v2);
    let dot11 = v1.dot(v1);
    let dot12 = v1.dot(v2);

    let mut y = dot11 * dot02 - dot01 * dot12;
    let mut z = dot00 * dot12 - dot01 * dot02;
    let q = dot00 * dot11 - dot01 * dot01;

    // Triangle is degenerate
    if q == 0.0 {
        return None;
    }

    y /= q;
    z /= q;
    let x = 1.0 - y - z;
    Some(DVec3::new(x, y, z))
}

/// Determines if a 2D point is inside a triangle defined by three 2D points.
///
/// Returns true only if the point is strictly inside (not on edges or vertices).
///
/// Maps to `pointInsideTriangle(point, p0, p1, p2)`
pub fn point_inside_triangle(
    point: (f64, f64),
    p0: (f64, f64),
    p1: (f64, f64),
    p2: (f64, f64),
) -> bool {
    // Use barycentric coordinate approach
    let (px, py) = point;
    let (x1, y1) = p0;
    let (x2, y2) = p1;
    let (x3, y3) = p2;

    let x1mx3 = x1 - x3;
    let x3mx2 = x3 - x2;
    let y2my3 = y2 - y3;
    let y1my3 = y1 - y3;
    let inverse_det = 1.0 / (y2my3 * x1mx3 + x3mx2 * y1my3);
    let dpx = px - x3;
    let dpy = py - y3;

    let u = (y2my3 * dpx + x3mx2 * dpy) * inverse_det;
    let v = (-y1my3 * dpx + x1mx3 * dpy) * inverse_det;
    let w = 1.0 - u - v;

    // Strictly inside: all coordinates must be > 0 (not on edge)
    u > 0.0 && v > 0.0 && w > 0.0
}

/// Splits an array into a specified number of sub-arrays.
///
/// Maps to `subdivideArray(array, numberOfArrays)`
pub fn subdivide_array<T: Clone>(array: &[T], number_of_arrays: usize) -> Vec<Vec<T>> {
    debug_assert!(number_of_arrays > 0, "number_of_arrays must be > 0");

    let length = array.len();
    if length == 0 {
        return Vec::new();
    }

    let mut result: Vec<Vec<T>> = Vec::with_capacity(number_of_arrays);
    let mut i = 0;
    for _ in 0..number_of_arrays {
        let remaining = length - i;
        let remaining_arrays = number_of_arrays - result.len();
        let count = (remaining + remaining_arrays - 1) / remaining_arrays;
        let end = (i + count).min(length);
        if i < end {
            result.push(array[i..end].to_vec());
        }
        i = end;
    }
    result
}

/// Sorts an array in place using a stable sort (merge sort semantics).
/// Maps to CesiumJS `Core/mergeSort.js`
///
/// The comparator returns an Ordering: Less if a should come before b.
pub fn merge_sort<T, F>(array: &mut [T], comparator: F)
where
    F: Fn(&T, &T) -> std::cmp::Ordering,
{
    // Rust's sort_by is a stable sort (adaptive merge sort + insertion sort),
    // which matches CesiumJS mergeSort semantics exactly.
    array.sort_by(comparator);
}
