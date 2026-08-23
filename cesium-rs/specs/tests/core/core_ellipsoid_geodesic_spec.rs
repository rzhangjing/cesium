use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::ellipsoid_geodesic::EllipsoidGeodesic;
use cesium_core::math::CesiumMath;

#[test]
fn computes_surface_distance() {
    let start = Cartographic::new(0.0, 0.0, 0.0);
    let end = Cartographic::new(std::f64::consts::FRAC_PI_2, 0.0, 0.0);
    let geo = EllipsoidGeodesic::new(
        Some(start),
        Some(end),
        None,
        None,
        Some(Ellipsoid::WGS84),
    );
    // Quarter of the Earth's circumference ≈ pi/2 * a ≈ 10,018 km
    assert!(geo.surface_distance() > 0.0);
    // Should be roughly a quarter of Earth's circumference
    let expected_approx = std::f64::consts::FRAC_PI_2 * Ellipsoid::WGS84.maximum_radius();
    assert!(
        (geo.surface_distance() - expected_approx).abs() / expected_approx < 0.01,
        "surface_distance {} vs expected ~{}",
        geo.surface_distance(),
        expected_approx
    );
}

#[test]
fn interpolate_at_start() {
    let start = Cartographic::new(0.0, 0.0, 0.0);
    let end = Cartographic::new(std::f64::consts::FRAC_PI_2, 0.0, 0.0);
    let geo = EllipsoidGeodesic::new(
        Some(start),
        Some(end),
        None,
        None,
        Some(Ellipsoid::WGS84),
    );
    let pos = geo.interpolate_using_fraction(0.0);
    assert!((pos.longitude - start.longitude).abs() < CesiumMath::EPSILON10);
    assert!((pos.latitude - start.latitude).abs() < CesiumMath::EPSILON10);
}

#[test]
fn interpolate_at_end() {
    let start = Cartographic::new(0.0, 0.0, 0.0);
    let end = Cartographic::new(std::f64::consts::FRAC_PI_2, 0.0, 0.0);
    let geo = EllipsoidGeodesic::new(
        Some(start),
        Some(end),
        None,
        None,
        Some(Ellipsoid::WGS84),
    );
    let pos = geo.interpolate_using_fraction(1.0);
    assert!((pos.longitude - end.longitude).abs() < CesiumMath::EPSILON10);
    assert!((pos.latitude - end.latitude).abs() < CesiumMath::EPSILON10);
}

#[test]
fn interpolate_at_midpoint() {
    let start = Cartographic::new(0.0, 0.0, 0.0);
    let end = Cartographic::new(std::f64::consts::FRAC_PI_2, 0.0, 0.0);
    let geo = EllipsoidGeodesic::new(
        Some(start),
        Some(end),
        None,
        None,
        Some(Ellipsoid::WGS84),
    );
    let pos = geo.interpolate_using_fraction(0.5);
    let expected_lon = std::f64::consts::FRAC_PI_4;
    assert!((pos.longitude - expected_lon).abs() < CesiumMath::EPSILON10);
}

#[test]
fn same_start_and_end_has_zero_distance() {
    let start = Cartographic::new(1.0, 2.0, 0.0);
    let geo = EllipsoidGeodesic::new(
        Some(start.clone()),
        Some(start),
        None,
        None,
        Some(Ellipsoid::WGS84),
    );
    assert!(geo.surface_distance().abs() < CesiumMath::EPSILON10);
}
