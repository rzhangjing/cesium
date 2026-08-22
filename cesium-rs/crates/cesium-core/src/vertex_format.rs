//! Ported from `packages/engine/Source/Core/VertexFormat.js`.
//!
//! A vertex format defines what attributes make up a vertex.

/// A vertex format defines what attributes make up a vertex. A `VertexFormat` can
/// be provided to a [`Geometry`](crate::Geometry) to request that certain
/// properties be computed, e.g., just position, position and normal, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct VertexFormat {
    /// When `true`, the vertex has a 3D position attribute (64-bit float, 3 components).
    pub position: bool,
    /// When `true`, the vertex has a normal attribute (32-bit float, 3 components).
    pub normal: bool,
    /// When `true`, the vertex has a 2D texture coordinate attribute (32-bit float, 2 components).
    pub st: bool,
    /// When `true`, the vertex has a bitangent attribute (32-bit float, 3 components).
    pub bitangent: bool,
    /// When `true`, the vertex has a tangent attribute (32-bit float, 3 components).
    pub tangent: bool,
    /// When `true`, the vertex has an RGB color attribute (8-bit unsigned byte, 3 components).
    pub color: bool,
}

impl Default for VertexFormat {
    fn default() -> Self {
        Self {
            position: false,
            normal: false,
            st: false,
            bitangent: false,
            tangent: false,
            color: false,
        }
    }
}

impl VertexFormat {
    /// Creates a new `VertexFormat` with the specified boolean flags.
    pub fn new(
        position: bool,
        normal: bool,
        st: bool,
        bitangent: bool,
        tangent: bool,
        color: bool,
    ) -> Self {
        Self {
            position,
            normal,
            st,
            bitangent,
            tangent,
            color,
        }
    }

    /// An immutable vertex format with only a position attribute.
    pub fn position_only() -> Self {
        Self {
            position: true,
            ..Default::default()
        }
    }

    /// An immutable vertex format with position and normal attributes.
    pub fn position_and_normal() -> Self {
        Self {
            position: true,
            normal: true,
            ..Default::default()
        }
    }

    /// An immutable vertex format with position, normal, and st attributes.
    pub fn position_normal_and_st() -> Self {
        Self {
            position: true,
            normal: true,
            st: true,
            ..Default::default()
        }
    }

    /// An immutable vertex format with position and st attributes.
    pub fn position_and_st() -> Self {
        Self {
            position: true,
            st: true,
            ..Default::default()
        }
    }

    /// An immutable vertex format with position and color attributes.
    pub fn position_and_color() -> Self {
        Self {
            position: true,
            color: true,
            ..Default::default()
        }
    }

    /// An immutable vertex format with all attributes: position, normal, st, tangent, bitangent.
    pub fn all() -> Self {
        Self {
            position: true,
            normal: true,
            st: true,
            tangent: true,
            bitangent: true,
            color: false,
        }
    }

    /// The default vertex format: position, normal, and st.
    pub fn default_format() -> Self {
        Self::position_normal_and_st()
    }

    /// The number of elements used to pack the object into an array.
    pub const PACKED_LENGTH: usize = 6;

    /// Stores the provided instance into the provided array.
    ///
    /// # Panics (debug)
    /// Panics if `array.len() < starting_index + Self::PACKED_LENGTH`.
    pub fn pack(&self, array: &mut [f64], starting_index: usize) {
        debug_assert!(
            array.len() >= starting_index + Self::PACKED_LENGTH,
            "array too small"
        );
        array[starting_index] = if self.position { 1.0 } else { 0.0 };
        array[starting_index + 1] = if self.normal { 1.0 } else { 0.0 };
        array[starting_index + 2] = if self.st { 1.0 } else { 0.0 };
        array[starting_index + 3] = if self.tangent { 1.0 } else { 0.0 };
        array[starting_index + 4] = if self.bitangent { 1.0 } else { 0.0 };
        array[starting_index + 5] = if self.color { 1.0 } else { 0.0 };
    }

    /// Retrieves an instance from a packed array.
    pub fn unpack(array: &[f64], starting_index: usize, result: Option<Self>) -> Self {
        let mut r = result.unwrap_or_default();
        r.position = array[starting_index] == 1.0;
        r.normal = array[starting_index + 1] == 1.0;
        r.st = array[starting_index + 2] == 1.0;
        r.tangent = array[starting_index + 3] == 1.0;
        r.bitangent = array[starting_index + 4] == 1.0;
        r.color = array[starting_index + 5] == 1.0;
        r
    }

    /// Duplicates a `VertexFormat` instance.
    pub fn clone_into(&self, result: Option<Self>) -> Self {
        result.unwrap_or_else(|| self.clone())
    }
}
