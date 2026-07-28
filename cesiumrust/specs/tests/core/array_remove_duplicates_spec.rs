//! Core/arrayRemoveDuplicatesSpec.js → Rust integration tests
//! 25 original it() blocks → 21 A-class tests ported
//!
//! Skipped C-class tests:
//! - "returns undefined" - Rust uses Option/empty slice (compile-time safety)
//! - "anonymous types" / "Spherical type" - Rust is statically typed (DVec3 covers the logic)
//! - "doesn't modify removedIndices length===1" - merged into no-duplicates test

use cesium_geospatial::array_utils::array_remove_duplicates;
use cesium_geospatial::math_utils::EPSILON10;
use glam::DVec3;

/// CesiumJS Cartesian3.equalsEpsilon - checks both absolute and relative epsilon
fn vec3_equals_epsilon(left: &DVec3, right: &DVec3, epsilon: f64) -> bool {
    let dx = (left.x - right.x).abs();
    let dy = (left.y - right.y).abs();
    let dz = (left.z - right.z).abs();
    // CesiumJS equalsEpsilon: abs(l-r) <= epsilon * max(1, abs(l), abs(r))
    let ex = epsilon * left.x.abs().max(right.x.abs()).max(1.0);
    let ey = epsilon * left.y.abs().max(right.y.abs()).max(1.0);
    let ez = epsilon * left.z.abs().max(right.z.abs()).max(1.0);
    dx <= ex && dy <= ey && dz <= ez
}

// ============================================================================
// No duplicates
// ============================================================================

#[test]
fn returns_positions_if_none_removed_length_1() {
    let positions = vec![DVec3::ZERO];
    let (result, removed) = array_remove_duplicates(&positions, vec3_equals_epsilon, false);
    assert_eq!(result.len(), 1);
    assert!(removed.is_empty());
}

#[test]
fn returns_positions_if_none_removed_length_gt_1() {
    let positions = vec![DVec3::ZERO, DVec3::X, DVec3::Y, DVec3::Z];
    let (result, removed) = array_remove_duplicates(&positions, vec3_equals_epsilon, false);
    assert_eq!(result.len(), 4);
    assert!(removed.is_empty());
}

#[test]
fn wrapping_returns_positions_if_none_removed() {
    let positions = vec![DVec3::ZERO, DVec3::X, DVec3::Y, DVec3::Z];
    let (result, removed) = array_remove_duplicates(&positions, vec3_equals_epsilon, true);
    assert_eq!(result.len(), 4);
    assert!(removed.is_empty());
}

// ============================================================================
// Basic duplicate removal
// ============================================================================

#[test]
fn removes_duplicates() {
    let positions = vec![
        DVec3::splat(1.0),
        DVec3::splat(1.0),
        DVec3::splat(1.0),
        DVec3::splat(1.0),
        DVec3::splat(2.0),
        DVec3::splat(3.0),
        DVec3::splat(3.0),
    ];
    let expected = vec![DVec3::splat(1.0), DVec3::splat(2.0), DVec3::splat(3.0)];
    let (result, _) = array_remove_duplicates(&positions, vec3_equals_epsilon, false);
    assert_eq!(result, expected);
}

#[test]
fn doesnt_remove_nonadjacent_duplicates() {
    let positions = vec![
        DVec3::splat(1.0),
        DVec3::splat(1.0),
        DVec3::splat(1.0),
        DVec3::splat(1.0),
        DVec3::splat(2.0),
        DVec3::splat(1.0),
        DVec3::splat(3.0),
        DVec3::splat(3.0),
    ];
    let expected = vec![
        DVec3::splat(1.0),
        DVec3::splat(2.0),
        DVec3::splat(1.0),
        DVec3::splat(3.0),
    ];
    let (result, _) = array_remove_duplicates(&positions, vec3_equals_epsilon, false);
    assert_eq!(result, expected);
}

#[test]
fn works_with_empty_array() {
    let positions: Vec<DVec3> = vec![];
    let (result, removed) = array_remove_duplicates(&positions, vec3_equals_epsilon, false);
    assert!(result.is_empty());
    assert!(removed.is_empty());
}

// ============================================================================
// Epsilon behavior
// ============================================================================

#[test]
fn removes_positions_within_absolute_epsilon10() {
    let positions = vec![
        DVec3::new(1.0, 1.0, 1.0),
        DVec3::new(1.0, 2.0, 3.0),
        DVec3::new(1.0, 2.0, 3.0 + EPSILON10),
    ];
    let expected = vec![DVec3::new(1.0, 1.0, 1.0), DVec3::new(1.0, 2.0, 3.0)];
    let (result, _) = array_remove_duplicates(&positions, vec3_equals_epsilon, false);
    assert_eq!(result, expected);
}

#[test]
fn removes_positions_within_relative_epsilon10() {
    let positions = vec![
        DVec3::new(0.0, 0.0, 1000000.0),
        DVec3::new(0.0, 0.0, 3000000.0),
        DVec3::new(0.0, 0.0, 3000000.0002),
    ];
    let expected = vec![
        DVec3::new(0.0, 0.0, 1000000.0),
        DVec3::new(0.0, 0.0, 3000000.0),
    ];
    let (result, _) = array_remove_duplicates(&positions, vec3_equals_epsilon, false);
    assert_eq!(result, expected);
}

#[test]
fn keeps_positions_that_add_up_past_relative_epsilon10() {
    let eighty_percent = 0.8 * EPSILON10;
    let positions = vec![
        DVec3::new(0.0, 0.0, 1.0),
        DVec3::new(0.0, 0.0, 1.0 + eighty_percent),
        DVec3::new(0.0, 0.0, 1.0 + 2.0 * eighty_percent),
        DVec3::new(0.0, 0.0, 1.0 + 3.0 * eighty_percent),
    ];
    // First and second are within epsilon → second removed
    // Third is compared to first (v0 stays at first): 2*0.8=1.6 > 1.0 epsilon → kept
    // Fourth compared to third: diff = 0.8*epsilon < epsilon → removed
    let expected = vec![
        DVec3::new(0.0, 0.0, 1.0),
        DVec3::new(0.0, 0.0, 1.0 + 2.0 * eighty_percent),
    ];
    let (result, _) = array_remove_duplicates(&positions, vec3_equals_epsilon, false);
    assert_eq!(result, expected);
}

// ============================================================================
// Wrap-around behavior
// ============================================================================

#[test]
fn doesnt_remove_first_last_without_wrapping() {
    let positions = vec![
        DVec3::splat(1.0),
        DVec3::splat(2.0),
        DVec3::splat(3.0),
        DVec3::splat(1.0),
    ];
    let (result, removed) = array_remove_duplicates(&positions, vec3_equals_epsilon, false);
    assert_eq!(result.len(), 4);
    assert!(removed.is_empty());
}

#[test]
fn wrapping_removes_duplicate_first_and_last() {
    let positions = vec![
        DVec3::splat(1.0),
        DVec3::splat(2.0),
        DVec3::splat(3.0),
        DVec3::splat(1.0),
    ];
    let expected = vec![DVec3::splat(1.0), DVec3::splat(2.0), DVec3::splat(3.0)];
    let (result, _) = array_remove_duplicates(&positions, vec3_equals_epsilon, true);
    assert_eq!(result, expected);
}

#[test]
fn wrapping_removes_duplicates_including_first_and_last() {
    let positions = vec![
        DVec3::splat(1.0),
        DVec3::splat(1.0),
        DVec3::splat(2.0),
        DVec3::splat(2.0),
        DVec3::splat(3.0),
        DVec3::splat(1.0),
        DVec3::splat(1.0),
    ];
    let expected = vec![DVec3::splat(1.0), DVec3::splat(2.0), DVec3::splat(3.0)];
    let (result, _) = array_remove_duplicates(&positions, vec3_equals_epsilon, true);
    assert_eq!(result, expected);
}

#[test]
fn wrapping_removes_string_of_duplicates_at_end() {
    let positions = vec![
        DVec3::splat(1.0),
        DVec3::splat(1.0),
        DVec3::splat(2.0),
        DVec3::splat(3.0),
        DVec3::splat(1.0),
        DVec3::splat(1.0),
        DVec3::splat(1.0),
        DVec3::splat(1.0),
        DVec3::splat(1.0),
    ];
    let expected = vec![DVec3::splat(1.0), DVec3::splat(2.0), DVec3::splat(3.0)];
    let (result, _) = array_remove_duplicates(&positions, vec3_equals_epsilon, true);
    assert_eq!(result, expected);
}

#[test]
fn wrapping_doesnt_remove_nonadjacent_duplicates() {
    let positions = vec![
        DVec3::splat(1.0),
        DVec3::splat(2.0),
        DVec3::splat(1.0),
        DVec3::splat(3.0),
        DVec3::splat(1.0),
    ];
    // Wrap-around: last(1,1,1)==first(1,1,1) → remove last
    // But also adjacents: no adjacent duplicates
    let expected = vec![
        DVec3::splat(1.0),
        DVec3::splat(2.0),
        DVec3::splat(1.0),
        DVec3::splat(3.0),
    ];
    let (result, _) = array_remove_duplicates(&positions, vec3_equals_epsilon, true);
    assert_eq!(result, expected);
}

// ============================================================================
// removedIndices tracking
// ============================================================================

#[test]
fn removed_indices_empty_when_no_duplicates_length_1() {
    let positions = vec![DVec3::ZERO];
    let (_, removed) = array_remove_duplicates(&positions, vec3_equals_epsilon, false);
    assert!(removed.is_empty());
}

#[test]
fn removed_indices_empty_when_no_duplicates_length_gt_1() {
    let positions = vec![DVec3::ZERO, DVec3::X, DVec3::Y, DVec3::Z];
    let (_, removed) = array_remove_duplicates(&positions, vec3_equals_epsilon, false);
    assert!(removed.is_empty());
}

#[test]
fn removed_indices_modified_when_duplicates() {
    let positions = vec![DVec3::ZERO, DVec3::X, DVec3::X, DVec3::Y, DVec3::Z, DVec3::Z];
    let expected = vec![DVec3::ZERO, DVec3::X, DVec3::Y, DVec3::Z];
    let (result, removed) = array_remove_duplicates(&positions, vec3_equals_epsilon, false);
    assert_eq!(result, expected);
    assert_eq!(removed, vec![2, 5]);
}

#[test]
fn removed_indices_empty_without_wrapping_when_first_eq_last() {
    let positions = vec![
        DVec3::splat(1.0),
        DVec3::splat(2.0),
        DVec3::splat(3.0),
        DVec3::splat(1.0),
    ];
    let (_, removed) = array_remove_duplicates(&positions, vec3_equals_epsilon, false);
    assert!(removed.is_empty());
}

#[test]
fn removed_indices_wrapped_when_first_eq_last() {
    let positions = vec![DVec3::ZERO, DVec3::X, DVec3::Y, DVec3::Z, DVec3::ZERO];
    let expected = vec![DVec3::ZERO, DVec3::X, DVec3::Y, DVec3::Z];
    let (result, removed) = array_remove_duplicates(&positions, vec3_equals_epsilon, true);
    assert_eq!(result, expected);
    assert_eq!(removed, vec![4]);
}

#[test]
fn removed_indices_with_duplicates_and_wrapping() {
    let positions = vec![
        DVec3::ZERO,
        DVec3::ZERO,
        DVec3::X,
        DVec3::Y,
        DVec3::Y,
        DVec3::Z,
        DVec3::Z,
        DVec3::ZERO,
    ];
    let expected = vec![DVec3::ZERO, DVec3::X, DVec3::Y, DVec3::Z];
    let (result, removed) = array_remove_duplicates(&positions, vec3_equals_epsilon, true);
    assert_eq!(result, expected);
    assert_eq!(removed, vec![1, 4, 6, 7]);
}

#[test]
fn wrapping_removed_indices_with_string_of_duplicates() {
    let positions = vec![
        DVec3::splat(1.0),
        DVec3::splat(1.0),
        DVec3::splat(2.0),
        DVec3::splat(3.0),
        DVec3::splat(1.0),
        DVec3::splat(1.0),
        DVec3::splat(1.0),
        DVec3::splat(1.0),
        DVec3::splat(1.0),
    ];
    let expected = vec![DVec3::splat(1.0), DVec3::splat(2.0), DVec3::splat(3.0)];
    let (result, removed) = array_remove_duplicates(&positions, vec3_equals_epsilon, true);
    assert_eq!(result, expected);
    assert_eq!(removed, vec![1, 4, 5, 6, 7, 8]);
}

#[test]
fn wrapping_removed_indices_with_multiple_strings() {
    let positions = vec![
        DVec3::splat(1.0),
        DVec3::splat(1.0),
        DVec3::splat(2.0),
        DVec3::splat(3.0),
        DVec3::splat(3.0),
        DVec3::splat(1.0),
        DVec3::splat(1.0),
        DVec3::splat(1.0),
        DVec3::splat(3.0),
        DVec3::splat(3.0),
        DVec3::splat(1.0),
        DVec3::splat(1.0),
    ];
    let expected = vec![
        DVec3::splat(1.0),
        DVec3::splat(2.0),
        DVec3::splat(3.0),
        DVec3::splat(1.0),
        DVec3::splat(3.0),
    ];
    let (result, removed) = array_remove_duplicates(&positions, vec3_equals_epsilon, true);
    assert_eq!(result, expected);
    assert_eq!(removed, vec![1, 4, 6, 7, 9, 10, 11]);
}
