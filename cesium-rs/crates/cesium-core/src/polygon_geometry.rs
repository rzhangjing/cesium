//! Ported from `packages/engine/Source/Core/PolygonGeometry.js`.
//!
//! A description of a polygon on an ellipsoid.
//!
//! DEVIATION: JS `createGeometry` uses `EllipsoidTangentPlane` for 2D
//! projection and `GeometryPipeline.combineInstances` for merging. The
//! Rust port uses a cartographic (lon/lat) projection and manual merging.
//!
//! DEVIATION: JS `createProjectTo2d` has special handling for polygons
//! spanning large extents or crossing the equator. The Rust port uses a
//! simple cartographic projection for all cases.

use std::collections::HashMap;

use crate::arc_type::ArcType;
use crate::bounding_sphere::BoundingSphere;
use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::component_datatype::ComponentDatatype;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_offset_attribute::GeometryOffsetAttribute;
use crate::geometry_type::GeometryType;
use crate::index_datatype::{IndexDatatype, IndexStorage};
use crate::math::CesiumMath;
use crate::polygon_geometry_library::{
    PolygonGeometryLibrary, PolygonResultEntry,
};
use crate::polygon_hierarchy::PolygonHierarchy;
use crate::polygon_pipeline::PolygonPipeline;
use crate::primitive_type::PrimitiveType;
use crate::vertex_format::VertexFormat;

/// A description of a polygon on an ellipsoid. Polygon geometry can be
/// rendered with both `Primitive` and `GroundPrimitive`.
#[derive(Debug, Clone)]
pub struct PolygonGeometry {
    polygon_hierarchy: PolygonHierarchy,
    ellipsoid: Ellipsoid,
    vertex_format: VertexFormat,
    height: f64,
    extruded_height: f64,
    granularity: f64,
    st_rotation: f64,
    per_position_height: bool,
    close_top: bool,
    close_bottom: bool,
    offset_attribute: Option<GeometryOffsetAttribute>,
    arc_type: ArcType,
}

impl PolygonGeometry {
    /// Creates a new `PolygonGeometry`.
    pub fn new(
        positions: Vec<Cartesian3>,
        ellipsoid: Option<Ellipsoid>,
        vertex_format: Option<VertexFormat>,
        height: Option<f64>,
        extruded_height: Option<f64>,
        granularity: Option<f64>,
        st_rotation: Option<f64>,
        per_position_height: Option<bool>,
        close_top: Option<bool>,
        close_bottom: Option<bool>,
        offset_attribute: Option<GeometryOffsetAttribute>,
        arc_type: Option<ArcType>,
    ) -> Self {
        let height = height.unwrap_or(0.0);
        let extruded_height = extruded_height.unwrap_or(height);
        Self {
            polygon_hierarchy: PolygonHierarchy::new(positions, Vec::new()),
            ellipsoid: ellipsoid.unwrap_or(Ellipsoid::WGS84),
            vertex_format: vertex_format.unwrap_or_default(),
            height: height.max(extruded_height),
            extruded_height: height.min(extruded_height),
            granularity: granularity.unwrap_or(CesiumMath::RADIANS_PER_DEGREE),
            st_rotation: st_rotation.unwrap_or(0.0),
            per_position_height: per_position_height.unwrap_or(false),
            close_top: close_top.unwrap_or(true),
            close_bottom: close_bottom.unwrap_or(true),
            offset_attribute,
            arc_type: arc_type.unwrap_or(ArcType::Geodesic),
        }
    }
}

fn write_index(storage: &mut IndexStorage, index: usize, value: u32) {
    match storage {
        IndexStorage::U16(v) => v[index] = value as u16,
        IndexStorage::U32(v) => v[index] = value,
    }
}

fn read_index(storage: &IndexStorage, index: usize) -> u32 {
    match storage {
        IndexStorage::U16(v) => v[index] as u32,
        IndexStorage::U32(v) => v[index],
    }
}

/// Computes the geometric representation of a polygon, including vertices,
/// indices, and a bounding sphere.
///
/// Port of `PolygonGeometry.createGeometry`.
pub fn create_geometry(polygon_geometry: &PolygonGeometry) -> Option<Geometry> {
    let ellipsoid = &polygon_geometry.ellipsoid;
    let polygon_hierarchy = &polygon_geometry.polygon_hierarchy;
    let per_position_height = polygon_geometry.per_position_height;
    let vertex_format = &polygon_geometry.vertex_format;

    let outer_positions = &polygon_hierarchy.positions;
    if outer_positions.len() < 3 {
        return None;
    }

    // Project positions to 2D using cartographic (lon/lat) projection
    let project_fn = |positions: &[Cartesian3]| -> Option<Vec<Cartesian2>> {
        let mut result = Vec::with_capacity(positions.len());
        let mut carto = Cartographic::default();
        for p in positions {
            ellipsoid.cartesian_to_cartographic(p, &mut carto);
            result.push(Cartesian2::new(carto.longitude, carto.latitude));
        }
        Some(result)
    };

    let results = PolygonGeometryLibrary::polygons_from_hierarchy(
        polygon_hierarchy,
        false,
        &project_fn,
        !per_position_height,
        ellipsoid,
        None,
    );

    if results.hierarchy.is_empty() {
        return None;
    }

    let height = polygon_geometry.height;
    let extruded_height = polygon_geometry.extruded_height;
    let extrude = !CesiumMath::equals_epsilon(
        height,
        extruded_height,
        Some(0.0),
        Some(CesiumMath::EPSILON2),
    );

    let polygons = &results.polygons;
    let mut geometries: Vec<Geometry> = Vec::new();

    for polygon in polygons {
        let geo = PolygonGeometryLibrary::create_geometry_from_positions(
            ellipsoid,
            polygon,
            None,
            polygon_geometry.granularity,
            per_position_height,
            vertex_format,
            polygon_geometry.arc_type,
        );
        geometries.push(geo);
    }

    if geometries.is_empty() {
        return None;
    }

    // Scale to height (non-extruded case)
    if !extrude {
        for geo in &mut geometries {
            if let Some(pos_attr) = geo.attributes.get_mut("position") {
                let mut vals = pos_attr.values.clone();
                PolygonPipeline::scale_to_geodetic_height(
                    Some(&mut vals),
                    Some(height),
                    Some(ellipsoid),
                    Some(true),
                );
                pos_attr.values = vals;
            }
        }

        // Add offset attribute if needed
        if let Some(offset_attr) = polygon_geometry.offset_attribute {
            for geo in &mut geometries {
                let length = geo.attributes.get("position").map(|a| a.values.len()).unwrap_or(0);
                let offset_value = if offset_attr == GeometryOffsetAttribute::None { 0 } else { 1 };
                let apply_offset = vec![offset_value as f64; length / 3];
                geo.attributes.insert(
                    "applyOffset".to_string(),
                    GeometryAttribute::new(
                        ComponentDatatype::UnsignedByte,
                        1,
                        false,
                        apply_offset,
                    ),
                );
            }
        }
    } else {
        // Extruded case: scale to geodetic height extruded
        for geo in &mut geometries {
            PolygonGeometryLibrary::scale_to_geodetic_height_extruded(
                Some(geo),
                height,
                extruded_height,
                Some(ellipsoid.clone()),
                per_position_height,
            );

            if let Some(offset_attr) = polygon_geometry.offset_attribute {
                let length = geo.attributes.get("position").map(|a| a.values.len()).unwrap_or(0);
                let vertex_count = length / 3;
                let apply_offset: Vec<f64> = if offset_attr == GeometryOffsetAttribute::Top {
                    let mut v = vec![0.0f64; vertex_count];
                    for i in 0..vertex_count / 2 {
                        v[i] = 1.0;
                    }
                    v
                } else {
                    let ov = if offset_attr == GeometryOffsetAttribute::None { 0 } else { 1 };
                    vec![ov as f64; vertex_count]
                };
                geo.attributes.insert(
                    "applyOffset".to_string(),
                    GeometryAttribute::new(
                        ComponentDatatype::UnsignedByte,
                        1,
                        false,
                        apply_offset,
                    ),
                );
            }
        }
    }

    // Merge geometries if multiple
    let final_geometry = if geometries.len() == 1 {
        geometries.into_iter().next().unwrap()
    } else {
        merge_geometries(geometries)
    };

    // Remove position if vertex_format doesn't request it
    if !vertex_format.position {
        let mut geo = final_geometry;
        geo.attributes.remove("position");
        // Re-add a dummy position for Geometry validity
        let pos = geo.attributes.values().next();
        if pos.is_none() {
            return None;
        }
        Some(Geometry::with_all(
            geo.attributes,
            geo.indices,
            Some(geo.primitive_type),
            geo.bounding_sphere,
            GeometryType::None,
            None,
            polygon_geometry.offset_attribute.map(|_| "applyOffset".to_string()),
        ))
    } else {
        let mut geo = final_geometry;
        // Update bounding sphere from position
        let pos_values = geo.attributes.get("position").map(|a| a.values.clone()).unwrap_or_default();
        let bounding_sphere = BoundingSphere::from_vertices(&pos_values, None, Some(3), None);
        geo.bounding_sphere = Some(bounding_sphere);
        geo.offset_attribute = polygon_geometry.offset_attribute.map(|_| "applyOffset".to_string());
        Some(geo)
    }
}

/// Merge multiple geometries into one, combining attributes and indices.
fn merge_geometries(geometries: Vec<Geometry>) -> Geometry {
    let mut merged_attrs: HashMap<String, GeometryAttribute> = HashMap::new();
    let mut merged_indices_vec: Vec<u32> = Vec::new();
    let mut vertex_offset = 0u32;

    let attr_keys: Vec<String> = geometries
        .first()
        .map(|g| g.attributes.keys().cloned().collect())
        .unwrap_or_default();

    for key in &attr_keys {
        let mut merged_values = Vec::new();
        for geo in &geometries {
            if let Some(attr) = geo.attributes.get(key) {
                merged_values.extend_from_slice(&attr.values);
            }
        }
        if !merged_values.is_empty() {
            let (dt, comp) = geometries
                .first()
                .and_then(|g| g.attributes.get(key))
                .map(|a| (a.component_datatype, a.components_per_attribute))
                .unwrap_or((ComponentDatatype::Double, 3));
            merged_attrs.insert(
                key.clone(),
                GeometryAttribute::new(dt, comp, false, merged_values),
            );
        }
    }

    for geo in &geometries {
        let pos_len = geo
            .attributes
            .get("position")
            .map(|a| a.values.len() / 3)
            .unwrap_or(0);
        if let Some(indices) = &geo.indices {
            for i in 0..indices.len() {
                let v = read_index(indices, i);
                merged_indices_vec.push(v + vertex_offset);
            }
        }
        vertex_offset += pos_len as u32;
    }

    let total_vertices = vertex_offset as usize;
    let mut merged_indices =
        IndexDatatype::create_typed_array(total_vertices, merged_indices_vec.len());
    for (i, &v) in merged_indices_vec.iter().enumerate() {
        write_index(&mut merged_indices, i, v);
    }

    let pos_values = merged_attrs
        .get("position")
        .map(|a| a.values.clone())
        .unwrap_or_default();
    let bounding_sphere = BoundingSphere::from_vertices(&pos_values, None, Some(3), None);

    Geometry::with_all(
        merged_attrs,
        Some(merged_indices),
        Some(PrimitiveType::Triangles),
        Some(bounding_sphere),
        GeometryType::None,
        None,
        None,
    )
}
