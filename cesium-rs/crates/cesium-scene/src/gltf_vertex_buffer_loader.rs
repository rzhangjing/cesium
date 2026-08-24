//! Ported from `packages/engine/Source/Scene/GltfVertexBufferLoader.js`.
//!
//! Loads a vertex buffer from a glTF buffer view.
//!
//! The GPU vertex buffer itself is created by [`GltfVertexBufferLoader::
//! create_buffer`] against a renderer [`Context`] once the load has
//! produced bytes (mirrors `Buffer.createVertexBuffer` inside the JS
//! `loadBuffer` job).
//!
//! DEVIATION: the job scheduler and Draco decode (+ quantization info) and
//! SPZ decode remain deferred; `load_buffer` keeps the buffer-view bytes
//! pending until `create_buffer` uploads them (the JS uploads through the
//! ResourceCache job queue).
//!
//! DEVIATION: the JS loader obtains the buffer view through the
//! `ResourceCache`; the Rust port composes a [`GltfBufferViewLoader`]
//! directly against the in-memory glTF (embedded buffers) or caller
//! supplied external bytes.

use cesium_core::runtime_error::RuntimeError;
use cesium_renderer::buffer::Buffer;
use cesium_renderer::buffer_usage::BufferUsage;
use cesium_renderer::context::Context;

use crate::gltf_buffer_view_loader::GltfBufferViewLoader;
use crate::gltf_loader::GltfJson;
use crate::resource_loader_state::ResourceLoaderState;

/// Mirrors `hasDracoCompression`: the Draco extension defines compression
/// for the given attribute semantic.
fn has_draco_compression(draco: Option<&serde_json::Value>, semantic: Option<&str>) -> bool {
    match (draco, semantic) {
        (Some(draco), Some(semantic)) => draco
            .get("attributes")
            .and_then(|attributes| attributes.get(semantic))
            .is_some(),
        _ => false,
    }
}

/// Options for [`GltfVertexBufferLoader::try_new`], mirroring the JS
/// constructor's `options` object.
pub struct GltfVertexBufferLoaderOptions {
    /// The bufferView ID corresponding to the vertex buffer.
    pub buffer_view_id: Option<u32>,
    /// The primitive containing the Draco extension (required when Draco is
    /// effective; Draco decode itself is deferred).
    pub primitive: Option<serde_json::Value>,
    /// The Draco extension object.
    pub draco: Option<serde_json::Value>,
    /// The SPZ extension object (deferred).
    pub spz: Option<serde_json::Value>,
    /// The attribute semantic, e.g. POSITION or NORMAL.
    pub attribute_semantic: Option<String>,
    /// The accessor id (required when Draco is effective).
    pub accessor_id: Option<u32>,
    /// The cache key of the resource.
    pub cache_key: Option<String>,
    /// Load the vertex buffer as a GPU vertex buffer (the bytes are kept
    /// pending until [`GltfVertexBufferLoader::create_buffer`] uploads
    /// them).
    pub load_buffer: bool,
    /// Load the vertex buffer as a typed array.
    pub load_typed_array: bool,
}

/// Loads a vertex buffer from a glTF buffer view.
///
/// Rust analogue of the CesiumJS `GltfVertexBufferLoader` (`ResourceLoader`
/// interface); see module docs for the CPU-side deviations.
pub struct GltfVertexBufferLoader {
    buffer_view_id: Option<u32>,
    draco: Option<serde_json::Value>,
    spz: Option<serde_json::Value>,
    attribute_semantic: Option<String>,
    accessor_id: Option<u32>,
    cache_key: Option<String>,
    load_buffer: bool,
    load_typed_array: bool,
    typed_array: Option<Vec<u8>>,
    /// The buffer-view bytes held pending GPU upload when `load_buffer` is
    /// requested without `load_typed_array`.
    pending_bytes: Option<Vec<u8>>,
    /// The GPU vertex buffer created by [`Self::create_buffer`].
    gpu_buffer: Option<Buffer>,
    state: ResourceLoaderState,
}

impl GltfVertexBufferLoader {
    /// Creates a new GltfVertexBufferLoader.
    ///
    /// # Errors
    /// Returns [`RuntimeError`]s mirroring the JS `DeveloperError` checks:
    /// neither load flag set, not exactly one vertex buffer source
    /// effective, or a Draco requirement missing.
    pub fn try_new(
        options: GltfVertexBufferLoaderOptions,
    ) -> Result<GltfVertexBufferLoader, RuntimeError> {
        if !options.load_buffer && !options.load_typed_array {
            return Err(RuntimeError::new(Some(
                "At least one of loadBuffer and loadTypedArray must be true.",
            )));
        }

        let has_buffer_view_id = options.buffer_view_id.is_some();
        let has_primitive = options.primitive.is_some();
        let has_draco = has_draco_compression(
            options.draco.as_ref(),
            options.attribute_semantic.as_deref(),
        );
        let has_attribute_semantic = options.attribute_semantic.is_some();
        let has_accessor_id = options.accessor_id.is_some();
        let has_spz = options.spz.is_some();
        let source_count = usize::from(has_buffer_view_id)
            + usize::from(has_draco)
            + usize::from(has_spz);
        if source_count != 1 {
            return Err(RuntimeError::new(Some(
                "Exactly one vertex buffer source must be effective: options.bufferViewId, options.spz, or options.draco for options.attributeSemantic.",
            )));
        }

        if has_draco && !has_attribute_semantic {
            return Err(RuntimeError::new(Some(
                "When options.draco is defined options.attributeSemantic must also be defined.",
            )));
        }

        if has_draco && !has_accessor_id {
            return Err(RuntimeError::new(Some(
                "When options.draco is defined options.accessorId must also be defined.",
            )));
        }

        if has_draco && !has_primitive {
            return Err(RuntimeError::new(Some(
                "When options.draco is defined options.primitive must also be defined.",
            )));
        }

        Ok(GltfVertexBufferLoader {
            buffer_view_id: options.buffer_view_id,
            draco: options.draco,
            spz: options.spz,
            attribute_semantic: options.attribute_semantic,
            accessor_id: options.accessor_id,
            cache_key: options.cache_key,
            load_buffer: options.load_buffer,
            load_typed_array: options.load_typed_array,
            typed_array: None,
            pending_bytes: None,
            gpu_buffer: None,
            state: ResourceLoaderState::Unloaded,
        })
    }

    /// The cache key of the resource.
    pub fn cache_key(&self) -> Option<&str> {
        self.cache_key.as_deref()
    }

    /// The typed array containing vertex buffer data (defined after a
    /// successful load when `load_typed_array` is true).
    pub fn typed_array(&self) -> Option<&[u8]> {
        self.typed_array.as_deref()
    }

    /// The current loader state.
    pub fn state(&self) -> ResourceLoaderState {
        self.state
    }

    /// The attribute semantic, when provided.
    pub fn attribute_semantic(&self) -> Option<&str> {
        self.attribute_semantic.as_deref()
    }

    /// The accessor id, when provided.
    pub fn accessor_id(&self) -> Option<u32> {
        self.accessor_id
    }

    /// Loads the vertex buffer.
    ///
    /// Mirrors `load()` / `loadFromBufferView` for uncompressed vertex
    /// data.
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when the SPZ or Draco path is effective
    /// (both deferred), or the buffer view cannot be loaded.
    pub fn load(&mut self, gltf: &GltfJson) -> Result<(), RuntimeError> {
        self.state = ResourceLoaderState::Loading;

        if self.spz.is_some() {
            // DEVIATION: loadFromSpz is deferred.
            self.state = ResourceLoaderState::Failed;
            return Err(RuntimeError::new(Some(
                "Failed to load vertex buffer\nSPZ decoding is not supported yet.",
            )));
        }

        if has_draco_compression(self.draco.as_ref(), self.attribute_semantic.as_deref()) {
            // DEVIATION: loadFromDraco is deferred to the GPU integration
            // track (Draco decode + quantization).
            self.state = ResourceLoaderState::Failed;
            return Err(RuntimeError::new(Some(
                "Failed to load vertex buffer\nDraco decoding is not supported yet.",
            )));
        }

        self.load_from_buffer_view(gltf, None)
    }

    /// Loads the vertex buffer using externally fetched buffer bytes for
    /// the underlying buffer view.
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when the buffer view cannot be loaded.
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
        let result = (|| -> Result<Vec<u8>, RuntimeError> {
            let buffer_view_id = self.buffer_view_id.ok_or_else(|| {
                RuntimeError::new(Some(
                    "Failed to load vertex buffer\nNo bufferViewId.",
                ))
            })?;

            let mut buffer_view_loader =
                GltfBufferViewLoader::try_new(gltf, buffer_view_id, None)?;
            match external_bytes {
                Some(buffer_bytes) => buffer_view_loader.load_external(buffer_bytes)?,
                None => buffer_view_loader.load(gltf)?,
            }

            buffer_view_loader
                .typed_array()
                .map(|bytes| bytes.to_vec())
                .ok_or_else(|| {
                    RuntimeError::new(Some(
                        "Failed to load vertex buffer\nBuffer view is not loaded.",
                    ))
                })
        })();

        match result {
            Ok(typed_array) => {
                self.typed_array = if self.load_typed_array {
                    Some(typed_array.clone())
                } else {
                    None
                };
                self.pending_bytes = if self.load_buffer {
                    // Mirrors `process()`: the GPU buffer keeps the data;
                    // the wgpu port holds the bytes until create_buffer
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
        self.pending_bytes = None;
        self.gpu_buffer = None;
    }

    /// Whether `loadBuffer` was requested.
    pub fn load_buffer(&self) -> bool {
        self.load_buffer
    }

    /// Creates the GPU vertex buffer from the loaded bytes.
    ///
    /// Rust analogue of the JS `loadBuffer` job's
    /// `Buffer.createVertexBuffer` call. Consumes the pending bytes (the
    /// GPU buffer keeps the data, mirroring the JS `process()` drop).
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when no bytes are pending a GPU upload
    /// (load not run, `load_buffer` not requested, or already uploaded).
    pub fn create_buffer(&mut self, context: &Context) -> Result<(), RuntimeError> {
        let bytes = self.pending_bytes.take().ok_or_else(|| {
            RuntimeError::new(Some(
                "Failed to create vertex buffer\nNo buffer data pending upload.",
            ))
        })?;
        self.gpu_buffer = Some(context.create_vertex_buffer(
            Some(&bytes),
            None,
            BufferUsage::StaticDraw,
        ));
        Ok(())
    }

    /// The GPU vertex buffer (defined after a successful
    /// [`Self::create_buffer`]).
    pub fn buffer(&self) -> Option<&Buffer> {
        self.gpu_buffer.as_ref()
    }

    /// Takes the GPU vertex buffer out of the loader (moves it to the
    /// caller's vertex array assembly).
    pub fn take_buffer(&mut self) -> Option<Buffer> {
        self.gpu_buffer.take()
    }
}
