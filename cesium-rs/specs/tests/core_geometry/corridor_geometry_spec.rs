//! Mirror of `packages/engine/Specs/Core/CorridorGeometrySpec.js`.
//!
//! Ports the `createGeometry`-returns-undefined, positions, all vertex
//! attributes, and extruded tests.
//!
//! DEVIATION: JS uses an options object; the Rust port uses
//! `CorridorGeometry::new` with positional parameters.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::corner_type::CornerType;
use cesium_core::corridor_geometry::{create_geometry, CorridorGeometry};
use cesium_core::vertex_format::VertexFormat;

#[test]
fn create_geometry_returns_undefined_without_2_unique_positions() {
    let positions =
        Cartesian3::from_degrees_array(&[90.0, -30.0, 90.0, -30.0], None, None);
    let geometry = create_geometry(&CorridorGeometry::new(
        positions,
        10000.0,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    ));
    assert!(geometry.is_none());
}

#[test]
fn computes_positions() {
    let positions =
        Cartesian3::from_degrees_array(&[90.0, -30.0, 90.0, -35.0], None, None);
    let m = create_geometry(&CorridorGeometry::new(
        positions,
        30000.0,
        None,
        Some(VertexFormat::position_only()),
        None,
        None,
        Some(CornerType::Mitered),
        None,
        None,
        None,
    ));
    let m = m.expect("geometry should not be None");

    // 6 left + 6 right
    let num_vertices = 12;
    // 5 segments x 2 triangles per segment
    let num_triangles = 10;
    assert_eq!(
        m.attributes.get("position").unwrap().values.len(),
        num_vertices * 3
    );
    assert_eq!(m.indices.as_ref().unwrap().len(), num_triangles * 3);
}

#[test]
fn compute_all_vertex_attributes() {
    let positions =
        Cartesian3::from_degrees_array(&[90.0, -30.0, 90.0, -35.0], None, None);
    let m = create_geometry(&CorridorGeometry::new(
        positions,
        30000.0,
        None,
        Some(VertexFormat::all()),
        None,
        None,
        Some(CornerType::Mitered),
        None,
        None,
        None,
    ));
    let m = m.expect("geometry should not be None");

    let num_vertices = 12;
    let num_triangles = 10;
    assert_eq!(
        m.attributes.get("position").unwrap().values.len(),
        num_vertices * 3
    );
    assert_eq!(
        m.attributes.get("st").unwrap().values.len(),
        num_vertices * 2
    );
    assert_eq!(
        m.attributes.get("normal").unwrap().values.len(),
        num_vertices * 3
    );
    assert_eq!(
        m.attributes.get("tangent").unwrap().values.len(),
        num_vertices * 3
    );
    assert_eq!(
        m.attributes.get("bitangent").unwrap().values.len(),
        num_vertices * 3
    );
    assert_eq!(m.indices.as_ref().unwrap().len(), num_triangles * 3);
}

#[test]
fn computes_positions_extruded() {
    let positions =
        Cartesian3::from_degrees_array(&[90.0, -30.0, 90.0, -35.0], None, None);
    let m = create_geometry(&CorridorGeometry::new(
        positions,
        30000.0,
        None,
        Some(VertexFormat::position_only()),
        None,
        Some(30000.0), // extrudedHeight
        Some(CornerType::Mitered),
        None,
        None,
        None,
    ));
    let m = m.expect("geometry should not be None");

    // 6 positions x 4 for a box at each position x 3 to duplicate for normals
    let num_vertices = 72;
    // 5 segments * 8 triangles per segment + 2 triangles x 2 ends
    let num_triangles = 44;
    assert_eq!(
        m.attributes.get("position").unwrap().values.len(),
        num_vertices * 3
    );
    assert_eq!(m.indices.as_ref().unwrap().len(), num_triangles * 3);
}
