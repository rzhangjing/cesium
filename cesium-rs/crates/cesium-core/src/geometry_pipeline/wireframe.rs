//! Wireframe conversion: `toWireframe`, `createLineSegmentsForVectors`.

use std::collections::HashMap;

use crate::bounding_sphere::BoundingSphere;
use crate::component_datatype::ComponentDatatype;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::index_datatype::IndexStorage;
use crate::primitive_type::PrimitiveType;

/// Converts triangle indices to line indices for wireframe rendering.
pub fn to_wireframe(geometry: &mut Geometry) {
    if let Some(ref indices) = geometry.indices {
        let new_indices = match geometry.primitive_type {
            PrimitiveType::Triangles => triangles_to_lines(indices),
            PrimitiveType::TriangleStrip => triangle_strip_to_lines(indices),
            PrimitiveType::TriangleFan => triangle_fan_to_lines(indices),
            _ => return,
        };
        geometry.indices = Some(new_indices);
        geometry.primitive_type = PrimitiveType::Lines;
    }
}

fn triangles_to_lines(triangles: &IndexStorage) -> IndexStorage {
    match triangles {
        IndexStorage::U16(t) => {
            let count = t.len();
            let size = (count / 3) * 6;
            let mut lines = vec![0u16; size];
            let mut idx = 0;
            for i in (0..count).step_by(3) {
                add_triangle(&mut lines, &mut idx, t[i], t[i + 1], t[i + 2]);
            }
            IndexStorage::U16(lines)
        }
        IndexStorage::U32(t) => {
            let count = t.len();
            let size = (count / 3) * 6;
            let mut lines = vec![0u32; size];
            let mut idx = 0;
            for i in (0..count).step_by(3) {
                add_triangle_u32(&mut lines, &mut idx, t[i], t[i + 1], t[i + 2]);
            }
            IndexStorage::U32(lines)
        }
    }
}

fn triangle_strip_to_lines(triangles: &IndexStorage) -> IndexStorage {
    match triangles {
        IndexStorage::U16(t) => {
            let count = t.len();
            if count < 3 { return IndexStorage::U16(vec![]); }
            let size = (count - 2) * 6;
            let mut lines = vec![0u16; size];
            add_triangle(&mut lines, &mut 0, t[0], t[1], t[2]);
            let mut idx = 6;
            for i in 3..count {
                add_triangle(&mut lines, &mut idx, t[i - 1], t[i], t[i - 2]);
            }
            IndexStorage::U16(lines)
        }
        IndexStorage::U32(t) => {
            let count = t.len();
            if count < 3 { return IndexStorage::U32(vec![]); }
            let size = (count - 2) * 6;
            let mut lines = vec![0u32; size];
            add_triangle_u32(&mut lines, &mut 0, t[0], t[1], t[2]);
            let mut idx = 6;
            for i in 3..count {
                add_triangle_u32(&mut lines, &mut idx, t[i - 1], t[i], t[i - 2]);
            }
            IndexStorage::U32(lines)
        }
    }
}

fn triangle_fan_to_lines(triangles: &IndexStorage) -> IndexStorage {
    match triangles {
        IndexStorage::U16(t) => {
            if t.is_empty() { return IndexStorage::U16(vec![]); }
            let count = t.len() - 1;
            let size = (count - 1) * 6;
            let mut lines = vec![0u16; size];
            let base = t[0];
            let mut idx = 0;
            for i in 1..count {
                add_triangle(&mut lines, &mut idx, base, t[i], t[i + 1]);
            }
            IndexStorage::U16(lines)
        }
        IndexStorage::U32(t) => {
            if t.is_empty() { return IndexStorage::U32(vec![]); }
            let count = t.len() - 1;
            let size = (count - 1) * 6;
            let mut lines = vec![0u32; size];
            let base = t[0];
            let mut idx = 0;
            for i in 1..count {
                add_triangle_u32(&mut lines, &mut idx, base, t[i], t[i + 1]);
            }
            IndexStorage::U32(lines)
        }
    }
}

fn add_triangle(lines: &mut [u16], index: &mut usize, i0: u16, i1: u16, i2: u16) {
    lines[*index] = i0; *index += 1;
    lines[*index] = i1; *index += 1;
    lines[*index] = i1; *index += 1;
    lines[*index] = i2; *index += 1;
    lines[*index] = i2; *index += 1;
    lines[*index] = i0; *index += 1;
}

fn add_triangle_u32(lines: &mut [u32], index: &mut usize, i0: u32, i1: u32, i2: u32) {
    lines[*index] = i0; *index += 1;
    lines[*index] = i1; *index += 1;
    lines[*index] = i1; *index += 1;
    lines[*index] = i2; *index += 1;
    lines[*index] = i2; *index += 1;
    lines[*index] = i0; *index += 1;
}

/// Creates line segments for a vector attribute (e.g. normals, tangents).
pub fn create_line_segments_for_vectors(
    geometry: &Geometry,
    attribute_name: Option<&str>,
    length: Option<f64>,
) -> Option<Geometry> {
    let attr_name = attribute_name.unwrap_or("normal");
    let len = length.unwrap_or(10000.0);

    let positions = geometry.attributes.get("position")?;
    let vectors = geometry.attributes.get(attr_name)?;

    let pos_vals = &positions.values;
    let vec_vals = &vectors.values;
    let pos_len = pos_vals.len();

    let mut new_positions = vec![0.0f64; 2 * pos_len];
    let mut j = 0;
    for i in (0..pos_len).step_by(3) {
        new_positions[j] = pos_vals[i]; j += 1;
        new_positions[j] = pos_vals[i + 1]; j += 1;
        new_positions[j] = pos_vals[i + 2]; j += 1;
        new_positions[j] = pos_vals[i] + vec_vals[i] * len; j += 1;
        new_positions[j] = pos_vals[i + 1] + vec_vals[i + 1] * len; j += 1;
        new_positions[j] = pos_vals[i + 2] + vec_vals[i + 2] * len; j += 1;
    }

    let new_bs = geometry.bounding_sphere.as_ref().map(|bs| {
        BoundingSphere::new(bs.center.clone(), bs.radius + len)
    });

    let mut attributes: HashMap<String, GeometryAttribute> = HashMap::new();
    attributes.insert(
        "position".to_string(),
        GeometryAttribute::new(ComponentDatatype::Double, 3, false, new_positions),
    );

    Some(Geometry::new(
        attributes,
        None,
        Some(PrimitiveType::Lines),
        new_bs,
    ))
}
