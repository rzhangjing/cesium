//! Tests for `cesium_core::pixel_format`.
//!
//! Mirrors `packages/engine/Specs/Core/PixelFormatSpec.js`.

use cesium_core::pixel_format::{PixelFormat, TypedArray, PIXEL_DATATYPE_HALF_FLOAT};
use cesium_core::webgl_constants::WebGLConstants;

#[test]
fn flip_y_works() {
    let width = 1;
    let height = 2;
    let values = vec![255u8, 0, 0, 0, 255, 0];
    let expected_values = vec![0u8, 255, 0, 255, 0, 0];
    let data_buffer = TypedArray::U8(values);

    let flipped = PixelFormat::flip_y(
        &data_buffer,
        PixelFormat::Rgb,
        WebGLConstants::UNSIGNED_BYTE,
        width,
        height,
    );
    assert_eq!(flipped, TypedArray::U8(expected_values));
}

#[test]
fn flip_y_returns_early_if_height_is_1() {
    let width = 1;
    let height = 1;
    let values = vec![255u8, 255, 255];
    let data_buffer = TypedArray::U8(values.clone());

    let flipped = PixelFormat::flip_y(
        &data_buffer,
        PixelFormat::Rgb,
        WebGLConstants::UNSIGNED_BYTE,
        width,
        height,
    );
    // DEVIATION: JS asserts identity (`toBe(dataBuffer)`); Rust returns an
    // equivalent buffer because `flip_y` takes the view by reference.
    assert_eq!(flipped, TypedArray::U8(values));
}

#[test]
fn returns_the_correct_internal_formats_for_float() {
    let internal_format_r32f =
        PixelFormat::Red.to_internal_format(WebGLConstants::FLOAT, true);
    assert_eq!(internal_format_r32f, WebGLConstants::R32F);

    let internal_format_rg32f =
        PixelFormat::Rg.to_internal_format(WebGLConstants::FLOAT, true);
    assert_eq!(internal_format_rg32f, WebGLConstants::RG32F);

    let internal_format_rgb32f =
        PixelFormat::Rgb.to_internal_format(WebGLConstants::FLOAT, true);
    assert_eq!(internal_format_rgb32f, WebGLConstants::RGB32F);

    let internal_format_rgba32f =
        PixelFormat::Rgba.to_internal_format(WebGLConstants::FLOAT, true);
    assert_eq!(internal_format_rgba32f, WebGLConstants::RGBA32F);
}

#[test]
fn returns_the_correct_internal_formats_for_half_float() {
    let internal_format_r16f =
        PixelFormat::Red.to_internal_format(PIXEL_DATATYPE_HALF_FLOAT, true);
    assert_eq!(internal_format_r16f, WebGLConstants::R16F);

    let internal_format_rg16f =
        PixelFormat::Rg.to_internal_format(PIXEL_DATATYPE_HALF_FLOAT, true);
    assert_eq!(internal_format_rg16f, WebGLConstants::RG16F);

    let internal_format_rgb16f =
        PixelFormat::Rgb.to_internal_format(PIXEL_DATATYPE_HALF_FLOAT, true);
    assert_eq!(internal_format_rgb16f, WebGLConstants::RGB16F);

    let internal_format_rgba16f =
        PixelFormat::Rgba.to_internal_format(PIXEL_DATATYPE_HALF_FLOAT, true);
    assert_eq!(internal_format_rgba16f, WebGLConstants::RGBA16F);
}

#[test]
fn returns_the_correct_internal_formats_for_unsigned_byte() {
    let internal_format_r8 =
        PixelFormat::Red.to_internal_format(WebGLConstants::UNSIGNED_BYTE, true);
    assert_eq!(internal_format_r8, WebGLConstants::R8);

    let internal_format_rg8 =
        PixelFormat::Rg.to_internal_format(WebGLConstants::UNSIGNED_BYTE, true);
    assert_eq!(internal_format_rg8, WebGLConstants::RG8);

    let internal_format_rgb8 =
        PixelFormat::Rgb.to_internal_format(WebGLConstants::UNSIGNED_BYTE, true);
    assert_eq!(internal_format_rgb8, WebGLConstants::RGB8);

    let internal_format_rgba8 =
        PixelFormat::Rgba.to_internal_format(WebGLConstants::UNSIGNED_BYTE, true);
    assert_eq!(internal_format_rgba8, WebGLConstants::RGBA8);
}

#[test]
fn create_typed_array_rgba_2x2() {
    // Mirrors the D4 differential case `pf.createTypedArray.RGBA.2x2`:
    // `new Uint8Array(componentsLength(RGBA) * 2 * 2)` => 16 zeroed elements.
    let array = PixelFormat::Rgba.create_typed_array(
        WebGLConstants::UNSIGNED_BYTE,
        2,
        2,
    );
    assert_eq!(array, TypedArray::U8(vec![0u8; 16]));
}

#[test]
fn validate_recognizes_all_formats() {
    let valid: [u32; 26] = [
        0x1902, 0x84F9, 0x1906, 0x1903, 0x8227, 0x1907, 0x1908, 0x8D94, 0x8228,
        0x8D98, 0x8D99, 0x1909, 0x190A, 0x83F0, 0x83F1, 0x83F2, 0x83F3, 0x8C00,
        0x8C01, 0x8C02, 0x8C03, 0x93B0, 0x8D64, 0x9274, 0x9278, 0x8E8C,
    ];
    for value in valid {
        assert!(PixelFormat::validate(value), "expected {value:#x} to be valid");
    }
    assert!(!PixelFormat::validate(0));
    assert!(!PixelFormat::validate(0x1401));
}
