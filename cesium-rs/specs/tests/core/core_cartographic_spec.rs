use cesium_core::cartographic::Cartographic;
use cesium_core::math::CesiumMath;

#[test]
fn default_constructor() {
    let c = Cartographic::default();
    assert_eq!(c.longitude, 0.0);
    assert_eq!(c.latitude, 0.0);
    assert_eq!(c.height, 0.0);
}

#[test]
fn constructor_with_parameters() {
    let c = Cartographic::new(1.0, 2.0, 3.0);
    assert_eq!(c.longitude, 1.0);
    assert_eq!(c.latitude, 2.0);
    assert_eq!(c.height, 3.0);
}

#[test]
fn from_radians() {
    let c = Cartographic::from_radians_new(
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::FRAC_PI_4,
        Some(100.0),
    );
    assert_eq!(c.longitude, std::f64::consts::FRAC_PI_2);
    assert_eq!(c.latitude, std::f64::consts::FRAC_PI_4);
    assert_eq!(c.height, 100.0);
}

#[test]
fn from_radians_defaults_height_to_zero() {
    let c = Cartographic::from_radians_new(
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::FRAC_PI_4,
        None,
    );
    assert_eq!(c.height, 0.0);
}

#[test]
fn from_degrees() {
    let c = Cartographic::from_degrees_new(90.0, 45.0, Some(100.0));
    assert!((c.longitude - std::f64::consts::FRAC_PI_2).abs() < CesiumMath::EPSILON14);
    assert!((c.latitude - std::f64::consts::FRAC_PI_4).abs() < CesiumMath::EPSILON14);
    assert_eq!(c.height, 100.0);
}

#[test]
fn from_degrees_defaults_height() {
    let c = Cartographic::from_degrees_new(90.0, 45.0, None);
    assert!((c.longitude - std::f64::consts::FRAC_PI_2).abs() < CesiumMath::EPSILON14);
    assert!((c.latitude - std::f64::consts::FRAC_PI_4).abs() < CesiumMath::EPSILON14);
    assert_eq!(c.height, 0.0);
}

#[test]
fn equals() {
    let c = Cartographic::new(1.0, 2.0, 3.0);
    assert_eq!(c, Cartographic::new(1.0, 2.0, 3.0));
    assert_ne!(c, Cartographic::new(2.0, 2.0, 3.0));
    assert_ne!(c, Cartographic::new(1.0, 3.0, 3.0));
    assert_ne!(c, Cartographic::new(1.0, 2.0, 4.0));
}

#[test]
fn to_string_formats_like_js() {
    // Mirror: CartographicSpec it("toString")
    let cartographic = Cartographic::new(1.123, 2.345, 6.789);
    assert_eq!(cartographic.to_string(), "(1.123, 2.345, 6.789)");
}

#[test]
fn to_string_uses_js_number_semantics() {
    // Phase 2 diff regression (D5, case carto.toString.g6): Infinity must
    // print as the JS string `Infinity`, not Rust's `inf`, and integer-valued
    // components must print without a trailing `.0`.
    let cartographic = Cartographic::new(0.0, f64::INFINITY, 0.0);
    assert_eq!(cartographic.to_string(), "(0, Infinity, 0)");

    let cartographic = Cartographic::new(f64::NEG_INFINITY, f64::NAN, -0.0);
    assert_eq!(cartographic.to_string(), "(-Infinity, NaN, 0)");
}

#[test]
fn zero_constant() {
    assert_eq!(Cartographic::ZERO.longitude, 0.0);
    assert_eq!(Cartographic::ZERO.latitude, 0.0);
    assert_eq!(Cartographic::ZERO.height, 0.0);
}
