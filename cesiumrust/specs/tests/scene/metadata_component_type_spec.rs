//! Scene/MetadataComponentTypeSpec.js → Rust integration tests
//!
//! Original: 37 it() → 20 A-class (17 C-class: throws)
//! Tests: getMinimum(1) + getMaximum(1) + isIntegerType(1) + isUnsignedIntegerType(1) +
//!        normalize(4) + unnormalize(5) + getSizeInBytes(1) + fromComponentDatatype(1) +
//!        toComponentDatatype(3) + category(1) + downcastFunction(1)

use cesium_scene::metadata_component_type::{MetadataComponentType, ScalarCategory};

#[test]
fn test_get_minimum() {
    assert_eq!(MetadataComponentType::Int8.get_minimum(), -128.0);
    assert_eq!(MetadataComponentType::Uint8.get_minimum(), 0.0);
    assert_eq!(MetadataComponentType::Int16.get_minimum(), -32768.0);
    assert_eq!(MetadataComponentType::Uint16.get_minimum(), 0.0);
    assert_eq!(MetadataComponentType::Int32.get_minimum(), -2147483648.0);
    assert_eq!(MetadataComponentType::Uint32.get_minimum(), 0.0);
    assert_eq!(
        MetadataComponentType::Float32.get_minimum(),
        -f32::MAX as f64
    );
    assert_eq!(MetadataComponentType::Float64.get_minimum(), -f64::MAX);
}

#[test]
fn test_get_maximum() {
    assert_eq!(MetadataComponentType::Int8.get_maximum(), 127.0);
    assert_eq!(MetadataComponentType::Uint8.get_maximum(), 255.0);
    assert_eq!(MetadataComponentType::Int16.get_maximum(), 32767.0);
    assert_eq!(MetadataComponentType::Uint16.get_maximum(), 65535.0);
    assert_eq!(MetadataComponentType::Int32.get_maximum(), 2147483647.0);
    assert_eq!(MetadataComponentType::Uint32.get_maximum(), 4294967295.0);
    assert_eq!(
        MetadataComponentType::Float32.get_maximum(),
        f32::MAX as f64
    );
    assert_eq!(MetadataComponentType::Float64.get_maximum(), f64::MAX);
}

#[test]
fn test_is_integer_type() {
    assert!(MetadataComponentType::Int8.is_integer_type());
    assert!(MetadataComponentType::Uint8.is_integer_type());
    assert!(MetadataComponentType::Int16.is_integer_type());
    assert!(MetadataComponentType::Uint16.is_integer_type());
    assert!(MetadataComponentType::Int32.is_integer_type());
    assert!(MetadataComponentType::Uint32.is_integer_type());
    assert!(MetadataComponentType::Int64.is_integer_type());
    assert!(MetadataComponentType::Uint64.is_integer_type());
    assert!(!MetadataComponentType::Float32.is_integer_type());
    assert!(!MetadataComponentType::Float64.is_integer_type());
}

#[test]
fn test_is_unsigned_integer_type() {
    assert!(!MetadataComponentType::Int8.is_unsigned_integer_type());
    assert!(MetadataComponentType::Uint8.is_unsigned_integer_type());
    assert!(!MetadataComponentType::Int16.is_unsigned_integer_type());
    assert!(MetadataComponentType::Uint16.is_unsigned_integer_type());
    assert!(!MetadataComponentType::Int32.is_unsigned_integer_type());
    assert!(MetadataComponentType::Uint32.is_unsigned_integer_type());
    assert!(!MetadataComponentType::Int64.is_unsigned_integer_type());
    assert!(MetadataComponentType::Uint64.is_unsigned_integer_type());
    assert!(!MetadataComponentType::Float32.is_unsigned_integer_type());
    assert!(!MetadataComponentType::Float64.is_unsigned_integer_type());
}

#[test]
fn test_normalize_signed_integers() {
    // INT8: min=-128, max=127
    let t = MetadataComponentType::Int8;
    assert_eq!(t.normalize(-128.0), -1.0); // max(-128/127, -1) = -1
    assert_eq!(t.normalize(-127.0), -1.0); // -127/127 = -1.0
    assert_eq!(t.normalize(0.0), 0.0);
    assert_eq!(t.normalize(127.0), 1.0);

    // INT16: min=-32768, max=32767
    let t = MetadataComponentType::Int16;
    assert_eq!(t.normalize(-32768.0), -1.0);
    assert_eq!(t.normalize(0.0), 0.0);
    assert_eq!(t.normalize(32767.0), 1.0);
}

#[test]
fn test_normalize_unsigned_integers() {
    let t = MetadataComponentType::Uint8;
    assert_eq!(t.normalize(0.0), 0.0);
    assert_eq!(t.normalize(51.0), 0.2); // 51/255 = 0.2
    assert_eq!(t.normalize(255.0), 1.0);

    let t = MetadataComponentType::Uint16;
    assert_eq!(t.normalize(0.0), 0.0);
    assert_eq!(t.normalize(65535.0), 1.0);
}

#[test]
fn test_normalize_int64() {
    let t = MetadataComponentType::Int64;
    let max = i64::MAX as f64;
    assert_eq!(t.normalize(0.0), 0.0);
    assert_eq!(t.normalize(max), 1.0);
    // min/max → -1.0 (clamped)
    assert_eq!(t.normalize(i64::MIN as f64), -1.0);
}

#[test]
fn test_normalize_uint64() {
    let t = MetadataComponentType::Uint64;
    let max = u64::MAX as f64;
    assert_eq!(t.normalize(0.0), 0.0);
    assert_eq!(t.normalize(max), 1.0);
}

#[test]
fn test_unnormalize_signed() {
    // INT8: max=127
    let t = MetadataComponentType::Int8;
    assert_eq!(t.unnormalize(-1.0), -127.0); // sign(-1)*round(1*127) = -127
    assert_eq!(t.unnormalize(0.0), 0.0);
    assert_eq!(t.unnormalize(1.0), 127.0);

    // INT16: max=32767
    let t = MetadataComponentType::Int16;
    assert_eq!(t.unnormalize(-1.0), -32767.0);
    assert_eq!(t.unnormalize(0.0), 0.0);
    assert_eq!(t.unnormalize(1.0), 32767.0);
}

#[test]
fn test_unnormalize_unsigned() {
    let t = MetadataComponentType::Uint8;
    assert_eq!(t.unnormalize(0.0), 0.0);
    assert_eq!(t.unnormalize(0.2), 51.0); // round(0.2*255) = 51
    assert_eq!(t.unnormalize(1.0), 255.0);

    let t = MetadataComponentType::Uint16;
    assert_eq!(t.unnormalize(0.0), 0.0);
    assert_eq!(t.unnormalize(1.0), 65535.0);
}

#[test]
fn test_unnormalize_int64() {
    let t = MetadataComponentType::Int64;
    let max = i64::MAX as f64;
    assert_eq!(t.unnormalize(0.0), 0.0);
    assert_eq!(t.unnormalize(1.0), max);
    assert_eq!(t.unnormalize(-1.0), -max);
}

#[test]
fn test_unnormalize_uint64() {
    let t = MetadataComponentType::Uint64;
    let max = u64::MAX as f64;
    assert_eq!(t.unnormalize(0.0), 0.0);
    assert_eq!(t.unnormalize(1.0), max);
}

#[test]
fn test_unnormalize_clamps() {
    assert_eq!(MetadataComponentType::Int8.unnormalize(-1.1), -127.0);
    assert_eq!(MetadataComponentType::Uint8.unnormalize(-0.1), 0.0);
    assert_eq!(MetadataComponentType::Int8.unnormalize(1.1), 127.0);
    assert_eq!(MetadataComponentType::Uint8.unnormalize(1.1), 255.0);
}

#[test]
fn test_get_size_in_bytes() {
    assert_eq!(MetadataComponentType::Int8.get_size_in_bytes(), 1);
    assert_eq!(MetadataComponentType::Uint8.get_size_in_bytes(), 1);
    assert_eq!(MetadataComponentType::Int16.get_size_in_bytes(), 2);
    assert_eq!(MetadataComponentType::Uint16.get_size_in_bytes(), 2);
    assert_eq!(MetadataComponentType::Int32.get_size_in_bytes(), 4);
    assert_eq!(MetadataComponentType::Uint32.get_size_in_bytes(), 4);
    assert_eq!(MetadataComponentType::Int64.get_size_in_bytes(), 8);
    assert_eq!(MetadataComponentType::Uint64.get_size_in_bytes(), 8);
    assert_eq!(MetadataComponentType::Float32.get_size_in_bytes(), 4);
    assert_eq!(MetadataComponentType::Float64.get_size_in_bytes(), 8);
}

#[test]
fn test_from_component_datatype() {
    assert_eq!(
        MetadataComponentType::from_component_datatype(5120),
        Some(MetadataComponentType::Int8)
    );
    assert_eq!(
        MetadataComponentType::from_component_datatype(5121),
        Some(MetadataComponentType::Uint8)
    );
    assert_eq!(
        MetadataComponentType::from_component_datatype(5122),
        Some(MetadataComponentType::Int16)
    );
    assert_eq!(
        MetadataComponentType::from_component_datatype(5123),
        Some(MetadataComponentType::Uint16)
    );
    assert_eq!(
        MetadataComponentType::from_component_datatype(5124),
        Some(MetadataComponentType::Int32)
    );
    assert_eq!(
        MetadataComponentType::from_component_datatype(5125),
        Some(MetadataComponentType::Uint32)
    );
    assert_eq!(
        MetadataComponentType::from_component_datatype(5126),
        Some(MetadataComponentType::Float32)
    );
    assert_eq!(
        MetadataComponentType::from_component_datatype(5130),
        Some(MetadataComponentType::Float64)
    );
}

#[test]
fn test_to_component_datatype() {
    assert_eq!(MetadataComponentType::Int8.to_component_datatype(), Some(5120));
    assert_eq!(MetadataComponentType::Uint8.to_component_datatype(), Some(5121));
    assert_eq!(MetadataComponentType::Int16.to_component_datatype(), Some(5122));
    assert_eq!(MetadataComponentType::Uint16.to_component_datatype(), Some(5123));
    assert_eq!(MetadataComponentType::Int32.to_component_datatype(), Some(5124));
    assert_eq!(MetadataComponentType::Uint32.to_component_datatype(), Some(5125));
    assert_eq!(MetadataComponentType::Float32.to_component_datatype(), Some(5126));
    assert_eq!(MetadataComponentType::Float64.to_component_datatype(), Some(5130));
}

#[test]
fn test_to_component_datatype_returns_none_for_64bit() {
    assert_eq!(MetadataComponentType::Int64.to_component_datatype(), None);
    assert_eq!(MetadataComponentType::Uint64.to_component_datatype(), None);
}

#[test]
fn test_category() {
    assert_eq!(MetadataComponentType::Int8.category(), ScalarCategory::Integer);
    assert_eq!(MetadataComponentType::Uint8.category(), ScalarCategory::UnsignedInteger);
    assert_eq!(MetadataComponentType::Int16.category(), ScalarCategory::Integer);
    assert_eq!(MetadataComponentType::Uint16.category(), ScalarCategory::UnsignedInteger);
    assert_eq!(MetadataComponentType::Int32.category(), ScalarCategory::Integer);
    assert_eq!(MetadataComponentType::Uint32.category(), ScalarCategory::UnsignedInteger);
    assert_eq!(MetadataComponentType::Int64.category(), ScalarCategory::Integer);
    assert_eq!(MetadataComponentType::Uint64.category(), ScalarCategory::UnsignedInteger);
    assert_eq!(MetadataComponentType::Float32.category(), ScalarCategory::Float);
    assert_eq!(MetadataComponentType::Float64.category(), ScalarCategory::Float);
}

#[test]
fn test_downcast_function() {
    // INT64 downcast clamps to INT32 range
    assert_eq!(MetadataComponentType::Int64.downcast(123456789012345.0), 2147483647.0);
    assert_eq!(MetadataComponentType::Int64.downcast(-123456789012345.0), -2147483648.0);

    // UINT64 downcast clamps to UINT32 range
    assert_eq!(MetadataComponentType::Uint64.downcast(123456789012345.0), 4294967295.0);
    assert_eq!(MetadataComponentType::Uint64.downcast(-1.0), 0.0);

    // FLOAT64 downcast converts to f32 precision
    let value = 1.337123456789_f64;
    let downcast = MetadataComponentType::Float64.downcast(value);
    assert_eq!(downcast, (value as f32) as f64);
    assert_ne!(downcast, value);
}
