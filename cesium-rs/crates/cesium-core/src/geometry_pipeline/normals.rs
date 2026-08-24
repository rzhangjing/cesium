//! Ported from `packages/engine/Source/Core/GeometryPipeline.js`
//! (section: computeNormal / computeTangentAndBitangent).

use crate::cartesian3::Cartesian3;
use crate::component_datatype::ComponentDatatype;
use crate::developer_error::throw_developer_error;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::math::CesiumMath;
use crate::primitive_type::PrimitiveType;

struct VertexNormalData {
    index_offset: usize,
    count: usize,
    current_count: usize,
}

/// Computes per-vertex normals for a geometry containing `TRIANGLES` by
/// averaging the normals of all triangles incident to the vertex. The result
/// is a new `normal` attribute added to the geometry. This assumes a
/// counter-clockwise winding order.
///
/// Port of `GeometryPipeline.computeNormal(geometry)`.
///
/// # Panics (debug)
/// - If `position` attribute values are missing.
/// - If `indices` is missing, or its length is `< 2` or not a multiple of 3.
/// - If `primitive_type` is not `TRIANGLES`.
pub fn compute_normal(geometry: &mut Geometry) {
    if cfg!(debug_assertions) {
        if geometry.attributes.get("position").is_none() {
            throw_developer_error("geometry.attributes.position.values is required.");
        }
        if geometry.indices.is_none() {
            throw_developer_error("geometry.indices is required.");
        }
        let indices = geometry.indices.as_ref().unwrap();
        if indices.len() < 2 || indices.len() % 3 != 0 {
            throw_developer_error(
                "geometry.indices length must be greater than 0 and be a multiple of 3.",
            );
        }
        if geometry.primitive_type != PrimitiveType::Triangles {
            throw_developer_error("geometry.primitiveType must be PrimitiveType.TRIANGLES.");
        }
    }

    let indices = geometry.indices.clone().unwrap();
    let vertices = geometry.attributes["position"].values.clone();
    let num_vertices = vertices.len() / 3;
    let num_indices = indices.len();

    let mut normals_per_vertex: Vec<VertexNormalData> = (0..num_vertices)
        .map(|_| VertexNormalData {
            index_offset: 0,
            count: 0,
            current_count: 0,
        })
        .collect();
    let mut normals_per_triangle: Vec<Cartesian3> = vec![Cartesian3::ZERO; num_indices / 3];
    let mut normal_indices: Vec<usize> = vec![0; num_indices];

    let mut j = 0usize;
    let mut i = 0usize;
    while i < num_indices {
        let i0 = index_at(&indices, i) as usize;
        let i1 = index_at(&indices, i + 1) as usize;
        let i2 = index_at(&indices, i + 2) as usize;
        let i03 = i0 * 3;
        let i13 = i1 * 3;
        let i23 = i2 * 3;

        let v0 = Cartesian3::new(vertices[i03], vertices[i03 + 1], vertices[i03 + 2]);
        let v1_in = Cartesian3::new(vertices[i13], vertices[i13 + 1], vertices[i13 + 2]);
        let v2_in = Cartesian3::new(vertices[i23], vertices[i23 + 1], vertices[i23 + 2]);
        let mut v1 = Cartesian3::ZERO;
        let mut v2 = Cartesian3::ZERO;

        normals_per_vertex[i0].count += 1;
        normals_per_vertex[i1].count += 1;
        normals_per_vertex[i2].count += 1;

        Cartesian3::subtract(&v1_in, &v0, &mut v1);
        Cartesian3::subtract(&v2_in, &v0, &mut v2);
        normals_per_triangle[j] = Cartesian3::cross_new(&v1, &v2);
        j += 1;
        i += 3;
    }

    let mut index_offset = 0usize;
    for data in normals_per_vertex.iter_mut() {
        data.index_offset += index_offset;
        index_offset += data.count;
    }

    j = 0;
    i = 0;
    while i < num_indices {
        for k in 0..3 {
            let vertex_index = index_at(&indices, i + k) as usize;
            let vertex_normal_data = &mut normals_per_vertex[vertex_index];
            let index = vertex_normal_data.index_offset + vertex_normal_data.current_count;
            normal_indices[index] = j;
            vertex_normal_data.current_count += 1;
        }
        j += 1;
        i += 3;
    }

    let mut normal_values = vec![0.0f64; num_vertices * 3];
    for vertex in 0..num_vertices {
        let i3 = vertex * 3;
        let count = normals_per_vertex[vertex].count;
        let offset = normals_per_vertex[vertex].index_offset;
        let mut normal = Cartesian3::ZERO;
        if count > 0 {
            for k in 0..count {
                let mut acc = Cartesian3::ZERO;
                Cartesian3::add(&normal, &normals_per_triangle[normal_indices[offset + k]], &mut acc);
                normal = acc;
            }

            // We can run into an issue where a vertex is used with 2 primitives
            // that have opposite winding order.
            if Cartesian3::equals_epsilon(
                Some(&Cartesian3::ZERO),
                Some(&normal),
                Some(CesiumMath::EPSILON10),
                Some(CesiumMath::EPSILON10),
            ) {
                normal = normals_per_triangle[normal_indices[offset]];
            }
        }

        // We end up with a zero vector probably because of a degenerate triangle
        if Cartesian3::equals_epsilon(
            Some(&Cartesian3::ZERO),
            Some(&normal),
            Some(CesiumMath::EPSILON10),
            Some(CesiumMath::EPSILON10),
        ) {
            // Default to (0,0,1)
            normal.z = 1.0;
        }

        let mut normalized = Cartesian3::ZERO;
        Cartesian3::normalize(&normal, &mut normalized);
        normal_values[i3] = normalized.x;
        normal_values[i3 + 1] = normalized.y;
        normal_values[i3 + 2] = normalized.z;
    }

    geometry.attributes.insert(
        "normal".to_string(),
        GeometryAttribute::new(ComponentDatatype::Float, 3, false, normal_values),
    );
}

/// Computes per-vertex tangents and bitangents for a geometry containing
/// `TRIANGLES`. The result is new `tangent` and `bitangent` attributes added
/// to the geometry. This assumes a counter-clockwise winding order.
///
/// Based on [Computing Tangent Space Basis Vectors for an Arbitrary Mesh]
/// (http://www.terathon.com/code/tangent.html) by Eric Lengyel.
///
/// Port of `GeometryPipeline.computeTangentAndBitangent(geometry)`.
///
/// # Panics (debug)
/// - If `position`, `normal`, `st` attribute values are missing.
/// - If `indices` is missing, or its length is `< 2` or not a multiple of 3.
/// - If `primitive_type` is not `TRIANGLES`.
pub fn compute_tangent_and_bitangent(geometry: &mut Geometry) {
    if cfg!(debug_assertions) {
        if geometry.attributes.get("position").is_none() {
            throw_developer_error("geometry.attributes.position.values is required.");
        }
        if geometry.attributes.get("normal").is_none() {
            throw_developer_error("geometry.attributes.normal.values is required.");
        }
        if geometry.attributes.get("st").is_none() {
            throw_developer_error("geometry.attributes.st.values is required.");
        }
        if geometry.indices.is_none() {
            throw_developer_error("geometry.indices is required.");
        }
        let indices = geometry.indices.as_ref().unwrap();
        if indices.len() < 2 || indices.len() % 3 != 0 {
            throw_developer_error(
                "geometry.indices length must be greater than 0 and be a multiple of 3.",
            );
        }
        if geometry.primitive_type != PrimitiveType::Triangles {
            throw_developer_error("geometry.primitiveType must be PrimitiveType.TRIANGLES.");
        }
    }

    let vertices = geometry.attributes["position"].values.clone();
    let normals = geometry.attributes["normal"].values.clone();
    let st = geometry.attributes["st"].values.clone();
    let indices = geometry.indices.clone().unwrap();

    let num_vertices = vertices.len() / 3;
    let num_indices = indices.len();
    let mut tan1 = vec![0.0f64; num_vertices * 3];

    let mut i = 0usize;
    while i < num_indices {
        let i0 = index_at(&indices, i) as usize;
        let i1 = index_at(&indices, i + 1) as usize;
        let i2 = index_at(&indices, i + 2) as usize;
        let i03 = i0 * 3;
        let i13 = i1 * 3;
        let i23 = i2 * 3;
        let i02 = i0 * 2;
        let i12 = i1 * 2;
        let i22 = i2 * 2;

        let ux = vertices[i03];
        let uy = vertices[i03 + 1];
        let uz = vertices[i03 + 2];

        let wx = st[i02];
        let wy = st[i02 + 1];
        let t1 = st[i12 + 1] - wy;
        let t2 = st[i22 + 1] - wy;

        let r = 1.0 / ((st[i12] - wx) * t2 - (st[i22] - wx) * t1);
        let sdirx = (t2 * (vertices[i13] - ux) - t1 * (vertices[i23] - ux)) * r;
        let sdiry = (t2 * (vertices[i13 + 1] - uy) - t1 * (vertices[i23 + 1] - uy)) * r;
        let sdirz = (t2 * (vertices[i13 + 2] - uz) - t1 * (vertices[i23 + 2] - uz)) * r;

        tan1[i03] += sdirx;
        tan1[i03 + 1] += sdiry;
        tan1[i03 + 2] += sdirz;

        tan1[i13] += sdirx;
        tan1[i13 + 1] += sdiry;
        tan1[i13 + 2] += sdirz;

        tan1[i23] += sdirx;
        tan1[i23 + 1] += sdiry;
        tan1[i23 + 2] += sdirz;

        i += 3;
    }

    let mut tangent_values = vec![0.0f64; num_vertices * 3];
    let mut bitangent_values = vec![0.0f64; num_vertices * 3];

    for vertex in 0..num_vertices {
        let i03 = vertex * 3;
        let i13 = i03 + 1;
        let i23 = i03 + 2;

        let n = Cartesian3::new(normals[i03], normals[i03 + 1], normals[i03 + 2]);
        let mut t = Cartesian3::new(tan1[i03], tan1[i03 + 1], tan1[i03 + 2]);
        let scalar = Cartesian3::dot(&n, &t);
        let normal_scale = Cartesian3::multiply_by_scalar_new(&n, scalar);
        let mut t_out = Cartesian3::ZERO;
        Cartesian3::subtract(&t, &normal_scale, &mut t_out);
        Cartesian3::normalize(&t_out, &mut t);

        tangent_values[i03] = t.x;
        tangent_values[i13] = t.y;
        tangent_values[i23] = t.z;

        Cartesian3::cross(&n, &t, &mut t_out);
        Cartesian3::normalize(&t_out, &mut t);

        bitangent_values[i03] = t.x;
        bitangent_values[i13] = t.y;
        bitangent_values[i23] = t.z;
    }

    geometry.attributes.insert(
        "tangent".to_string(),
        GeometryAttribute::new(ComponentDatatype::Float, 3, false, tangent_values),
    );
    geometry.attributes.insert(
        "bitangent".to_string(),
        GeometryAttribute::new(ComponentDatatype::Float, 3, false, bitangent_values),
    );
}

fn index_at(storage: &crate::index_datatype::IndexStorage, index: usize) -> u32 {
    match storage {
        crate::index_datatype::IndexStorage::U16(v) => v[index] as u32,
        crate::index_datatype::IndexStorage::U32(v) => v[index],
    }
}
