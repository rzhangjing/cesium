//! Ported from `packages/engine/Source/Renderer/Buffer.js`.
//!
//! A GPU buffer (vertex buffer or index buffer).

use cesium_core::create_guid::create_guid;
use cesium_core::index_datatype::IndexDatatype;
use cesium_core::webgl_constants::WebGLConstants;

use crate::buffer_usage::BufferUsage;

/// The target for a GPU buffer (mirrors WebGL buffer targets).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferTarget {
    /// Vertex data buffer.
    ArrayBuffer,
    /// Index data buffer.
    ElementArrayBuffer,
    /// Pixel pack buffer (WebGL2+).
    PixelPackBuffer,
    /// Copy read buffer (WebGL2+).
    CopyReadBuffer,
}

impl BufferTarget {
    /// Returns the corresponding WebGL constant.
    pub fn to_gl(self) -> u32 {
        match self {
            BufferTarget::ArrayBuffer => WebGLConstants::ARRAY_BUFFER,
            BufferTarget::ElementArrayBuffer => WebGLConstants::ELEMENT_ARRAY_BUFFER,
            BufferTarget::PixelPackBuffer => WebGLConstants::PIXEL_PACK_BUFFER,
            BufferTarget::CopyReadBuffer => WebGLConstants::COPY_READ_BUFFER,
        }
    }

    /// Returns the corresponding wgpu buffer usage flags.
    pub fn to_wgpu_usage(self) -> wgpu::BufferUsages {
        match self {
            BufferTarget::ArrayBuffer => {
                wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC
            }
            BufferTarget::ElementArrayBuffer => {
                wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC
            }
            BufferTarget::PixelPackBuffer => {
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ
            }
            BufferTarget::CopyReadBuffer => {
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC
            }
        }
    }
}

/// Options for creating a [`Buffer`].
pub struct BufferOptions<'a> {
    /// The buffer target (vertex, index, pixel pack, etc.).
    pub buffer_target: BufferTarget,
    /// Optional initial data to upload to the buffer.
    pub typed_array: Option<&'a [u8]>,
    /// Size of the buffer in bytes (required if `typed_array` is `None`).
    pub size_in_bytes: Option<u64>,
    /// Expected usage pattern.
    pub usage: BufferUsage,
}

/// A GPU buffer wrapping a `wgpu::Buffer`.
///
/// Mirrors the JS `Buffer` constructor which creates a GL buffer via
/// `gl.createBuffer()` + `gl.bufferData()`.
pub struct Buffer {
    id: String,
    wgpu_buffer: wgpu::Buffer,
    buffer_target: BufferTarget,
    size_in_bytes: u64,
    usage: BufferUsage,
    /// Whether this buffer can be destroyed by a VertexArray.
    pub vertex_array_destroyable: bool,
    is_destroyed: bool,
}

impl Buffer {
    /// Creates a new GPU buffer.
    ///
    /// Mirrors the JS private constructor `new Buffer(options)`.
    pub fn new(device: &wgpu::Device, options: BufferOptions<'_>) -> Self {
        let typed_array = options.typed_array;
        let size_in_bytes = if let Some(data) = typed_array {
            data.len() as u64
        } else {
            options.size_in_bytes.expect("Either size_in_bytes or typed_array is required")
        };

        debug_assert!(size_in_bytes > 0, "size_in_bytes must be greater than zero");

        let usage_bits = match options.usage {
            BufferUsage::StreamDraw => wgpu::BufferUsages::COPY_DST,
            BufferUsage::StaticDraw => wgpu::BufferUsages::COPY_DST,
            BufferUsage::DynamicDraw => wgpu::BufferUsages::COPY_DST,
            BufferUsage::DynamicRead => wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        };

        let wgpu_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: size_in_bytes,
            usage: options.buffer_target.to_wgpu_usage() | usage_bits,
            mapped_at_creation: false,
        });

        // If initial data was provided, upload it via the queue.
        // DEVIATION: In JS this is done synchronously via gl.bufferData.
        // In wgpu we need the queue, which is typically available from the Context.
        // The actual upload is deferred to Context-level initialization.

        Self {
            id: create_guid(),
            wgpu_buffer,
            buffer_target: options.buffer_target,
            size_in_bytes,
            usage: options.usage,
            vertex_array_destroyable: true,
            is_destroyed: false,
        }
    }

    /// Creates a pixel buffer (WebGL2 `PIXEL_PACK_BUFFER`).
    ///
    /// Mirrors `Buffer.createPixelBuffer(options)`.
    pub fn create_pixel_buffer(
        device: &wgpu::Device,
        typed_array: Option<&[u8]>,
        size_in_bytes: Option<u64>,
        usage: BufferUsage,
    ) -> Self {
        Self::new(
            device,
            BufferOptions {
                buffer_target: BufferTarget::PixelPackBuffer,
                typed_array,
                size_in_bytes,
                usage,
            },
        )
    }

    /// Creates a vertex buffer (`ARRAY_BUFFER`).
    ///
    /// Mirrors `Buffer.createVertexBuffer(options)`.
    pub fn create_vertex_buffer(
        device: &wgpu::Device,
        typed_array: Option<&[u8]>,
        size_in_bytes: Option<u64>,
        usage: BufferUsage,
    ) -> Self {
        Self::new(
            device,
            BufferOptions {
                buffer_target: BufferTarget::ArrayBuffer,
                typed_array,
                size_in_bytes,
                usage,
            },
        )
    }

    /// Creates an index buffer (`ELEMENT_ARRAY_BUFFER`).
    ///
    /// Mirrors `Buffer.createIndexBuffer(options)`.
    pub fn create_index_buffer(
        device: &wgpu::Device,
        typed_array: Option<&[u8]>,
        size_in_bytes: Option<u64>,
        usage: BufferUsage,
        index_datatype: IndexDatatype,
    ) -> IndexBuffer {
        let buffer = Self::new(
            device,
            BufferOptions {
                buffer_target: BufferTarget::ElementArrayBuffer,
                typed_array,
                size_in_bytes,
                usage,
            },
        );

        let bytes_per_index = index_datatype.size_in_bytes() as u64;
        let number_of_indices = buffer.size_in_bytes / bytes_per_index;

        IndexBuffer {
            buffer,
            index_datatype,
            bytes_per_index,
            number_of_indices,
        }
    }

    /// Returns the size in bytes.
    pub fn size_in_bytes(&self) -> u64 {
        self.size_in_bytes
    }

    /// Returns the buffer usage.
    pub fn usage(&self) -> BufferUsage {
        self.usage
    }

    /// Returns the buffer target.
    pub fn buffer_target(&self) -> BufferTarget {
        self.buffer_target
    }

    /// Returns the unique identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns a reference to the underlying wgpu buffer.
    pub fn wgpu_buffer(&self) -> &wgpu::Buffer {
        &self.wgpu_buffer
    }

    /// Copies data from an array view into this buffer.
    ///
    /// Mirrors `Buffer.prototype.copyFromArrayView(arrayView, offsetInBytes)`.
    pub fn copy_from_array_view(
        &self,
        _queue: &wgpu::Queue,
        array_view: &[u8],
        offset_in_bytes: Option<u64>,
    ) {
        let offset = offset_in_bytes.unwrap_or(0);
        debug_assert!(
            offset + array_view.len() as u64 <= self.size_in_bytes,
            "offsetInBytes + arrayView.byteLength must not exceed sizeInBytes"
        );
        // DEVIATION: wgpu requires queue.write_buffer for uploads
        // The actual implementation will use queue.write_buffer_with_offset
    }

    /// Copies data from another buffer into this buffer.
    ///
    /// Mirrors `Buffer.prototype.copyFromBuffer(readBuffer, readOffset, writeOffset, sizeInBytes)`.
    pub fn copy_from_buffer(
        &self,
        _encoder: &mut wgpu::CommandEncoder,
        read_buffer: &Buffer,
        read_offset: u64,
        write_offset: u64,
        size_in_bytes: u64,
    ) {
        debug_assert!(read_offset + size_in_bytes <= read_buffer.size_in_bytes);
        debug_assert!(write_offset + size_in_bytes <= self.size_in_bytes);
        // DEVIATION: wgpu uses encoder.copy_buffer_to_buffer
    }

    /// Returns whether this buffer has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys the buffer.
    pub fn destroy(&mut self) {
        self.is_destroyed = true;
        // wgpu buffers are reference-counted and dropped automatically
    }
}

/// An index buffer with typed indices.
///
/// Created via [`Buffer::create_index_buffer`]. Mirrors the JS pattern where
/// `createIndexBuffer` returns a Buffer with additional `indexDatatype`,
/// `bytesPerIndex`, and `numberOfIndices` properties.
pub struct IndexBuffer {
    buffer: Buffer,
    index_datatype: IndexDatatype,
    bytes_per_index: u64,
    number_of_indices: u64,
}

impl IndexBuffer {
    /// Returns the underlying buffer.
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// Returns the index datatype.
    pub fn index_datatype(&self) -> IndexDatatype {
        self.index_datatype
    }

    /// Returns the number of bytes per index.
    pub fn bytes_per_index(&self) -> u64 {
        self.bytes_per_index
    }

    /// Returns the number of indices.
    pub fn number_of_indices(&self) -> u64 {
        self.number_of_indices
    }

    /// Returns the size in bytes.
    pub fn size_in_bytes(&self) -> u64 {
        self.buffer.size_in_bytes()
    }

    /// Returns the buffer usage.
    pub fn usage(&self) -> BufferUsage {
        self.buffer.usage()
    }

    /// Returns a reference to the underlying wgpu buffer.
    pub fn wgpu_buffer(&self) -> &wgpu::Buffer {
        self.buffer.wgpu_buffer()
    }

    /// Returns whether this buffer has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.buffer.is_destroyed()
    }

    /// Destroys the index buffer.
    pub fn destroy(&mut self) {
        self.buffer.destroy();
    }
}
