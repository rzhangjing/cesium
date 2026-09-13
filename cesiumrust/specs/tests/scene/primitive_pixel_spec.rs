//! Core/PrimitiveTypeSpec.js + Core/PixelFormatSpec.js → Rust integration tests
//!
//! PrimitiveType: 3 it() → 3 A-class
//! PixelFormat: 5 it() → 2 A-class (3 C-class: WebGL context)
//! Total: 5 tests

use cesium_geospatial::wireframe::PrimitiveType;
use cesium_scene::render_state::PixelFormat;

// ─── PrimitiveType ─────────────────────────────────────────────────────────

#[test]
fn test_primitive_type_validate() {
    assert!(PrimitiveType::Points.validate());
    assert!(PrimitiveType::Lines.validate());
    assert!(PrimitiveType::LineLoop.validate());
    assert!(PrimitiveType::LineStrip.validate());
    assert!(PrimitiveType::Triangles.validate());
    assert!(PrimitiveType::TriangleStrip.validate());
    assert!(PrimitiveType::TriangleFan.validate());
}

#[test]
fn test_primitive_type_is_lines() {
    assert!(!PrimitiveType::Points.is_lines());
    assert!(PrimitiveType::Lines.is_lines());
    assert!(PrimitiveType::LineLoop.is_lines());
    assert!(PrimitiveType::LineStrip.is_lines());
    assert!(!PrimitiveType::Triangles.is_lines());
    assert!(!PrimitiveType::TriangleStrip.is_lines());
    assert!(!PrimitiveType::TriangleFan.is_lines());
}

#[test]
fn test_primitive_type_is_triangles() {
    assert!(!PrimitiveType::Points.is_triangles());
    assert!(!PrimitiveType::Lines.is_triangles());
    assert!(!PrimitiveType::LineLoop.is_triangles());
    assert!(!PrimitiveType::LineStrip.is_triangles());
    assert!(PrimitiveType::Triangles.is_triangles());
    assert!(PrimitiveType::TriangleStrip.is_triangles());
    assert!(PrimitiveType::TriangleFan.is_triangles());
}

// ─── PixelFormat ───────────────────────────────────────────────────────────

#[test]
fn test_pixel_format_flip_y() {
    let width = 1;
    let height = 2;
    let values: &[u8] = &[255, 0, 0, 0, 255, 0]; // row0=[255,0,0], row1=[0,255,0]
    let expected: &[u8] = &[0, 255, 0, 255, 0, 0]; // flipped

    let flipped = PixelFormat::flip_y(values, PixelFormat::Rgb, width, height);
    assert_eq!(flipped, expected);
}

#[test]
fn test_pixel_format_flip_y_height_1() {
    let width = 1;
    let height = 1;
    let values: &[u8] = &[255, 255, 255];

    let flipped = PixelFormat::flip_y(values, PixelFormat::Rgb, width, height);
    assert_eq!(flipped, values);
}
