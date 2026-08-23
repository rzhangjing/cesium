use cesium_core::component_datatype::ComponentDatatype;
use cesium_core::webgl_constants::WebGLConstants;

#[test]
fn size_in_bytes() {
    assert_eq!(ComponentDatatype::Byte.size_in_bytes(), 1);
    assert_eq!(ComponentDatatype::UnsignedByte.size_in_bytes(), 1);
    assert_eq!(ComponentDatatype::Short.size_in_bytes(), 2);
    assert_eq!(ComponentDatatype::UnsignedShort.size_in_bytes(), 2);
    assert_eq!(ComponentDatatype::Int.size_in_bytes(), 4);
    assert_eq!(ComponentDatatype::UnsignedInt.size_in_bytes(), 4);
    assert_eq!(ComponentDatatype::Float.size_in_bytes(), 4);
    assert_eq!(ComponentDatatype::Double.size_in_bytes(), 8);
}

#[test]
fn validate_works() {
    assert!(ComponentDatatype::validate(WebGLConstants::BYTE));
    assert!(ComponentDatatype::validate(WebGLConstants::UNSIGNED_BYTE));
    assert!(ComponentDatatype::validate(WebGLConstants::SHORT));
    assert!(ComponentDatatype::validate(WebGLConstants::UNSIGNED_SHORT));
    assert!(ComponentDatatype::validate(WebGLConstants::INT));
    assert!(ComponentDatatype::validate(WebGLConstants::UNSIGNED_INT));
    assert!(ComponentDatatype::validate(WebGLConstants::FLOAT));
    assert!(ComponentDatatype::validate(WebGLConstants::DOUBLE));
    assert!(!ComponentDatatype::validate(9999));
}

#[test]
fn from_name_works() {
    assert_eq!(ComponentDatatype::from_name("BYTE"), Some(ComponentDatatype::Byte));
    assert_eq!(ComponentDatatype::from_name("UNSIGNED_BYTE"), Some(ComponentDatatype::UnsignedByte));
    assert_eq!(ComponentDatatype::from_name("SHORT"), Some(ComponentDatatype::Short));
    assert_eq!(ComponentDatatype::from_name("UNSIGNED_SHORT"), Some(ComponentDatatype::UnsignedShort));
    assert_eq!(ComponentDatatype::from_name("INT"), Some(ComponentDatatype::Int));
    assert_eq!(ComponentDatatype::from_name("UNSIGNED_INT"), Some(ComponentDatatype::UnsignedInt));
    assert_eq!(ComponentDatatype::from_name("FLOAT"), Some(ComponentDatatype::Float));
    assert_eq!(ComponentDatatype::from_name("DOUBLE"), Some(ComponentDatatype::Double));
    assert_eq!(ComponentDatatype::from_name("INVALID"), None);
}

#[test]
fn try_from_u32_works() {
    assert_eq!(ComponentDatatype::try_from_u32(WebGLConstants::BYTE), Some(ComponentDatatype::Byte));
    assert_eq!(ComponentDatatype::try_from_u32(WebGLConstants::FLOAT), Some(ComponentDatatype::Float));
    assert_eq!(ComponentDatatype::try_from_u32(9999), None);
}

#[test]
fn dequantize_works() {
    // UnsignedByte: 0→0.0, 255→1.0
    assert!((ComponentDatatype::dequantize(0.0, ComponentDatatype::UnsignedByte) - 0.0).abs() < 1e-10);
    assert!((ComponentDatatype::dequantize(255.0, ComponentDatatype::UnsignedByte) - 1.0).abs() < 1e-10);

    // Short: 0→0.0, 32767→1.0
    assert!((ComponentDatatype::dequantize(0.0, ComponentDatatype::Short) - 0.0).abs() < 1e-10);
    assert!((ComponentDatatype::dequantize(32767.0, ComponentDatatype::Short) - 1.0).abs() < 1e-10);

    // Byte: -128→-1.0 (clamped), 127→1.0
    assert!((ComponentDatatype::dequantize(-128.0, ComponentDatatype::Byte) - (-1.0)).abs() < 1e-10);
    assert!((ComponentDatatype::dequantize(127.0, ComponentDatatype::Byte) - 1.0).abs() < 1e-10);

    // Float passes through
    assert_eq!(ComponentDatatype::dequantize(3.14, ComponentDatatype::Float), 3.14);
}
