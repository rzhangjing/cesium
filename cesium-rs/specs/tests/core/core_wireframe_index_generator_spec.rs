//! Tests for `cesium_core::WireframeIndexGenerator`.

use cesium_core::primitive_type::PrimitiveType;
use cesium_core::wireframe_index_generator::{
    create_wireframe_indices, get_wireframe_indices_count,
};

#[test]
fn wireframe_from_triangles_vertex_count() {
    // 3 vertices = 1 triangle → 6 wireframe indices (3 edges × 2)
    let result = create_wireframe_indices(PrimitiveType::Triangles, 3, None);
    assert!(result.is_some());
    let wireframe = result.unwrap();
    assert_eq!(wireframe.len(), 6);
}

#[test]
fn wireframe_from_triangles_correct_edges() {
    let result = create_wireframe_indices(PrimitiveType::Triangles, 3, None).unwrap();
    // Triangle (0,1,2) → edges: (0,1), (1,2), (2,0)
    assert_eq!(result[0], 0);
    assert_eq!(result[1], 1);
    assert_eq!(result[2], 1);
    assert_eq!(result[3], 2);
    assert_eq!(result[4], 2);
    assert_eq!(result[5], 0);
}

#[test]
fn wireframe_from_two_triangles() {
    // 6 vertices = 2 triangles → 12 wireframe indices
    let result = create_wireframe_indices(PrimitiveType::Triangles, 6, None).unwrap();
    assert_eq!(result.len(), 12);
}

#[test]
fn wireframe_from_triangle_indices() {
    let original = vec![10, 20, 30];
    let result = create_wireframe_indices(PrimitiveType::Triangles, 3, Some(&original)).unwrap();
    assert_eq!(result.len(), 6);
    // Edges: (10,20), (20,30), (30,10)
    assert_eq!(result[0], 10);
    assert_eq!(result[1], 20);
    assert_eq!(result[2], 20);
    assert_eq!(result[3], 30);
    assert_eq!(result[4], 30);
    assert_eq!(result[5], 10);
}

#[test]
fn wireframe_from_triangle_strip() {
    // 4 vertices in a strip → 2 triangles
    let result = create_wireframe_indices(PrimitiveType::TriangleStrip, 4, None).unwrap();
    // 2 + (4-2)*4 = 2 + 8 = 10
    assert_eq!(result.len(), 10);
}

#[test]
fn wireframe_from_triangle_fan() {
    // 4 vertices in a fan → 2 triangles
    let result = create_wireframe_indices(PrimitiveType::TriangleFan, 4, None).unwrap();
    assert_eq!(result.len(), 10);
}

#[test]
fn wireframe_from_points_returns_none() {
    let result = create_wireframe_indices(PrimitiveType::Points, 3, None);
    assert!(result.is_none());
}

#[test]
fn wireframe_from_lines_returns_none() {
    let result = create_wireframe_indices(PrimitiveType::Lines, 4, None);
    assert!(result.is_none());
}

#[test]
fn get_wireframe_indices_count_triangles() {
    let count = get_wireframe_indices_count(PrimitiveType::Triangles, 9);
    assert_eq!(count, 18); // 9 * 2
}

#[test]
fn get_wireframe_indices_count_triangle_strip() {
    let count = get_wireframe_indices_count(PrimitiveType::TriangleStrip, 5);
    // 2 + (5-2)*4 = 2 + 12 = 14
    assert_eq!(count, 14);
}

#[test]
fn get_wireframe_indices_count_triangle_fan() {
    let count = get_wireframe_indices_count(PrimitiveType::TriangleFan, 5);
    assert_eq!(count, 14);
}
