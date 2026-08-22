//! Ported from `packages/engine/Source/Core/IndexDatatype.js`.
//!
//! Constants for WebGL index datatypes.

use crate::webgl_constants::WebGLConstants;

/// Constants for WebGL index datatypes. These correspond to the `type`
/// parameter of `gl.drawElements`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum IndexDatatype {
    /// 8-bit unsigned byte.
    UnsignedByte = WebGLConstants::UNSIGNED_BYTE,
    /// 16-bit unsigned short.
    UnsignedShort = WebGLConstants::UNSIGNED_SHORT,
    /// 32-bit unsigned int.
    UnsignedInt = WebGLConstants::UNSIGNED_INT,
}

impl IndexDatatype {
    /// Returns the size, in bytes, of the corresponding datatype.
    ///
    /// # Panics
    /// Panics (debug) if the value is not a valid `IndexDatatype`.
    pub fn size_in_bytes(self) -> usize {
        match self {
            IndexDatatype::UnsignedByte => 1,
            IndexDatatype::UnsignedShort => 2,
            IndexDatatype::UnsignedInt => 4,
        }
    }

    /// Gets the datatype with a given size in bytes.
    ///
    /// # Panics
    /// Panics (debug) if `size_in_bytes` is not 1, 2, or 4.
    pub fn from_size_in_bytes(size_in_bytes: usize) -> Self {
        debug_assert!(
            matches!(size_in_bytes, 1 | 2 | 4),
            "Size in bytes cannot be mapped to an IndexDatatype"
        );
        match size_in_bytes {
            1 => IndexDatatype::UnsignedByte,
            2 => IndexDatatype::UnsignedShort,
            4 => IndexDatatype::UnsignedInt,
            _ => IndexDatatype::UnsignedShort, // unreachable in debug
        }
    }

    /// Validates that the provided index datatype is valid.
    pub fn validate(index_datatype: u32) -> bool {
        matches!(
            index_datatype,
            WebGLConstants::UNSIGNED_BYTE
                | WebGLConstants::UNSIGNED_SHORT
                | WebGLConstants::UNSIGNED_INT
        )
    }

    /// Creates an index storage vector for the given number of vertices.
    ///
    /// If `number_of_vertices >= 65536`, returns `Vec<u32>`; otherwise `Vec<u16>`.
    /// The returned vector is zero-filled with `length` elements.
    pub fn create_typed_array(
        number_of_vertices: usize,
        length: usize,
    ) -> IndexStorage {
        if number_of_vertices >= 65536 {
            IndexStorage::U32(vec![0u32; length])
        } else {
            IndexStorage::U16(vec![0u16; length])
        }
    }

    /// Gets the `IndexDatatype` for a given storage variant.
    pub fn from_storage(storage: &IndexStorage) -> Self {
        match storage {
            IndexStorage::U16(_) => IndexDatatype::UnsignedShort,
            IndexStorage::U32(_) => IndexDatatype::UnsignedInt,
        }
    }

    /// Try to convert from a raw `u32` value.
    pub fn try_from_u32(value: u32) -> Option<Self> {
        match value {
            WebGLConstants::UNSIGNED_BYTE => Some(IndexDatatype::UnsignedByte),
            WebGLConstants::UNSIGNED_SHORT => Some(IndexDatatype::UnsignedShort),
            WebGLConstants::UNSIGNED_INT => Some(IndexDatatype::UnsignedInt),
            _ => None,
        }
    }
}

/// Storage for index data — either 16-bit or 32-bit depending on vertex count.
#[derive(Debug, Clone)]
pub enum IndexStorage {
    /// 16-bit indices (up to 65535 vertices).
    U16(Vec<u16>),
    /// 32-bit indices (65536+ vertices).
    U32(Vec<u32>),
}

impl IndexStorage {
    /// Returns the number of indices.
    pub fn len(&self) -> usize {
        match self {
            IndexStorage::U16(v) => v.len(),
            IndexStorage::U32(v) => v.len(),
        }
    }

    /// Returns `true` if empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Push a `u32` index value (will be narrowed to `u16` if needed).
    pub fn push(&mut self, value: u32) {
        match self {
            IndexStorage::U16(v) => v.push(value as u16),
            IndexStorage::U32(v) => v.push(value),
        }
    }
}
