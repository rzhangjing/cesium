//! Port of packages/engine/Specs/Core/WebMercatorProjectionSpec.js

use std::f64::consts::PI;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::math::CesiumMath;
use cesium_core::web_mercator_projection::WebMercatorProjection;

fn approx_eq_eps(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() < eps
}

fn cartesian3_approx_eq_eps(a: &Cartesian3, b: &Cartesian3, eps: f64) -> bool {
    approx_eq_eps(a.x, b.x, eps) && approx_eq_eps(a.y, b.y, eps) && approx_eq_eps(a.z, b.z, eps)
}

#[test]
fn construct_default() {
    let projection = WebMercatorProjection::new(None);
    assert_eq!(*projection.ellipsoid(), Ellipsoid::WGS84);
}

#[test]
fn construct_with_ellipsoid() {
    let ellipsoid = Ellipsoid::UNIT_SPHERE;
    let projection = WebMercatorProjection::new(Some(ellipsoid.clone()));
    assert_eq!(*projection.ellipsoid(), ellipsoid);
}

#[test]
fn project_zero() {
    let height = 10.0;
    let cartographic = Cartographic::new(0.0, 0.0, height);
    let projection = WebMercatorProjection::new(None);
    let result = projection.project(&cartographic);
    assert!(cartesian3_approx_eq_eps(&result, &Cartesian3::new(0.0, 0.0, height), 1e-10));
}

#[test]
fn project_wgs84() {
    let ellipsoid = Ellipsoid::WGS84;
    let cartographic = Cartographic::new(PI, CesiumMath::PI_OVER_FOUR, 0.0);

    // expected equations from Wolfram MathWorld
    let expected = Cartesian3::new(
        ellipsoid.maximum_radius() * cartographic.longitude,
        ellipsoid.maximum_radius()
            * (PI / 4.0 + cartographic.latitude / 2.0).tan().ln(),
        0.0,
    );

    let projection = WebMercatorProjection::new(Some(ellipsoid));
    let result = projection.project(&cartographic);
    assert!(cartesian3_approx_eq_eps(&result, &expected, CesiumMath::EPSILON8));
}

#[test]
fn project_unit_sphere() {
    let ellipsoid = Ellipsoid::UNIT_SPHERE;
    let cartographic = Cartographic::new(-PI, CesiumMath::PI_OVER_FOUR, 0.0);

    let expected = Cartesian3::new(
        ellipsoid.maximum_radius() * cartographic.longitude,
        ellipsoid.maximum_radius()
            * (PI / 4.0 + cartographic.latitude / 2.0).tan().ln(),
        0.0,
    );

    let projection = WebMercatorProjection::new(Some(ellipsoid));
    let result = projection.project(&cartographic);
    assert!(cartesian3_approx_eq_eps(&result, &expected, CesiumMath::EPSILON15));
}

#[test]
fn project_with_result() {
    let ellipsoid = Ellipsoid::WGS84;
    let cartographic = Cartographic::new(PI, CesiumMath::PI_OVER_FOUR, 0.0);

    let expected = Cartesian3::new(
        ellipsoid.maximum_radius() * cartographic.longitude,
        ellipsoid.maximum_radius()
            * (PI / 4.0 + cartographic.latitude / 2.0).tan().ln(),
        0.0,
    );

    let projection = WebMercatorProjection::new(Some(ellipsoid));
    let mut result = Cartesian3::new(0.0, 0.0, 0.0);
    projection.project_into(&cartographic, &mut result);
    assert!(cartesian3_approx_eq_eps(&result, &expected, CesiumMath::EPSILON8));
}

#[test]
fn unproject_roundtrip() {
    let cartographic = Cartographic::new(CesiumMath::PI_OVER_TWO, CesiumMath::PI_OVER_FOUR, 12.0);
    let projection = WebMercatorProjection::new(None);
    let projected = projection.project(&cartographic);
    let unprojected = projection.unproject(&projected);
    assert!(approx_eq_eps(unprojected.longitude, cartographic.longitude, CesiumMath::EPSILON14));
    assert!(approx_eq_eps(unprojected.latitude, cartographic.latitude, CesiumMath::EPSILON14));
    assert!(approx_eq_eps(unprojected.height, cartographic.height, CesiumMath::EPSILON14));
}

#[test]
fn unproject_with_result() {
    let cartographic = Cartographic::new(CesiumMath::PI_OVER_TWO, CesiumMath::PI_OVER_FOUR, 12.0);
    let projection = WebMercatorProjection::new(None);
    let projected = projection.project(&cartographic);
    let mut result = Cartographic::default();
    projection.unproject_into(&projected, &mut result);
    assert!(approx_eq_eps(result.longitude, cartographic.longitude, CesiumMath::EPSILON14));
    assert!(approx_eq_eps(result.latitude, cartographic.latitude, CesiumMath::EPSILON14));
    assert!(approx_eq_eps(result.height, cartographic.height, CesiumMath::EPSILON14));
}

#[test]
fn unproject_corners() {
    let projection = WebMercatorProjection::new(None);

    // Southwest
    let sw = projection.unproject(&Cartesian3::new(-20037508.342787, -20037508.342787, 0.0));
    assert!(approx_eq_eps(sw.longitude, -PI, CesiumMath::EPSILON12));
    assert!(approx_eq_eps(sw.latitude, CesiumMath::to_radians(-85.05112878), CesiumMath::EPSILON11));

    // Southeast
    let se = projection.unproject(&Cartesian3::new(20037508.342787, -20037508.342787, 0.0));
    assert!(approx_eq_eps(se.longitude, PI, CesiumMath::EPSILON12));
    assert!(approx_eq_eps(se.latitude, CesiumMath::to_radians(-85.05112878), CesiumMath::EPSILON11));

    // Northeast
    let ne = projection.unproject(&Cartesian3::new(20037508.342787, 20037508.342787, 0.0));
    assert!(approx_eq_eps(ne.longitude, PI, CesiumMath::EPSILON12));
    assert!(approx_eq_eps(ne.latitude, CesiumMath::to_radians(85.05112878), CesiumMath::EPSILON11));

    // Northwest
    let nw = projection.unproject(&Cartesian3::new(-20037508.342787, 20037508.342787, 0.0));
    assert!(approx_eq_eps(nw.longitude, -PI, CesiumMath::EPSILON12));
    assert!(approx_eq_eps(nw.latitude, CesiumMath::to_radians(85.05112878), CesiumMath::EPSILON11));
}

#[test]
fn project_corners() {
    let max_latitude = WebMercatorProjection::MAXIMUM_LATITUDE;
    let projection = WebMercatorProjection::new(None);

    // Southwest
    let sw = projection.project(&Cartographic::new(-PI, -max_latitude, 0.0));
    assert!(approx_eq_eps(sw.x, -20037508.342787, CesiumMath::EPSILON3));
    assert!(approx_eq_eps(sw.y, -20037508.342787, CesiumMath::EPSILON3));

    // Southeast
    let se = projection.project(&Cartographic::new(PI, -max_latitude, 0.0));
    assert!(approx_eq_eps(se.x, 20037508.342787, CesiumMath::EPSILON3));
    assert!(approx_eq_eps(se.y, -20037508.342787, CesiumMath::EPSILON3));

    // Northeast
    let ne = projection.project(&Cartographic::new(PI, max_latitude, 0.0));
    assert!(approx_eq_eps(ne.x, 20037508.342787, CesiumMath::EPSILON3));
    assert!(approx_eq_eps(ne.y, 20037508.342787, CesiumMath::EPSILON3));

    // Northwest
    let nw = projection.project(&Cartographic::new(-PI, max_latitude, 0.0));
    assert!(approx_eq_eps(nw.x, -20037508.342787, CesiumMath::EPSILON3));
    assert!(approx_eq_eps(nw.y, 20037508.342787, CesiumMath::EPSILON3));
}

#[test]
fn projected_y_clamped_to_valid_latitude_range() {
    let projection = WebMercatorProjection::new(None);

    let south_pole = projection.project(&Cartographic::new(0.0, -CesiumMath::PI_OVER_TWO, 0.0));
    let south_limit = projection.project(&Cartographic::new(0.0, -WebMercatorProjection::MAXIMUM_LATITUDE, 0.0));
    assert!((south_pole.y - south_limit.y).abs() < 1e-10);

    let north_pole = projection.project(&Cartographic::new(0.0, CesiumMath::PI_OVER_TWO, 0.0));
    let north_limit = projection.project(&Cartographic::new(0.0, WebMercatorProjection::MAXIMUM_LATITUDE, 0.0));
    assert!((north_pole.y - north_limit.y).abs() < 1e-10);
}

#[test]
fn mercator_angle_to_geodetic_latitude() {
    let lat = WebMercatorProjection::mercator_angle_to_geodetic_latitude(0.0);
    assert!(approx_eq_eps(lat, 0.0, 1e-15));
}

#[test]
fn geodetic_latitude_to_mercator_angle() {
    let angle = WebMercatorProjection::geodetic_latitude_to_mercator_angle(0.0);
    assert!(approx_eq_eps(angle, 0.0, 1e-15));
}
