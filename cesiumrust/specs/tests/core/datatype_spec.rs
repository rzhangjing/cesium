//! Core/ComponentDatatypeSpec.js + Core/IndexDatatypeSpec.js → Rust integration tests
//!
//! ComponentDatatypeSpec.js: 13 original it() blocks → 5 A-class tests ported
//! IndexDatatypeSpec.js: 14 original it() blocks → 5 A-class tests ported
//!
//! Omitted C-class tests (JS typed-array / DeveloperError throws):
//! - ComponentDatatype: fromTypedArray throws(1), createTypedArray(2), createArrayBufferView(4),
//!   createTypedArray throws(2), fromName throws(1) = 10 C-class
//! - IndexDatatype: createTypedArray throws(1), createTypedArrayFromArrayBuffer(4+throws3),
//!   getSizeInBytes throws(1), fromTypedArray throws(1) = 10 C-class
//!
//! Note: JS `fromTypedArray` maps to Rust `from_gl_value` (type inference from WebGL constant).
//! JS `createTypedArray`/`createArrayBufferView` are C-class (JS-specific typed array creation).

use cesium_geospatial::attribute_compression::{ComponentDatatype, IndexDatatype};

// ============================================================================
// ComponentDatatype
// ============================================================================

#[test]
fn component_datatype_from_gl_value_works() {
    // Maps to "fromTypedArray works" — each JS typed array maps to a GL constant
    assert_eq!(
        ComponentDatatype::from_gl_value(0x1400),
        Some(ComponentDatatype::Byte)
    );
    assert_eq!(
        ComponentDatatype::from_gl_value(0x1401),
        Some(ComponentDatatype::UnsignedByte)
    );
    assert_eq!(
        ComponentDatatype::from_gl_value(0x1402),
        Some(ComponentDatatype::Short)
    );
    assert_eq!(
        ComponentDatatype::from_gl_value(0x1403),
        Some(ComponentDatatype::UnsignedShort)
    );
    assert_eq!(
        ComponentDatatype::from_gl_value(0x1404),
        Some(ComponentDatatype::Int)
    );
    assert_eq!(
        ComponentDatatype::from_gl_value(0x1405),
        Some(ComponentDatatype::UnsignedInt)
    );
    assert_eq!(
        ComponentDatatype::from_gl_value(0x1406),
        Some(ComponentDatatype::Float)
    );
    assert_eq!(
        ComponentDatatype::from_gl_value(0x140A),
        Some(ComponentDatatype::Double)
    );
    // Invalid value
    assert_eq!(ComponentDatatype::from_gl_value(0x9999), None);
}

#[test]
fn component_datatype_validate_works() {
    // All enum variants are valid
    assert!(ComponentDatatype::Byte.validate());
    assert!(ComponentDatatype::UnsignedByte.validate());
    assert!(ComponentDatatype::Short.validate());
    assert!(ComponentDatatype::UnsignedShort.validate());
    assert!(ComponentDatatype::Int.validate());
    assert!(ComponentDatatype::UnsignedInt.validate());
    assert!(ComponentDatatype::Float.validate());
    assert!(ComponentDatatype::Double.validate());
}

#[test]
fn component_datatype_get_size_in_bytes() {
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
fn component_datatype_from_name_works() {
    assert_eq!(
        ComponentDatatype::from_name("BYTE"),
        Some(ComponentDatatype::Byte)
    );
    assert_eq!(
        ComponentDatatype::from_name("UNSIGNED_BYTE"),
        Some(ComponentDatatype::UnsignedByte)
    );
    assert_eq!(
        ComponentDatatype::from_name("SHORT"),
        Some(ComponentDatatype::Short)
    );
    assert_eq!(
        ComponentDatatype::from_name("UNSIGNED_SHORT"),
        Some(ComponentDatatype::UnsignedShort)
    );
    assert_eq!(
        ComponentDatatype::from_name("INT"),
        Some(ComponentDatatype::Int)
    );
    assert_eq!(
        ComponentDatatype::from_name("UNSIGNED_INT"),
        Some(ComponentDatatype::UnsignedInt)
    );
    assert_eq!(
        ComponentDatatype::from_name("FLOAT"),
        Some(ComponentDatatype::Float)
    );
    assert_eq!(
        ComponentDatatype::from_name("DOUBLE"),
        Some(ComponentDatatype::Double)
    );
    // Invalid name
    assert_eq!(ComponentDatatype::from_name("INVALID"), None);
}

#[test]
fn component_datatype_gl_value_roundtrip() {
    // Verify gl_value → from_gl_value roundtrip for all variants
    let all = [
        ComponentDatatype::Byte,
        ComponentDatatype::UnsignedByte,
        ComponentDatatype::Short,
        ComponentDatatype::UnsignedShort,
        ComponentDatatype::Int,
        ComponentDatatype::UnsignedInt,
        ComponentDatatype::Float,
        ComponentDatatype::Double,
    ];
    for dt in &all {
        assert_eq!(ComponentDatatype::from_gl_value(dt.gl_value()), Some(*dt));
    }
}

// ============================================================================
// IndexDatatype
// ============================================================================

#[test]
fn index_datatype_validate_validates_input() {
    assert!(IndexDatatype::UnsignedByte.validate());
    assert!(IndexDatatype::UnsignedShort.validate());
    assert!(IndexDatatype::UnsignedInt.validate());
}

#[test]
fn index_datatype_create_typed_array_logic() {
    // Maps to "createTypedArray creates array":
    // numberOfVertices < 65536 → UNSIGNED_SHORT (2 bytes)
    // numberOfVertices >= 65536 → UNSIGNED_INT (4 bytes)
    let dt = IndexDatatype::for_vertex_count(3);
    assert_eq!(dt.size_in_bytes(), 2); // Uint16Array.BYTES_PER_ELEMENT
    assert_eq!(dt, IndexDatatype::UnsignedShort);

    let dt = IndexDatatype::for_vertex_count(IndexDatatype::SIXTY_FOUR_KILOBYTES + 1);
    assert_eq!(dt.size_in_bytes(), 4); // Uint32Array.BYTES_PER_ELEMENT
    assert_eq!(dt, IndexDatatype::UnsignedInt);
}

#[test]
fn index_datatype_get_size_in_bytes_returns_size() {
    assert_eq!(IndexDatatype::UnsignedByte.size_in_bytes(), 1);
    assert_eq!(IndexDatatype::UnsignedShort.size_in_bytes(), 2);
    assert_eq!(IndexDatatype::UnsignedInt.size_in_bytes(), 4);
}

#[test]
fn index_datatype_from_name_works() {
    assert_eq!(
        IndexDatatype::from_name("UNSIGNED_BYTE"),
        Some(IndexDatatype::UnsignedByte)
    );
    assert_eq!(
        IndexDatatype::from_name("UNSIGNED_SHORT"),
        Some(IndexDatatype::UnsignedShort)
    );
    assert_eq!(
        IndexDatatype::from_name("UNSIGNED_INT"),
        Some(IndexDatatype::UnsignedInt)
    );
    assert_eq!(IndexDatatype::from_name("INVALID"), None);
}

#[test]
fn index_datatype_from_gl_value_works() {
    assert_eq!(
        IndexDatatype::from_gl_value(0x1401),
        Some(IndexDatatype::UnsignedByte)
    );
    assert_eq!(
        IndexDatatype::from_gl_value(0x1403),
        Some(IndexDatatype::UnsignedShort)
    );
    assert_eq!(
        IndexDatatype::from_gl_value(0x1405),
        Some(IndexDatatype::UnsignedInt)
    );
    // Invalid
    assert_eq!(IndexDatatype::from_gl_value(0x1400), None); // BYTE is not an index type
    assert_eq!(IndexDatatype::from_gl_value(0x9999), None);
}
