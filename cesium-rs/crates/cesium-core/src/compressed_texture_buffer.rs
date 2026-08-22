//! Ported from `packages/engine/Source/Core/CompressedTextureBuffer.js`.

/// Describes a compressed texture and contains a compressed texture buffer.
#[derive(Debug, Clone)]
pub struct CompressedTextureBuffer {
    format: u32,
    datatype: u32,
    width: u32,
    height: u32,
    buffer: Vec<u8>,
}

impl CompressedTextureBuffer {
    pub fn new(
        internal_format: u32,
        pixel_datatype: u32,
        width: u32,
        height: u32,
        buffer: Vec<u8>,
    ) -> Self {
        Self {
            format: internal_format,
            datatype: pixel_datatype,
            width,
            height,
            buffer,
        }
    }

    /// The format of the compressed texture.
    pub fn internal_format(&self) -> u32 {
        self.format
    }

    /// The datatype of the compressed texture.
    pub fn pixel_datatype(&self) -> u32 {
        self.datatype
    }

    /// The width of the texture.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The height of the texture.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The compressed texture buffer.
    pub fn buffer_view(&self) -> &[u8] {
        &self.buffer
    }

    /// The compressed texture buffer. Alias for buffer_view.
    pub fn array_buffer_view(&self) -> &[u8] {
        &self.buffer
    }
}
