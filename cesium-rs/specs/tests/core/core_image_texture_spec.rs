//! Tests for get_image_pixels, get_magic, get_string_from_typed_array,
//! get_json_from_typed_array, get_image_from_typed_array,
//! compressed_texture_buffer.

use cesium_core::compressed_texture_buffer::CompressedTextureBuffer;
use cesium_core::get_image_from_typed_array::GetImageFromTypedArray;
use cesium_core::get_image_pixels::get_image_pixels;
use cesium_core::get_json_from_typed_array::get_json_from_typed_array;
use cesium_core::get_magic::get_magic;
use cesium_core::get_string_from_typed_array::get_string_from_typed_array;

// --- get_image_pixels ---
#[test]
fn image_pixels_full_return() {
    let rgba = vec![255, 0, 0, 255, 0, 255, 0, 255]; // 2 pixels
    let result = get_image_pixels(&rgba, 2, 1, None, None);
    assert_eq!(result.len(), 8);
    assert_eq!(result, rgba);
}

#[test]
fn image_pixels_sub_rectangle() {
    // 2x2 image (16 bytes), request 1x2 sub-rectangle
    let rgba = vec![
        255, 0, 0, 255, 0, 255, 0, 255, // row 0
        0, 0, 255, 255, 255, 255, 0, 255, // row 1
    ];
    let result = get_image_pixels(&rgba, 2, 2, Some(1), Some(2));
    assert_eq!(result.len(), 1 * 2 * 4);
    // First pixel of each row
    assert_eq!(result[0..4], rgba[0..4]);
    assert_eq!(result[4..8], rgba[8..12]);
}

#[test]
fn image_pixels_defaults_to_image_dimensions() {
    let rgba = vec![1, 2, 3, 4]; // 1x1
    let result = get_image_pixels(&rgba, 1, 1, None, None);
    assert_eq!(result, rgba);
}

// --- get_string_from_typed_array ---
#[test]
fn string_from_typed_array_full() {
    let data = b"hello world";
    let s = get_string_from_typed_array(data, None, None);
    assert_eq!(s, "hello world");
}

#[test]
fn string_from_typed_array_with_offset() {
    let data = b"hello world";
    let s = get_string_from_typed_array(data, Some(6), None);
    assert_eq!(s, "world");
}

#[test]
fn string_from_typed_array_with_offset_and_length() {
    let data = b"hello world";
    let s = get_string_from_typed_array(data, Some(0), Some(5));
    assert_eq!(s, "hello");
}

#[test]
fn string_from_typed_array_empty() {
    let data = b"";
    let s = get_string_from_typed_array(data, None, None);
    assert_eq!(s, "");
}

// --- get_magic ---
#[test]
fn magic_reads_first_4_bytes() {
    let data = b"glTF\x02\x00\x00\x00";
    let m = get_magic(data, None);
    assert_eq!(m, "glTF");
}

#[test]
fn magic_with_offset() {
    let data = b"\x00\x00glTF";
    let m = get_magic(data, Some(2));
    assert_eq!(m, "glTF");
}

#[test]
fn magic_short_data() {
    let data = b"ab";
    let m = get_magic(data, None);
    assert_eq!(m, "ab");
}

#[test]
fn magic_empty_data() {
    let data = b"";
    let m = get_magic(data, None);
    assert_eq!(m, "");
}

// --- get_json_from_typed_array ---
#[test]
fn json_from_typed_array_full() {
    let data = br#"{"key":"value"}"#;
    let s = get_json_from_typed_array(data, None, None);
    assert_eq!(s, r#"{"key":"value"}"#);
}

#[test]
fn json_from_typed_array_with_offset() {
    let data = br#"prefix{"key":"value"}"#;
    let s = get_json_from_typed_array(data, Some(6), Some(15));
    assert_eq!(s, r#"{"key":"value"}"#);
}

// --- get_image_from_typed_array (stub) ---
#[test]
fn get_image_from_typed_array_new() {
    let _ = GetImageFromTypedArray::new();
    let _ = GetImageFromTypedArray::default();
}

// --- CompressedTextureBuffer ---
#[test]
fn compressed_texture_buffer_new() {
    let buf = CompressedTextureBuffer::new(0x83F1, 0x1401, 256, 256, vec![0u8; 1024]);
    assert_eq!(buf.internal_format(), 0x83F1);
    assert_eq!(buf.pixel_datatype(), 0x1401);
    assert_eq!(buf.width(), 256);
    assert_eq!(buf.height(), 256);
}

#[test]
fn compressed_texture_buffer_buffer_view() {
    let data = vec![1, 2, 3, 4];
    let buf = CompressedTextureBuffer::new(0, 0, 2, 2, data.clone());
    assert_eq!(buf.buffer_view(), &data[..]);
    assert_eq!(buf.array_buffer_view(), &data[..]);
}

#[test]
fn compressed_texture_buffer_clone() {
    let buf = CompressedTextureBuffer::new(1, 2, 4, 4, vec![0; 64]);
    let cloned = buf.clone();
    assert_eq!(cloned.internal_format(), 1);
    assert_eq!(cloned.width(), 4);
}
