//! Ported from `packages/engine/Source/Scene/GltfJsonLoader.js`.
//!
//! Loads a glTF JSON from a glTF or GLB asset.
//!
//! The CesiumJS `GltfJsonLoader` is a `ResourceLoader` wired into the
//! `ResourceCache` promise pipeline. The Rust port exposes the same
//! processing steps (`processGltfTypedArray` / `processGltfJson` /
//! `decodeDataUris` / version + extension validation) as synchronous
//! functions operating on in-memory bytes; network fetching of external
//! buffer URIs is deferred to the async resource cache (T5).

use cesium_core::get_magic::get_magic;
use cesium_core::is_data_uri::is_data_uri;
use cesium_core::runtime_error::RuntimeError;

use crate::gltf_loader::GltfJson;
use crate::gltf_pipeline::parse_glb::parse_glb;
use crate::model::model_utility::ModelUtility;

/// The state of the loader, mirroring `ResourceLoaderState`
/// (`UNLOADED` / `LOADING` / `READY` / `FAILED`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GltfJsonLoaderState {
    /// Not yet loaded.
    Unloaded,
    /// Loading in progress.
    Loading,
    /// Loading completed successfully.
    Ready,
    /// Loading failed.
    Failed,
}

/// Loads a glTF JSON from a glTF or GLB.
///
/// Rust analogue of the CesiumJS `GltfJsonLoader` (`ResourceLoader`
/// interface). The parsed glTF JSON is available through
/// [`GltfJsonLoader::gltf`] once the state is
/// [`GltfJsonLoaderState::Ready`].
pub struct GltfJsonLoader {
    gltf: Option<GltfJson>,
    state: GltfJsonLoaderState,
    cache_key: Option<String>,
}

impl GltfJsonLoader {
    /// Creates a new GltfJsonLoader.
    pub fn new() -> Self {
        Self {
            gltf: None,
            state: GltfJsonLoaderState::Unloaded,
            cache_key: None,
        }
    }

    /// Creates a GltfJsonLoader with a cache key.
    pub fn with_cache_key(cache_key: String) -> Self {
        Self {
            gltf: None,
            state: GltfJsonLoaderState::Unloaded,
            cache_key: Some(cache_key),
        }
    }

    /// The cache key of the resource.
    pub fn cache_key(&self) -> Option<&str> {
        self.cache_key.as_deref()
    }

    /// The current loader state.
    pub fn state(&self) -> GltfJsonLoaderState {
        self.state
    }

    /// The glTF JSON (available when [`Self::state`] is
    /// [`GltfJsonLoaderState::Ready`]).
    pub fn gltf(&self) -> Option<&GltfJson> {
        self.gltf.as_ref()
    }

    /// Loads the glTF JSON from a typed array containing glTF or GLB bytes.
    ///
    /// Mirrors `GltfJsonLoader.load()` / `processGltfTypedArray`.
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when the bytes are neither valid glTF
    /// JSON nor a valid GLB container, or validation fails (unsupported
    /// version / required extension).
    pub fn load_from_typed_array(&mut self, typed_array: &[u8]) -> Result<(), RuntimeError> {
        self.state = GltfJsonLoaderState::Loading;
        let gltf = if get_magic(typed_array, None) == "glTF" {
            parse_glb(typed_array)?
        } else {
            get_json_from_typed_array(typed_array)?
        };
        self.process_gltf_json(gltf)
    }

    /// Loads the glTF JSON from an already-parsed JSON string.
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when parsing or validation fails.
    pub fn load_from_json_string(&mut self, json: &str) -> Result<(), RuntimeError> {
        self.state = GltfJsonLoaderState::Loading;
        let gltf: GltfJson = serde_json::from_str(json).map_err(|e| {
            RuntimeError::new(Some(&format!("Failed to load glTF: {e}")))
        })?;
        self.process_gltf_json(gltf)
    }

    /// Loads the glTF JSON from an already-parsed [`GltfJson`] value.
    ///
    /// Mirrors `processGltfJson(this, gltfJson)`.
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when validation fails.
    pub fn load_from_gltf_json(&mut self, gltf: GltfJson) -> Result<(), RuntimeError> {
        self.state = GltfJsonLoaderState::Loading;
        self.process_gltf_json(gltf)
    }

    fn process_gltf_json(&mut self, mut gltf: GltfJson) -> Result<(), RuntimeError> {
        let result = (|| -> Result<(), RuntimeError> {
            decode_data_uris(&mut gltf);

            // DEVIATION: CesiumJS runs updateVersion() here to upgrade
            // glTF 1.0 assets to 2.0; the upgrade pipeline is deferred, so
            // only the final version check is mirrored.
            let version = gltf.asset.version.as_str();
            if version != "1.0" && version != "2.0" {
                return Err(RuntimeError::new(Some(&format!(
                    "Unsupported glTF version: {version}"
                ))));
            }

            if !gltf.extensions_required.is_empty() {
                ModelUtility::check_supported_extensions(&gltf.extensions_required)?;
            }

            Ok(())
        })();

        match result {
            Ok(()) => {
                self.gltf = Some(gltf);
                self.state = GltfJsonLoaderState::Ready;
                Ok(())
            }
            Err(error) => {
                self.gltf = None;
                self.state = GltfJsonLoaderState::Failed;
                Err(error)
            }
        }
    }
}

impl Default for GltfJsonLoader {
    fn default() -> Self { Self::new() }
}

/// Parses glTF JSON from a UTF-8 typed array.
///
/// Mirrors `Core/getJsonFromTypedArray.js`.
///
/// # Errors
/// Returns a [`RuntimeError`] when the bytes are not valid UTF-8 or JSON.
pub fn get_json_from_typed_array(typed_array: &[u8]) -> Result<GltfJson, RuntimeError> {
    let text = std::str::from_utf8(typed_array).map_err(|e| {
        RuntimeError::new(Some(&format!("Failed to decode glTF JSON: {e}")))
    })?;
    serde_json::from_str(text).map_err(|e| {
        RuntimeError::new(Some(&format!("Failed to load glTF: {e}")))
    })
}

/// Decodes base64 data URI buffers in place.
///
/// Mirrors `decodeDataUris(gltf)` in GltfJsonLoader.js: for every buffer
/// with a data URI (and no embedded source), the URI is decoded into
/// `buffer.data` and the URI is removed.
fn decode_data_uris(gltf: &mut GltfJson) {
    for buffer in gltf.buffers.iter_mut() {
        let buffer_uri = buffer.uri.clone();
        if buffer.data.is_none()
            && buffer_uri
                .as_deref()
                .map(|uri| is_data_uri(Some(uri)))
                .unwrap_or(false)
        {
            if let Some(uri) = buffer_uri {
                if let Some(bytes) = decode_base64_data_uri(&uri) {
                    // Delete the data URI to keep the cached glTF JSON small
                    buffer.uri = None;
                    buffer.data = Some(bytes);
                }
            }
        }
    }
}

/// Decodes a `data:` URI (base64 or percent-encoded payload) into bytes.
///
/// Rust analogue of the data-URI branch of `Resource.fetchArrayBuffer`.
fn decode_base64_data_uri(uri: &str) -> Option<Vec<u8>> {
    let rest = uri.strip_prefix("data:")?;
    let comma = rest.find(',')?;
    let metadata = &rest[..comma];
    let payload = &rest[comma + 1..];
    if metadata.ends_with(";base64") {
        decode_base64(payload)
    } else {
        // Percent-encoded payload
        percent_decode(payload)
    }
}

fn decode_base64(input: &str) -> Option<Vec<u8>> {
    const DECODE: [i8; 256] = build_base64_table();
    let mut output = Vec::with_capacity(input.len() * 3 / 4);
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;
    for &byte in input.as_bytes() {
        if byte == b'=' || byte == b'\n' || byte == b'\r' || byte == b' ' {
            continue;
        }
        let value = DECODE[byte as usize];
        if value < 0 {
            return None;
        }
        buffer = (buffer << 6) | value as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Some(output)
}

const fn build_base64_table() -> [i8; 256] {
    let mut table = [-1i8; 256];
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut i = 0;
    while i < alphabet.len() {
        table[alphabet[i] as usize] = i as i8;
        i += 1;
    }
    table
}

fn percent_decode(input: &str) -> Option<Vec<u8>> {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let high = hex_value(bytes[i + 1])?;
            let low = hex_value(bytes[i + 2])?;
            output.push(high * 16 + low);
            i += 3;
        } else {
            output.push(bytes[i]);
            i += 1;
        }
    }
    Some(output)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
