//! Tests for terrain-related modules: TerrainEncoding, TerrainProvider
//! (grid indices, skirt calculations), TerrainMesh, Geometry,
//! GeometryAttributes, and related data types.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::component_datatype::ComponentDatatype;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::geometry::Geometry;
use cesium_core::geometry_attribute::GeometryAttribute;
use cesium_core::geometry_attributes::GeometryAttributes;
use cesium_core::geometry_type::GeometryType;
use cesium_core::index_datatype::IndexStorage;
use cesium_core::primitive_type::PrimitiveType;
use cesium_core::rectangle::Rectangle;
use cesium_core::terrain_encoding::TerrainEncoding;
use cesium_core::terrain_provider::{
    get_estimated_level_zero_geometric_error_for_a_heightmap,
    get_regular_grid_indices,
    get_regular_grid_indices_and_edge_indices,
    get_skirt_index_count,
    get_skirt_index_count_with_filled_corners,
    get_skirt_vertex_count,
};
use std::collections::HashMap;

// --- TerrainEncoding ---
#[test]
fn terrain_encoding_basic_stride() {
    let enc = TerrainEncoding::new(false, false, 1.0, 0.0);
    assert_eq!(enc.stride, 6); // X,Y,Z,H,U,V
    assert!(!enc.has_vertex_normals);
    assert!(!enc.has_water_mask);
}

#[test]
fn terrain_encoding_with_normals() {
    let enc = TerrainEncoding::new(true, false, 1.0, 0.0);
    assert_eq!(enc.stride, 9); // 6 + 3 normals
}

#[test]
fn terrain_encoding_with_water_mask() {
    let enc = TerrainEncoding::new(false, true, 1.0, 0.0);
    assert_eq!(enc.stride, 7); // 6 + 1 water mask
}

#[test]
fn terrain_encoding_with_both() {
    let enc = TerrainEncoding::new(true, true, 1.0, 0.0);
    assert_eq!(enc.stride, 10); // 6 + 3 + 1
}

// --- TerrainProvider grid functions ---
#[test]
fn regular_grid_indices_2x2() {
    let indices = get_regular_grid_indices(2, 2);
    // 2x2 grid → 1 cell → 6 indices (2 triangles)
    assert_eq!(indices.len(), 6);
}

#[test]
fn regular_grid_indices_3x3() {
    let indices = get_regular_grid_indices(3, 3);
    // 3x3 grid → 4 cells → 24 indices
    assert_eq!(indices.len(), 24);
}

#[test]
fn regular_grid_indices_cached() {
    let a = get_regular_grid_indices(4, 4);
    let b = get_regular_grid_indices(4, 4);
    assert_eq!(a, b); // same result from cache
}

#[test]
fn regular_grid_indices_and_edge_indices() {
    let result = get_regular_grid_indices_and_edge_indices(3, 3);
    assert_eq!(result.west_indices_south_to_north.len(), 3);
    assert_eq!(result.south_indices_east_to_west.len(), 3);
    assert_eq!(result.east_indices_north_to_south.len(), 3);
    assert_eq!(result.north_indices_west_to_east.len(), 3);
}

#[test]
fn skirt_vertex_count_calculation() {
    let west = vec![0, 1, 2];
    let south = vec![3, 4, 5];
    let east = vec![6, 7, 8];
    let north = vec![9, 10, 11];
    let count = get_skirt_vertex_count(&west, &south, &east, &north);
    assert_eq!(count, 12);
}

#[test]
fn skirt_index_count_calculation() {
    let count = get_skirt_index_count(12);
    // (12 - 4) * 2 * 3 = 48
    assert_eq!(count, 48);
}

#[test]
fn skirt_index_count_with_filled_corners() {
    let count = get_skirt_index_count_with_filled_corners(12);
    // ((12 - 4) * 2 + 4) * 3 = 60
    assert_eq!(count, 60);
}

#[test]
fn estimated_geometric_error_for_heightmap() {
    let ellipsoid = Ellipsoid::WGS84.clone();
    let error = get_estimated_level_zero_geometric_error_for_a_heightmap(&ellipsoid, 65.0, 2);
    assert!(error > 0.0);
}

// --- Geometry ---
#[test]
fn geometry_new_with_position() {
    let mut attrs = HashMap::new();
    attrs.insert(
        "position".to_string(),
        GeometryAttribute::new(ComponentDatatype::Double, 3, false, vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
    );
    let geo = Geometry::new(attrs, None, None, None);
    assert_eq!(geo.compute_number_of_vertices(), Some(2));
    assert_eq!(geo.primitive_type, PrimitiveType::Triangles);
}

#[test]
fn geometry_with_indices() {
    let mut attrs = HashMap::new();
    attrs.insert(
        "position".to_string(),
        GeometryAttribute::new(ComponentDatatype::Double, 3, false, vec![0.0; 12]),
    );
    let geo = Geometry::new(
        attrs,
        Some(IndexStorage::U16(vec![0, 1, 2, 0, 2, 3])),
        Some(PrimitiveType::Triangles),
        None,
    );
    assert!(geo.indices.is_some());
}

#[test]
fn geometry_with_all_options() {
    let mut attrs = HashMap::new();
    attrs.insert(
        "position".to_string(),
        GeometryAttribute::new(ComponentDatatype::Double, 3, false, vec![0.0; 9]),
    );
    let geo = Geometry::with_all(
        attrs,
        None,
        Some(PrimitiveType::Lines),
        None,
        GeometryType::Triangles,
        None,
        None,
    );
    assert_eq!(geo.geometry_type, GeometryType::Triangles);
    assert_eq!(geo.primitive_type, PrimitiveType::Lines);
}

// --- GeometryAttributes ---
#[test]
fn geometry_attributes_default_all_none() {
    let ga = GeometryAttributes::default();
    assert!(ga.position.is_none());
    assert!(ga.normal.is_none());
    assert!(ga.st.is_none());
    assert!(ga.bitangent.is_none());
    assert!(ga.tangent.is_none());
    assert!(ga.color.is_none());
}

#[test]
fn geometry_attributes_set_position() {
    let mut ga = GeometryAttributes::default();
    ga.position = Some(GeometryAttribute::new(
        ComponentDatatype::Double,
        3,
        false,
        vec![1.0, 2.0, 3.0],
    ));
    assert!(ga.position.is_some());
    assert_eq!(ga.position.unwrap().values.len(), 3);
}
