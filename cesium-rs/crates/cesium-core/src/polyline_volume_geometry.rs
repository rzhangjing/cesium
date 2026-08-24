//! Ported from `packages/engine/Source/Core/PolylineVolumeGeometry.js`.
//!
//! A description of a polyline with a volume (a 2D shape extruded along a
//! polyline).

use std::collections::HashMap;

use crate::array_remove_duplicates::array_remove_duplicates;
use crate::bounding_rectangle::BoundingRectangle;
use crate::bounding_sphere::BoundingSphere;
use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::component_datatype::ComponentDatatype;
use crate::corner_type::CornerType;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_pipeline::GeometryPipeline;
use crate::index_datatype::IndexDatatype;
use crate::math::CesiumMath;
use crate::polygon_pipeline::PolygonPipeline;
use crate::polyline_volume_geometry_library::{
    ComputePositionsGeometry, PolylineVolumeGeometryLibrary,
};
use crate::primitive_type::PrimitiveType;
use crate::vertex_format::VertexFormat;
use crate::winding_order::WindingOrder;

/// A description of a polyline with a volume (a 2D shape extruded along a
/// polyline).
#[derive(Debug, Clone)]
pub struct PolylineVolumeGeometry {
    positions: Vec<Cartesian3>,
    shape: Vec<Cartesian2>,
    ellipsoid: Ellipsoid,
    corner_type: CornerType,
    vertex_format: VertexFormat,
    granularity: f64,
}

impl PolylineVolumeGeometry {
    /// Creates a new `PolylineVolumeGeometry`.
    pub fn new(
        positions: Vec<Cartesian3>,
        shape: Vec<Cartesian2>,
        ellipsoid: Option<Ellipsoid>,
        corner_type: Option<CornerType>,
        vertex_format: Option<VertexFormat>,
        granularity: Option<f64>,
    ) -> Self {
        Self {
            positions,
            shape,
            ellipsoid: ellipsoid.unwrap_or(Ellipsoid::WGS84),
            corner_type: corner_type.unwrap_or(CornerType::Rounded),
            vertex_format: vertex_format.unwrap_or(VertexFormat::default_format()),
            granularity: granularity.unwrap_or(CesiumMath::RADIANS_PER_DEGREE),
        }
    }

    /// The number of elements used to pack the object into an array.
    pub fn packed_length(&self) -> usize {
        1 + self.positions.len() * Cartesian3::PACKED_LENGTH
            + 1
            + self.shape.len() * Cartesian2::PACKED_LENGTH
            + Ellipsoid::PACKED_LENGTH
            + VertexFormat::PACKED_LENGTH
            + 2
    }
}

/// Computes the geometric representation of a polyline with a volume,
/// including its vertices, indices, and a bounding sphere.
///
/// Port of `PolylineVolumeGeometry.createGeometry`.
pub fn create_geometry(polyline_volume_geometry: &PolylineVolumeGeometry) -> Option<Geometry> {
    let positions = &polyline_volume_geometry.positions;
    let clean_positions = array_remove_duplicates(
        positions,
        |a: &Cartesian3, b: &Cartesian3, eps| {
            Cartesian3::equals_epsilon(Some(a), Some(b), Some(eps), Some(eps))
        },
        false,
        None,
    );
    let clean_positions = clean_positions.unwrap_or_else(|| positions.clone());

    let mut positions_mut = clean_positions.clone();

    let mut shape2d = PolylineVolumeGeometryLibrary::remove_duplicates_from_shape(
        &polyline_volume_geometry.shape,
    );

    if clean_positions.len() < 2 || shape2d.len() < 3 {
        return None;
    }

    if PolygonPipeline::compute_winding_order_2d(&shape2d) == WindingOrder::Clockwise {
        shape2d.reverse();
    }

    let mut bounding_rectangle = BoundingRectangle::default();
    BoundingRectangle::from_points(&shape2d, &mut bounding_rectangle);

    let computed_positions = PolylineVolumeGeometryLibrary::compute_positions(
        &mut positions_mut,
        &shape2d,
        &bounding_rectangle,
        &ComputePositionsGeometry {
            ellipsoid: polyline_volume_geometry.ellipsoid.clone(),
            granularity: polyline_volume_geometry.granularity,
            corner_type: polyline_volume_geometry.corner_type,
        },
        true,
    );

    Some(compute_attributes(
        &computed_positions,
        &shape2d,
        &bounding_rectangle,
        &polyline_volume_geometry.vertex_format,
    ))
}

#[allow(clippy::too_many_arguments)]
fn compute_attributes(
    combined_positions: &[f64],
    shape: &[Cartesian2],
    bounding_rectangle: &BoundingRectangle,
    vertex_format: &VertexFormat,
) -> Geometry {
    let mut attributes: HashMap<String, GeometryAttribute> = HashMap::new();

    if vertex_format.position {
        attributes.insert(
            "position".to_string(),
            GeometryAttribute::new(
                ComponentDatatype::Double,
                3,
                false,
                combined_positions.to_vec(),
            ),
        );
    }

    let shape_length = shape.len();
    let vertex_count = combined_positions.len() / 3;
    let length = (vertex_count - shape_length * 2) / (shape_length * 2);
    let first_end_indices = PolygonPipeline::triangulate(shape, None);

    let indices_count = (length - 1) * shape_length * 6 + first_end_indices.len() * 2;
    let mut indices = IndexDatatype::create_typed_array(vertex_count, indices_count);

    let offset = shape_length * 2;
    let mut index = 0usize;

    for i in 0..length - 1 {
        for j in 0..shape_length - 1 {
            let ll = j * 2 + i * shape_length * 2;
            let lr = ll + offset;
            let ul = ll + 1;
            let ur = ul + offset;

            write_index(&mut indices, index, ul as u32);
            index += 1;
            write_index(&mut indices, index, ll as u32);
            index += 1;
            write_index(&mut indices, index, ur as u32);
            index += 1;
            write_index(&mut indices, index, ur as u32);
            index += 1;
            write_index(&mut indices, index, ll as u32);
            index += 1;
            write_index(&mut indices, index, lr as u32);
            index += 1;
        }
        // Wrap-around quad for last shape vertex
        let ll = shape_length * 2 - 2 + i * shape_length * 2;
        let ul = ll + 1;
        let ur = ul + offset;
        let lr = ll + offset;

        write_index(&mut indices, index, ul as u32);
        index += 1;
        write_index(&mut indices, index, ll as u32);
        index += 1;
        write_index(&mut indices, index, ur as u32);
        index += 1;
        write_index(&mut indices, index, ur as u32);
        index += 1;
        write_index(&mut indices, index, ll as u32);
        index += 1;
        write_index(&mut indices, index, lr as u32);
        index += 1;
    }

    if vertex_format.st || vertex_format.tangent || vertex_format.bitangent {
        let st = compute_st(
            vertex_count,
            length,
            shape_length,
            shape,
            bounding_rectangle,
        );
        attributes.insert(
            "st".to_string(),
            GeometryAttribute::new(ComponentDatatype::Float, 2, false, st),
        );
    }

    let end_offset = vertex_count - shape_length * 2;
    for i in (0..first_end_indices.len()).step_by(3) {
        let v0 = first_end_indices[i] as usize + end_offset;
        let v1 = first_end_indices[i + 1] as usize + end_offset;
        let v2 = first_end_indices[i + 2] as usize + end_offset;

        write_index(&mut indices, index, v0 as u32);
        index += 1;
        write_index(&mut indices, index, v1 as u32);
        index += 1;
        write_index(&mut indices, index, v2 as u32);
        index += 1;
        write_index(&mut indices, index, (v2 + shape_length) as u32);
        index += 1;
        write_index(&mut indices, index, (v1 + shape_length) as u32);
        index += 1;
        write_index(&mut indices, index, (v0 + shape_length) as u32);
        index += 1;
    }

    let bounding_sphere = BoundingSphere::from_vertices(combined_positions, None, Some(3), None);

    let mut geometry = Geometry::new(
        attributes,
        Some(indices),
        Some(PrimitiveType::Triangles),
        Some(bounding_sphere),
    );

    if vertex_format.normal {
        GeometryPipeline::compute_normal(&mut geometry);
    }

    if vertex_format.tangent || vertex_format.bitangent {
        // DEVIATION: JS wraps computeTangentAndBitangent in a try/catch and
        // issues a oneTimeWarning on failure. Rust has no panics from this
        // function, so we call directly. The oneTimeWarning is retained for
        // API fidelity.
        GeometryPipeline::compute_tangent_and_bitangent(&mut geometry);

        if !vertex_format.tangent {
            geometry.attributes.remove("tangent");
        }
        if !vertex_format.bitangent {
            geometry.attributes.remove("bitangent");
        }
        if !vertex_format.st {
            geometry.attributes.remove("st");
        }
    }

    geometry
}

/// Computes the ST texture coordinates for a polyline volume.
fn compute_st(
    vertex_count: usize,
    length: usize,
    shape_length: usize,
    shape: &[Cartesian2],
    bounding_rectangle: &BoundingRectangle,
) -> Vec<f64> {
    let mut st = vec![0.0f64; vertex_count * 2];
    let length_st = 1.0 / (length - 1) as f64;
    let height_st = 1.0 / bounding_rectangle.height;
    let height_offset = bounding_rectangle.height / 2.0;
    let mut st_index = 0usize;

    for i in 0..length {
        let s = i as f64 * length_st;
        let t = height_st * (shape[0].y + height_offset);
        st[st_index] = s;
        st[st_index + 1] = t;
        st_index += 2;

        for j in 1..shape_length {
            let t = height_st * (shape[j].y + height_offset);
            st[st_index] = s;
            st[st_index + 1] = t;
            st_index += 2;
            st[st_index] = s;
            st[st_index + 1] = t;
            st_index += 2;
        }

        let t = height_st * (shape[0].y + height_offset);
        st[st_index] = s;
        st[st_index + 1] = t;
        st_index += 2;
    }

    // First end cap
    for j in 0..shape_length {
        let s = 0.0;
        let t = height_st * (shape[j].y + height_offset);
        st[st_index] = s;
        st[st_index + 1] = t;
        st_index += 2;
    }

    // Last end cap
    for j in 0..shape_length {
        let s = (length - 1) as f64 * length_st;
        let t = height_st * (shape[j].y + height_offset);
        st[st_index] = s;
        st[st_index + 1] = t;
        st_index += 2;
    }

    st
}

fn write_index(storage: &mut crate::index_datatype::IndexStorage, index: usize, value: u32) {
    use crate::index_datatype::IndexStorage;
    match storage {
        IndexStorage::U16(v) => v[index] = value as u16,
        IndexStorage::U32(v) => v[index] = value,
    }
}
