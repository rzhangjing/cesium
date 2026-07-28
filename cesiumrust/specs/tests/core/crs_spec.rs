//! CRS (Coordinate Reference Systems) spec - Datum + Projections
//! Tests: Datum constants/derived, HelmertTransform, MolodenskyTransform, DatumConverter,
//!        WebMercator, UTM, PolarStereographic, Equirectangular

use cesium_crs::{
    Datum, DatumConverter, Equirectangular, GeographicCoordinate, HelmertTransform,
    MolodenskyTransform, PolarStereographic, ProjectedCoordinate, Utm, UtmZone, WebMercator,
    get_helmert_transform, transform_ecef,
};
use glam::DVec3;

const TOL: f64 = 1e-6;

// ═══════════════════════════════════════════════════════════════════════════════
// Datum
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn datum_wgs84_constants() {
    let d = Datum::WGS84;
    assert_eq!(d.semi_major_axis, 6378137.0);
    assert!((d.inverse_flattening - 298.257223563).abs() < 1e-12);
    assert!((d.flattening() - 1.0 / 298.257223563).abs() < 1e-15);
}

#[test]
fn datum_semi_minor_axis() {
    let b = Datum::WGS84.semi_minor_axis();
    // Known value: 6356752.314245179
    assert!((b - 6356752.314245).abs() < 0.001);
}

#[test]
fn datum_eccentricity_squared() {
    let e2 = Datum::WGS84.eccentricity_squared();
    assert!((e2 - 0.00669437999014).abs() < 1e-12);
}

#[test]
fn datum_radii_vector() {
    let r = Datum::WGS84.radii();
    assert!((r.x - 6378137.0).abs() < TOL);
    assert!((r.y - 6378137.0).abs() < TOL);
    assert!((r.z - 6356752.314245).abs() < 0.001);
}

#[test]
fn datum_cgcs2000_differs_from_wgs84() {
    let wgs = Datum::WGS84;
    let cgcs = Datum::CGCS2000;
    assert_eq!(wgs.semi_major_axis, cgcs.semi_major_axis);
    // Different inverse flattening
    assert!((wgs.inverse_flattening - cgcs.inverse_flattening).abs() > 1e-6);
}

// ═══════════════════════════════════════════════════════════════════════════════
// HelmertTransform
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn helmert_identity_no_change() {
    let t = HelmertTransform::IDENTITY;
    let ecef = DVec3::new(6378137.0, 1000.0, -2000.0);
    let result = t.apply(ecef);
    assert!((result - ecef).length() < 1e-6);
}

#[test]
fn helmert_translation_only() {
    let t = HelmertTransform {
        dx: 10.0, dy: -20.0, dz: 30.0,
        rx: 0.0, ry: 0.0, rz: 0.0, ds: 0.0,
    };
    let result = t.apply(DVec3::ZERO);
    assert!((result.x - 10.0).abs() < TOL);
    assert!((result.y - (-20.0)).abs() < TOL);
    assert!((result.z - 30.0).abs() < TOL);
}

#[test]
fn helmert_inverse_roundtrip() {
    let t = HelmertTransform::WGS84_TO_NAD83;
    let inv = t.inverse();
    let ecef = DVec3::new(1234567.0, -4567890.0, 4000000.0);
    let transformed = t.apply(ecef);
    let recovered = inv.apply(transformed);
    assert!((recovered - ecef).length() < 0.01);
}

#[test]
fn helmert_wgs84_to_cgcs2000_is_identity() {
    let t = HelmertTransform::WGS84_TO_CGCS2000;
    let ecef = DVec3::new(6378137.0, 0.0, 0.0);
    let result = t.apply(ecef);
    assert!((result - ecef).length() < 1e-6);
}

#[test]
fn helmert_scale_factor() {
    let t = HelmertTransform {
        dx: 0.0, dy: 0.0, dz: 0.0,
        rx: 0.0, ry: 0.0, rz: 0.0,
        ds: 1.0, // 1 ppm
    };
    let ecef = DVec3::new(1000000.0, 0.0, 0.0);
    let result = t.apply(ecef);
    // Should scale by 1 + 1e-6
    assert!((result.x - 1000001.0).abs() < 0.001);
}

// ═══════════════════════════════════════════════════════════════════════════════
// MolodenskyTransform
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn molodensky_simple_translation() {
    let t = MolodenskyTransform { dx: -87.0, dy: -98.0, dz: -121.0 };
    let ecef = DVec3::new(4000000.0, 500000.0, 5000000.0);
    let result = t.apply(ecef);
    assert!((result.x - (4000000.0 - 87.0)).abs() < TOL);
    assert!((result.y - (500000.0 - 98.0)).abs() < TOL);
    assert!((result.z - (5000000.0 - 121.0)).abs() < TOL);
}

// ═══════════════════════════════════════════════════════════════════════════════
// DatumConverter
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn datum_converter_equator_prime_meridian() {
    let conv = DatumConverter::new(Datum::WGS84);
    let ecef = conv.geographic_to_ecef(0.0, 0.0, 0.0);
    assert!((ecef.x - 6378137.0).abs() < 1.0);
    assert!(ecef.y.abs() < 1.0);
    assert!(ecef.z.abs() < 1.0);
}

#[test]
fn datum_converter_north_pole() {
    let conv = DatumConverter::new(Datum::WGS84);
    let ecef = conv.geographic_to_ecef(0.0, std::f64::consts::FRAC_PI_2, 0.0);
    assert!(ecef.x.abs() < 1.0);
    assert!(ecef.y.abs() < 1.0);
    assert!((ecef.z - Datum::WGS84.semi_minor_axis()).abs() < 1.0);
}

#[test]
fn datum_converter_roundtrip() {
    let conv = DatumConverter::new(Datum::WGS84);
    let lon = 2.0_f64.to_radians();
    let lat = 49.0_f64.to_radians();
    let h = 150.0;
    let ecef = conv.geographic_to_ecef(lon, lat, h);
    let (lon2, lat2, h2) = conv.ecef_to_geographic(ecef);
    assert!((lon2 - lon).abs() < 1e-10);
    assert!((lat2 - lat).abs() < 1e-10);
    assert!((h2 - h).abs() < 0.01);
}

#[test]
fn datum_converter_southern_hemisphere() {
    let conv = DatumConverter::new(Datum::WGS84);
    let lon = (-58.0_f64).to_radians(); // Buenos Aires
    let lat = (-34.6_f64).to_radians();
    let ecef = conv.geographic_to_ecef(lon, lat, 25.0);
    let (lon2, lat2, h2) = conv.ecef_to_geographic(ecef);
    assert!((lon2 - lon).abs() < 1e-10);
    assert!((lat2 - lat).abs() < 1e-10);
    assert!((h2 - 25.0).abs() < 0.01);
}

// ═══════════════════════════════════════════════════════════════════════════════
// transform_ecef / get_helmert_transform
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn transform_ecef_same_datum_identity() {
    let ecef = DVec3::new(1000.0, 2000.0, 3000.0);
    let result = transform_ecef(ecef, &Datum::WGS84, &Datum::WGS84);
    assert!((result - ecef).length() < 1e-10);
}

#[test]
fn get_helmert_known_pair() {
    assert!(get_helmert_transform(&Datum::WGS84, &Datum::CGCS2000).is_some());
    assert!(get_helmert_transform(&Datum::WGS84, &Datum::NAD83).is_some());
    assert!(get_helmert_transform(&Datum::AIRY_1830, &Datum::CGCS2000).is_none());
}

// ═══════════════════════════════════════════════════════════════════════════════
// WebMercator
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn web_mercator_origin() {
    let wm = WebMercator::default();
    let geo = GeographicCoordinate::from_radians(0.0, 0.0);
    let proj = wm.project(&geo).unwrap();
    assert!(proj.x.abs() < TOL);
    assert!(proj.y.abs() < TOL);
}

#[test]
fn web_mercator_roundtrip() {
    let wm = WebMercator::default();
    let original = GeographicCoordinate::from_degrees(-73.9857, 40.7484);
    let proj = wm.project(&original).unwrap();
    let recovered = wm.unproject(&proj);
    assert!((recovered.longitude - original.longitude).abs() < TOL);
    assert!((recovered.latitude - original.latitude).abs() < TOL);
}

#[test]
fn web_mercator_max_latitude_clip() {
    let wm = WebMercator::default();
    let at_max = GeographicCoordinate::from_radians(0.0, WebMercator::MAX_LATITUDE);
    assert!(wm.project(&at_max).is_some());
    let beyond = GeographicCoordinate::from_radians(0.0, WebMercator::MAX_LATITUDE + 0.01);
    assert!(wm.project(&beyond).is_none());
}

#[test]
fn web_mercator_degrees_api() {
    let wm = WebMercator::default();
    let proj = wm.project_degrees(0.0, 51.5).unwrap();
    assert!(proj.x.abs() < 1.0);
    assert!(proj.y > 6_000_000.0 && proj.y < 7_000_000.0);
    let (lon, lat) = wm.unproject_to_degrees(&proj);
    assert!((lon - 0.0).abs() < TOL);
    assert!((lat - 51.5).abs() < TOL);
}

// ═══════════════════════════════════════════════════════════════════════════════
// UTM
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn utm_zone_from_lon_lat() {
    let nyc = UtmZone::from_lon_lat(-73.9857, 40.7484);
    assert_eq!(nyc.zone, 18);
    assert!(nyc.north);

    let sydney = UtmZone::from_lon_lat(151.2093, -33.8688);
    assert_eq!(sydney.zone, 56);
    assert!(!sydney.north);
}

#[test]
fn utm_zone_epsg_code() {
    let north = UtmZone { zone: 33, north: true };
    assert_eq!(north.epsg_code(), 32633);
    let south = UtmZone { zone: 33, north: false };
    assert_eq!(south.epsg_code(), 32733);
}

#[test]
fn utm_zone_central_meridian() {
    let z1 = UtmZone { zone: 1, north: true };
    assert!((z1.central_meridian().to_degrees() - (-177.0)).abs() < TOL);
    let z31 = UtmZone { zone: 31, north: true };
    assert!((z31.central_meridian().to_degrees() - 3.0).abs() < TOL);
}

#[test]
fn utm_project_roundtrip() {
    let utm = Utm::default();
    let original = GeographicCoordinate::from_degrees(-73.9857, 40.7484);
    let (proj, zone) = utm.project(&original, None);
    let recovered = utm.unproject(&proj, &zone);
    assert!((recovered.longitude - original.longitude).abs() < TOL);
    assert!((recovered.latitude - original.latitude).abs() < TOL);
}

#[test]
fn utm_false_easting_at_central_meridian() {
    let utm = Utm::default();
    let geo = GeographicCoordinate::from_degrees(3.0, 0.0);
    let (proj, _) = utm.project(&geo, Some(31));
    assert!((proj.x - 500000.0).abs() < 1.0);
    assert!(proj.y.abs() < 1.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// PolarStereographic
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn polar_stereographic_north_pole_origin() {
    let ps = PolarStereographic::default();
    let geo = GeographicCoordinate::from_radians(0.0, std::f64::consts::FRAC_PI_2);
    let proj = ps.project(&geo);
    assert!(proj.x.abs() < 1.0);
    assert!(proj.y.abs() < 1.0);
}

#[test]
fn polar_stereographic_roundtrip() {
    let ps = PolarStereographic::default();
    let original = GeographicCoordinate::from_degrees(45.0, 85.0);
    let proj = ps.project(&original);
    let recovered = ps.unproject(&proj);
    assert!((recovered.longitude - original.longitude).abs() < 0.01_f64.to_radians());
    assert!((recovered.latitude - original.latitude).abs() < 0.01_f64.to_radians());
}

// ═══════════════════════════════════════════════════════════════════════════════
// Equirectangular
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn equirectangular_origin() {
    let eq = Equirectangular::default();
    let geo = GeographicCoordinate::from_radians(0.0, 0.0);
    let proj = eq.project(&geo);
    assert!(proj.x.abs() < TOL);
    assert!(proj.y.abs() < TOL);
}

#[test]
fn equirectangular_roundtrip() {
    let eq = Equirectangular::default();
    let original = GeographicCoordinate::from_degrees(120.0, 30.0);
    let proj = eq.project(&original);
    let recovered = eq.unproject(&proj);
    assert!((recovered.longitude - original.longitude).abs() < TOL);
    assert!((recovered.latitude - original.latitude).abs() < TOL);
}

#[test]
fn equirectangular_linear_scaling() {
    let eq = Equirectangular::default();
    let geo = GeographicCoordinate::from_radians(1.0, 0.5);
    let proj = eq.project(&geo);
    assert!((proj.x - eq.radius * 1.0).abs() < TOL);
    assert!((proj.y - eq.radius * 0.5).abs() < TOL);
}

// ═══════════════════════════════════════════════════════════════════════════════
// ProjectedCoordinate / GeographicCoordinate helpers
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn projected_coordinate_vec2_conversion() {
    let pc = ProjectedCoordinate::new(100.0, 200.0);
    let v = pc.to_vec2();
    assert!((v.x - 100.0).abs() < TOL);
    assert!((v.y - 200.0).abs() < TOL);
    let back = ProjectedCoordinate::from_vec2(v);
    assert_eq!(back, pc);
}

#[test]
fn geographic_coordinate_degrees() {
    let geo = GeographicCoordinate::from_degrees(180.0, 90.0);
    assert!((geo.longitude - std::f64::consts::PI).abs() < TOL);
    assert!((geo.latitude - std::f64::consts::FRAC_PI_2).abs() < TOL);
    assert!((geo.longitude_degrees() - 180.0).abs() < TOL);
    assert!((geo.latitude_degrees() - 90.0).abs() < TOL);
}
