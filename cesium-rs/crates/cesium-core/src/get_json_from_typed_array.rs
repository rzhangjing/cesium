//! Ported from `packages/engine/Source/Core/getJsonFromTypedArray.js`.
//!
//! Parses JSON from a byte slice.

use crate::get_string_from_typed_array::get_string_from_typed_array;

/// Parses JSON from a byte slice, returning the raw JSON string.
///
/// In Rust, callers should deserialize the returned string using `serde_json`
/// or a similar library.
pub fn get_json_from_typed_array(
    uint8_array: &[u8],
    byte_offset: Option<usize>,
    byte_length: Option<usize>,
) -> String {
    get_string_from_typed_array(uint8_array, byte_offset, byte_length)
}
