//! Scene/AttributeTypeSpec.js → Rust integration tests
//!
//! Original: 7 it() → 4 A-class (3 C-class: throws)
//! Tests: getMathType(1) + getGlslType(1) + getNumberOfComponents(1) +
//!        getAttributeLocationCount(1)

use cesium_scene::attribute_type::AttributeType;

#[test]
fn test_get_math_type() {
    assert_eq!(AttributeType::Scalar.get_math_type_name(), "Number");
    assert_eq!(AttributeType::Vec2.get_math_type_name(), "Cartesian2");
    assert_eq!(AttributeType::Vec3.get_math_type_name(), "Cartesian3");
    assert_eq!(AttributeType::Vec4.get_math_type_name(), "Cartesian4");
    assert_eq!(AttributeType::Mat2.get_math_type_name(), "Matrix2");
    assert_eq!(AttributeType::Mat3.get_math_type_name(), "Matrix3");
    assert_eq!(AttributeType::Mat4.get_math_type_name(), "Matrix4");
}

#[test]
fn test_get_glsl_type() {
    assert_eq!(AttributeType::Scalar.get_glsl_type(), "float");
    assert_eq!(AttributeType::Vec2.get_glsl_type(), "vec2");
    assert_eq!(AttributeType::Vec3.get_glsl_type(), "vec3");
    assert_eq!(AttributeType::Vec4.get_glsl_type(), "vec4");
    assert_eq!(AttributeType::Mat2.get_glsl_type(), "mat2");
    assert_eq!(AttributeType::Mat3.get_glsl_type(), "mat3");
    assert_eq!(AttributeType::Mat4.get_glsl_type(), "mat4");
}

#[test]
fn test_get_number_of_components() {
    assert_eq!(AttributeType::Scalar.get_number_of_components(), 1);
    assert_eq!(AttributeType::Vec2.get_number_of_components(), 2);
    assert_eq!(AttributeType::Vec3.get_number_of_components(), 3);
    assert_eq!(AttributeType::Vec4.get_number_of_components(), 4);
    assert_eq!(AttributeType::Mat2.get_number_of_components(), 4);
    assert_eq!(AttributeType::Mat3.get_number_of_components(), 9);
    assert_eq!(AttributeType::Mat4.get_number_of_components(), 16);
}

#[test]
fn test_get_attribute_location_count() {
    assert_eq!(AttributeType::Scalar.get_attribute_location_count(), 1);
    assert_eq!(AttributeType::Vec2.get_attribute_location_count(), 1);
    assert_eq!(AttributeType::Vec3.get_attribute_location_count(), 1);
    assert_eq!(AttributeType::Vec4.get_attribute_location_count(), 1);
    assert_eq!(AttributeType::Mat2.get_attribute_location_count(), 2);
    assert_eq!(AttributeType::Mat3.get_attribute_location_count(), 3);
    assert_eq!(AttributeType::Mat4.get_attribute_location_count(), 4);
}

#[test]
fn test_from_str() {
    assert_eq!(AttributeType::from_str("SCALAR"), Some(AttributeType::Scalar));
    assert_eq!(AttributeType::from_str("VEC2"), Some(AttributeType::Vec2));
    assert_eq!(AttributeType::from_str("VEC3"), Some(AttributeType::Vec3));
    assert_eq!(AttributeType::from_str("VEC4"), Some(AttributeType::Vec4));
    assert_eq!(AttributeType::from_str("MAT2"), Some(AttributeType::Mat2));
    assert_eq!(AttributeType::from_str("MAT3"), Some(AttributeType::Mat3));
    assert_eq!(AttributeType::from_str("MAT4"), Some(AttributeType::Mat4));
    assert_eq!(AttributeType::from_str("Invalid"), None);
}
