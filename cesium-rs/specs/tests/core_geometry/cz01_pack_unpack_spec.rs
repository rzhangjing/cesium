//! Mirror tests for the CZ-01 pack/unpack and rectangle API cluster.
//!
//! Mirrors (selected `it(...)` cases):
//! - `packages/engine/Specs/Core/CorridorGeometrySpec.js`
//!   ("packed length", "pack", "unpack")
//! - `packages/engine/Specs/Core/CorridorOutlineGeometrySpec.js`
//!   ("packed length", "pack", "unpack")
//! - `packages/engine/Specs/Core/PolygonGeometrySpec.js`
//!   ("pack", "unpack", "computeRectangleFromPositions",
//!    "textureCoordinateRotationPoints")
//! - `packages/engine/Specs/Core/StereographicSpec.js` ("fromCartesian")

use cesium_core::arc_type::ArcType;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::corner_type::CornerType;
use cesium_core::corridor_geometry::CorridorGeometry;
use cesium_core::corridor_outline_geometry::CorridorOutlineGeometry;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::math::CesiumMath;
use cesium_core::perspective_frustum::PerspectiveFrustum;
use cesium_core::polygon_geometry::PolygonGeometry;
use cesium_core::polyline_geometry::PolylineGeometry;
use cesium_core::quaternion::Quaternion;
use cesium_core::rectangle::Rectangle;
use cesium_core::stereographic::Stereographic;
use cesium_core::vertex_format::VertexFormat;

fn near(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() < eps
}

// --- CorridorGeometry pack/unpack (CorridorGeometrySpec.js) ---

#[test]
fn corridor_packed_length() {
    let positions = Cartesian3::from_degrees_array(&[90.0, -30.0, 90.0, -35.0], None, None);
    let corridor = CorridorGeometry::new(
        positions,
        30000.0,
        None,
        Some(VertexFormat::position_only()),
        Some(100000.0),
        Some(200000.0),
        Some(CornerType::Mitered),
        Some(0.01),
        None,
        None,
    );
    // 1 + positions * 3 + Ellipsoid(3) + VertexFormat(6) + 7
    assert_eq!(corridor.packed_length(), 1 + 2 * 3 + 3 + 6 + 7);
}

#[test]
fn corridor_pack_and_unpack() {
    let positions = Cartesian3::from_degrees_array(
        &[90.0, -30.0, 90.0, -35.0, 92.0, -33.0],
        None,
        None,
    );
    let corridor = CorridorGeometry::new(
        positions,
        30000.0,
        None,
        Some(VertexFormat::position_only()),
        Some(100000.0),
        Some(200000.0),
        Some(CornerType::Beveled),
        Some(0.01),
        Some(true),
        None,
    );
    let mut packed = vec![0.0f64; corridor.packed_length()];
    corridor.pack(&mut packed, None);
    let unpacked = CorridorGeometry::unpack(&packed, None, None);
    assert_eq!(
        unpacked.positions().len(),
        corridor.positions().len()
    );
    for (a, b) in unpacked.positions().iter().zip(corridor.positions().iter()) {
        assert!(Cartesian3::equals_epsilon(Some(a), Some(b), Some(CesiumMath::EPSILON14), None));
    }
    assert_eq!(unpacked.width(), corridor.width());
    assert_eq!(unpacked.height(), corridor.height());
    assert_eq!(unpacked.extruded_height(), corridor.extruded_height());
    assert_eq!(unpacked.corner_type(), corridor.corner_type());
    assert_eq!(unpacked.granularity(), corridor.granularity());
    assert_eq!(unpacked.ellipsoid(), corridor.ellipsoid());
}

// --- CorridorOutlineGeometry pack/unpack (CorridorOutlineGeometrySpec.js) ---

#[test]
fn corridor_outline_pack_and_unpack() {
    let positions = Cartesian3::from_degrees_array(&[90.0, -30.0, 90.0, -35.0], None, None);
    let corridor = CorridorOutlineGeometry::new(
        positions,
        30000.0,
        None,
        Some(100000.0),
        Some(200000.0),
        Some(CornerType::Beveled),
        Some(0.01),
        None,
    );
    // 1 + positions * 3 + Ellipsoid(3) + 6
    assert_eq!(corridor.packed_length(), 1 + 2 * 3 + 3 + 6);

    let mut packed = vec![0.0f64; corridor.packed_length()];
    corridor.pack(&mut packed, None);
    let unpacked = CorridorOutlineGeometry::unpack(&packed, None, None);
    // JS goes through the constructor on the no-result path, re-applying the
    // height/extrudedHeight min/max normalization.
    assert_eq!(unpacked.width(), 30000.0);
    assert_eq!(unpacked.height(), 200000.0);
    assert_eq!(unpacked.extruded_height(), 100000.0);
    assert_eq!(unpacked.corner_type(), CornerType::Beveled);
    assert_eq!(unpacked.granularity(), 0.01);
}

// --- PolygonGeometry pack/unpack (PolygonGeometrySpec.js) ---

#[test]
fn polygon_pack_and_unpack() {
    let positions = Cartesian3::from_degrees_array(
        &[-72.0, 40.0, -70.0, 35.0, -75.0, 30.0, -70.0, 30.0, -68.0, 40.0],
        None,
        None,
    );
    // Note: JS debug check forbids perPositionHeight together with height,
    // so perPositionHeight is left false here.
    let polygon = PolygonGeometry::from_positions(
        positions.clone(),
        None,
        Some(VertexFormat::position_only()),
        Some(10.0),
        Some(20.0),
        Some(0.01),
        Some(0.5),
        Some(false),
        Some(false),
        Some(true),
        None,
        Some(ArcType::Rhumb),
        None,
    );
    let mut packed = vec![0.0f64; polygon.packed_length()];
    polygon.pack(&mut packed, None);
    // The final packed slot stores packedLength itself (JS behavior).
    assert_eq!(*packed.last().unwrap(), polygon.packed_length() as f64);

    let unpacked = PolygonGeometry::unpack(&packed, None, None);
    assert_eq!(unpacked.packed_length(), polygon.packed_length());

    // Round trip with a provided result object as well.
    let mut result = unpacked.clone();
    let again = PolygonGeometry::unpack(&packed, None, Some(&mut result));
    assert_eq!(again.packed_length(), polygon.packed_length());
}

#[test]
fn polygon_create_shadow_volume_uses_min_max_heights() {
    let positions = Cartesian3::from_degrees_array(
        &[-72.0, 40.0, -70.0, 35.0, -75.0, 30.0, -70.0, 30.0],
        None,
        None,
    );
    let polygon = PolygonGeometry::new(
        positions,
        None,
        None,
        None,
        None,
        Some(0.01),
        Some(0.5),
        None,
        None,
        None,
        None,
        Some(ArcType::Rhumb),
    );
    let min_height_func = |_: f64, _: &Ellipsoid| -100.0;
    let max_height_func = |_: f64, _: &Ellipsoid| 300.0;
    let shadow = PolygonGeometry::create_shadow_volume(
        &polygon,
        &min_height_func,
        &max_height_func,
    );
    // Constructor normalization: height = max, extruded = min.
    let mut packed = vec![0.0f64; shadow.packed_length()];
    shadow.pack(&mut packed, None);
    let unpacked = PolygonGeometry::unpack(&packed, None, None);
    let mut packed2 = vec![0.0f64; unpacked.packed_length()];
    unpacked.pack(&mut packed2, None);
    assert_eq!(packed, packed2);
}

#[test]
fn polygon_texture_coordinate_rotation_points_default() {
    let positions = Cartesian3::from_degrees_array(
        &[-72.0, 40.0, -70.0, 35.0, -75.0, 30.0],
        None,
        None,
    );
    let polygon = PolygonGeometry::new(
        positions,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    assert_eq!(
        polygon.texture_coordinate_rotation_points(),
        [0.0, 0.0, 0.0, 1.0, 1.0, 0.0]
    );
}

// --- PolygonGeometry.computeRectangleFromPositions (PolygonGeometrySpec.js) ---

#[test]
fn compute_rectangle_from_positions_with_less_than_three_positions() {
    let positions = Cartesian3::from_degrees_array(&[0.0, 0.0, 1.0, 1.0], None, None);
    let rectangle = PolygonGeometry::compute_rectangle_from_positions(
        &positions,
        None,
        None,
        None,
    );
    assert_eq!(rectangle, Rectangle::default());
}

#[test]
fn compute_rectangle_from_positions_rhumb_square() {
    // Rhumb edges never exceed the endpoint latitudes, so the rectangle is
    // exactly the corner bounds.
    let positions = Cartesian3::from_degrees_array(
        &[0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0],
        None,
        None,
    );
    let rectangle = PolygonGeometry::compute_rectangle_from_positions(
        &positions,
        None,
        Some(ArcType::Rhumb),
        None,
    );
    let degree = CesiumMath::RADIANS_PER_DEGREE;
    assert!(near(rectangle.west, 0.0, CesiumMath::EPSILON10));
    assert!(near(rectangle.east, degree, CesiumMath::EPSILON10));
    assert!(near(rectangle.south, 0.0, CesiumMath::EPSILON10));
    assert!(near(rectangle.north, degree, CesiumMath::EPSILON10));
}

#[test]
fn compute_rectangle_from_positions_geodesic_bulges_north() {
    // Geodesic edges bulge towards the nearer pole, so north must exceed the
    // corner latitudes.
    let positions = Cartesian3::from_degrees_array(
        &[0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0],
        None,
        None,
    );
    let rectangle = PolygonGeometry::compute_rectangle_from_positions(
        &positions,
        None,
        Some(ArcType::Geodesic),
        None,
    );
    let degree = CesiumMath::RADIANS_PER_DEGREE;
    assert!(near(rectangle.west, 0.0, CesiumMath::EPSILON10));
    assert!(near(rectangle.east, degree, CesiumMath::EPSILON10));
    assert!(near(rectangle.south, 0.0, CesiumMath::EPSILON10));
    assert!(rectangle.north >= degree);
}

#[test]
fn compute_rectangle_from_positions_polygon_containing_north_pole() {
    // A ring circling the pole at 80 degrees north (CCW seen from above)
    // contains the north pole; the rectangle must span all longitudes and
    // reach PI/2.
    let positions = Cartesian3::from_degrees_array(
        &[
            0.0, 80.0, 90.0, 80.0, 180.0, 80.0, -90.0, 80.0,
        ],
        None,
        None,
    );
    let rectangle = PolygonGeometry::compute_rectangle_from_positions(
        &positions,
        None,
        Some(ArcType::Rhumb),
        None,
    );
    assert!(near(rectangle.north, CesiumMath::PI_OVER_TWO, CesiumMath::EPSILON10));
    assert!(near(rectangle.east, std::f64::consts::PI, CesiumMath::EPSILON10));
    assert!(near(rectangle.west, -std::f64::consts::PI, CesiumMath::EPSILON10));
}

// --- Stereographic.fromCartesian (StereographicSpec.js) ---

#[test]
fn stereographic_from_cartesian_north_hemisphere() {
    let ellipsoid = Ellipsoid::WGS84;
    let cartesian = Cartesian3::from_radians_new(
        CesiumMath::to_radians(45.0),
        CesiumMath::to_radians(30.0),
        None,
        Some(ellipsoid.radii_squared()),
    );
    let polar = Stereographic::from_cartesian(&cartesian, None);
    assert!(polar.is_north_pole());
    assert!(near(polar.longitude(), CesiumMath::to_radians(45.0), CesiumMath::EPSILON10));
    assert!(near(
        polar.get_latitude(Some(&ellipsoid)),
        CesiumMath::to_radians(30.0),
        CesiumMath::EPSILON7
    ));
}

#[test]
fn stereographic_from_cartesian_south_hemisphere() {
    let ellipsoid = Ellipsoid::WGS84;
    let cartesian = Cartesian3::from_radians_new(
        CesiumMath::to_radians(-90.0),
        CesiumMath::to_radians(-40.0),
        None,
        Some(ellipsoid.radii_squared()),
    );
    let polar = Stereographic::from_cartesian(&cartesian, None);
    assert!(!polar.is_north_pole());
    assert!(near(polar.longitude(), CesiumMath::to_radians(-90.0), CesiumMath::EPSILON10));
    assert!(near(
        polar.get_latitude(Some(&ellipsoid)),
        CesiumMath::to_radians(-40.0),
        CesiumMath::EPSILON7
    ));
}

// --- PolylineGeometry / RectangleOutlineGeometry / FrustumGeometry smoke ---

#[test]
fn polyline_pack_unpack_roundtrip() {
    let positions = Cartesian3::from_degrees_array(&[0.0, 0.0, 1.0, 1.0], None, None);
    let polyline = PolylineGeometry::new(positions, None, None, None, None, None, None);
    let mut packed = vec![0.0f64; polyline.packed_length()];
    polyline.pack(&mut packed, None);
    let unpacked = PolylineGeometry::unpack(&packed, None, None);
    assert_eq!(unpacked.packed_length(), polyline.packed_length());
}

#[test]
fn rectangle_outline_pack_unpack_roundtrip() {
    let rectangle = Rectangle::new(
        -CesiumMath::PI_OVER_TWO,
        -CesiumMath::PI_OVER_FOUR,
        CesiumMath::PI_OVER_TWO,
        CesiumMath::PI_OVER_FOUR,
    );
    let geo = cesium_core::rectangle_outline_geometry::RectangleOutlineGeometry::from_options(
        rectangle,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    let mut packed = vec![
        0.0f64;
        cesium_core::rectangle_outline_geometry::RectangleOutlineGeometry::PACKED_LENGTH
    ];
    geo.pack(&mut packed, None);
    let unpacked =
        cesium_core::rectangle_outline_geometry::RectangleOutlineGeometry::unpack(
            &packed, None, None,
        );
    let mut packed2 = vec![
        0.0f64;
        cesium_core::rectangle_outline_geometry::RectangleOutlineGeometry::PACKED_LENGTH
    ];
    unpacked.pack(&mut packed2, None);
    assert_eq!(packed, packed2);
}

#[test]
fn frustum_geometry_pack_unpack_and_create_geometry() {
    let mut frustum = PerspectiveFrustum::new();
    frustum.near = 1.0;
    frustum.far = 2.0;
    frustum.fov = Some(CesiumMath::to_radians(60.0));
    frustum.aspect_ratio = Some(1.0);

    let fg = cesium_core::frustum_geometry::FrustumGeometry::from_frustum(
        cesium_core::frustum_geometry::FrustumKind::Perspective(frustum),
        Cartesian3::new(0.0, 0.0, 0.0),
        Quaternion::new(0.0, 0.0, 0.0, 1.0),
        None,
        None,
    );
    let mut packed = vec![0.0f64; fg.packed_length()];
    fg.pack(&mut packed, None);
    let unpacked = cesium_core::frustum_geometry::FrustumGeometry::unpack(&packed, None, None);
    assert_eq!(unpacked.packed_length(), fg.packed_length());

    let geometry = cesium_core::frustum_geometry::FrustumGeometry::create_geometry(&fg);
    assert!(geometry.is_some());
}
