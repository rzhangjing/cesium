//! Ported from `packages/engine/Source/Renderer/VertexArray.js`.
//!
//! Defines the structure of vertex data for rendering.

use crate::buffer::{Buffer, IndexBuffer};

/// A vertex attribute descriptor.
pub struct VertexAttribute {
    /// The index of the attribute.
    pub index: u32,
    /// The buffer providing data for this attribute.
    pub buffer: Buffer,
    /// The number of components per vertex attribute (1-4).
    pub components_per_attribute: u32,
    /// The data type of each component.
    pub component_datatype: wgpu::VertexFormat,
    /// Whether integer data should be normalized.
    pub normalize: bool,
    /// The byte offset between consecutive attributes.
    pub stride_in_bytes: u32,
    /// The byte offset to the first attribute in the buffer.
    pub offset_in_bytes: u32,
}

/// A vertex array defining the structure of vertex data.
///
/// DEVIATION: In wgpu, vertex layouts are described in the render pipeline
/// descriptor rather than as persistent objects. This struct captures the
/// logical vertex layout.
pub struct VertexArray {
    attributes: Vec<VertexAttribute>,
    index_buffer: Option<IndexBuffer>,
    is_destroyed: bool,
}

impl VertexArray {
    /// Creates a new vertex array.
    pub fn new(attributes: Vec<VertexAttribute>, index_buffer: Option<IndexBuffer>) -> Self {
        Self { attributes, index_buffer, is_destroyed: false }
    }

    /// Returns the vertex attributes.
    pub fn attributes(&self) -> &[VertexAttribute] { &self.attributes }

    /// Returns the index buffer, if any.
    pub fn index_buffer(&self) -> Option<&IndexBuffer> { self.index_buffer.as_ref() }

    /// Returns whether this vertex array has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys the vertex array.
    pub fn destroy(&mut self) {
        for attr in &mut self.attributes {
            attr.buffer.destroy();
        }
        if let Some(ref mut ib) = self.index_buffer {
            ib.destroy();
        }
        self.is_destroyed = true;
    }
}
