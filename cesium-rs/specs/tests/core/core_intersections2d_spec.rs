use cesium_core::intersections2d::Intersections2D;
use cesium_core::math::CesiumMath;

#[test]
fn eliminates_triangle_entirely_on_wrong_side() {
    let result = Intersections2D::clip_triangle_at_axis_aligned_threshold(0.1, false, 0.2, 0.3, 0.4);
    assert!(result.is_empty());
}

#[test]
fn keeps_triangle_entirely_on_correct_side() {
    let result = Intersections2D::clip_triangle_at_axis_aligned_threshold(0.1, true, 0.2, 0.3, 0.4);
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], 0.0);
    assert_eq!(result[1], 1.0);
    assert_eq!(result[2], 2.0);
}

#[test]
fn clips_when_point0_on_wrong_side_above() {
    let result = Intersections2D::clip_triangle_at_axis_aligned_threshold(0.5, false, 0.6, 0.4, 0.2);
    assert_eq!(result.len(), 10);
    assert_eq!(result[0], 1.0);
    assert_eq!(result[1], 2.0);
    // First interpolated vertex: edge 0→2
    assert_eq!(result[2], -1.0);
    assert_eq!(result[3], 0.0);
    assert_eq!(result[4], 2.0);
    assert!((result[5] - 0.25).abs() < CesiumMath::EPSILON14);
    // Second interpolated vertex: edge 0→1
    assert_eq!(result[6], -1.0);
    assert_eq!(result[7], 0.0);
    assert_eq!(result[8], 1.0);
    assert!((result[9] - 0.5).abs() < CesiumMath::EPSILON14);
}

#[test]
fn clips_when_point0_on_wrong_side_below() {
    let result = Intersections2D::clip_triangle_at_axis_aligned_threshold(0.5, true, 0.4, 0.6, 0.8);
    assert_eq!(result.len(), 10);
    assert_eq!(result[0], 1.0);
    assert_eq!(result[1], 2.0);
}

#[test]
fn compute_barycentric_coordinates_at_vertex() {
    // At vertex (0,0) of triangle (0,0)(1,0)(0,1)
    let result = Intersections2D::compute_barycentric_coordinates(0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0);
    assert!((result.x - 1.0).abs() < CesiumMath::EPSILON14);
    assert!((result.y - 0.0).abs() < CesiumMath::EPSILON14);
    assert!((result.z - 0.0).abs() < CesiumMath::EPSILON14);
}

#[test]
fn compute_barycentric_coordinates_at_centroid() {
    let result = Intersections2D::compute_barycentric_coordinates(
        1.0 / 3.0, 1.0 / 3.0,
        0.0, 0.0, 1.0, 0.0, 0.0, 1.0,
    );
    let third = 1.0 / 3.0;
    assert!((result.x - third).abs() < CesiumMath::EPSILON14);
    assert!((result.y - third).abs() < CesiumMath::EPSILON14);
    assert!((result.z - third).abs() < CesiumMath::EPSILON14);
}

#[test]
fn line_segment_intersection_works() {
    // Two crossing segments: (0,0)→(1,1) and (1,0)→(0,1)
    let result = Intersections2D::compute_line_segment_line_segment_intersection(
        0.0, 0.0, 1.0, 1.0,
        1.0, 0.0, 0.0, 1.0,
    );
    assert!(result.is_some());
    let pt = result.unwrap();
    assert!((pt.x - 0.5).abs() < CesiumMath::EPSILON14);
    assert!((pt.y - 0.5).abs() < CesiumMath::EPSILON14);
}

#[test]
fn line_segment_intersection_returns_none_for_parallel() {
    let result = Intersections2D::compute_line_segment_line_segment_intersection(
        0.0, 0.0, 1.0, 0.0,
        0.0, 1.0, 1.0, 1.0,
    );
    assert!(result.is_none());
}
