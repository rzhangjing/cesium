//! Ported from `packages/engine/Source/Core/CoplanarPolygonOutlineGeometry.js`.
//!
//! A description of the outline of a polygon composed of arbitrary coplanar
//! positions.
//!
//! DEVIATION: JS merges per-ring geometries via `GeometryInstance` +
//! `GeometryPipeline.combineInstances`; the Rust port concatenates them
//! directly (same attribute/index result).

use std::collections::HashMap;

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::component_datatype::ComponentDatatype;
use crate::coplanar_polygon_geometry_library::CoplanarPolygonGeometryLibrary;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::index_datatype::{IndexDatatype, IndexStorage};
use crate::polygon_geometry_library::PolygonGeometryLibrary;
use crate::polygon_hierarchy::PolygonHierarchy;
use crate::primitive_type::PrimitiveType;

/// A description of the outline of a polygon composed of arbitrary coplanar
/// positions.
#[derive(Debug, Clone, Default)]
pub struct CoplanarPolygonOutlineGeometry {
    polygon_hierarchy: PolygonHierarchy,
}

impl CoplanarPolygonOutlineGeometry {
    /// Creates a new `CoplanarPolygonOutlineGeometry` from a polygon
    /// hierarchy (JS constructor).
    pub fn from_hierarchy(polygon_hierarchy: PolygonHierarchy) -> Self {
        Self { polygon_hierarchy }
    }

    /// Creates a new `CoplanarPolygonOutlineGeometry` from a flat ring of
    /// positions (kept for backwards compatibility; equivalent to JS
    /// `CoplanarPolygonOutlineGeometry.fromPositions`).
    pub fn new(positions: Vec<Cartesian3>) -> Self {
        Self::from_positions(positions)
    }

    /// A description of a coplanar polygon outline from an array of positions
    /// (JS `CoplanarPolygonOutlineGeometry.fromPositions`).
    pub fn from_positions(positions: Vec<Cartesian3>) -> Self {
        Self::from_hierarchy(PolygonHierarchy::new(positions, Vec::new()))
    }

    /// The polygon hierarchy (JS `_polygonHierarchy`).
    pub fn polygon_hierarchy(&self) -> &PolygonHierarchy {
        &self.polygon_hierarchy
    }

    /// The number of elements used to pack the object into an array (JS
    /// instance property `packedLength`).
    pub fn packed_length(&self) -> usize {
        PolygonGeometryLibrary::compute_hierarchy_packed_length(&self.polygon_hierarchy) + 1
    }

    /// Stores this instance into `array` (JS
    /// `CoplanarPolygonOutlineGeometry.pack`).
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        let si = starting_index.unwrap_or(0);
        let packed = crate::coplanar_polygon_geometry::pack_hierarchy_3d_pub(
            &self.polygon_hierarchy,
            array,
            si,
        );
        array[packed] = self.packed_length() as f64;
    }

    /// Retrieves an instance from a packed array (JS
    /// `CoplanarPolygonOutlineGeometry.unpack`).
    pub fn unpack(array: &[f64], starting_index: Option<usize>, result: Option<&mut Self>) -> Self {
        let si = starting_index.unwrap_or(0);
        let (polygon_hierarchy, next) =
            crate::coplanar_polygon_geometry::unpack_hierarchy_3d_pub(array, si);
        let _packed_length = array[next];

        match result {
            None => Self::from_hierarchy(polygon_hierarchy),
            Some(r) => {
                r.polygon_hierarchy = polygon_hierarchy;
                r.clone()
            }
        }
    }

    /// Computes the geometric representation of an arbitrary coplanar polygon
    /// outline, including its vertices, indices, and a bounding sphere (JS
    /// `CoplanarPolygonOutlineGeometry.createGeometry`).
    pub fn create_geometry(&self) -> Option<Geometry> {
        let polygon_hierarchy = &self.polygon_hierarchy;

        let outer_positions = crate::array_remove_duplicates::array_remove_duplicates(
            &polygon_hierarchy.positions,
            |a: &Cartesian3, b: &Cartesian3, eps: f64| {
                Cartesian3::equals_epsilon(Some(a), Some(b), Some(eps), Some(eps))
            },
            true,
            None,
        )
        .unwrap_or_else(|| polygon_hierarchy.positions.clone());
        if outer_positions.len() < 3 {
            return None;
        }
        let is_valid = CoplanarPolygonGeometryLibrary::valid_outline(&outer_positions);
        if !is_valid {
            return None;
        }

        // JS passes `false` for scaleToEllipsoidSurface and leaves the
        // ellipsoid undefined (unused when not scaling).
        let polygons = PolygonGeometryLibrary::polygon_outlines_from_hierarchy(
            polygon_hierarchy,
            false,
            &Ellipsoid::WGS84,
        );

        if polygons.is_empty() {
            return None;
        }

        let geometries: Vec<Geometry> = polygons
            .iter()
            .map(|positions| create_geometry_from_positions(positions))
            .collect();

        // JS merges via GeometryPipeline.combineInstances(instances)[0]; the
        // geometries share attribute layout, so direct concatenation yields
        // the same result.
        let geometry = merge_geometries(geometries);
        let bounding_sphere = BoundingSphere::from_points(&polygon_hierarchy.positions, None);

        Some(Geometry::with_all(
            geometry.attributes,
            geometry.indices,
            Some(geometry.primitive_type),
            Some(bounding_sphere),
            crate::geometry_type::GeometryType::None,
            None,
            None,
        ))
    }
}

/// Mirrors the private JS `createGeometryFromPositions` helper.
fn create_geometry_from_positions(positions: &[Cartesian3]) -> Geometry {
    let length = positions.len();
    let mut flat_positions = vec![0.0f64; length * 3];
    let mut indices = IndexDatatype::create_typed_array(length, length * 2);

    let mut position_index = 0usize;
    let mut index = 0usize;

    for (i, position) in positions.iter().enumerate() {
        flat_positions[position_index] = position.x;
        position_index += 1;
        flat_positions[position_index] = position.y;
        position_index += 1;
        flat_positions[position_index] = position.z;
        position_index += 1;

        match &mut indices {
            IndexStorage::U16(v) => {
                v[index] = i as u16;
                index += 1;
                v[index] = ((i + 1) % length) as u16;
                index += 1;
            }
            IndexStorage::U32(v) => {
                v[index] = i as u32;
                index += 1;
                v[index] = ((i + 1) % length) as u32;
                index += 1;
            }
        }
    }

    let mut attributes: HashMap<String, GeometryAttribute> = HashMap::new();
    attributes.insert(
        "position".to_string(),
        GeometryAttribute::new(ComponentDatatype::Double, 3, false, flat_positions),
    );

    Geometry::with_all(
        attributes,
        Some(indices),
        Some(PrimitiveType::Lines),
        None,
        crate::geometry_type::GeometryType::None,
        None,
        None,
    )
}

/// Merges per-ring geometries into one (mirrors the attribute/index
/// concatenation of `GeometryPipeline.combineInstances`).
fn merge_geometries(geometries: Vec<Geometry>) -> Geometry {
    if geometries.len() == 1 {
        return geometries.into_iter().next().unwrap();
    }

    let mut merged_positions: Vec<f64> = Vec::new();
    let mut merged_indices_vec: Vec<u32> = Vec::new();
    let mut vertex_offset = 0u32;

    for geo in &geometries {
        let pos_len = geo
            .attributes
            .get("position")
            .map(|a| a.values.len() / 3)
            .unwrap_or(0);
        if let Some(attr) = geo.attributes.get("position") {
            merged_positions.extend_from_slice(&attr.values);
        }
        if let Some(indices) = &geo.indices {
            for i in 0..indices.len() {
                let v = match indices {
                    IndexStorage::U16(v) => v[i] as u32,
                    IndexStorage::U32(v) => v[i],
                };
                merged_indices_vec.push(v + vertex_offset);
            }
        }
        vertex_offset += pos_len as u32;
    }

    let mut attributes: HashMap<String, GeometryAttribute> = HashMap::new();
    attributes.insert(
        "position".to_string(),
        GeometryAttribute::new(ComponentDatatype::Double, 3, false, merged_positions),
    );

    let total_vertices = vertex_offset as usize;
    let mut merged_indices =
        IndexDatatype::create_typed_array(total_vertices, merged_indices_vec.len());
    for (i, &v) in merged_indices_vec.iter().enumerate() {
        match &mut merged_indices {
            IndexStorage::U16(v) => v[i] = merged_indices_vec[i] as u16,
            IndexStorage::U32(v) => v[i] = merged_indices_vec[i],
        }
    }

    Geometry::with_all(
        attributes,
        Some(merged_indices),
        Some(PrimitiveType::Lines),
        None,
        crate::geometry_type::GeometryType::None,
        None,
        None,
    )
}
