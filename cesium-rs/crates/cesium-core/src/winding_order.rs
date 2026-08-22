//! Ported from `packages/engine/Source/Core/WindingOrder.js`.
//!
//! Winding order defines the order of vertices for a triangle to be considered front-facing.

use crate::webgl_constants::WebGLConstants;

/// Winding order defines the order of vertices for a triangle to be considered front-facing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum WindingOrder {
    /// Vertices are in clockwise order.
    Clockwise = WebGLConstants::CW,
    /// Vertices are in counter-clockwise order.
    CounterClockwise = WebGLConstants::CCW,
}

impl WindingOrder {
    /// Validates that the provided winding order is a valid [`WindingOrder`].
    pub fn validate(winding_order: u32) -> bool {
        winding_order == WebGLConstants::CW || winding_order == WebGLConstants::CCW
    }

    /// Try to convert from a raw `u32` value.
    pub fn try_from_u32(value: u32) -> Option<Self> {
        match value {
            WebGLConstants::CW => Some(WindingOrder::Clockwise),
            WebGLConstants::CCW => Some(WindingOrder::CounterClockwise),
            _ => None,
        }
    }
}
