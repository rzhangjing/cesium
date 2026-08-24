//! Ported from `packages/engine/Source/Scene/GltfImageLoader.js`.
//!
//! Loads a glTF image (embedded in a buffer view or a `data:` URI) and
//! decodes it to RGBA pixels.
//!
//! DEVIATION: the JS loader fetches external image URIs through the
//! `ResourceCache` (async network); the Rust port decodes embedded
//! buffer-view images and `data:` URIs synchronously, and accepts
//! externally fetched bytes via [`GltfImageLoader::load_external`].
//! External file/http URIs return a load error (fetch deferred).
//!
//! DEVIATION: image decode uses the `image` crate (PNG/JPEG); the JS
//! delegates to `ImageLoader`/browser decoders (which additionally handle
//! KTX2 — KTX2 remains deferred).

use cesium_core::runtime_error::RuntimeError;

use crate::gltf_buffer_view_loader::GltfBufferViewLoader;
use crate::gltf_loader::GltfJson;
use crate::resource_loader_state::ResourceLoaderState;

/// A decoded image (RGBA8 pixels), mirroring the JS `image` result
/// (`ImageBitmap` / `HTMLImageElement`; the Rust port keeps raw pixels).
#[derive(Debug, Clone)]
pub struct DecodedImage {
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
    /// Row-major RGBA8 pixels (width × height × 4 bytes).
    pub pixels: Vec<u8>,
}

/// Options for [`GltfImageLoader::try_new`], mirroring the JS
/// constructor's `options` object.
pub struct GltfImageLoaderOptions {
    /// The image ID to load.
    pub image_id: u32,
    /// The cache key of the resource.
    pub cache_key: Option<String>,
}

/// Loads glTF images.
///
/// Rust analogue of the CesiumJS `GltfImageLoader` (`ResourceLoader`
/// interface); see module docs for the deviations.
pub struct GltfImageLoader {
    image_id: u32,
    cache_key: Option<String>,
    image: Option<DecodedImage>,
    state: ResourceLoaderState,
}

impl GltfImageLoader {
    /// Creates a new GltfImageLoader.
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when the image ID is out of range.
    pub fn try_new(
        gltf: &GltfJson,
        options: GltfImageLoaderOptions,
    ) -> Result<GltfImageLoader, RuntimeError> {
        if options.image_id as usize >= gltf.images.len() {
            return Err(RuntimeError::new(Some(&format!(
                "imageId {} is out of range.",
                options.image_id
            ))));
        }
        Ok(GltfImageLoader {
            image_id: options.image_id,
            cache_key: options.cache_key,
            image: None,
            state: ResourceLoaderState::Unloaded,
        })
    }

    /// The cache key of the resource.
    pub fn cache_key(&self) -> Option<&str> {
        self.cache_key.as_deref()
    }

    /// The decoded image (defined after a successful load).
    pub fn image(&self) -> Option<&DecodedImage> {
        self.image.as_ref()
    }

    /// The current loader state.
    pub fn state(&self) -> ResourceLoaderState {
        self.state
    }

    /// Loads the image from the in-memory glTF.
    ///
    /// Mirrors `load()` / `loadFromBufferView` for images embedded in a
    /// buffer view, plus `data:` URI images.
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when the image has neither a buffer
    /// view nor an embedded `data:` URI, or decode fails.
    pub fn load(&mut self, gltf: &GltfJson) -> Result<(), RuntimeError> {
        self.state = ResourceLoaderState::Loading;

        let image_header = &gltf.images[self.image_id as usize];

        let result = if let Some(buffer_view_id) = image_header.buffer_view {
            let mut buffer_view_loader =
                GltfBufferViewLoader::try_new(gltf, buffer_view_id, None)?;
            buffer_view_loader.load(gltf)?;
            let bytes = buffer_view_loader.typed_array().ok_or_else(|| {
                RuntimeError::new(Some(
                    "Failed to load image\nBuffer view is not loaded.",
                ))
            })?;
            decode_image(bytes)
        } else if let Some(uri) = &image_header.uri {
            if let Some(encoded) = uri.strip_prefix("data:") {
                let base64 = encoded
                    .split_once(",")
                    .map(|(_, payload)| payload)
                    .unwrap_or("");
                let bytes = decode_base64(base64).ok_or_else(|| {
                    RuntimeError::new(Some("Failed to load image\nInvalid data URI."))
                })?;
                decode_image(&bytes)
            } else {
                // DEVIATION: external URI fetching is deferred; the caller
                // supplies the bytes via load_external.
                Err(RuntimeError::new(Some(
                    "Failed to load image\nExternal image URIs must be fetched by the caller and passed to load_external.",
                )))
            }
        } else {
            Err(RuntimeError::new(Some(
                "Failed to load image\nImage has no bufferView or URI.",
            )))
        };

        match result {
            Ok(decoded) => {
                self.image = Some(decoded);
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

    /// Loads the image from externally fetched image bytes (PNG/JPEG).
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when decode fails.
    pub fn load_external(&mut self, image_bytes: &[u8]) -> Result<(), RuntimeError> {
        self.state = ResourceLoaderState::Loading;
        match decode_image(image_bytes) {
            Ok(decoded) => {
                self.image = Some(decoded);
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
        self.image = None;
    }
}

/// Decodes PNG/JPEG bytes into RGBA8 pixels via the `image` crate
/// (mirrors the browser image decode step of the JS `ImageLoader`).
fn decode_image(bytes: &[u8]) -> Result<DecodedImage, RuntimeError> {
    let dynamic = image::load_from_memory(bytes).map_err(|error| {
        RuntimeError::new(Some(&format!("Failed to load image\n{error}")))
    })?;
    let rgba = dynamic.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(DecodedImage {
        width,
        height,
        pixels: rgba.into_raw(),
    })
}

/// Minimal standard-alphabet base64 decoder (with padding), used for
/// `data:` URI images.
fn decode_base64(input: &str) -> Option<Vec<u8>> {
    fn value(byte: u8) -> Option<u8> {
        match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes: Vec<u8> = input
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    let bytes: Vec<u8> = bytes
        .into_iter()
        .take_while(|byte| *byte != b'=')
        .collect();
    let mut output = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let mut accumulator: u32 = 0;
        for (i, byte) in chunk.iter().enumerate() {
            accumulator |= u32::from(value(*byte)?) << (18 - 6 * i);
        }
        output.push((accumulator >> 16) as u8);
        if chunk.len() > 2 {
            output.push((accumulator >> 8) as u8);
        }
        if chunk.len() > 3 {
            output.push(accumulator as u8);
        }
    }
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_base64_round_trips() {
        // "hello world" in standard base64.
        let decoded = decode_base64("aGVsbG8gd29ybGQ=").unwrap();
        assert_eq!(decoded, b"hello world");
    }
}
