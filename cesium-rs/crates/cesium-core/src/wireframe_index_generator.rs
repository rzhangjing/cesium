//! Ported from `packages/engine/Source/Core/WireframeIndexGenerator.js`.
//!
//! Generates wireframe indices from triangle primitives.

use crate::primitive_type::PrimitiveType;

/// Generates wireframe indices for a primitive.
pub fn create_wireframe_indices(
    primitive_type: PrimitiveType,
    vertex_count: usize,
    original_indices: Option<&[u32]>,
) -> Option<Vec<u32>> {
    match primitive_type {
        PrimitiveType::Triangles => {
            if let Some(orig) = original_indices {
                Some(create_wireframe_from_triangle_indices(vertex_count, orig))
            } else {
                Some(create_wireframe_from_triangles(vertex_count))
            }
        }
        PrimitiveType::TriangleStrip => {
            if let Some(orig) = original_indices {
                Some(create_wireframe_from_triangle_strip_indices(vertex_count, orig))
            } else {
                Some(create_wireframe_from_triangle_strip(vertex_count))
            }
        }
        PrimitiveType::TriangleFan => {
            if let Some(orig) = original_indices {
                Some(create_wireframe_from_triangle_fan_indices(vertex_count, orig))
            } else {
                Some(create_wireframe_from_triangle_fan(vertex_count))
            }
        }
        _ => None,
    }
}

/// Gets the number of wireframe indices for a primitive type.
pub fn get_wireframe_indices_count(primitive_type: PrimitiveType, original_count: usize) -> usize {
    match primitive_type {
        PrimitiveType::Triangles => original_count * 2,
        PrimitiveType::TriangleStrip | PrimitiveType::TriangleFan => {
            let num_triangles = original_count - 2;
            2 + num_triangles * 4
        }
        _ => original_count,
    }
}

fn create_wireframe_from_triangles(vertex_count: usize) -> Vec<u32> {
    let mut wireframe = vec![0u32; vertex_count * 2];
    let mut index = 0;
    let mut i = 0;
    while i < vertex_count {
        wireframe[index] = i as u32;
        wireframe[index + 1] = (i + 1) as u32;
        wireframe[index + 2] = (i + 1) as u32;
        wireframe[index + 3] = (i + 2) as u32;
        wireframe[index + 4] = (i + 2) as u32;
        wireframe[index + 5] = i as u32;
        index += 6;
        i += 3;
    }
    wireframe
}

fn create_wireframe_from_triangle_indices(vertex_count: usize, original: &[u32]) -> Vec<u32> {
    let count = original.len();
    let mut wireframe = vec![0u32; count * 2];
    let mut index = 0;
    let mut i = 0;
    while i < count {
        let p0 = original[i];
        let p1 = original[i + 1];
        let p2 = original[i + 2];
        wireframe[index] = p0;
        wireframe[index + 1] = p1;
        wireframe[index + 2] = p1;
        wireframe[index + 3] = p2;
        wireframe[index + 4] = p2;
        wireframe[index + 5] = p0;
        index += 6;
        i += 3;
    }
    wireframe
}

fn create_wireframe_from_triangle_strip(vertex_count: usize) -> Vec<u32> {
    let num_triangles = vertex_count - 2;
    let count = 2 + num_triangles * 4;
    let mut wireframe = vec![0u32; count];
    let mut index = 0;
    wireframe[index] = 0;
    wireframe[index + 1] = 1;
    index += 2;
    for i in 0..num_triangles {
        wireframe[index] = (i + 1) as u32;
        wireframe[index + 1] = (i + 2) as u32;
        wireframe[index + 2] = (i + 2) as u32;
        wireframe[index + 3] = i as u32;
        index += 4;
    }
    wireframe
}

fn create_wireframe_from_triangle_strip_indices(
    _vertex_count: usize,
    original: &[u32],
) -> Vec<u32> {
    let count = original.len();
    let num_triangles = count - 2;
    let wireframe_count = 2 + num_triangles * 4;
    let mut wireframe = vec![0u32; wireframe_count];
    let mut index = 0;
    wireframe[index] = original[0];
    wireframe[index + 1] = original[1];
    index += 2;
    for i in 0..num_triangles {
        let p0 = original[i];
        let p1 = original[i + 1];
        let p2 = original[i + 2];
        wireframe[index] = p1;
        wireframe[index + 1] = p2;
        wireframe[index + 2] = p2;
        wireframe[index + 3] = p0;
        index += 4;
    }
    wireframe
}

fn create_wireframe_from_triangle_fan(vertex_count: usize) -> Vec<u32> {
    let num_triangles = vertex_count - 2;
    let count = 2 + num_triangles * 4;
    let mut wireframe = vec![0u32; count];
    let mut index = 0;
    wireframe[index] = 0;
    wireframe[index + 1] = 1;
    index += 2;
    for i in 0..num_triangles {
        wireframe[index] = (i + 1) as u32;
        wireframe[index + 1] = (i + 2) as u32;
        wireframe[index + 2] = (i + 2) as u32;
        wireframe[index + 3] = 0;
        index += 4;
    }
    wireframe
}

fn create_wireframe_from_triangle_fan_indices(
    _vertex_count: usize,
    original: &[u32],
) -> Vec<u32> {
    let count = original.len();
    let num_triangles = count - 2;
    let wireframe_count = 2 + num_triangles * 4;
    let mut wireframe = vec![0u32; wireframe_count];
    let mut index = 0;
    let first_point = original[0];
    wireframe[index] = first_point;
    wireframe[index + 1] = original[1];
    index += 2;
    for i in 0..num_triangles {
        let p1 = original[i + 1];
        let p2 = original[i + 2];
        wireframe[index] = p1;
        wireframe[index + 1] = p2;
        wireframe[index + 2] = p2;
        wireframe[index + 3] = first_point;
        index += 4;
    }
    wireframe
}
