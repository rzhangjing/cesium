//! Ported from `packages/engine/Source/Core/PolylineVolumeOutlineGeometry.js`.
//!
//! A description of the outline of a polyline with a volume (a 2D shape
//! extruded along a polyline).

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
use crate::index_datatype::IndexDatatype;
use crate::math::CesiumMath;
use crate::polygon_pipeline::PolygonPipeline;
use crate::polyline_volume_geometry_library::{
    ComputePositionsGeometry, PolylineVolumeGeometryLibrary,
};
use crate::primitive_type::PrimitiveType;
use crate::winding_order::WindingOrder;

/// A description of the outline of a polyline with a volume.
#[derive(Debug, Clone)]
pub struct PolylineVolumeOutlineGeometry {
    positions: Vec<Cartesian3>,
    shape: Vec<Cartesian2>,
    ellipsoid: Ellipsoid,
    corner_type: CornerType,
    granularity: f64,
}

impl PolylineVolumeOutlineGeometry {
    /// Creates a new `PolylineVolumeOutlineGeometry`.
    pub fn new(
        positions: Vec<Cartesian3>,
        shape: Vec<Cartesian2>,
        ellipsoid: Option<Ellipsoid>,
        corner_type: Option<CornerType>,
        granularity: Option<f64>,
    ) -> Self {
        Self {
            positions,
            shape,
            ellipsoid: ellipsoid.unwrap_or(Ellipsoid::WGS84),
            corner_type: corner_type.unwrap_or(CornerType::Rounded),
            granularity: granularity.unwrap_or(CesiumMath::RADIANS_PER_DEGREE),
        }
    }

    /// The number of elements used to pack the object into an array.
    pub fn packed_length(&self) -> usize {
        1 + self.positions.len() * Cartesian3::PACKED_LENGTH
            + 1
            + self.shape.len() * Cartesian2::PACKED_LENGTH
            + Ellipsoid::PACKED_LENGTH
            + 2
    }
}

/// Computes the geometric representation of the outline of a polyline with a
/// volume, including its vertices, indices, and a bounding sphere.
///
/// Port of `PolylineVolumeOutlineGeometry.createGeometry`.
pub fn create_geometry(
    polyline_volume_outline_geometry: &PolylineVolumeOutlineGeometry,
) -> Option<Geometry> {
    let positions = &polyline_volume_outline_geometry.positions;
    let clean_positions = array_remove_duplicates(
        positions,
        |a: &Cartesian3, b: &Cartesian3, eps| {
            Cartesian3::equals_epsilon(Some(a), Some(b), Some(eps), Some(eps))
        },
        false,
        None,
    );
    let clean_positions = clean_positions.unwrap_or_else(|| positions.clone());

    let mut shape2d = PolylineVolumeGeometryLibrary::remove_duplicates_from_shape(
        &polyline_volume_outline_geometry.shape,
    );

    if clean_positions.len() < 2 || shape2d.len() < 3 {
        return None;
    }

    if PolygonPipeline::compute_winding_order_2d(&shape2d) == WindingOrder::Clockwise {
        shape2d.reverse();
    }

    let mut bounding_rectangle = BoundingRectangle::default();
    BoundingRectangle::from_points(&shape2d, &mut bounding_rectangle);

    let mut positions_mut = clean_positions.clone();
    let computed_positions = PolylineVolumeGeometryLibrary::compute_positions(
        &mut positions_mut,
        &shape2d,
        &bounding_rectangle,
        &ComputePositionsGeometry {
            ellipsoid: polyline_volume_outline_geometry.ellipsoid.clone(),
            granularity: polyline_volume_outline_geometry.granularity,
            corner_type: polyline_volume_outline_geometry.corner_type,
        },
        false,
    );

    Some(compute_attributes(&computed_positions, &shape2d))
}

fn compute_attributes(positions: &[f64], shape: &[Cartesian2]) -> Geometry {
    let mut attributes: HashMap<String, GeometryAttribute> = HashMap::new();
    attributes.insert(
        "position".to_string(),
        GeometryAttribute::new(ComponentDatatype::Double, 3, false, positions.to_vec()),
    );

    let shape_length = shape.len();
    let vertex_count = positions.len() / 3;
    let position_length = positions.len() / 3;
    let shape_count = position_length / shape_length;

    let mut indices = IndexDatatype::create_typed_array(
        vertex_count,
        2 * shape_length * (shape_count + 1),
    );

    let mut index = 0usize;

    // First end cap outline
    let i = 0;
    let offset = i * shape_length;
    for j in 0..shape_length - 1 {
        write_index(&mut indices, index, (j + offset) as u32);
        index += 1;
        write_index(&mut indices, index, (j + offset + 1) as u32);
        index += 1;
    }
    write_index(&mut indices, index, (shape_length - 1 + offset) as u32);
    index += 1;
    write_index(&mut indices, index, offset as u32);
    index += 1;

    // Last end cap outline
    let i = shape_count - 1;
    let offset = i * shape_length;
    for j in 0..shape_length - 1 {
        write_index(&mut indices, index, (j + offset) as u32);
        index += 1;
        write_index(&mut indices, index, (j + offset + 1) as u32);
        index += 1;
    }
    write_index(&mut indices, index, (shape_length - 1 + offset) as u32);
    index += 1;
    write_index(&mut indices, index, offset as u32);
    index += 1;

    // Side walls
    for i in 0..shape_count - 1 {
        let first_offset = shape_length * i;
        let second_offset = first_offset + shape_length;
        for j in 0..shape_length {
            write_index(&mut indices, index, (j + first_offset) as u32);
            index += 1;
            write_index(&mut indices, index, (j + second_offset) as u32);
            index += 1;
        }
    }

    let bounding_sphere = BoundingSphere::from_vertices(positions, None, Some(3), None);

    Geometry::new(
        attributes,
        Some(indices),
        Some(PrimitiveType::Lines),
        Some(bounding_sphere),
    )
}

fn write_index(storage: &mut crate::index_datatype::IndexStorage, index: usize, value: u32) {
    use crate::index_datatype::IndexStorage;
    match storage {
        IndexStorage::U16(v) => v[index] = value as u16,
        IndexStorage::U32(v) => v[index] = value,
    }
}
