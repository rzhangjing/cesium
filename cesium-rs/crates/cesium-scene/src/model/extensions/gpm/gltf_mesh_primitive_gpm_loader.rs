//! Ported from `packages/engine/Source/Scene/Model/Extensions/Gpm/GltfMeshPrimitiveGpmLoader.js`.
//!
//! Loads the glTF `NGA_gpm_local` extension from a glTF mesh primitive
//! and stores it in a `MeshPrimitiveGpmLocal` object.
//!
//! DEVIATION: the GPU pipeline of the JS loader (texture loading through
//! `ResourceCache`, and the final conversion into `StructuralMetadata`
//! with `PropertyTexture` instances) depends on the model runtime
//! pipeline which is not ported yet (tracked as MD-01, GPU-required).
//! The pure-logic surface is fully ported: extension parsing into
//! `MeshPrimitiveGpmLocal`, PPE texture class JSON generation and the
//! metadata schema JSON generation.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use cesium_core::runtime_error::RuntimeError;

use crate::model::extensions::gpm::mesh_primitive_gpm_local::MeshPrimitiveGpmLocal;
use crate::model::extensions::gpm::ppe_metadata::PpeMetadata;
use crate::model::extensions::gpm::ppe_source::PpeSource;
use crate::model::extensions::gpm::ppe_texture::PpeTexture;
use crate::resource_loader_state::ResourceLoaderState;

/// Loads glTF `NGA_gpm_local` from a glTF mesh primitive.
///
/// Port of `GltfMeshPrimitiveGpmLoader` (ResourceLoader interface).
pub struct GltfMeshPrimitiveGpmLoader {
    extension: Option<Value>,
    cache_key: Option<String>,
    asynchronous: bool,
    texture_ids: Vec<usize>,
    mesh_primitive_gpm_local: Option<MeshPrimitiveGpmLocal>,
    state: ResourceLoaderState,
    destroyed: bool,
}

impl GltfMeshPrimitiveGpmLoader {
    /// Creates a new loader for the given `NGA_gpm_local` extension
    /// object of a mesh primitive.
    ///
    /// Port of the `GltfMeshPrimitiveGpmLoader(options)` constructor.
    ///
    /// DEVIATION: the JS constructor additionally requires GPU-side
    /// options (`gltfResource`, `baseResource`, `supportedImageFormats`,
    /// `frameState`) which only feed the texture loading path; see the
    /// module-level DEVIATION note.
    pub fn new(extension: Value, cache_key: Option<String>, asynchronous: Option<bool>) -> Self {
        let texture_ids = gather_used_texture_ids(&extension)
            .into_keys()
            .collect();
        Self {
            extension: Some(extension),
            cache_key,
            asynchronous: asynchronous.unwrap_or(true),
            texture_ids,
            mesh_primitive_gpm_local: None,
            state: ResourceLoaderState::Unloaded,
            destroyed: false,
        }
    }

    /// The cache key of the resource (port of the `cacheKey` getter).
    pub fn cache_key(&self) -> Option<&str> {
        self.cache_key.as_deref()
    }

    /// Whether WebGL resource creation is spread over several frames
    /// (port of the `asynchronous` option, defaults to `true`).
    pub fn asynchronous(&self) -> bool {
        self.asynchronous
    }

    /// The parsed GPM extension information from the mesh primitive
    /// (port of the `meshPrimitiveGpmLocal` getter).
    pub fn mesh_primitive_gpm_local(&self) -> Option<&MeshPrimitiveGpmLocal> {
        self.mesh_primitive_gpm_local.as_ref()
    }

    /// The texture ids used by the PPE textures of the extension
    /// (port of the internal `_textureIds` bookkeeping).
    pub fn texture_ids(&self) -> &[usize] {
        &self.texture_ids
    }

    /// The current loader state.
    pub fn state(&self) -> ResourceLoaderState {
        self.state
    }

    /// Port of `ResourceLoader.prototype.isDestroyed`.
    pub fn is_destroyed(&self) -> bool {
        self.destroyed
    }

    /// Loads the resource.
    ///
    /// Port of `GltfMeshPrimitiveGpmLoader.prototype.load`.
    ///
    /// DEVIATION: the JS version asynchronously loads the PPE textures
    /// through `ResourceCache` (GPU); this port completes the loading
    /// phase synchronously because the texture runtime is not ported
    /// yet (MD-01, GPU-required). The extension data itself needs no
    /// texture contents for parsing.
    pub fn load(&mut self) {
        if self.state != ResourceLoaderState::Unloaded {
            return;
        }
        self.state = ResourceLoaderState::Loading;
        // No asynchronous GPU work to await (see DEVIATION above).
        self.state = ResourceLoaderState::Loaded;
    }

    /// Processes the resource until it becomes ready.
    ///
    /// Port of `GltfMeshPrimitiveGpmLoader.prototype.process`.
    ///
    /// DEVIATION: the JS version first processes the GPU texture loaders
    /// and converts the result into a `StructuralMetadata` object; both
    /// steps require the unported model runtime (MD-01, GPU-required).
    /// This port performs the pure-logic part: converting the JSON
    /// representation of the `ppeTextures` into `PpeTexture` objects
    /// stored in a `MeshPrimitiveGpmLocal`.
    pub fn process(&mut self) -> Result<bool, RuntimeError> {
        if self.state == ResourceLoaderState::Ready {
            return Ok(true);
        }
        if self.state != ResourceLoaderState::Loaded {
            return Ok(false);
        }

        let extension = match &self.extension {
            Some(extension) => extension.clone(),
            None => return Ok(false),
        };

        let ppe_textures = parse_ppe_textures(&extension)?;
        self.mesh_primitive_gpm_local = Some(MeshPrimitiveGpmLocal::new(ppe_textures));
        self.state = ResourceLoaderState::Ready;
        Ok(true)
    }

    /// Unloads the resource.
    ///
    /// Port of `GltfMeshPrimitiveGpmLoader.prototype.unload` (the GPU
    /// texture unload loop is part of the module-level DEVIATION).
    pub fn unload(&mut self) {
        self.texture_ids.clear();
        self.extension = None;
        self.mesh_primitive_gpm_local = None;
    }

    /// Port of `ResourceLoader.prototype.destroy`.
    pub fn destroy(&mut self) {
        self.unload();
        self.destroyed = true;
    }
}

/// Gathers the used texture ids from the given `NGA_gpm_local`
/// extension object of a mesh primitive.
///
/// Port of `gatherUsedTextureIds(gpmExtension)`. The JS version returns
/// a plain object keyed by texture index; the Rust port uses a
/// `BTreeMap` which iterates in the same numeric-ascending order as JS
/// integer-like object keys.
pub fn gather_used_texture_ids(gpm_extension: &Value) -> BTreeMap<usize, Value> {
    let mut texture_ids = BTreeMap::new();
    if let Some(ppe_textures) = gpm_extension.get("ppeTextures").and_then(|v| v.as_array()) {
        for ppe_texture in ppe_textures {
            if let Some(index) = ppe_texture.get("index").and_then(|v| v.as_u64()) {
                // The texture is a valid textureInfo.
                texture_ids.insert(index as usize, ppe_texture.clone());
            }
        }
    }
    texture_ids
}

/// Converts the JSON representation of the `ppeTextures` that are found
/// in the extension JSON into `PpeTexture` objects.
///
/// Port of the `ppeTextures` parsing part of
/// `GltfMeshPrimitiveGpmLoader.prototype.process`.
///
/// # Errors
/// Returns a `RuntimeError` for malformed entries.
///
/// DEVIATION: the JS version accepts any string as `traits.source`
/// (the `PpeSource` enum is not validated); this port requires a valid
/// `PpeSource` value because the Rust enum cannot hold unknown strings.
pub fn parse_ppe_textures(extension: &Value) -> Result<Vec<PpeTexture>, RuntimeError> {
    let mut ppe_textures = Vec::new();
    if let Some(ppe_textures_json) = extension.get("ppeTextures").and_then(|v| v.as_array()) {
        for ppe_texture_json in ppe_textures_json {
            let traits_json = ppe_texture_json
                .get("traits")
                .ok_or_else(|| RuntimeError::new(Some("ppeTexture.traits is required")))?;
            let source = PpeSource::from_str(
                traits_json
                    .get("source")
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
            )
            .ok_or_else(|| {
                RuntimeError::new(Some("Invalid PPE source in NGA_gpm_local ppeTexture traits"))
            })?;
            let traits = PpeMetadata::new(
                source,
                traits_json.get("min").and_then(|v| v.as_f64()),
                traits_json.get("max").and_then(|v| v.as_f64()),
            );
            let index = ppe_texture_json
                .get("index")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| RuntimeError::new(Some("ppeTexture.index is required")))?
                as usize;
            let ppe_texture = PpeTexture::new(
                traits,
                index,
                ppe_texture_json.get("texCoord").and_then(|v| v.as_u64()).map(|v| v as usize),
                ppe_texture_json.get("noData").and_then(|v| v.as_f64()),
                ppe_texture_json.get("offset").and_then(|v| v.as_f64()),
                ppe_texture_json.get("scale").and_then(|v| v.as_f64()),
            );
            ppe_textures.push(ppe_texture);
        }
    }
    Ok(ppe_textures)
}

/// Creates the JSON description of a metadata class that treats the
/// given PPE texture as a property texture property.
///
/// Port of `GltfMeshPrimitiveGpmLoader._createPpeTextureClassJson`.
///
/// Given that `offset` and `scale` may only be applied to integer
/// property values when they are `normalized`, the values will be
/// declared as `normalized` here. The normalization factor will later
/// have to be cancelled out, with the `scale` being multiplied by 255.
pub fn create_ppe_texture_class_json(ppe_texture: &PpeTexture, index: usize) -> Value {
    let traits = ppe_texture.traits();
    let ppe_property_name = traits.source().as_str();

    let offset = ppe_texture.offset().unwrap_or(0.0);
    let scale = ppe_texture.scale().unwrap_or(1.0) * 255.0;

    let mut property = json!({
        "name": "PPE",
        "type": "SCALAR",
        "componentType": "UINT8",
        "normalized": true,
        "offset": offset,
        "scale": scale,
    });
    let property_object = property.as_object_mut().expect("json! object");
    // The JS class JSON only contains `min`/`max` when they are defined.
    if let Some(min) = traits.min() {
        property_object.insert("min".to_string(), json!(min));
    }
    if let Some(max) = traits.max() {
        property_object.insert("max".to_string(), json!(max));
    }

    json!({
        "name": format!("PPE texture class {}", index),
        "properties": {
            ppe_property_name: property,
        },
    })
}

/// Creates the JS `JSON.stringify` representation of a PPE texture
/// class JSON with the same key insertion order as the JS source
/// (`name`, then `properties` with `name`, `type`, `componentType`,
/// `normalized`, `offset`, `scale`, `min`, `max`).
fn ppe_texture_class_json_string(ppe_texture: &PpeTexture, index: usize) -> String {
    let traits = ppe_texture.traits();
    let offset = ppe_texture.offset().unwrap_or(0.0);
    let scale = ppe_texture.scale().unwrap_or(1.0) * 255.0;

    let min_part = traits
        .min()
        .map(|min| format!(",\"min\":{}", json!(min)))
        .unwrap_or_default();
    let max_part = traits
        .max()
        .map(|max| format!(",\"max\":{}", json!(max)))
        .unwrap_or_default();

    format!(
        "{{\"name\":\"PPE texture class {}\",\"properties\":{{\"{}\":{{\"name\":\"PPE\",\"type\":\"SCALAR\",\"componentType\":\"UINT8\",\"normalized\":true,\"offset\":{},\"scale\":{}{}{}}}}}}}",
        index,
        traits.source().as_str(),
        json!(offset),
        json!(scale),
        min_part,
        max_part,
    )
}

/// Creates an array of strings that serve as identifiers for PPE
/// textures.
///
/// Port of `GltfMeshPrimitiveGpmLoader._collectPpeTexturePropertyIdentifiers`.
pub fn collect_ppe_texture_property_identifiers(
    mesh_primitive_gpm_local: &MeshPrimitiveGpmLocal,
) -> Vec<String> {
    mesh_primitive_gpm_local
        .ppe_textures()
        .iter()
        .enumerate()
        .map(|(i, ppe_texture)| ppe_texture_class_json_string(ppe_texture, i))
        .collect()
}

/// Returns the metadata schema JSON for the PPE textures in the given
/// `MeshPrimitiveGpmLocal` instance.
///
/// Port of `GltfMeshPrimitiveGpmLoader._obtainPpeTexturesMetadataSchema`
/// up to the `MetadataSchema.fromJson` call.
///
/// DEVIATION: the JS version returns a `MetadataSchema` object and
/// caches it globally by identifier key; `MetadataSchema` is not ported
/// yet (model pipeline, MD-01), so this port returns the schema JSON and
/// does not cache it. `schema_index` mirrors the JS cache-size suffix of
/// the generated schema id.
pub fn obtain_ppe_textures_metadata_schema_json(
    mesh_primitive_gpm_local: &MeshPrimitiveGpmLocal,
    schema_index: usize,
) -> Value {
    let mut classes = serde_json::Map::new();
    for (i, ppe_texture) in mesh_primitive_gpm_local.ppe_textures().iter().enumerate() {
        let class_id = format!("ppeTexture_{}", i);
        classes.insert(
            class_id,
            create_ppe_texture_class_json(ppe_texture, i),
        );
    }
    json!({
        "id": format!("PPE_TEXTURE_SCHEMA_{}", schema_index),
        "classes": Value::Object(classes),
    })
}
