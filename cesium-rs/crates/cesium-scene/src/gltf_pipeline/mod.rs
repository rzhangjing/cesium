//! glTF pipeline subsystem — glTF processing and optimization.
//! Ported from `packages/engine/Source/Scene/GltfPipeline/`.
//!
//! The pipeline functions operate on a glTF asset represented as
//! [`serde_json::Value`] (the Rust analogue of the plain JavaScript object
//! CesiumJS mutates in place), mirroring CesiumJS `gltf-pipeline`
//! traversal/conversion helpers one to one.
//!
//! DEVIATION: CesiumJS keeps each buffer's binary payload inside the glTF
//! object itself (`buffer.extras._pipeline.source`, a `Uint8Array` view).
//! `serde_json::Value` cannot carry raw bytes, so the Rust port keeps the
//! binary payloads in a parallel side table
//! ([`PipelineBufferSources`]) indexed by buffer index; `addBuffer` /
//! `Remove.buffer` maintain the invariant that the side table length tracks
//! `gltf["buffers"]`.

use serde_json::Value;

/// Rust analogue of `buffer.extras._pipeline.source` (a `Uint8Array` view:
/// `source.buffer` + `source.byteOffset`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PipelineBufferSource {
    /// The binary payload (`source.buffer`).
    pub buffer: Vec<u8>,
    /// Byte offset into `buffer` (`source.byteOffset`).
    pub byte_offset: usize,
}

impl PipelineBufferSource {
    /// Creates a new source over owned bytes with a zero byte offset.
    pub fn new(buffer: Vec<u8>) -> Self {
        Self {
            buffer,
            byte_offset: 0,
        }
    }

    /// The number of bytes available from `byte_offset` (the JS
    /// `source.length` of the `Uint8Array` view).
    pub fn len(&self) -> usize {
        self.buffer.len() - self.byte_offset.min(self.buffer.len())
    }

    /// Whether the view is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The view bytes (`source` seen as a `Uint8Array`).
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer[self.byte_offset.min(self.buffer.len())..]
    }
}

/// Per-buffer binary sources, indexed in parallel with `gltf["buffers"]`.
/// `None` entries mark buffers without an attached binary payload (e.g.
/// external `uri` buffers that were never fetched).
pub type PipelineBufferSources = Vec<Option<PipelineBufferSource>>;

/// Rust analogue of CesiumJS `defined()`: a value is defined when it exists
/// and is not `null`.
pub(crate) fn defined(value: Option<&Value>) -> bool {
    matches!(value, Some(v) if !v.is_null())
}

/// Stringifies an index or JSON id the way JavaScript coerces object keys
/// (numbers become their decimal string form).
pub(crate) fn key_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

pub mod add_buffer;
pub mod add_defaults;
pub mod add_extensions_required;
pub mod add_extensions_used;
pub mod add_pipeline_extras;
pub mod add_to_array;
pub mod find_accessor_min_max;
pub mod for_each;
pub mod for_each_texture_in_material;
pub mod get_accessor_byte_stride;
pub mod get_component_reader;
pub mod move_technique_render_states;
pub mod move_techniques_to_extension;
pub mod number_of_components_for_type;
pub mod parse_glb;
pub mod read_accessor_packed;
pub mod remove_extension;
pub mod remove_extensions_required;
pub mod remove_extensions_used;
pub mod remove_pipeline_extras;
pub mod remove_unused_elements;
pub mod update_accessor_component_types;
pub mod update_version;
pub mod uses_extension;
