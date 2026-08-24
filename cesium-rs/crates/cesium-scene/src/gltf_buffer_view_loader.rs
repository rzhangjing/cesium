//! Ported from `packages/engine/Source/Scene/GltfBufferViewLoader.js`.
//!
//! Loads a glTF buffer view as a CPU-side byte slice.
//!
//! DEVIATION: the JS loader resolves buffers through the `ResourceCache`
//! promise pipeline (network fetch for external buffers, GPU buffer
//! accounting). The Rust port operates synchronously on in-memory bytes:
//! embedded buffers are read from `GltfBuffer::data`, external buffers are
//! supplied by the caller via [`GltfBufferViewLoader::load_external`]
//! (network fetching is deferred to the async resource cache, T5).
//!
//! DEVIATION: meshopt compression decoding (`EXT_meshopt_compression` /
//! `KHR_meshopt_compression`) is parsed but not decoded yet (no meshopt
//! decoder in the workspace); loading such a view returns an error.

use cesium_core::runtime_error::RuntimeError;

use crate::gltf_loader::{GltfBufferView, GltfJson};
use crate::resource_loader_state::ResourceLoaderState;

/// Returns the meshopt compression extension object, KHR_meshopt_compression
/// or EXT_meshopt_compression, on a glTF bufferView or buffer. If both are
/// present, KHR is preferred. Mirrors `findMeshoptExtension`.
fn find_meshopt_extension(
    extensions: Option<&serde_json::Value>,
) -> Option<&serde_json::Value> {
    let extensions = extensions?;
    extensions
        .get("KHR_meshopt_compression")
        .or_else(|| extensions.get("EXT_meshopt_compression"))
}

/// Loads a glTF buffer view.
///
/// Rust analogue of the CesiumJS `GltfBufferViewLoader` (`ResourceLoader`
/// interface); see module docs for the CPU-side deviations.
pub struct GltfBufferViewLoader {
    buffer_id: u32,
    byte_offset: u32,
    byte_length: u32,
    has_meshopt: bool,
    meshopt_byte_stride: Option<u32>,
    meshopt_count: Option<u32>,
    meshopt_mode: Option<String>,
    meshopt_filter: String,
    cache_key: Option<String>,
    typed_array: Option<Vec<u8>>,
    state: ResourceLoaderState,
}

impl GltfBufferViewLoader {
    /// Creates a new GltfBufferViewLoader.
    ///
    /// Mirrors `new GltfBufferViewLoader(options)` (the `resourceCache`,
    /// `gltfResource` and `baseResource` options belong to the deferred
    /// async pipeline and have no CPU-side analogue).
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when the buffer view ID is out of range.
    pub fn try_new(
        gltf: &GltfJson,
        buffer_view_id: u32,
        cache_key: Option<String>,
    ) -> Result<GltfBufferViewLoader, RuntimeError> {
        let buffer_view = gltf
            .buffer_views
            .get(buffer_view_id as usize)
            .ok_or_else(|| {
                RuntimeError::new(Some(&format!(
                    "bufferViewId {buffer_view_id} is out of range."
                )))
            })?;

        Self::from_buffer_view(buffer_view, cache_key)
    }

    /// Resolves the buffer/meshopt fields of a buffer view, mirroring the
    /// constructor body of the JS loader.
    fn from_buffer_view(
        buffer_view: &GltfBufferView,
        cache_key: Option<String>,
    ) -> Result<GltfBufferViewLoader, RuntimeError> {
        let mut buffer_id = buffer_view.buffer;
        let mut byte_offset = buffer_view.byte_offset;
        let mut byte_length = buffer_view.byte_length;

        let mut has_meshopt = false;
        let mut meshopt_byte_stride = None;
        let mut meshopt_count = None;
        let mut meshopt_mode = None;
        let mut meshopt_filter = "NONE".to_string();

        if let Some(meshopt) = find_meshopt_extension(buffer_view.extensions.as_ref()) {
            let meshopt_buffer = meshopt.get("buffer").and_then(|v| v.as_u64()).ok_or_else(
                || RuntimeError::new(Some("meshopt extension is missing buffer.")),
            )?;
            buffer_id = meshopt_buffer as u32;
            byte_offset = meshopt
                .get("byteOffset")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            byte_length = meshopt
                .get("byteLength")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| {
                    RuntimeError::new(Some("meshopt extension is missing byteLength."))
                })? as u32;

            has_meshopt = true;
            meshopt_byte_stride =
                meshopt.get("byteStride").and_then(|v| v.as_u64()).map(|v| v as u32);
            meshopt_count = meshopt.get("count").and_then(|v| v.as_u64()).map(|v| v as u32);
            meshopt_mode = meshopt
                .get("mode")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());
            meshopt_filter = meshopt
                .get("filter")
                .and_then(|v| v.as_str())
                .unwrap_or("NONE")
                .to_string();
        }

        Ok(GltfBufferViewLoader {
            buffer_id,
            byte_offset,
            byte_length,
            has_meshopt,
            meshopt_byte_stride,
            meshopt_count,
            meshopt_mode,
            meshopt_filter,
            cache_key,
            typed_array: None,
            state: ResourceLoaderState::Unloaded,
        })
    }

    /// The cache key of the resource.
    pub fn cache_key(&self) -> Option<&str> {
        self.cache_key.as_deref()
    }

    /// The typed array containing buffer view data (defined after a
    /// successful load).
    pub fn typed_array(&self) -> Option<&[u8]> {
        self.typed_array.as_deref()
    }

    /// The current loader state.
    pub fn state(&self) -> ResourceLoaderState {
        self.state
    }

    /// Whether the buffer view uses meshopt compression.
    pub fn has_meshopt(&self) -> bool {
        self.has_meshopt
    }

    /// The meshopt byte stride (when the view uses meshopt compression).
    pub fn meshopt_byte_stride(&self) -> Option<u32> {
        self.meshopt_byte_stride
    }

    /// The meshopt element count (when the view uses meshopt compression).
    pub fn meshopt_count(&self) -> Option<u32> {
        self.meshopt_count
    }

    /// The meshopt mode (when the view uses meshopt compression).
    pub fn meshopt_mode(&self) -> Option<&str> {
        self.meshopt_mode.as_deref()
    }

    /// The meshopt filter (defaults to `"NONE"`).
    pub fn meshopt_filter(&self) -> &str {
        &self.meshopt_filter
    }

    /// Loads the buffer view.
    ///
    /// Mirrors `load()` for embedded buffers; external buffers (buffers
    /// with a non-data `uri`) cannot be fetched on the CPU path and yield a
    /// [`RuntimeError`] directing the caller to
    /// [`GltfBufferViewLoader::load_external`].
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when the buffer is missing data, the
    /// view uses (unsupported) meshopt compression, or the slice is out of
    /// bounds.
    pub fn load(&mut self, gltf: &GltfJson) -> Result<(), RuntimeError> {
        self.state = ResourceLoaderState::Loading;

        let result = (|| -> Result<Vec<u8>, RuntimeError> {
            let buffer = gltf.buffers.get(self.buffer_id as usize).ok_or_else(|| {
                RuntimeError::new(Some(&format!(
                    "bufferId {} is out of range.",
                    self.buffer_id
                )))
            })?;

            // Mirrors getBufferLoader: buffers with a URI are external,
            // everything else is embedded (extras._pipeline.source in JS).
            if buffer.uri.is_some() {
                let uri = buffer.uri.as_deref().unwrap_or("");
                return Err(RuntimeError::new(Some(&format!(
                    "Failed to load buffer view\nExternal buffer must be fetched by the caller: {uri}"
                ))));
            }

            buffer
                .data
                .as_deref()
                .ok_or_else(|| {
                    RuntimeError::new(Some(
                        "Failed to load buffer view\nEmbedded buffer has no data.",
                    ))
                })
                .map(|bytes| bytes.to_vec())
        })();

        match result {
            Ok(source) => self.load_from_buffer_bytes(&source),
            Err(error) => {
                self.unload();
                self.state = ResourceLoaderState::Failed;
                Err(error)
            }
        }
    }

    /// Loads the buffer view from externally fetched buffer bytes,
    /// mirroring the external-buffer branch of `loadResources`.
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when the view uses (unsupported) meshopt
    /// compression or the slice is out of bounds.
    pub fn load_external(&mut self, buffer_bytes: &[u8]) -> Result<(), RuntimeError> {
        self.state = ResourceLoaderState::Loading;
        self.load_from_buffer_bytes(buffer_bytes)
    }

    /// Slices the buffer view out of the buffer bytes and (eventually)
    /// decodes meshopt data.
    fn load_from_buffer_bytes(&mut self, buffer_bytes: &[u8]) -> Result<(), RuntimeError> {
        let result = (|| -> Result<Vec<u8>, RuntimeError> {
            let start = self.byte_offset as usize;
            let end = start + self.byte_length as usize;
            if end > buffer_bytes.len() {
                return Err(RuntimeError::new(Some(
                    "Failed to load buffer view\nBuffer view is out of bounds.",
                )));
            }
            let mut typed_array = buffer_bytes[start..end].to_vec();

            if self.has_meshopt {
                // DEVIATION: MeshoptDecoder.decodeGltfBuffer is deferred;
                // the meshoptimizer crate is not part of the workspace yet.
                typed_array.clear();
                return Err(RuntimeError::new(Some(
                    "Failed to load buffer view\nmeshopt decoding is not supported yet.",
                )));
            }

            Ok(typed_array)
        })();

        match result {
            Ok(typed_array) => {
                self.typed_array = Some(typed_array);
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
    }
}
