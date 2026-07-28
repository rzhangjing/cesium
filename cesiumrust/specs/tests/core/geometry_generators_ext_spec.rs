//! Geometry generators (Ellipse/Corridor/Wall/Polyline/CoplanarPolygon/Frustum)
//! + Simon1994PlanetaryPositions + IauOrientation → Rust integration tests.
//!
//! Maps to CesiumJS:
//! - Core/EllipseGeometry.js, Core/CorridorGeometry.js, Core/WallGeometry.js
//! - Core/PolylineGeometry.js, Core/CoplanarPolygonGeometry.js
//! - Core/Simon1994PlanetaryPositions.js, Core/Iau2000Orientation.js
//!
//! A-class tests: vertex/index counts, bounding sphere, planetary positions.
//! C-class omitted: throws, pack/unpack, offsetAttribute (GPU-specific).

use cesium_geospatial::geometry::{
    coplanar_polygon_geometry, corridor_geometry, corridor_outline_geometry,
    ellipse_geometry, ellipse_outline_geometry, frustum_geometry,
    ground_polyline_geometry, polyline_geometry, wall_geometry, wall_outline_geometry,
    CoplanarPolygonOptions, CornerType, CorridorOptions, EllipseOptions, FrustumDef,
    GroundPolylineOptions, PolylineOptions, WallOptions,
};
use cesium_geospatial::geometry::VertexFormat;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::frustum::PerspectiveFrustum;
use cesium_geospatial::simon1994_planetary_positions::{
    compute_moon_position_in_earth_inertial_frame,
    compute_sun_position_in_earth_inertial_frame,
};
use cesium_geospatial::iau_orientation::compute_moon;
use cesium_time::julian_date::{JulianDate, TimeStandard};
use glam::{DQuat, DVec3};

fn wgs84() -> Ellipsoid {
    Ellipsoid::WGS84
}

fn equator_center() -> DVec3 {
    // Cartesian3.fromDegrees(0, 0) on WGS84
    wgs84().cartographic_to_cartesian(
        &cesium_geospatial::cartographic::Cartographic::from_radians(0.0, 0.0, 0.0),
    )
}

fn from_degrees(lon: f64, lat: f64, h: f64) -> DVec3 {
    wgs84().cartographic_to_cartesian(
        &cesium_geospatial::cartographic::Cartographic::from_degrees(lon, lat, h),
    )
}

// === Simon1994PlanetaryPositions ===

#[test]
fn simon1994_sun_position_j2000() {
    // CesiumJS: new JulianDate(2451545, 0, TimeStandard.TAI)
    let date = JulianDate::with_time_standard(2451545.0, 0.0, TimeStandard::TAI);
    let sun = compute_sun_position_in_earth_inertial_frame(&date);
    // Expected from STK Components (relative epsilon ~1e-11)
    let eps = 1.0e-11;
    assert!((sun.x - 26500268539.790234).abs() < eps * 26500268539.0_f64.abs().max(1.0));
    assert!((sun.y - (-132756447253.27325)).abs() < eps * 132756447253.0_f64.abs().max(1.0));
    assert!((sun.z - (-57556483362.533806)).abs() < eps * 57556483362.0_f64.abs().max(1.0));
}

#[test]
fn simon1994_sun_position_2013() {
    let date = JulianDate::with_time_standard(2456401.5, 0.0, TimeStandard::TAI);
    let sun = compute_sun_position_in_earth_inertial_frame(&date);
    let eps = 1.0e-11;
    assert!((sun.x - 131512388940.33589).abs() < eps * 131512388940.0_f64.abs().max(1.0));
    assert!((sun.y - 66661342667.949928).abs() < eps * 66661342667.0_f64.abs().max(1.0));
    assert!((sun.z - 28897975607.905258).abs() < eps * 28897975607.0_f64.abs().max(1.0));
}

#[test]
fn simon1994_moon_position_j2000() {
    let date = JulianDate::with_time_standard(2451545.0, 0.0, TimeStandard::TAI);
    let moon = compute_moon_position_in_earth_inertial_frame(&date);
    let eps = 1.0e-10;
    assert!((moon.x - (-291632410.61232185)).abs() < eps * 291632410.0_f64.abs().max(1.0));
    assert!((moon.y - (-266522146.36821631)).abs() < eps * 266522146.0_f64.abs().max(1.0));
    assert!((moon.z - (-75994518.081043154)).abs() < eps * 75994518.0_f64.abs().max(1.0));
}

#[test]
fn simon1994_moon_position_2013() {
    let date = JulianDate::with_time_standard(2456401.5, 0.0, TimeStandard::TAI);
    let moon = compute_moon_position_in_earth_inertial_frame(&date);
    let eps = 1.0e-10;
    assert!((moon.x - (-223792974.4736526)).abs() < eps * 223792974.0_f64.abs().max(1.0));
    assert!((moon.y - 315772435.34490639).abs() < eps * 315772435.0_f64.abs().max(1.0));
    assert!((moon.z - 97913011.236112773).abs() < eps * 97913011.0_f64.abs().max(1.0));
}

// === IauOrientation ===

#[test]
fn iau_compute_moon_j2000() {
    let date = JulianDate::with_time_standard(2451545.0, 0.0, TimeStandard::TAI);
    let params = compute_moon(&date);
    // Right ascension and declination should be finite radians
    assert!(params.right_ascension.is_finite());
    assert!(params.declination.is_finite());
    assert!(params.rotation.is_finite());
    assert!(params.rotation_rate.is_finite());
    // Rotation rate should be positive (Moon rotates)
    assert!(params.rotation_rate > 0.0);
}

// === EllipseGeometry ===

#[test]
fn ellipse_computes_positions() {
    let opts = EllipseOptions {
        center: equator_center(),
        semi_major_axis: 1.0,
        semi_minor_axis: 1.0,
        granularity: 0.1,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = ellipse_geometry(&opts, VertexFormat::POSITION_ONLY);
    // CesiumJS: 16 vertices (rows 1+4+6+4+1), 22 triangles (rows 3+8+8+3)
    assert_eq!(geo.positions.len(), 16);
    assert_eq!(geo.indices.len(), 22 * 3);
    // Bounding sphere radius ~1 (small ellipse on surface)
    assert!((geo.bounding_sphere.radius - 1.0).abs() < 0.01);
}

#[test]
fn ellipse_all_vertex_attributes() {
    let opts = EllipseOptions {
        center: equator_center(),
        semi_major_axis: 1.0,
        semi_minor_axis: 1.0,
        granularity: 0.1,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = ellipse_geometry(&opts, VertexFormat::ALL);
    let nv = geo.positions.len();
    assert_eq!(nv, 16);
    assert!(geo.normals.is_some());
    assert_eq!(geo.normals.as_ref().unwrap().len(), nv);
    assert!(geo.tex_coords.is_some());
    assert_eq!(geo.tex_coords.as_ref().unwrap().len(), nv);
}

#[test]
fn ellipse_outline_computes_positions() {
    let opts = EllipseOptions {
        center: equator_center(),
        semi_major_axis: 1.0,
        semi_minor_axis: 1.0,
        granularity: 0.1,
        ellipsoid: wgs84(),
        ..Default::default()
    };
    let geo = ellipse_outline_geometry(&opts);
    // Outline: ring of line segments
    assert!(geo.positions.len() >= 8);
    assert!(geo.indices.len() >= 8);
    // Indices are pairs (lines)
    assert_eq!(geo.indices.len() % 2, 0);
}

// === CorridorGeometry ===

#[test]
fn corridor_computes_positions() {
    let e = wgs84();
    let p0 = from_degrees(0.0, 0.0, 0.0);
    let p1 = from_degrees(1.0, 0.0, 0.0);
    let opts = CorridorOptions {
        positions: vec![p0, p1],
        width: 100000.0,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: e,
        ..Default::default()
    };
    let geo = corridor_geometry(&opts, VertexFormat::POSITION_ONLY);
    assert!(geo.positions.len() >= 4);
    assert!(geo.indices.len() >= 6);
    assert!(geo.bounding_sphere.radius > 0.0);
}

#[test]
fn corridor_outline_computes_positions() {
    let e = wgs84();
    let p0 = from_degrees(0.0, 0.0, 0.0);
    let p1 = from_degrees(1.0, 0.0, 0.0);
    let opts = CorridorOptions {
        positions: vec![p0, p1],
        width: 100000.0,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: e,
        ..Default::default()
    };
    let geo = corridor_outline_geometry(&opts);
    assert!(geo.positions.len() >= 4);
    assert!(geo.indices.len() >= 4);
}

#[test]
fn corridor_corner_types() {
    let e = wgs84();
    let p0 = from_degrees(0.0, 0.0, 0.0);
    let p1 = from_degrees(1.0, 0.0, 0.0);
    let p2 = from_degrees(1.0, 1.0, 0.0);

    for ct in [CornerType::Rounded, CornerType::Mitered, CornerType::Beveled] {
        let opts = CorridorOptions {
            positions: vec![p0, p1, p2],
            width: 100000.0,
            corner_type: ct,
            granularity: std::f64::consts::PI / 180.0,
            ellipsoid: e,
            ..Default::default()
        };
        let geo = corridor_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert!(geo.positions.len() >= 4, "corner_type {:?} should produce vertices", ct);
        assert!(geo.indices.len() >= 6, "corner_type {:?} should produce triangles", ct);
    }
}

// === WallGeometry ===

#[test]
fn wall_computes_positions() {
    let e = wgs84();
    let positions: Vec<DVec3> = (0..4).map(|i| from_degrees(i as f64, 0.0, 0.0)).collect();

    let opts = WallOptions {
        positions,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: e,
        ..Default::default()
    };
    let geo = wall_geometry(&opts, VertexFormat::POSITION_ONLY);
    // Wall: 2 rows (top+bottom) * num_arc_points
    assert!(geo.positions.len() >= 8);
    assert!(geo.indices.len() >= 6);
    assert!(geo.bounding_sphere.radius > 0.0);
}

#[test]
fn wall_with_min_max_heights() {
    let e = wgs84();
    let positions: Vec<DVec3> = (0..3).map(|i| from_degrees(i as f64, 0.0, 0.0)).collect();

    let opts = WallOptions {
        positions: positions.clone(),
        maximum_heights: Some(vec![10000.0, 10000.0, 10000.0]),
        minimum_heights: Some(vec![0.0, 0.0, 0.0]),
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: e,
        ..Default::default()
    };
    let geo = wall_geometry(&opts, VertexFormat::POSITION_ONLY);
    assert!(geo.positions.len() >= 6);
    assert!(geo.indices.len() >= 6);
}

#[test]
fn wall_from_constant_heights() {
    let e = wgs84();
    let positions: Vec<DVec3> = (0..3).map(|i| from_degrees(i as f64, 0.0, 0.0)).collect();

    let opts = WallOptions::from_constant_heights(
        positions,
        Some(0.0),
        Some(50000.0),
        e,
    );
    let geo = wall_geometry(&opts, VertexFormat::POSITION_ONLY);
    assert!(geo.positions.len() >= 6);
    assert!(geo.indices.len() >= 6);
}

#[test]
fn wall_outline_computes_positions() {
    let e = wgs84();
    let positions: Vec<DVec3> = (0..3).map(|i| from_degrees(i as f64, 0.0, 0.0)).collect();

    let opts = WallOptions {
        positions,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: e,
        ..Default::default()
    };
    let geo = wall_outline_geometry(&opts);
    assert!(geo.positions.len() >= 4);
    assert!(geo.indices.len() >= 4);
    assert_eq!(geo.indices.len() % 2, 0);
}

// === PolylineGeometry ===

#[test]
fn polyline_computes_positions() {
    let e = wgs84();
    let p0 = from_degrees(0.0, 0.0, 0.0);
    let p1 = from_degrees(1.0, 0.0, 0.0);
    let opts = PolylineOptions {
        positions: vec![p0, p1],
        width: 10000.0,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: e,
    };
    let geo = polyline_geometry(&opts, VertexFormat::POSITION_ONLY);
    // Ribbon: at least 2 segments * 2 vertices per cross-section
    assert!(geo.positions.len() >= 4);
    assert!(geo.indices.len() >= 6);
    assert!(geo.bounding_sphere.radius > 0.0);
}

#[test]
fn polyline_empty_with_less_than_2_positions() {
    let e = wgs84();
    let p0 = from_degrees(0.0, 0.0, 0.0);
    let opts = PolylineOptions {
        positions: vec![p0],
        width: 10000.0,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: e,
    };
    let geo = polyline_geometry(&opts, VertexFormat::POSITION_ONLY);
    assert!(geo.positions.is_empty());
    assert!(geo.indices.is_empty());
}

// === CoplanarPolygonGeometry ===

#[test]
fn coplanar_polygon_triangle() {
    let e = wgs84();
    let p0 = from_degrees(0.0, 0.0, 0.0);
    let p1 = from_degrees(1.0, 0.0, 0.0);
    let p2 = from_degrees(0.5, 1.0, 0.0);
    let opts = CoplanarPolygonOptions {
        positions: vec![p0, p1, p2],
        ellipsoid: e,
        ..Default::default()
    };
    let geo = coplanar_polygon_geometry(&opts, VertexFormat::POSITION_ONLY);
    // Triangle: 3 vertices, 1 triangle (3 indices)
    assert_eq!(geo.positions.len(), 3);
    assert_eq!(geo.indices.len(), 3);
    assert!(geo.bounding_sphere.radius > 0.0);
}

#[test]
fn coplanar_polygon_quad() {
    let e = wgs84();
    let p0 = from_degrees(0.0, 0.0, 0.0);
    let p1 = from_degrees(1.0, 0.0, 0.0);
    let p2 = from_degrees(1.0, 1.0, 0.0);
    let p3 = from_degrees(0.0, 1.0, 0.0);
    let opts = CoplanarPolygonOptions {
        positions: vec![p0, p1, p2, p3],
        ellipsoid: e,
        ..Default::default()
    };
    let geo = coplanar_polygon_geometry(&opts, VertexFormat::POSITION_ONLY);
    // Quad: 4 vertices, 2 triangles (6 indices)
    assert_eq!(geo.positions.len(), 4);
    assert_eq!(geo.indices.len(), 6);
}

// === FrustumGeometry ===

#[test]
fn frustum_geometry_computes_positions() {
    let pf = PerspectiveFrustum::new(
        std::f64::consts::FRAC_PI_4,
        1.0,
        1.0,
        100.0,
    );
    let def = FrustumDef::Perspective(pf);
    let geo = frustum_geometry(&def, DVec3::ZERO, DQuat::IDENTITY, VertexFormat::POSITION_ONLY);
    // Frustum: 6 planes * 4 vertices = 24
    assert!(geo.positions.len() >= 8);
    assert!(geo.indices.len() >= 12);
    assert!(geo.bounding_sphere.radius > 0.0);
}

// === GroundPolylineGeometry ===

#[test]
fn ground_polyline_computes_positions() {
    let e = wgs84();
    let p0 = from_degrees(0.0, 0.0, 0.0);
    let p1 = from_degrees(1.0, 0.0, 0.0);
    let opts = GroundPolylineOptions {
        positions: vec![p0, p1],
        width: 5000.0,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: e,
        ..Default::default()
    };
    let geo = ground_polyline_geometry(&opts, VertexFormat::POSITION_ONLY);
    assert!(geo.positions.len() >= 4);
    assert!(geo.indices.len() >= 6);
}
