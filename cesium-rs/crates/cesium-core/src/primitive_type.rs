//! Ported from `packages/engine/Source/Core/PrimitiveType.js`.
//!
//! The type of a geometric primitive, i.e., points, lines, and triangles.

use crate::webgl_constants::WebGLConstants;

/// The type of a geometric primitive, i.e., points, lines, and triangles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum PrimitiveType {
    /// Points primitive where each vertex (or index) is a separate point.
    Points = WebGLConstants::POINTS,
    /// Lines primitive where each two vertices (or indices) is a line segment.
    Lines = WebGLConstants::LINES,
    /// Line loop primitive where each vertex after the first connects a line to
    /// the previous vertex, and the last vertex implicitly connects to the first.
    LineLoop = WebGLConstants::LINE_LOOP,
    /// Line strip primitive where each vertex after the first connects a line to
    /// the previous vertex.
    LineStrip = WebGLConstants::LINE_STRIP,
    /// Triangles primitive where each three vertices (or indices) is a triangle.
    Triangles = WebGLConstants::TRIANGLES,
    /// Triangle strip primitive where each vertex after the first two connect to
    /// the previous two vertices forming a triangle.
    TriangleStrip = WebGLConstants::TRIANGLE_STRIP,
    /// Triangle fan primitive where each vertex after the first two connect to
    /// the previous vertex and the first vertex forming a triangle.
    TriangleFan = WebGLConstants::TRIANGLE_FAN,
}

impl PrimitiveType {
    /// Returns `true` if the primitive type is a line variant.
    pub fn is_lines(self) -> bool {
        matches!(
            self,
            PrimitiveType::Lines | PrimitiveType::LineLoop | PrimitiveType::LineStrip
        )
    }

    /// Returns `true` if the primitive type is a triangle variant.
    pub fn is_triangles(self) -> bool {
        matches!(
            self,
            PrimitiveType::Triangles | PrimitiveType::TriangleStrip | PrimitiveType::TriangleFan
        )
    }

    /// Validates that the provided primitive type is a valid [`PrimitiveType`].
    pub fn validate(primitive_type: u32) -> bool {
        matches!(
            primitive_type,
            WebGLConstants::POINTS
                | WebGLConstants::LINES
                | WebGLConstants::LINE_LOOP
                | WebGLConstants::LINE_STRIP
                | WebGLConstants::TRIANGLES
                | WebGLConstants::TRIANGLE_STRIP
                | WebGLConstants::TRIANGLE_FAN
        )
    }

    /// Try to convert from a raw `u32` value.
    pub fn try_from_u32(value: u32) -> Option<Self> {
        match value {
            WebGLConstants::POINTS => Some(PrimitiveType::Points),
            WebGLConstants::LINES => Some(PrimitiveType::Lines),
            WebGLConstants::LINE_LOOP => Some(PrimitiveType::LineLoop),
            WebGLConstants::LINE_STRIP => Some(PrimitiveType::LineStrip),
            WebGLConstants::TRIANGLES => Some(PrimitiveType::Triangles),
            WebGLConstants::TRIANGLE_STRIP => Some(PrimitiveType::TriangleStrip),
            WebGLConstants::TRIANGLE_FAN => Some(PrimitiveType::TriangleFan),
            _ => None,
        }
    }
}
