//! Port of packages/engine/Specs/Core/GeographicProjectionSpec.js

use std::f64::consts::PI;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::geographic_projection::GeographicProjection;
use cesium_core::math::CesiumMath;

const EPSILON10: f64 = 1e-10;

fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < EPSILON10
}

fn cartesian3_approx_eq(a: &Cartesian3, b: &Cartesian3) -> bool {
    approx_eq(a.x, b.x) && approx_eq(a.y, b.y) && approx_eq(a.z, b.z)
}

#[test]
fn construct_default() {
    let projection = GeographicProjection::new(None);
    assert_eq!(*projection.ellipsoid(), Ellipsoid::WGS84);
}

#[test]
fn construct_with_ellipsoid() {
    let ellipsoid = Ellipsoid::UNIT_SPHERE;
    let projection = GeographicProjection::new(Some(ellipsoid.clone()));
    assert_eq!(*projection.ellipsoid(), ellipsoid);
}

#[test]
fn project_zero() {
    let height = 10.0;
    let cartographic = Cartographic::new(0.0, 0.0, height);
    let projection = GeographicProjection::new(None);
    let result = projection.project(&cartographic);
    assert!(cartesian3_approx_eq(&result, &Cartesian3::new(0.0, 0.0, height)));
}

#[test]
fn project_wgs84() {
    let ellipsoid = Ellipsoid::WGS84;
    let cartographic = Cartographic::new(PI, CesiumMath::PI_OVER_TWO, 0.0);
    let expected = Cartesian3::new(
        PI * ellipsoid.radii().x,
        CesiumMath::PI_OVER_TWO * ellipsoid.radii().x,
        0.0,
    );
    let projection = GeographicProjection::new(Some(ellipsoid));
    let result = projection.project(&cartographic);
    assert!(cartesian3_approx_eq(&result, &expected));
}

#[test]
fn project_unit_sphere() {
    let ellipsoid = Ellipsoid::UNIT_SPHERE;
    let cartographic = Cartographic::new(-PI, CesiumMath::PI_OVER_TWO, 0.0);
    let expected = Cartesian3::new(-PI, CesiumMath::PI_OVER_TWO, 0.0);
    let projection = GeographicProjection::new(Some(ellipsoid));
    let result = projection.project(&cartographic);
    assert!(cartesian3_approx_eq(&result, &expected));
}

#[test]
fn project_with_result() {
    let ellipsoid = Ellipsoid::WGS84;
    let cartographic = Cartographic::new(PI, CesiumMath::PI_OVER_TWO, 0.0);
    let expected = Cartesian3::new(
        PI * ellipsoid.radii().x,
        CesiumMath::PI_OVER_TWO * ellipsoid.radii().x,
        0.0,
    );
    let projection = GeographicProjection::new(Some(ellipsoid));
    let mut result = Cartesian3::new(0.0, 0.0, 0.0);
    projection.project_into(&cartographic, &mut result);
    assert!(cartesian3_approx_eq(&result, &expected));
}

#[test]
fn unproject_roundtrip() {
    let cartographic = Cartographic::new(CesiumMath::PI_OVER_TWO, CesiumMath::PI_OVER_FOUR, 12.0);
    let projection = GeographicProjection::new(None);
    let projected = projection.project(&cartographic);
    let unprojected = projection.unproject(&projected);
    assert!(approx_eq(unprojected.longitude, cartographic.longitude));
    assert!(approx_eq(unprojected.latitude, cartographic.latitude));
    assert!(approx_eq(unprojected.height, cartographic.height));
}

#[test]
fn unproject_with_result() {
    let cartographic = Cartographic::new(CesiumMath::PI_OVER_TWO, CesiumMath::PI_OVER_FOUR, 12.0);
    let projection = GeographicProjection::new(None);
    let projected = projection.project(&cartographic);
    let mut result = Cartographic::default();
    projection.unproject_into(&projected, &mut result);
    assert!(approx_eq(result.longitude, cartographic.longitude));
    assert!(approx_eq(result.latitude, cartographic.latitude));
    assert!(approx_eq(result.height, cartographic.height));
}
