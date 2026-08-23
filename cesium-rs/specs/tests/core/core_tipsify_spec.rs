//! Tests for `cesium_core::tipsify` and `cesium_core::calculate_acmr`.

use cesium_core::tipsify::{calculate_acmr, tipsify, TipsifyOptions};

#[test]
fn tipsify_empty_indices_returns_empty() {
    let options = TipsifyOptions {
        indices: &[],
        maximum_index: None,
        cache_size: 24,
    };
    let result = tipsify(&options);
    assert!(result.is_empty());
}

#[test]
fn tipsify_single_triangle_returns_same() {
    let indices = vec![0, 1, 2];
    let options = TipsifyOptions {
        indices: &indices,
        maximum_index: None,
        cache_size: 24,
    };
    let result = tipsify(&options);
    assert_eq!(result.len(), 3);
    // Should contain the same three indices
    assert!(result.contains(&0));
    assert!(result.contains(&1));
    assert!(result.contains(&2));
}

#[test]
fn tipsify_two_triangles_optimizes() {
    // Two triangles sharing an edge
    let indices = vec![0, 1, 2, 1, 3, 2];
    let options = TipsifyOptions {
        indices: &indices,
        maximum_index: None,
        cache_size: 24,
    };
    let result = tipsify(&options);
    assert_eq!(result.len(), 6);
}

#[test]
fn tipsify_invalid_length_returns_empty() {
    // Not a multiple of 3
    let indices = vec![0, 1];
    let options = TipsifyOptions {
        indices: &indices,
        maximum_index: None,
        cache_size: 24,
    };
    let result = tipsify(&options);
    assert!(result.is_empty());
}

#[test]
fn calculate_acmr_single_triangle() {
    let indices = vec![0, 1, 2];
    let options = TipsifyOptions {
        indices: &indices,
        maximum_index: None,
        cache_size: 24,
    };
    let acmr = calculate_acmr(&options);
    // Single triangle: ACMR > 0 (all vertices are cache misses)
    assert!(acmr > 0.0);
}

#[test]
fn calculate_acmr_invalid_indices_returns_zero() {
    let indices = vec![0, 1];
    let options = TipsifyOptions {
        indices: &indices,
        maximum_index: None,
        cache_size: 24,
    };
    let acmr = calculate_acmr(&options);
    assert_eq!(acmr, 0.0);
}

#[test]
fn calculate_acmr_shared_vertices_lower_than_disjoint() {
    // Two triangles sharing an edge
    let shared = vec![0, 1, 2, 1, 3, 2];
    let shared_options = TipsifyOptions {
        indices: &shared,
        maximum_index: None,
        cache_size: 24,
    };
    let shared_acmr = calculate_acmr(&shared_options);

    // Two disjoint triangles
    let disjoint = vec![0, 1, 2, 10, 11, 12];
    let disjoint_options = TipsifyOptions {
        indices: &disjoint,
        maximum_index: None,
        cache_size: 24,
    };
    let disjoint_acmr = calculate_acmr(&disjoint_options);

    // Shared vertices should have lower ACMR
    assert!(shared_acmr < disjoint_acmr);
}

#[test]
fn tipsify_with_maximum_index() {
    let indices = vec![0, 1, 2];
    let options = TipsifyOptions {
        indices: &indices,
        maximum_index: Some(100),
        cache_size: 24,
    };
    let result = tipsify(&options);
    assert_eq!(result.len(), 3);
}
