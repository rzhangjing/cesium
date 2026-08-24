//! Ported from `packages/engine/Source/Scene/GltfIndexBufferLoader.js`.
//!
//! Loads an index buffer from a glTF accessor.
//!
//! The GPU index buffer itself is created by [`GltfIndexBufferLoader::
//! create_buffer`] against a renderer [`Context`] once the load has
//! decoded the indices (mirrors `Buffer.createIndexBuffer` inside the JS
//! `loadBuffer` job).
//!
//! DEVIATION: the job scheduler and Draco decoding remain deferred;
//! `load_buffer` keeps the decoded indices pending until `create_buffer`
//! uploads them (the JS uploads through the ResourceCache job queue).
//!
//! DEVIATION: wgpu has no 8-bit index format, so `UNSIGNED_BYTE` indices
//! are widened to 16-bit at GPU upload time (WebGL accepts Uint8 indices
//! directly).
//!
//! DEVIATION: the JS loader obtains the buffer view through the
//! `ResourceCache`; the Rust port composes a [`GltfBufferViewLoader`]
//! directly against the in-memory glTF (embedded buffers) or caller
//! supplied external bytes.

use cesium_core::index_datatype::IndexDatatype;
use cesium_core::runtime_error::RuntimeError;
use cesium_renderer::buffer::IndexBuffer;
use cesium_renderer::buffer_usage::BufferUsage;
use cesium_renderer::context::Context;

use crate::gltf_buffer_view_loader::GltfBufferViewLoader;
use crate::gltf_loader::GltfJson;
use crate::resource_loader_state::ResourceLoaderState;

/// The decoded indices, mirroring the JS union
/// `Uint8Array | Uint16Array | Uint32Array`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndicesTypedArray {
    /// 8-bit indices (`IndexDatatype.UNSIGNED_BYTE`).
    U8(Vec<u8>),
    /// 16-bit indices (`IndexDatatype.UNSIGNED_SHORT`).
    U16(Vec<u16>),
    /// 32-bit indices (`IndexDatatype.UNSIGNED_INT`).
    U32(Vec<u32>),
}

impl IndicesTypedArray {
    /// The number of indices.
    pub fn len(&self) -> usize {
        match self {
            IndicesTypedArray::U8(v) => v.len(),
            IndicesTypedArray::U16(v) => v.len(),
            IndicesTypedArray::U32(v) => v.len(),
        }
    }

    /// Returns `true` when there are no indices.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The size in bytes of the underlying storage.
    pub fn byte_length(&self) -> usize {
        match self {
            IndicesTypedArray::U8(v) => v.len(),
            IndicesTypedArray::U16(v) => v.len() * 2,
            IndicesTypedArray::U32(v) => v.len() * 4,
        }
    }
}

/// Options for [`GltfIndexBufferLoader::try_new`], mirroring the JS
/// constructor's `options` object.
pub struct GltfIndexBufferLoaderOptions {
    /// The accessor ID corresponding to the index buffer.
    pub accessor_id: u32,
    /// The Draco extension object (`KHR_draco_mesh_compression`). Draco
    /// decoding is deferred; supplying this returns a load error.
    pub draco: Option<serde_json::Value>,
    /// The cache key of the resource.
    pub cache_key: Option<String>,
    /// Load the index buffer as a GPU index buffer (the decoded indices
    /// are kept pending until [`GltfIndexBufferLoader::create_buffer`]
    /// uploads them).
    pub load_buffer: bool,
    /// Load the index buffer as a typed array.
    pub load_typed_array: bool,
}

/// Loads an index buffer from a glTF accessor.
///
/// Rust analogue of the CesiumJS `GltfIndexBufferLoader` (`ResourceLoader`
/// interface); see module docs for the CPU-side deviations.
pub struct GltfIndexBufferLoader {
    accessor_id: u32,
    index_datatype: IndexDatatype,
    draco: Option<serde_json::Value>,
    cache_key: Option<String>,
    load_buffer: bool,
    load_typed_array: bool,
    typed_array: Option<IndicesTypedArray>,
    /// The decoded indices held pending GPU upload when `load_buffer` is
    /// requested without `load_typed_array`.
    pending_indices: Option<IndicesTypedArray>,
    /// The GPU index buffer created by [`Self::create_buffer`].
    gpu_buffer: Option<IndexBuffer>,
    state: ResourceLoaderState,
}

impl GltfIndexBufferLoader {
    /// Creates a new GltfIndexBufferLoader.
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] mirroring the JS `DeveloperError` checks
    /// (neither load flag set), when the accessor ID is out of range, or
    /// when the accessor component type is not an index datatype.
    pub fn try_new(
        gltf: &GltfJson,
        options: GltfIndexBufferLoaderOptions,
    ) -> Result<GltfIndexBufferLoader, RuntimeError> {
        if !options.load_buffer && !options.load_typed_array {
            return Err(RuntimeError::new(Some(
                "At least one of loadBuffer and loadTypedArray must be true.",
            )));
        }

        let accessor = gltf
            .accessors
            .get(options.accessor_id as usize)
            .ok_or_else(|| {
                RuntimeError::new(Some(&format!(
                    "accessorId {} is out of range.",
                    options.accessor_id
                )))
            })?;

        let index_datatype =
            IndexDatatype::try_from_u32(accessor.component_type).ok_or_else(|| {
                RuntimeError::new(Some(&format!(
                    "Invalid index datatype: {}",
                    accessor.component_type
                )))
            })?;

        Ok(GltfIndexBufferLoader {
            accessor_id: options.accessor_id,
            index_datatype,
            draco: options.draco,
            cache_key: options.cache_key,
            load_buffer: options.load_buffer,
            load_typed_array: options.load_typed_array,
            typed_array: None,
            pending_indices: None,
            gpu_buffer: None,
            state: ResourceLoaderState::Unloaded,
        })
    }

    /// The cache key of the resource.
    pub fn cache_key(&self) -> Option<&str> {
        self.cache_key.as_deref()
    }

    /// The typed array containing indices (defined after a successful load
    /// when `load_typed_array` is true).
    pub fn typed_array(&self) -> Option<&IndicesTypedArray> {
        self.typed_array.as_ref()
    }

    /// The index datatype after decode.
    pub fn index_datatype(&self) -> IndexDatatype {
        self.index_datatype
    }

    /// The current loader state.
    pub fn state(&self) -> ResourceLoaderState {
        self.state
    }

    /// Loads the index buffer.
    ///
    /// Mirrors `load()` / `loadFromBufferView` for uncompressed indices.
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when the accessor uses Draco compression
    /// (deferred), the accessor has no buffer view, or the buffer view
    /// cannot be loaded.
    pub fn load(&mut self, gltf: &GltfJson) -> Result<(), RuntimeError> {
        self.state = ResourceLoaderState::Loading;

        if self.draco.is_some() {
            // DEVIATION: loadFromDraco is deferred to the GPU integration
            // track (Draco decode).
            self.state = ResourceLoaderState::Failed;
            return Err(RuntimeError::new(Some(
                "Failed to load index buffer\nDraco decoding is not supported yet.",
            )));
        }

        self.load_from_buffer_view(gltf, None)
    }

    /// Loads the index buffer using externally fetched buffer bytes for the
    /// underlying buffer view.
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when decoding fails.
    pub fn load_external(
        &mut self,
        gltf: &GltfJson,
        buffer_bytes: &[u8],
    ) -> Result<(), RuntimeError> {
        self.state = ResourceLoaderState::Loading;
        self.load_from_buffer_view(gltf, Some(buffer_bytes))
    }

    fn load_from_buffer_view(
        &mut self,
        gltf: &GltfJson,
        external_bytes: Option<&[u8]>,
    ) -> Result<(), RuntimeError> {
        let result = (|| -> Result<IndicesTypedArray, RuntimeError> {
            let accessor = &gltf.accessors[self.accessor_id as usize];
            let buffer_view_id = accessor.buffer_view.ok_or_else(|| {
                RuntimeError::new(Some(
                    "Failed to load index buffer\nAccessor has no bufferView.",
                ))
            })?;

            let mut buffer_view_loader =
                GltfBufferViewLoader::try_new(gltf, buffer_view_id, None)?;
            match external_bytes {
                Some(buffer_bytes) => buffer_view_loader.load_external(buffer_bytes)?,
                None => buffer_view_loader.load(gltf)?,
            }

            let buffer_view_typed_array = buffer_view_loader
                .typed_array()
                .ok_or_else(|| {
                    RuntimeError::new(Some(
                        "Failed to load index buffer\nBuffer view is not loaded.",
                    ))
                })?
                .to_vec();

            create_indices_typed_array(gltf, self.accessor_id, &buffer_view_typed_array)
        })();

        match result {
            Ok(typed_array) => {
                self.typed_array = if self.load_typed_array {
                    Some(typed_array.clone())
                } else {
                    None
                };
                self.pending_indices = if self.load_buffer {
                    // Mirrors `process()`: the GPU buffer keeps the data;
                    // the wgpu port holds the indices until create_buffer
                    // uploads them.
                    Some(typed_array)
                } else {
                    None
                };
                self.state = ResourceLoaderState::Ready;
                Ok(())
            }
            Err(error) => {
                self.unload();
                self.state = ResourceLoaderState::Failed;
                Err(error)
            }
        }
    }

    /// Unloads the resource, mirroring `unload()`.
    pub fn unload(&mut self) {
        self.typed_array = None;
        self.pending_indices = None;
        self.gpu_buffer = None;
    }

    /// Whether `loadBuffer` was requested.
    pub fn load_buffer(&self) -> bool {
        self.load_buffer
    }

    /// Creates the GPU index buffer from the decoded indices.
    ///
    /// Rust analogue of the JS `loadBuffer` job's `Buffer.createIndexBuffer`
    /// call. Consumes the pending indices (the GPU buffer keeps the data,
    /// mirroring the JS `process()` drop).
    ///
    /// DEVIATION: 8-bit indices are widened to 16-bit (wgpu has no Uint8
    /// index format).
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when no indices are pending a GPU upload
    /// (load not run, `load_buffer` not requested, or already uploaded).
    pub fn create_buffer(&mut self, context: &Context) -> Result<(), RuntimeError> {
        let indices = self.pending_indices.take().ok_or_else(|| {
            RuntimeError::new(Some(
                "Failed to create index buffer\nNo index data pending upload.",
            ))
        })?;
        let (bytes, index_datatype) = match indices {
            IndicesTypedArray::U8(values) => {
                // DEVIATION: widen to u16 for wgpu.
                let widened: Vec<u8> = values
                    .iter()
                    .flat_map(|value| u16::from(*value).to_le_bytes())
                    .collect();
                (widened, IndexDatatype::UnsignedShort)
            }
            IndicesTypedArray::U16(values) => {
                let bytes: Vec<u8> = values
                    .iter()
                    .flat_map(|value| value.to_le_bytes())
                    .collect();
                (bytes, IndexDatatype::UnsignedShort)
            }
            IndicesTypedArray::U32(values) => {
                let bytes: Vec<u8> = values
                    .iter()
                    .flat_map(|value| value.to_le_bytes())
                    .collect();
                (bytes, IndexDatatype::UnsignedInt)
            }
        };
        self.gpu_buffer = Some(context.create_index_buffer(
            Some(&bytes),
            None,
            BufferUsage::StaticDraw,
            index_datatype,
        ));
        Ok(())
    }

    /// The GPU index buffer (defined after a successful
    /// [`Self::create_buffer`]).
    pub fn buffer(&self) -> Option<&IndexBuffer> {
        self.gpu_buffer.as_ref()
    }

    /// Takes the GPU index buffer out of the loader (moves it to the
    /// caller's vertex array assembly).
    pub fn take_buffer(&mut self) -> Option<IndexBuffer> {
        self.gpu_buffer.take()
    }
}

/// Mirrors `createIndicesTypedArray`: decodes `count` indices of the
/// accessor's component type out of the buffer view bytes, honoring the
/// unaligned-copy fallback.
fn create_indices_typed_array(
    gltf: &GltfJson,
    accessor_id: u32,
    buffer_view_typed_array: &[u8],
) -> Result<IndicesTypedArray, RuntimeError> {
    let accessor = &gltf.accessors[accessor_id as usize];
    let count = accessor.count as usize;
    let index_datatype = IndexDatatype::try_from_u32(accessor.component_type)
        .expect("validated in GltfIndexBufferLoader::try_new");
    let index_size = index_datatype.size_in_bytes();

    let mut byte_offset = accessor.byte_offset as usize;

    let bytes = if byte_offset % index_size != 0 {
        // Mirrors the unaligned fallback: copy the index region into a
        // fresh buffer aligned at offset 0.
        // DEVIATION: the JS deprecationWarning("index-buffer-unaligned")
        // is not mirrored.
        let byte_length = count * index_size;
        if byte_offset + byte_length > buffer_view_typed_array.len() {
            return Err(RuntimeError::new(Some(
                "Failed to load index buffer\nIndex data is out of bounds.",
            )));
        }
        byte_offset = 0;
        &buffer_view_typed_array[accessor.byte_offset as usize
            ..accessor.byte_offset as usize + byte_length]
    } else {
        buffer_view_typed_array
    };

    let byte_length = count * index_size;
    if byte_offset + byte_length > bytes.len() {
        return Err(RuntimeError::new(Some(
            "Failed to load index buffer\nIndex data is out of bounds.",
        )));
    }

    let data = &bytes[byte_offset..byte_offset + byte_length];

    Ok(match index_datatype {
        IndexDatatype::UnsignedByte => {
            IndicesTypedArray::U8(data.to_vec())
        }
        IndexDatatype::UnsignedShort => IndicesTypedArray::U16(
            data.chunks_exact(2)
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .collect(),
        ),
        IndexDatatype::UnsignedInt => IndicesTypedArray::U32(
            data.chunks_exact(4)
                .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect(),
        ),
    })
}
