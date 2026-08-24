//! Ported from `packages/engine/Source/Scene/GltfTextureLoader.js`.
//!
//! Loads a glTF texture: resolves the referenced image through a
//! [`GltfImageLoader`] and creates the GPU [`Texture`].
//!
//! DEVIATION: the JS loader obtains the image through the `ResourceCache`;
//! the Rust port composes a [`GltfImageLoader`] directly against the
//! in-memory glTF (embedded images) or caller supplied external bytes.
//!
//! DEVIATION: the glTF sampler parameters (wrap/filter) are recorded on
//! the created [`Texture`], but the wgpu renderer binds the shared
//! default sampler at draw time; mipmaps are not generated.

use std::sync::Arc;

use cesium_core::runtime_error::RuntimeError;
use cesium_renderer::context::Context;
use cesium_renderer::sampler::Sampler;
use cesium_renderer::texture::{Texture, TextureOptions, TextureSource};
use cesium_renderer::texture_wrap::TextureWrap;

use crate::gltf_image_loader::GltfImageLoader;
use crate::gltf_loader::GltfJson;
use crate::resource_loader_state::ResourceLoaderState;

/// Options for [`GltfTextureLoader::try_new`], mirroring the JS
/// constructor's `options` object.
pub struct GltfTextureLoaderOptions {
    /// The texture ID to load.
    pub texture_id: u32,
    /// The cache key of the resource.
    pub cache_key: Option<String>,
}

/// Loads glTF textures.
///
/// Rust analogue of the CesiumJS `GltfTextureLoader` (`ResourceLoader`
/// interface); see module docs for the deviations.
pub struct GltfTextureLoader {
    texture_id: u32,
    cache_key: Option<String>,
    image_loader: GltfImageLoader,
    /// The glTF sampler index referenced by the texture (for wrap/filter
    /// recording on the created [`Texture`]).
    sampler_id: Option<u32>,
    texture: Option<Arc<Texture>>,
    state: ResourceLoaderState,
}

impl GltfTextureLoader {
    /// Creates a new GltfTextureLoader.
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when the texture ID is out of range or
    /// the texture has no image source.
    pub fn try_new(
        gltf: &GltfJson,
        options: GltfTextureLoaderOptions,
    ) -> Result<GltfTextureLoader, RuntimeError> {
        let texture = gltf
            .textures
            .get(options.texture_id as usize)
            .ok_or_else(|| {
                RuntimeError::new(Some(&format!(
                    "textureId {} is out of range.",
                    options.texture_id
                )))
            })?;
        let image_id = texture.source.ok_or_else(|| {
            RuntimeError::new(Some(&format!(
                "Texture {} has no source image.",
                options.texture_id
            )))
        })?;
        let image_loader = GltfImageLoader::try_new(
            gltf,
            crate::gltf_image_loader::GltfImageLoaderOptions {
                image_id,
                cache_key: None,
            },
        )?;
        Ok(GltfTextureLoader {
            texture_id: options.texture_id,
            cache_key: options.cache_key,
            image_loader,
            sampler_id: texture.sampler,
            texture: None,
            state: ResourceLoaderState::Unloaded,
        })
    }

    /// The cache key of the resource.
    pub fn cache_key(&self) -> Option<&str> {
        self.cache_key.as_deref()
    }

    /// The texture ID being loaded.
    pub fn texture_id(&self) -> u32 {
        self.texture_id
    }

    /// The current loader state.
    pub fn state(&self) -> ResourceLoaderState {
        self.state
    }

    /// Loads the texture's image from the in-memory glTF.
    ///
    /// Mirrors `load()` (delegates to the image loader; the GPU texture
    /// is created by [`Self::create_texture`]).
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when the image cannot be decoded.
    pub fn load(&mut self, gltf: &GltfJson) -> Result<(), RuntimeError> {
        self.state = ResourceLoaderState::Loading;
        match self.image_loader.load(gltf) {
            Ok(()) => {
                self.state = ResourceLoaderState::Ready;
                Ok(())
            }
            Err(error) => {
                self.state = ResourceLoaderState::Failed;
                Err(error)
            }
        }
    }

    /// Loads the texture's image from externally fetched image bytes.
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when decode fails.
    pub fn load_external(&mut self, image_bytes: &[u8]) -> Result<(), RuntimeError> {
        self.state = ResourceLoaderState::Loading;
        match self.image_loader.load_external(image_bytes) {
            Ok(()) => {
                self.state = ResourceLoaderState::Ready;
                Ok(())
            }
            Err(error) => {
                self.state = ResourceLoaderState::Failed;
                Err(error)
            }
        }
    }

    /// Creates the GPU texture from the decoded image.
    ///
    /// Rust analogue of the JS `createTexture(context)` step of the
    /// texture loader job. Rows are padded to the 256-byte alignment
    /// `write_texture` requires.
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] when the image is not loaded.
    pub fn create_texture(
        &mut self,
        context: &Context,
        gltf: &GltfJson,
    ) -> Result<(), RuntimeError> {
        let image = self.image_loader.image().ok_or_else(|| {
            RuntimeError::new(Some(
                "Failed to create texture\nImage is not loaded.",
            ))
        })?;
        let width = image.width;
        let height = image.height;

        // Pad rows to the 256-byte alignment wgpu copies require.
        let row_bytes = width as usize * 4;
        let padded_row_bytes = (row_bytes + 255) & !255;
        let mut pixels = Vec::with_capacity(padded_row_bytes * height as usize);
        for row in 0..height as usize {
            let start = row * row_bytes;
            pixels.extend_from_slice(&image.pixels[start..start + row_bytes]);
            pixels.resize(pixels.len() + (padded_row_bytes - row_bytes), 0);
        }

        let sampler = self.create_sampler(gltf);
        let texture = Texture::new(
            context.device(),
            TextureOptions {
                source: None,
                width: Some(width),
                height: Some(height),
                sampler: Some(sampler),
                ..Default::default()
            },
        );
        texture.upload_source(
            context.queue(),
            &TextureSource {
                width,
                height,
                array_buffer_view: pixels,
            },
        );
        self.texture = Some(Arc::new(texture));
        Ok(())
    }

    /// Maps the glTF sampler's wrap modes onto the renderer sampler
    /// (filter/mipmap parameters fall back to the linear defaults;
    /// DEVIATION noted in the module docs).
    fn create_sampler(&self, gltf: &GltfJson) -> Sampler {
        let mut sampler = Sampler::new();
        if let Some(sampler_id) = self.sampler_id {
            if let Some(gltf_sampler) = gltf.samplers.get(sampler_id as usize) {
                sampler.wrap_s = wrap_mode(gltf_sampler.wrap_s);
                sampler.wrap_t = wrap_mode(gltf_sampler.wrap_t);
            }
        }
        sampler
    }

    /// The GPU texture (defined after a successful
    /// [`Self::create_texture`]).
    pub fn texture(&self) -> Option<Arc<Texture>> {
        self.texture.clone()
    }

    /// Unloads the resource, mirroring `unload()`.
    pub fn unload(&mut self) {
        self.image_loader.unload();
        self.texture = None;
    }
}

/// Maps a glTF wrap constant (10497 REPEAT / 33071 CLAMP_TO_EDGE /
/// 33648 MIRRORED_REPEAT) onto the renderer [`TextureWrap`].
fn wrap_mode(mode: u32) -> TextureWrap {
    match mode {
        33071 => TextureWrap::ClampToEdge,
        33648 => TextureWrap::MirroredRepeat,
        _ => TextureWrap::Repeat,
    }
}
