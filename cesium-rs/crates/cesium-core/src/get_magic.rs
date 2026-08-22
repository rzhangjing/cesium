//! Ported from `packages/engine/Source/Core/getMagic.js`.
//!
//! Reads a 4-byte magic number from a byte slice.

use crate::get_string_from_typed_array::get_string_from_typed_array;

/// Reads up to 4 bytes of magic string from a byte slice.
pub fn get_magic(uint8_array: &[u8], byte_offset: Option<usize>) -> String {
    let offset = byte_offset.unwrap_or(0);
    let len = std::cmp::min(4, uint8_array.len());
    get_string_from_typed_array(uint8_array, Some(offset), Some(len))
}
