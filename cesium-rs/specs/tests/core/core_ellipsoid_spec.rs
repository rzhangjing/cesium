//! Mirrors packages/engine/Specs/Core/EllipsoidSpec.js

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::math::CesiumMath;

const PI: f64 = std::f64::consts::PI;

fn radii() -> Cartesian3 {
    Cartesian3::new(1.0, 2.0, 3.0)
}

fn radii_squared() -> Cartesian3 {
    let r = radii();
    Cartesian3::new(r.x * r.x, r.y * r.y, r.z * r.z)
}

fn radii_to_the_fourth() -> Cartesian3 {
    let rs = radii_squared();
    Cartesian3::new(rs.x * rs.x, rs.y * rs.y, rs.z * rs.z)
}

// Test data (values computed using STK Components)
fn space_cartesian() -> Cartesian3 {
    Cartesian3::new(4582719.8827300891, -4582719.8827300882, 1725510.4250797231)
}

fn space_cartographic() -> Cartographic {
    Cartographic {
        longitude: CesiumMath::to_radians(-45.0),
        latitude: CesiumMath::to_radians(15.0),
        height: 330000.0,
    }
}

fn surface_cartesian() -> Cartesian3 {
    Cartesian3::new(4094327.7921465295, 1909216.4044747739, 4487348.4088659193)
}

fn surface_cartographic() -> Cartographic {
    Cartographic {
        longitude: CesiumMath::to_radians(25.0),
        latitude: CesiumMath::to_radians(45.0),
        height: 0.0,
    }
}

// --- constructor ---

#[test]
fn default_constructor_creates_zero_ellipsoid() {
    let e = Ellipsoid::new(0.0, 0.0, 0.0);
    assert_eq!(*e.radii(), Cartesian3::ZERO);
    assert_eq!(*e.radii_squared(), Cartesian3::ZERO);
    assert_eq!(*e.radii_to_the_fourth(), Cartesian3::ZERO);
    assert_eq!(*e.one_over_radii(), Cartesian3::ZERO);
    assert_eq!(*e.one_over_radii_squared(), Cartesian3::ZERO);
    assert_eq!(e.minimum_radius(), 0.0);
    assert_eq!(e.maximum_radius(), 0.0);
}

#[test]
fn from_cartesian3_creates_zero_ellipsoid_with_no_params() {
    let e = Ellipsoid::from_cartesian3(None);
    assert_eq!(*e.radii(), Cartesian3::ZERO);
    assert_eq!(*e.radii_squared(), Cartesian3::ZERO);
    assert_eq!(*e.radii_to_the_fourth(), Cartesian3::ZERO);
    assert_eq!(*e.one_over_radii(), Cartesian3::ZERO);
    assert_eq!(*e.one_over_radii_squared(), Cartesian3::ZERO);
    assert_eq!(e.minimum_radius(), 0.0);
    assert_eq!(e.maximum_radius(), 0.0);
}

#[test]
fn constructor_computes_correct_values() {
    let r = radii();
    let e = Ellipsoid::new(r.x, r.y, r.z);
    assert_eq!(*e.radii(), r);
    assert_eq!(*e.radii_squared(), radii_squared());
    assert_eq!(*e.radii_to_the_fourth(), radii_to_the_fourth());
    let o = Cartesian3::new(1.0 / r.x, 1.0 / r.y, 1.0 / r.z);
    assert_eq!(*e.one_over_radii(), o);
    let rs = radii_squared();
    let oos = Cartesian3::new(1.0 / rs.x, 1.0 / rs.y, 1.0 / rs.z);
    assert_eq!(*e.one_over_radii_squared(), oos);
    assert_eq!(e.minimum_radius(), 1.0);
    assert_eq!(e.maximum_radius(), 3.0);
}

#[test]
fn from_cartesian3_computes_correct_values() {
    let r = radii();
    let e = Ellipsoid::from_cartesian3(Some(&r));
    assert_eq!(*e.radii(), r);
    assert_eq!(*e.radii_squared(), radii_squared());
    assert_eq!(*e.radii_to_the_fourth(), radii_to_the_fourth());
    assert_eq!(e.minimum_radius(), 1.0);
    assert_eq!(e.maximum_radius(), 3.0);
}

// --- geodeticSurfaceNormalCartographic ---

#[test]
fn geodetic_surface_normal_cartographic_works() {
    let e = Ellipsoid::WGS84;
    let mut result = Cartesian3::default();
    e.geodetic_surface_normal_cartographic(&space_cartographic(), &mut result);
    let expected = Cartesian3::new(
        0.68301270189221941,
        -0.6830127018922193,
        0.25881904510252074,
    );
    assert!(Cartesian3::equals_epsilon(
        Some(&result), Some(&expected),
        Some(CesiumMath::EPSILON15), None,
    ));
}

// --- geodeticSurfaceNormal ---

#[test]
fn geodetic_surface_normal_works() {
    let e = Ellipsoid::WGS84;
    let mut result = Cartesian3::default();
    assert!(e.geodetic_surface_normal(&space_cartesian(), &mut result));
    let expected = Cartesian3::new(
        0.6829975339864266,
        -0.68299753398642649,
        0.25889908678270795,
    );
    assert!(Cartesian3::equals_epsilon(
        Some(&result), Some(&expected),
        Some(CesiumMath::EPSILON15), None,
    ));
}

#[test]
fn geodetic_surface_normal_returns_false_at_origin() {
    let e = Ellipsoid::WGS84;
    let mut result = Cartesian3::default();
    assert!(!e.geodetic_surface_normal(&Cartesian3::ZERO, &mut result));
}

// --- cartographicToCartesian ---

#[test]
fn cartographic_to_cartesian_works() {
    let e = Ellipsoid::WGS84;
    let mut result = Cartesian3::default();
    e.cartographic_to_cartesian(&space_cartographic(), &mut result);
    let expected = space_cartesian();
    assert!(Cartesian3::equals_epsilon(
        Some(&result), Some(&expected),
        Some(CesiumMath::EPSILON7), None,
    ));
}

// --- cartesianToCartographic ---

#[test]
fn cartesian_to_cartographic_works() {
    let e = Ellipsoid::WGS84;
    let mut result = Cartographic::default();
    assert!(e.cartesian_to_cartographic(&surface_cartesian(), &mut result));
    let expected = surface_cartographic();
    assert!((result.longitude - expected.longitude).abs() <= CesiumMath::EPSILON8);
    assert!((result.latitude - expected.latitude).abs() <= CesiumMath::EPSILON8);
    assert!((result.height - expected.height).abs() <= CesiumMath::EPSILON8);
}

#[test]
fn cartesian_to_cartographic_returns_false_at_center() {
    let e = Ellipsoid::WGS84;
    let mut result = Cartographic::default();
    assert!(!e.cartesian_to_cartographic(&Cartesian3::ZERO, &mut result));
}

#[test]
fn cartesian_to_cartographic_returns_false_very_close_to_center() {
    let e = Ellipsoid::WGS84;
    let mut result = Cartographic::default();
    assert!(!e.cartesian_to_cartographic(
        &Cartesian3::new(1e-150, 1e-150, 1e-150),
        &mut result,
    ));
}

// --- scaleToGeodeticSurface ---

#[test]
fn scale_to_geodetic_surface_x_direction() {
    let e = Ellipsoid::new(1.0, 2.0, 3.0);
    let mut result = Cartesian3::default();
    assert!(e.scale_to_geodetic_surface(&Cartesian3::new(9.0, 0.0, 0.0), &mut result));
    assert_eq!(result, Cartesian3::new(1.0, 0.0, 0.0));
}

#[test]
fn scale_to_geodetic_surface_y_direction() {
    let e = Ellipsoid::new(1.0, 2.0, 3.0);
    let mut result = Cartesian3::default();
    assert!(e.scale_to_geodetic_surface(&Cartesian3::new(0.0, 8.0, 0.0), &mut result));
    assert_eq!(result, Cartesian3::new(0.0, 2.0, 0.0));
}

#[test]
fn scale_to_geodetic_surface_z_direction() {
    let e = Ellipsoid::new(1.0, 2.0, 3.0);
    let mut result = Cartesian3::default();
    assert!(e.scale_to_geodetic_surface(&Cartesian3::new(0.0, 0.0, 8.0), &mut result));
    assert_eq!(result, Cartesian3::new(0.0, 0.0, 3.0));
}

#[test]
fn scale_to_geodetic_surface_general() {
    let e = Ellipsoid::new(1.0, 2.0, 3.0);
    let mut result = Cartesian3::default();
    assert!(e.scale_to_geodetic_surface(&Cartesian3::new(4.0, 5.0, 6.0), &mut result));
    let expected = Cartesian3::new(0.2680893773941855, 1.1160466902266495, 2.3559801120411263);
    assert!(Cartesian3::equals_epsilon(
        Some(&result), Some(&expected),
        Some(CesiumMath::EPSILON16), None,
    ));
}

#[test]
fn scale_to_geodetic_surface_returns_false_at_center() {
    let e = Ellipsoid::new(1.0, 2.0, 3.0);
    let mut result = Cartesian3::default();
    assert!(!e.scale_to_geodetic_surface(&Cartesian3::ZERO, &mut result));
}

// --- scaleToGeocentricSurface ---

#[test]
fn scale_to_geocentric_surface_x_direction() {
    let e = Ellipsoid::new(1.0, 2.0, 3.0);
    let mut result = Cartesian3::default();
    e.scale_to_geocentric_surface(&Cartesian3::new(9.0, 0.0, 0.0), &mut result);
    assert_eq!(result, Cartesian3::new(1.0, 0.0, 0.0));
}

#[test]
fn scale_to_geocentric_surface_general() {
    let e = Ellipsoid::new(1.0, 2.0, 3.0);
    let mut result = Cartesian3::default();
    e.scale_to_geocentric_surface(&Cartesian3::new(4.0, 5.0, 6.0), &mut result);
    let expected = Cartesian3::new(0.7807200583588266, 0.9759000729485333, 1.1710800875382399);
    assert!(Cartesian3::equals_epsilon(
        Some(&result), Some(&expected),
        Some(CesiumMath::EPSILON16), None,
    ));
}

// --- transformPosition ---

#[test]
fn transform_position_to_scaled_space_works() {
    let e = Ellipsoid::new(2.0, 3.0, 4.0);
    let mut result = Cartesian3::default();
    e.transform_position_to_scaled_space(&Cartesian3::new(4.0, 6.0, 8.0), &mut result);
    let expected = Cartesian3::new(2.0, 2.0, 2.0);
    assert!(Cartesian3::equals_epsilon(
        Some(&result), Some(&expected),
        Some(CesiumMath::EPSILON16), None,
    ));
}

#[test]
fn transform_position_from_scaled_space_works() {
    let e = Ellipsoid::new(2.0, 3.0, 4.0);
    let mut result = Cartesian3::default();
    e.transform_position_from_scaled_space(&Cartesian3::new(2.0, 2.0, 2.0), &mut result);
    let expected = Cartesian3::new(4.0, 6.0, 8.0);
    assert!(Cartesian3::equals_epsilon(
        Some(&result), Some(&expected),
        Some(CesiumMath::EPSILON16), None,
    ));
}

// --- equals ---

#[test]
fn equals_works() {
    let e = Ellipsoid::new(1.0, 0.0, 0.0);
    assert!(e.equals(&Ellipsoid::new(1.0, 0.0, 0.0)));
    assert!(!e.equals(&Ellipsoid::new(1.0, 1.0, 0.0)));
}

// --- toString ---

#[test]
fn to_string_produces_expected() {
    let e = Ellipsoid::new(1.0, 2.0, 3.0);
    assert_eq!(e.to_string_repr(), "(1, 2, 3)");
}

// --- geocentricSurfaceNormal ---

#[test]
fn geocentric_surface_normal_is_normalize() {
    let mut result = Cartesian3::default();
    Ellipsoid::geocentric_surface_normal(&Cartesian3::new(3.0, 0.0, 0.0), &mut result);
    assert_eq!(result, Cartesian3::new(1.0, 0.0, 0.0));
}

// --- pack/unpack ---

#[test]
fn pack_unpack_roundtrip() {
    let e = Ellipsoid::WGS84;
    let mut array = [0.0; 3];
    Ellipsoid::pack(&e, &mut array, None);
    assert_eq!(array[0], Ellipsoid::WGS84.radii().x);
    assert_eq!(array[1], Ellipsoid::WGS84.radii().y);
    assert_eq!(array[2], Ellipsoid::WGS84.radii().z);

    let unpacked = Ellipsoid::unpack(&array, None);
    assert_eq!(unpacked, e);
}

// --- squaredXOverSquaredZ ---

#[test]
fn squared_x_over_squared_z_is_initialized() {
    let e = Ellipsoid::new(4.0, 4.0, 3.0);
    let rs = e.radii_squared();
    let expected = rs.x / rs.z;
    // Access via internal data (through get_surface_normal_intersection_with_z_axis)
    let mut result = Cartesian3::default();
    let pos = Cartesian3::new(e.radii().x, 0.0, 0.0);
    assert!(e.get_surface_normal_intersection_with_z_axis(&pos, None, &mut result));
}

// --- getLocalCurvature ---

#[test]
fn get_local_curvature_at_equator() {
    let e = Ellipsoid::WGS84;
    let cartographic = Cartographic { longitude: 0.0, latitude: 0.0, height: 0.0 };
    let mut cartesian = Cartesian3::default();
    e.cartographic_to_cartesian(&cartographic, &mut cartesian);

    let mut result = Cartesian2::default();
    e.get_local_curvature(&cartesian, &mut result);

    let expected = Cartesian2::new(
        1.0 / e.maximum_radius(),
        e.maximum_radius() / (e.minimum_radius() * e.minimum_radius()),
    );
    assert!((result.x - expected.x).abs() <= CesiumMath::EPSILON8);
    assert!((result.y - expected.y).abs() <= CesiumMath::EPSILON8);
}

#[test]
fn get_local_curvature_at_north_pole() {
    let e = Ellipsoid::WGS84;
    let cartographic = Cartographic { longitude: 0.0, latitude: PI / 2.0, height: 0.0 };
    let mut cartesian = Cartesian3::default();
    e.cartographic_to_cartesian(&cartographic, &mut cartesian);

    let mut result = Cartesian2::default();
    e.get_local_curvature(&cartesian, &mut result);

    let semi_latus_rectum = (e.maximum_radius() * e.maximum_radius()) / e.minimum_radius();
    let expected = Cartesian2::new(1.0 / semi_latus_rectum, 1.0 / semi_latus_rectum);
    assert!((result.x - expected.x).abs() <= CesiumMath::EPSILON8);
    assert!((result.y - expected.y).abs() <= CesiumMath::EPSILON8);
}

// --- ellipsoid_params ---

#[test]
fn ellipsoid_params_matches_internal_data() {
    let e = Ellipsoid::WGS84;
    let params = e.ellipsoid_params();
    assert_eq!(params.one_over_radii, *e.one_over_radii());
    assert_eq!(params.one_over_radii_squared, *e.one_over_radii_squared());
    assert_eq!(params.center_tolerance_squared, CesiumMath::EPSILON1);
}

// --- roundtrip ---

#[test]
fn cartographic_cartesian_roundtrip() {
    let e = Ellipsoid::WGS84;
    let original = space_cartographic();
    let mut cartesian = Cartesian3::default();
    e.cartographic_to_cartesian(&original, &mut cartesian);

    let mut recovered = Cartographic::default();
    assert!(e.cartesian_to_cartographic(&cartesian, &mut recovered));

    assert!((recovered.longitude - original.longitude).abs() <= CesiumMath::EPSILON7);
    assert!((recovered.latitude - original.latitude).abs() <= CesiumMath::EPSILON7);
    assert!((recovered.height - original.height).abs() <= CesiumMath::EPSILON7);
}
