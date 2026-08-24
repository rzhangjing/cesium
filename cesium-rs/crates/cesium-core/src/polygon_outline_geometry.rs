//! Ported from `packages/engine/Source/Core/PolygonOutlineGeometry.js`.
//!
//! A description of the outline of a polygon.
//!
//! DEVIATION: JS `createGeometryFromPositions` uses `EllipsoidTangentPlane`
//! for 2D projection and subdivide functions for geodesic/rhumb arcs.
//! The Rust port uses a simpler approach: positions are used as-is without
//! subdivision, and winding order is determined via `PolygonPipeline`.

use std::collections::HashMap;

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::component_datatype::ComponentDatatype;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_offset_attribute::GeometryOffsetAttribute;
use crate::geometry_type::GeometryType;
use crate::index_datatype::{IndexDatatype, IndexStorage};
use crate::math::CesiumMath;
use crate::polygon_geometry_library::PolygonGeometryLibrary;
use crate::polygon_hierarchy::PolygonHierarchy;
use crate::polygon_pipeline::PolygonPipeline;
use crate::primitive_type::PrimitiveType;

/// A description of the outline of a polygon.
#[derive(Debug, Clone)]
pub struct PolygonOutlineGeometry {
    polygon_hierarchy: PolygonHierarchy,
    ellipsoid: Ellipsoid,
    height: f64,
    extruded_height: f64,
    granularity: f64,
    per_position_height: bool,
    offset_attribute: Option<GeometryOffsetAttribute>,
    arc_type: u32, // ArcType: 0=GEODESIC, 1=RHUMB
}

impl PolygonOutlineGeometry {
    /// Creates a new `PolygonOutlineGeometry`.
    pub fn new(
        positions: Vec<Cartesian3>,
        ellipsoid: Option<Ellipsoid>,
        height: Option<f64>,
        extruded_height: Option<f64>,
        granularity: Option<f64>,
        per_position_height: Option<bool>,
        offset_attribute: Option<GeometryOffsetAttribute>,
        arc_type: Option<u32>,
    ) -> Self {
        let height = height.unwrap_or(0.0);
        let extruded_height = extruded_height.unwrap_or(height);
        Self {
            polygon_hierarchy: PolygonHierarchy::new(positions, Vec::new()),
            ellipsoid: ellipsoid.unwrap_or(Ellipsoid::WGS84),
            height: height.max(extruded_height),
            extruded_height: height.min(extruded_height),
            granularity: granularity.unwrap_or(CesiumMath::RADIANS_PER_DEGREE),
            per_position_height: per_position_height.unwrap_or(false),
            offset_attribute,
            arc_type: arc_type.unwrap_or(0),
        }
    }
}

fn write_index(storage: &mut IndexStorage, index: usize, value: u32) {
    match storage {
        IndexStorage::U16(v) => v[index] = value as u16,
        IndexStorage::U32(v) => v[index] = value,
    }
}

/// Computes the geometric representation of a polygon outline.
///
/// Port of `PolygonOutlineGeometry.createGeometry`.
pub fn create_geometry(
    polygon_outline_geometry: &PolygonOutlineGeometry,
) -> Option<Geometry> {
    let ellipsoid = &polygon_outline_geometry.ellipsoid;
    let polygon_hierarchy = &polygon_outline_geometry.polygon_hierarchy;
    let per_position_height = polygon_outline_geometry.per_position_height;

    let polygons = PolygonGeometryLibrary::polygon_outlines_from_hierarchy(
        polygon_hierarchy,
        !per_position_height,
        ellipsoid,
    );

    if polygons.is_empty() {
        return None;
    }

    let height = polygon_outline_geometry.height;
    let extruded_height = polygon_outline_geometry.extruded_height;
    let extrude = !CesiumMath::equals_epsilon(
        height,
        extruded_height,
        Some(0.0),
        Some(CesiumMath::EPSILON2),
    );

    let mut geometries: Vec<(HashMap<String, GeometryAttribute>, IndexStorage)> = Vec::new();

    for positions in &polygons {
        if positions.is_empty() || positions.len() < 3 {
            continue;
        }

        let length = positions.len();
        let mut flat_positions = Vec::with_capacity(length * 3);
        for p in positions {
            flat_positions.push(p.x);
            flat_positions.push(p.y);
            flat_positions.push(p.z);
        }

        if extrude {
            // Extruded: top + bottom + wall
            let mut top_positions = flat_positions.clone();
            let mut bottom_positions = flat_positions.clone();

            PolygonPipeline::scale_to_geodetic_height(
                Some(&mut top_positions),
                Some(height),
                Some(ellipsoid),
                Some(true),
            );
            PolygonPipeline::scale_to_geodetic_height(
                Some(&mut bottom_positions),
                Some(extruded_height),
                Some(ellipsoid),
                Some(true),
            );

            let vertex_count = length;
            let mut final_positions = vec![0.0f64; vertex_count * 3 * 2];
            final_positions[..vertex_count * 3].copy_from_slice(&top_positions);
            final_positions[vertex_count * 3..].copy_from_slice(&bottom_positions);

            // Indices: top outline + bottom outline + wall
            let indices_count = vertex_count * 6; // top lines + bottom lines + wall lines
            let mut indices = IndexDatatype::create_typed_array(
                vertex_count * 2,
                indices_count,
            );
            let mut idx = 0usize;
            // Top outline
            for i in 0..vertex_count {
                write_index(&mut indices, idx, i as u32);
                idx += 1;
                write_index(&mut indices, idx, ((i + 1) % vertex_count) as u32);
                idx += 1;
            }
            // Bottom outline
            for i in 0..vertex_count {
                write_index(&mut indices, idx, (i + vertex_count) as u32);
                idx += 1;
                write_index(&mut indices, idx, ((i + 1) % vertex_count + vertex_count) as u32);
                idx += 1;
            }
            // Wall lines
            for i in 0..vertex_count {
                write_index(&mut indices, idx, i as u32);
                idx += 1;
                write_index(&mut indices, idx, (i + vertex_count) as u32);
                idx += 1;
            }

            let mut attrs = HashMap::new();
            attrs.insert(
                "position".to_string(),
                GeometryAttribute::new(ComponentDatatype::Double, 3, false, final_positions),
            );

            if let Some(offset_attr) = polygon_outline_geometry.offset_attribute {
                let offset_value = if offset_attr == GeometryOffsetAttribute::Top {
                    let mut v = vec![0.0f64; vertex_count * 2];
                    for i in 0..vertex_count {
                        v[i] = 1.0;
                    }
                    v
                } else {
                    let ov = if offset_attr == GeometryOffsetAttribute::None { 0 } else { 1 };
                    vec![ov as f64; vertex_count * 2]
                };
                attrs.insert(
                    "applyOffset".to_string(),
                    GeometryAttribute::new(ComponentDatatype::UnsignedByte, 1, false, offset_value),
                );
            }

            geometries.push((attrs, indices));
        } else {
            // Non-extruded: just outline
            let vertex_count = length;

            // Scale to height
            if height != 0.0 {
                PolygonPipeline::scale_to_geodetic_height(
                    Some(&mut flat_positions),
                    Some(height),
                    Some(ellipsoid),
                    Some(true),
                );
            }

            let indices_count = vertex_count * 2;
            let mut indices = IndexDatatype::create_typed_array(vertex_count, indices_count);
            let mut idx = 0usize;
            for i in 0..vertex_count {
                write_index(&mut indices, idx, i as u32);
                idx += 1;
                write_index(&mut indices, idx, ((i + 1) % vertex_count) as u32);
                idx += 1;
            }

            let mut attrs = HashMap::new();
            attrs.insert(
                "position".to_string(),
                GeometryAttribute::new(ComponentDatatype::Double, 3, false, flat_positions),
            );

            if let Some(offset_attr) = polygon_outline_geometry.offset_attribute {
                let offset_value = if offset_attr == GeometryOffsetAttribute::None { 0 } else { 1 };
                let apply_offset = vec![offset_value as f64; vertex_count];
                attrs.insert(
                    "applyOffset".to_string(),
                    GeometryAttribute::new(ComponentDatatype::UnsignedByte, 1, false, apply_offset),
                );
            }

            geometries.push((attrs, indices));
        }
    }

    if geometries.is_empty() {
        return None;
    }

    // Merge all geometries
    let mut all_positions = Vec::new();
    let mut all_indices_vec: Vec<u32> = Vec::new();
    let mut vertex_offset = 0u32;
    let mut all_apply_offset = Vec::new();
    let has_offset = geometries[0].0.contains_key("applyOffset");

    for (attrs, indices) in &geometries {
        let pos = &attrs["position"].values;
        all_positions.extend_from_slice(pos);
        if has_offset {
            if let Some(ao) = attrs.get("applyOffset") {
                all_apply_offset.extend_from_slice(&ao.values);
            }
        }
        let n = indices.len();
        for i in 0..n {
            let v = match indices {
                IndexStorage::U16(v) => v[i] as u32,
                IndexStorage::U32(v) => v[i],
            };
            all_indices_vec.push(v + vertex_offset);
        }
        vertex_offset += (pos.len() / 3) as u32;
    }

    let mut merged_attrs = HashMap::new();
    merged_attrs.insert(
        "position".to_string(),
        GeometryAttribute::new(ComponentDatatype::Double, 3, false, all_positions),
    );
    if has_offset {
        merged_attrs.insert(
            "applyOffset".to_string(),
            GeometryAttribute::new(ComponentDatatype::UnsignedByte, 1, false, all_apply_offset),
        );
    }

    let total_vertices = vertex_offset as usize;
    let mut merged_indices = IndexDatatype::create_typed_array(total_vertices, all_indices_vec.len());
    for (i, &v) in all_indices_vec.iter().enumerate() {
        write_index(&mut merged_indices, i, v);
    }

    let bounding_sphere = BoundingSphere::from_vertices(
        &merged_attrs["position"].values,
        None,
        Some(3),
        None,
    );

    let offset_attr_str = polygon_outline_geometry
        .offset_attribute
        .map(|_| "applyOffset".to_string());

    Some(Geometry::with_all(
        merged_attrs,
        Some(merged_indices),
        Some(PrimitiveType::Lines),
        Some(bounding_sphere),
        GeometryType::None,
        None,
        offset_attr_str,
    ))
}
