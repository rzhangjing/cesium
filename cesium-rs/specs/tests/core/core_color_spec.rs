use cesium_core::cartesian4::Cartesian4;
use cesium_core::color::Color;
use cesium_core::math::CesiumMath;

#[test]
fn default_constructor() {
    let c = Color::default();
    assert_eq!(c.red, 1.0);
    assert_eq!(c.green, 1.0);
    assert_eq!(c.blue, 1.0);
    assert_eq!(c.alpha, 1.0);
}

#[test]
fn constructor_with_parameters() {
    let c = Color::new(0.1, 0.2, 0.3, 0.4);
    assert_eq!(c.red, 0.1);
    assert_eq!(c.green, 0.2);
    assert_eq!(c.blue, 0.3);
    assert_eq!(c.alpha, 0.4);
}

#[test]
fn from_cartesian4() {
    let c4 = Cartesian4::new(0.5, 0.6, 0.7, 0.8);
    let c = Color::from_cartesian4(&c4);
    assert_eq!(c.red, 0.5);
    assert_eq!(c.green, 0.6);
    assert_eq!(c.blue, 0.7);
    assert_eq!(c.alpha, 0.8);
}

#[test]
fn from_bytes() {
    let c = Color::from_bytes(255, 128, 0, 255);
    assert!((c.red - 1.0).abs() < CesiumMath::EPSILON14);
    assert!((c.green - 128.0 / 255.0).abs() < 0.001);
    assert!((c.blue - 0.0).abs() < CesiumMath::EPSILON14);
    assert!((c.alpha - 1.0).abs() < CesiumMath::EPSILON14);
}

#[test]
fn from_rgba() {
    // RGBA = 0xFF8000FF → red=255, green=128, blue=0, alpha=255
    let c = Color::from_rgba(0xFF8000FF);
    assert!((c.red - 1.0).abs() < CesiumMath::EPSILON14);
    assert!((c.green - 128.0 / 255.0).abs() < 0.01);
    assert!((c.blue - 0.0).abs() < CesiumMath::EPSILON14);
    assert!((c.alpha - 1.0).abs() < CesiumMath::EPSILON14);
}

#[test]
fn from_hsl_red() {
    // HSL: hue=0 (red), sat=1, light=0.5 → pure red
    let c = Color::from_hsl(0.0, 1.0, 0.5, 1.0);
    assert!((c.red - 1.0).abs() < CesiumMath::EPSILON14);
    assert!((c.green).abs() < CesiumMath::EPSILON14);
    assert!((c.blue).abs() < CesiumMath::EPSILON14);
    assert_eq!(c.alpha, 1.0);
}

#[test]
fn from_hsl_white() {
    // HSL: hue=0, sat=0, light=1 → white
    let c = Color::from_hsl(0.0, 0.0, 1.0, 1.0);
    assert!((c.red - 1.0).abs() < CesiumMath::EPSILON14);
    assert!((c.green - 1.0).abs() < CesiumMath::EPSILON14);
    assert!((c.blue - 1.0).abs() < CesiumMath::EPSILON14);
}

#[test]
fn byte_to_float_and_back() {
    assert!((Color::byte_to_float(255) - 1.0).abs() < CesiumMath::EPSILON14);
    assert!((Color::byte_to_float(0) - 0.0).abs() < CesiumMath::EPSILON14);
    assert_eq!(Color::float_to_byte(1.0), 255);
    assert_eq!(Color::float_to_byte(0.0), 0);
}

#[test]
fn equals_works() {
    let c1 = Color::new(0.1, 0.2, 0.3, 0.4);
    let c2 = Color::new(0.1, 0.2, 0.3, 0.4);
    let c3 = Color::new(0.5, 0.2, 0.3, 0.4);
    assert!(Color::equals(&c1, &c2));
    assert!(!Color::equals(&c1, &c3));
}

#[test]
fn from_css_color_string_hex() {
    let c = Color::from_css_color_string("#FF0000").unwrap();
    assert!((c.red - 1.0).abs() < CesiumMath::EPSILON14);
    assert!((c.green).abs() < CesiumMath::EPSILON14);
    assert!((c.blue).abs() < CesiumMath::EPSILON14);
    assert_eq!(c.alpha, 1.0);
}

#[test]
fn from_css_color_string_rgb() {
    let c = Color::from_css_color_string("rgb(0, 255, 0)").unwrap();
    assert!((c.red).abs() < CesiumMath::EPSILON14);
    assert!((c.green - 1.0).abs() < CesiumMath::EPSILON14);
    assert!((c.blue).abs() < CesiumMath::EPSILON14);
}
