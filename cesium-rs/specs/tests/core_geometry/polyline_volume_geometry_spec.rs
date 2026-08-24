//! Mirror of `packages/engine/Specs/Core/PolylineVolumeGeometrySpec.js`.
//!
//! Ports the `createGeometry`-returns-undefined tests and the
//! vertex-count assertions for straight corridors.
//!
//! DEVIATION: JS uses an options object; the Rust port uses
//! `PolylineVolumeGeometry::new` with positional parameters.

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::corner_type::CornerType;
use cesium_core::math::CesiumMath;
use cesium_core::polyline_volume_geometry::{create_geometry, PolylineVolumeGeometry};
use cesium_core::vertex_format::VertexFormat;

/// Box shape used by the JS spec's `beforeAll`.
fn box_shape() -> Vec<Cartesian2> {
    vec![
        Cartesian2::new(-100.0, -100.0),
        Cartesian2::new(100.0, -100.0),
        Cartesian2::new(100.0, 100.0),
        Cartesian2::new(-100.0, 100.0),
    ]
}

#[test]
fn create_geometry_returns_undefined_without_2_unique_polyline_positions() {
    let geometry = create_geometry(&PolylineVolumeGeometry::new(
        vec![Cartesian3::default()],
        box_shape(),
        None,
        None,
        None,
        None,
    ));
    assert!(geometry.is_none());
}

#[test]
fn create_geometry_returns_undefined_without_3_unique_shape_positions() {
    let geometry = create_geometry(&PolylineVolumeGeometry::new(
        vec![Cartesian3::UNIT_X, Cartesian3::UNIT_Y],
        vec![
            Cartesian2::UNIT_X,
            Cartesian2::UNIT_X,
            Cartesian2::UNIT_X,
        ],
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
    let m = create_geometry(&PolylineVolumeGeometry::new(
        positions,
        box_shape(),
        None,
        Some(CornerType::Mitered),
        Some(VertexFormat::position_only()),
        None,
    ));
    let m = m.expect("geometry should not be None");

    // 6 positions * 4 box positions * 2 to duplicate for normals + 4 positions * 2 ends
    assert_eq!(m.attributes.get("position").unwrap().values.len(), 56 * 3);
    // 5 segments + 8 triangles per segment + 2 triangles * 2 ends
    assert_eq!(m.indices.as_ref().unwrap().len(), 44 * 3);
}

#[test]
fn computes_most_vertex_attributes() {
    let positions =
        Cartesian3::from_degrees_array(&[90.0, -30.0, 90.0, -35.0], None, None);
    let m = create_geometry(&PolylineVolumeGeometry::new(
        positions,
        box_shape(),
        None,
        Some(CornerType::Mitered),
        Some(VertexFormat::position_normal_and_st()),
        None,
    ));
    let m = m.expect("geometry should not be None");

    let num_vertices = 56;
    let num_triangles = 44;
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
    assert_eq!(m.indices.as_ref().unwrap().len(), num_triangles * 3);
}

#[test]
fn computes_straight_volume() {
    let positions = Cartesian3::from_degrees_array(
        &[-67.655, 0.0, -67.655, 15.0, -67.655, 20.0],
        None,
        None,
    );
    let m = create_geometry(&PolylineVolumeGeometry::new(
        positions,
        box_shape(),
        None,
        Some(CornerType::Beveled),
        Some(VertexFormat::position_only()),
        Some(CesiumMath::PI / 6.0),
    ));
    let m = m.expect("geometry should not be None");

    // 4 positions * 2 for duplication * 4 for shape
    assert_eq!(m.attributes.get("position").unwrap().values.len(), 32 * 3);
    // 2 segments * 8 triangles per segment + 2 * 2 ends
    assert_eq!(m.indices.as_ref().unwrap().len(), 20 * 3);
}
