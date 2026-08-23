//! Tests for WebMercatorProjection, ScaleToGeodeticSurface, EventHelper,
//! TileProviderError, TileEdge, VertexFormat, TilingScheme trait,
//! DecodeVectorPolylinePositions, and remaining stub modules.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::event_helper::EventHelper;
use cesium_core::math::CesiumMath;
use cesium_core::scale_to_geodetic_surface::{scale_to_geodetic_surface, scale_to_geodetic_surface_new};
use cesium_core::tile_edge::TileEdge;
use cesium_core::tile_provider_error::TileProviderError;
use cesium_core::vertex_format::VertexFormat;
use cesium_core::web_mercator_projection::WebMercatorProjection;

// --- WebMercatorProjection ---
#[test]
fn web_mercator_new_default_ellipsoid() {
    let proj = WebMercatorProjection::new(None);
    assert_eq!(proj.ellipsoid().maximum_radius(), Ellipsoid::WGS84.maximum_radius());
}

#[test]
fn web_mercator_project_unproject_roundtrip() {
    let proj = WebMercatorProjection::new(None);
    let cart = Cartographic::new(0.5, 0.3, 100.0);
    let projected = proj.project(&cart);
    let unprojected = proj.unproject(&projected);
    assert!((unprojected.longitude - cart.longitude).abs() < 1e-10);
    assert!((unprojected.latitude - cart.latitude).abs() < 1e-10);
    assert!((unprojected.height - cart.height).abs() < 1e-10);
}

#[test]
fn web_mercator_mercator_angle_to_geodetic_latitude() {
    let lat = WebMercatorProjection::mercator_angle_to_geodetic_latitude(CesiumMath::PI);
    assert!((lat - WebMercatorProjection::MAXIMUM_LATITUDE).abs() < 1e-10);
}

#[test]
fn web_mercator_geodetic_latitude_to_mercator_angle() {
    let angle = WebMercatorProjection::geodetic_latitude_to_mercator_angle(0.0);
    assert!((angle - 0.0).abs() < 1e-10);
}

#[test]
fn web_mercator_project_equator() {
    let proj = WebMercatorProjection::new(None);
    let cart = Cartographic::new(0.0, 0.0, 0.0);
    let result = proj.project(&cart);
    assert!((result.x).abs() < 1e-10);
    assert!((result.y).abs() < 1e-10);
}

// --- ScaleToGeodeticSurface ---
#[test]
fn scale_to_geodetic_surface_on_surface() {
    let ellipsoid = Ellipsoid::WGS84.clone();
    let radii = ellipsoid.radii();
    let one_over_radii = Cartesian3::new(1.0 / radii.x, 1.0 / radii.y, 1.0 / radii.z);
    let one_over_radii_squared = Cartesian3::new(
        1.0 / (radii.x * radii.x),
        1.0 / (radii.y * radii.y),
        1.0 / (radii.z * radii.z),
    );
    // Point on the equator at the prime meridian
    let cartesian = Cartesian3::new(radii.x * 2.0, 0.0, 0.0);
    let result = scale_to_geodetic_surface_new(
        &cartesian,
        &one_over_radii,
        &one_over_radii_squared,
        CesiumMath::EPSILON12,
    );
    assert!(result.is_some());
    let surface_point = result.unwrap();
    // Verify the point is on the ellipsoid surface
    let x2 = (surface_point.x / radii.x).powi(2);
    let y2 = (surface_point.y / radii.y).powi(2);
    let z2 = (surface_point.z / radii.z).powi(2);
    assert!((x2 + y2 + z2 - 1.0).abs() < 1e-6);
}

#[test]
fn scale_to_geodetic_surface_in_place() {
    let ellipsoid = Ellipsoid::WGS84.clone();
    let radii = ellipsoid.radii();
    let one_over_radii = Cartesian3::new(1.0 / radii.x, 1.0 / radii.y, 1.0 / radii.z);
    let one_over_radii_squared = Cartesian3::new(
        1.0 / (radii.x * radii.x),
        1.0 / (radii.y * radii.y),
        1.0 / (radii.z * radii.z),
    );
    let cartesian = Cartesian3::new(0.0, radii.y * 3.0, 0.0);
    let mut result = Cartesian3::default();
    let ok = scale_to_geodetic_surface(
        &cartesian,
        &one_over_radii,
        &one_over_radii_squared,
        CesiumMath::EPSILON12,
        &mut result,
    );
    assert!(ok);
    assert!((result.y - radii.y).abs() < 1e-2);
}

// --- EventHelper ---
#[test]
fn event_helper_remove_all() {
    use std::rc::Rc;
    use std::cell::RefCell;
    let counter = Rc::new(RefCell::new(0));
    let mut helper = EventHelper::new();
    let c = counter.clone();
    helper.add_removal(Box::new(move || { *c.borrow_mut() += 1; }));
    let c2 = counter.clone();
    helper.add_removal(Box::new(move || { *c2.borrow_mut() += 10; }));
    helper.remove_all();
    assert_eq!(*counter.borrow(), 11);
}

#[test]
fn event_helper_default() {
    let helper = EventHelper::default();
    let _ = helper;
}

// --- TileProviderError ---
#[test]
fn tile_provider_error_new() {
    let err = TileProviderError::new("test error".to_string(), Some(1), Some(2), Some(3), None);
    assert_eq!(err.message, "test error");
    assert_eq!(err.x, Some(1));
    assert_eq!(err.y, Some(2));
    assert_eq!(err.level, Some(3));
    assert_eq!(err.times_retried, 0);
    assert!(!err.retry);
}

#[test]
fn tile_provider_error_report_error_new() {
    let err = TileProviderError::report_error(None, "new error".to_string(), Some(0), Some(0), Some(0));
    assert_eq!(err.message, "new error");
    assert_eq!(err.times_retried, 0);
}

#[test]
fn tile_provider_error_report_error_existing() {
    let prev = TileProviderError::new("first".to_string(), Some(0), Some(0), Some(0), Some(0));
    let err = TileProviderError::report_error(Some(prev), "second".to_string(), Some(1), Some(1), Some(1));
    assert_eq!(err.message, "second");
    assert_eq!(err.times_retried, 1);
}

// --- TileEdge ---
#[test]
fn tile_edge_variants() {
    assert_eq!(TileEdge::West as i32, 0);
    assert_eq!(TileEdge::North as i32, 1);
    assert_eq!(TileEdge::East as i32, 2);
    assert_eq!(TileEdge::South as i32, 3);
    assert_eq!(TileEdge::Northwest as i32, 4);
    assert_eq!(TileEdge::Southeast as i32, 7);
}

// --- VertexFormat ---
#[test]
fn vertex_format_default_all_false() {
    let vf = VertexFormat::default();
    assert!(!vf.position);
    assert!(!vf.normal);
    assert!(!vf.st);
    assert!(!vf.tangent);
    assert!(!vf.bitangent);
    assert!(!vf.color);
}

#[test]
fn vertex_format_position_only() {
    let vf = VertexFormat::position_only();
    assert!(vf.position);
    assert!(!vf.normal);
}

#[test]
fn vertex_format_position_and_normal() {
    let vf = VertexFormat::position_and_normal();
    assert!(vf.position);
    assert!(vf.normal);
    assert!(!vf.st);
}

#[test]
fn vertex_format_all() {
    let vf = VertexFormat::all();
    assert!(vf.position);
    assert!(vf.normal);
    assert!(vf.st);
    assert!(vf.tangent);
    assert!(vf.bitangent);
    assert!(!vf.color);
}

#[test]
fn vertex_format_pack_unpack_roundtrip() {
    // new() params: position, normal, st, bitangent, tangent, color
    let original = VertexFormat::new(true, true, false, false, true, true);
    let mut array = vec![0.0f64; VertexFormat::PACKED_LENGTH];
    original.pack(&mut array, 0);
    let unpacked = VertexFormat::unpack(&array, 0, None);
    assert_eq!(unpacked, original);
}

#[test]
fn vertex_format_pack_with_offset() {
    let vf = VertexFormat::position_only();
    let mut array = vec![0.0f64; 10];
    vf.pack(&mut array, 3);
    assert_eq!(array[3], 1.0);
    assert_eq!(array[4], 0.0);
}

#[test]
fn vertex_format_clone_into() {
    let vf = VertexFormat::position_and_normal();
    let cloned = vf.clone_into(None);
    assert_eq!(cloned, vf);
}
