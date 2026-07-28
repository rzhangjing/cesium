//! Wireframe index generator.
//! Maps to CesiumJS `Core/WireframeIndexGenerator.js`

/// Primitive types for geometry rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    Points,
    Lines,
    LineLoop,
    LineStrip,
    Triangles,
    TriangleStrip,
    TriangleFan,
}

/// Returns the number of wireframe indices that will be generated
/// for the given primitive type and index count.
pub fn get_wireframe_indices_count(primitive_type: PrimitiveType, index_count: usize) -> usize {
    match primitive_type {
        PrimitiveType::Triangles => {
            // Each triangle (3 indices) becomes 3 line segments (6 indices)
            (index_count / 3) * 6
        }
        PrimitiveType::TriangleStrip | PrimitiveType::TriangleFan => {
            // First edge + 2 edges per triangle
            if index_count < 3 {
                0
            } else {
                2 + (index_count - 2) * 4
            }
        }
        _ => index_count,
    }
}

/// Creates wireframe indices for the given primitive type.
///
/// Returns None for non-triangle primitive types.
/// If `indices` is provided, uses those as the source indices;
/// otherwise generates sequential indices [0, 1, 2, ...].
pub fn create_wireframe_indices(
    primitive_type: PrimitiveType,
    index_count: usize,
    indices: Option<&[u32]>,
) -> Option<Vec<u32>> {
    match primitive_type {
        PrimitiveType::Triangles => {
            let triangle_count = index_count / 3;
            let mut result = Vec::with_capacity(triangle_count * 6);
            for i in 0..triangle_count {
                let (i0, i1, i2) = if let Some(idx) = indices {
                    (idx[i * 3], idx[i * 3 + 1], idx[i * 3 + 2])
                } else {
                    let base = (i * 3) as u32;
                    (base, base + 1, base + 2)
                };
                result.push(i0);
                result.push(i1);
                result.push(i1);
                result.push(i2);
                result.push(i2);
                result.push(i0);
            }
            Some(result)
        }
        PrimitiveType::TriangleStrip => {
            if index_count < 3 {
                return Some(Vec::new());
            }
            let get = |i: usize| -> u32 {
                if let Some(idx) = indices {
                    idx[i]
                } else {
                    i as u32
                }
            };
            let mut result = Vec::new();
            // First edge
            result.push(get(0));
            result.push(get(1));
            // For each triangle in the strip
            for i in 0..(index_count - 2) {
                let i0 = get(i);
                let i1 = get(i + 1);
                let i2 = get(i + 2);
                result.push(i1);
                result.push(i2);
                result.push(i2);
                result.push(i0);
            }
            Some(result)
        }
        PrimitiveType::TriangleFan => {
            if index_count < 3 {
                return Some(Vec::new());
            }
            let get = |i: usize| -> u32 {
                if let Some(idx) = indices {
                    idx[i]
                } else {
                    i as u32
                }
            };
            let mut result = Vec::new();
            // First edge
            result.push(get(0));
            result.push(get(1));
            // For each triangle in the fan (all share vertex 0)
            for i in 0..(index_count - 2) {
                let i0 = get(0); // center vertex
                let i1 = get(i + 1);
                let i2 = get(i + 2);
                result.push(i1);
                result.push(i2);
                result.push(i2);
                result.push(i0);
            }
            Some(result)
        }
        _ => None,
    }
}
