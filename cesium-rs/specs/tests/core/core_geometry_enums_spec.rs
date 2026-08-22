//! Specs for geometry enums — mirrors `Specs/Core/PrimitiveTypeSpec.js`,
//! `Specs/Core/WindingOrderSpec.js`, `Specs/Core/IndexDatatypeSpec.js`,
//! `Specs/Core/ComponentDatatypeSpec.js`.

use cesium_core::component_datatype::ComponentDatatype;
use cesium_core::index_datatype::IndexDatatype;
use cesium_core::primitive_type::PrimitiveType;
use cesium_core::winding_order::WindingOrder;

// --- PrimitiveType ---

#[test]
fn primitive_type_values() {
    assert_eq!(PrimitiveType::Points as u32, 0x0000);
    assert_eq!(PrimitiveType::Lines as u32, 0x0001);
    assert_eq!(PrimitiveType::LineLoop as u32, 0x0002);
    assert_eq!(PrimitiveType::LineStrip as u32, 0x0003);
    assert_eq!(PrimitiveType::Triangles as u32, 0x0004);
    assert_eq!(PrimitiveType::TriangleStrip as u32, 0x0005);
    assert_eq!(PrimitiveType::TriangleFan as u32, 0x0006);
}

#[test]
fn primitive_type_is_lines() {
    assert!(PrimitiveType::Lines.is_lines());
    assert!(PrimitiveType::LineLoop.is_lines());
    assert!(PrimitiveType::LineStrip.is_lines());
    assert!(!PrimitiveType::Triangles.is_lines());
    assert!(!PrimitiveType::Points.is_lines());
}

#[test]
fn primitive_type_is_triangles() {
    assert!(PrimitiveType::Triangles.is_triangles());
    assert!(PrimitiveType::TriangleStrip.is_triangles());
    assert!(PrimitiveType::TriangleFan.is_triangles());
    assert!(!PrimitiveType::Lines.is_triangles());
}

#[test]
fn primitive_type_validate() {
    assert!(PrimitiveType::validate(0x0000));
    assert!(PrimitiveType::validate(0x0006));
    assert!(!PrimitiveType::validate(0x0007));
}

// --- WindingOrder ---

#[test]
fn winding_order_values() {
    assert_eq!(WindingOrder::Clockwise as u32, 0x0900);
    assert_eq!(WindingOrder::CounterClockwise as u32, 0x0901);
}

#[test]
fn winding_order_validate() {
    assert!(WindingOrder::validate(0x0900));
    assert!(WindingOrder::validate(0x0901));
    assert!(!WindingOrder::validate(0x0902));
}

#[test]
fn winding_order_try_from_u32() {
    assert_eq!(WindingOrder::try_from_u32(0x0900), Some(WindingOrder::Clockwise));
    assert_eq!(WindingOrder::try_from_u32(0x0901), Some(WindingOrder::CounterClockwise));
    assert_eq!(WindingOrder::try_from_u32(999), None);
}

// --- IndexDatatype ---

#[test]
fn index_datatype_size_in_bytes() {
    assert_eq!(IndexDatatype::UnsignedByte.size_in_bytes(), 1);
    assert_eq!(IndexDatatype::UnsignedShort.size_in_bytes(), 2);
    assert_eq!(IndexDatatype::UnsignedInt.size_in_bytes(), 4);
}

#[test]
fn index_datatype_from_size_in_bytes() {
    assert_eq!(IndexDatatype::from_size_in_bytes(1), IndexDatatype::UnsignedByte);
    assert_eq!(IndexDatatype::from_size_in_bytes(2), IndexDatatype::UnsignedShort);
    assert_eq!(IndexDatatype::from_size_in_bytes(4), IndexDatatype::UnsignedInt);
}

#[test]
fn index_datatype_validate() {
    assert!(IndexDatatype::validate(0x1401)); // UNSIGNED_BYTE
    assert!(IndexDatatype::validate(0x1403)); // UNSIGNED_SHORT
    assert!(IndexDatatype::validate(0x1405)); // UNSIGNED_INT
    assert!(!IndexDatatype::validate(0x1406)); // FLOAT
}

// --- ComponentDatatype ---

#[test]
fn component_datatype_size_in_bytes() {
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
fn component_datatype_from_name() {
    assert_eq!(ComponentDatatype::from_name("FLOAT"), Some(ComponentDatatype::Float));
    assert_eq!(ComponentDatatype::from_name("DOUBLE"), Some(ComponentDatatype::Double));
    assert_eq!(ComponentDatatype::from_name("UNSIGNED_BYTE"), Some(ComponentDatatype::UnsignedByte));
    assert_eq!(ComponentDatatype::from_name("INVALID"), None);
}

#[test]
fn component_datatype_validate() {
    assert!(ComponentDatatype::validate(0x1406)); // FLOAT
    assert!(ComponentDatatype::validate(0x140a)); // DOUBLE
    assert!(!ComponentDatatype::validate(0x9999));
}

#[test]
fn component_datatype_dequantize() {
    let val = ComponentDatatype::dequantize(32767.0, ComponentDatatype::Short);
    assert!((val - 1.0).abs() < 1e-10);

    let val = ComponentDatatype::dequantize(255.0, ComponentDatatype::UnsignedByte);
    assert!((val - 1.0).abs() < 1e-10);

    let val = ComponentDatatype::dequantize(0.0, ComponentDatatype::UnsignedByte);
    assert!((val - 0.0).abs() < 1e-10);
}
