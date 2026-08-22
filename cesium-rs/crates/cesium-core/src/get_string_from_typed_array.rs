//! Ported from `packages/engine/Source/Core/getStringFromTypedArray.js`.
//!
//! Reads a UTF-8 string from a byte slice.

/// Decodes a UTF-8 string from a byte slice.
pub fn get_string_from_typed_array(
    uint8_array: &[u8],
    byte_offset: Option<usize>,
    byte_length: Option<usize>,
) -> String {
    let offset = byte_offset.unwrap_or(0);
    let length = byte_length.unwrap_or(uint8_array.len() - offset);
    let sub = &uint8_array[offset..offset + length];
    String::from_utf8_lossy(sub).into_owned()
}
