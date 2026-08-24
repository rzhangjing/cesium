//! Tests for remaining geometry types: outline geometries, coplanar polygon,
//! corridor outline, ellipse outline, ellipsoid outline, frustum geometry,
//! ground polyline, polygon outline, polyline volume, rectangle outline,
//! simple polyline, sphere outline, wall outline, plane geometry,
//! circle outline, and geometry library stubs.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartesian4::Cartesian4;
use cesium_core::circle_outline_geometry::CircleOutlineGeometry;
use cesium_core::coplanar_polygon_geometry::CoplanarPolygonGeometry;
use cesium_core::corridor_outline_geometry::CorridorOutlineGeometry;
use cesium_core::ellipse_outline_geometry::EllipseOutlineGeometry;
use cesium_core::ellipsoid_outline_geometry::EllipsoidOutlineGeometry;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::frustum_geometry::FrustumGeometry;
use cesium_core::frustum_outline_geometry::FrustumOutlineGeometry;
use cesium_core::ground_polyline_geometry::GroundPolylineGeometry;
use cesium_core::plane_geometry::PlaneGeometry;
use cesium_core::plane_outline_geometry::PlaneOutlineGeometry;
use cesium_core::polygon_outline_geometry::PolygonOutlineGeometry;
use cesium_core::cartesian2::Cartesian2;
use cesium_core::polyline_volume_outline_geometry::PolylineVolumeOutlineGeometry;
use cesium_core::rectangle::Rectangle;
use cesium_core::rectangle_outline_geometry::RectangleOutlineGeometry;
use cesium_core::simple_polyline_geometry::SimplePolylineGeometry;
use cesium_core::sphere_outline_geometry::SphereOutlineGeometry;
use cesium_core::vertex_format::VertexFormat;
use cesium_core::wall_outline_geometry::WallOutlineGeometry;

// --- CircleOutlineGeometry ---
#[test]
fn circle_outline_new() {
    let center = Cartesian3::new(1.0, 2.0, 3.0);
    let geo = CircleOutlineGeometry::new(center, 100.0, None, None, None, None, None);
    assert_eq!(geo.radius(), 100.0);
    assert_eq!(geo.center().x, 1.0);
}

// --- CoplanarPolygonGeometry ---
#[test]
fn coplanar_polygon_new() {
    let positions = vec![
        Cartesian3::new(0.0, 0.0, 0.0),
        Cartesian3::new(1.0, 0.0, 0.0),
        Cartesian3::new(1.0, 1.0, 0.0),
    ];
    let geo = CoplanarPolygonGeometry::new(positions, None);
    let _ = geo;
}

// --- CorridorOutlineGeometry ---
#[test]
fn corridor_outline_new() {
    let positions = vec![
        Cartesian3::new(0.0, 0.0, 0.0),
        Cartesian3::new(1.0, 0.0, 0.0),
    ];
    let geo = CorridorOutlineGeometry::new(positions, 10.0, None, None, None, None, None, None);
    let _ = geo;
}

// --- EllipseOutlineGeometry ---
#[test]
fn ellipse_outline_new() {
    let center = Cartesian3::new(1.0, 2.0, 3.0);
    let geo = EllipseOutlineGeometry::new(center, 100.0, 50.0, None, None, None, None, None, None, None);
    let _ = geo;
}

// --- EllipsoidOutlineGeometry ---
#[test]
fn ellipsoid_outline_new() {
    let geo = EllipsoidOutlineGeometry::new(None, None, None, None, None);
    let _ = geo;
}

// --- FrustumGeometry ---
#[test]
fn frustum_geometry_new() {
    let origin = Cartesian3::new(0.0, 0.0, 0.0);
    let orientation = Cartesian4::new(0.0, 0.0, -1.0, 0.0);
    let geo = FrustumGeometry::new(origin, orientation, 1.0, 100.0, 1.0, 1.0);
    let _ = geo;
}

// --- FrustumOutlineGeometry ---
#[test]
fn frustum_outline_new() {
    let origin = Cartesian3::new(0.0, 0.0, 0.0);
    let orientation = Cartesian4::new(0.0, 0.0, -1.0, 0.0);
    let geo = FrustumOutlineGeometry::new(origin, orientation, 1.0, 100.0, 1.0, 1.0);
    let _ = geo;
}

// --- GroundPolylineGeometry ---
#[test]
fn ground_polyline_new() {
    let positions = vec![
        Cartesian3::new(0.0, 0.0, 0.0),
        Cartesian3::new(1.0, 0.0, 0.0),
    ];
    let geo = GroundPolylineGeometry::new(positions, None, None);
    let _ = geo;
}

// --- PlaneGeometry ---
#[test]
fn plane_geometry_new() {
    let geo = PlaneGeometry::new(None);
    let _ = geo;
}

#[test]
fn plane_geometry_create_geometry() {
    let vf = VertexFormat { position: true, normal: true, st: true, tangent: true, bitangent: true, ..Default::default() };
    let geo = PlaneGeometry::new(Some(vf));
    let geom = geo.create_geometry();
    assert!(geom.attributes.contains_key("position"));
    assert!(geom.attributes.contains_key("normal"));
    assert!(geom.attributes.contains_key("st"));
    assert!(geom.attributes.contains_key("tangent"));
    assert!(geom.attributes.contains_key("bitangent"));
}

#[test]
fn plane_geometry_pack_unpack() {
    let vf = VertexFormat { position: true, normal: true, ..Default::default() };
    let original = PlaneGeometry::new(Some(vf));
    let mut array = vec![0.0f64; PlaneGeometry::PACKED_LENGTH];
    original.pack(&mut array, None);
    let unpacked = PlaneGeometry::unpack(&array, None);
    let _ = unpacked;
}

// --- PlaneOutlineGeometry ---
#[test]
fn plane_outline_create_geometry() {
    let geom = PlaneOutlineGeometry::create_geometry();
    assert!(geom.attributes.contains_key("position"));
}

// --- PolygonOutlineGeometry ---
#[test]
fn polygon_outline_new() {
    let positions = vec![
        Cartesian3::new(0.0, 0.0, 0.0),
        Cartesian3::new(1.0, 0.0, 0.0),
        Cartesian3::new(1.0, 1.0, 0.0),
    ];
    let geo = PolygonOutlineGeometry::new(positions, None, None, None, None, None, None, None);
    let _ = geo;
}

// --- PolylineVolumeOutlineGeometry ---
#[test]
fn polyline_volume_outline_new() {
    let shape = vec![
        Cartesian3::new(-1.0, -1.0, 0.0),
        Cartesian3::new(1.0, -1.0, 0.0),
        Cartesian3::new(1.0, 1.0, 0.0),
        Cartesian3::new(-1.0, 1.0, 0.0),
    ];
    let positions = vec![
        Cartesian3::new(0.0, 0.0, 0.0),
        Cartesian3::new(0.0, 0.0, 100.0),
    ];
    let shape: Vec<Cartesian2> = vec![
        Cartesian2 { x: -1.0, y: -1.0 },
        Cartesian2 { x: 1.0, y: -1.0 },
        Cartesian2 { x: 1.0, y: 1.0 },
        Cartesian2 { x: -1.0, y: 1.0 },
    ];
    let positions = vec![
        Cartesian3::new(0.0, 0.0, 0.0),
        Cartesian3::new(0.0, 0.0, 100.0),
    ];
    let geo = PolylineVolumeOutlineGeometry::new(positions, shape, None, None, None);
    let _ = geo;
}

// --- RectangleOutlineGeometry ---
#[test]
fn rectangle_outline_new() {
    let rect = Rectangle::new(-1.0, -1.0, 1.0, 1.0);
    let geo = RectangleOutlineGeometry::new(rect, None, None, None);
    let _ = geo;
}

// --- SimplePolylineGeometry ---
#[test]
fn simple_polyline_new() {
    let positions = vec![
        Cartesian3::new(0.0, 0.0, 0.0),
        Cartesian3::new(1.0, 0.0, 0.0),
        Cartesian3::new(2.0, 0.0, 0.0),
    ];
    let geo = SimplePolylineGeometry::new(positions, None, None, None, None, None);
    assert_eq!(geo.positions().len(), 3);
}

// --- SphereOutlineGeometry ---
#[test]
fn sphere_outline_new() {
    let geo = SphereOutlineGeometry::new(Some(5.0), None, None, None);
    assert_eq!(geo.radius(), 5.0);
}

#[test]
fn sphere_outline_defaults() {
    let geo = SphereOutlineGeometry::new(None, None, None, None);
    assert_eq!(geo.radius(), 1.0);
}

// --- WallOutlineGeometry ---
#[test]
fn wall_outline_new() {
    let positions = vec![
        Cartesian3::new(0.0, 0.0, 0.0),
        Cartesian3::new(1.0, 0.0, 0.0),
        Cartesian3::new(2.0, 0.0, 0.0),
    ];
    let geo = WallOutlineGeometry::new(positions, None, None, None, None);
    assert_eq!(geo.positions().len(), 3);
}

#[test]
fn wall_outline_from_constant_heights() {
    let positions = vec![
        Cartesian3::new(0.0, 0.0, 0.0),
        Cartesian3::new(1.0, 0.0, 0.0),
    ];
    let geo = WallOutlineGeometry::from_constant_heights(positions, Some(0.0), Some(100.0), None);
    assert_eq!(geo.positions().len(), 2);
}
