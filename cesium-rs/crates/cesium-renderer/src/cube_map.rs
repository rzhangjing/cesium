//! Ported from `packages/engine/Source/Renderer/CubeMap.js`.
//!
//! A cube map texture consisting of six 2D textures.

use cesium_core::create_guid::create_guid;
use crate::sampler::Sampler;

/// A cube map texture with six faces.
///
/// DEVIATION: In wgpu, cube maps are 2D array textures with 6 layers.
pub struct CubeMap {
    id: String,
    wgpu_texture: wgpu::Texture,
    width: u32,
    height: u32,
    sampler: Sampler,
    is_destroyed: bool,
}

impl CubeMap {
    /// Creates a new cube map texture.
    pub fn new(device: &wgpu::Device, width: u32, height: u32, sampler: Option<Sampler>) -> Self {
        let wgpu_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        Self {
            id: create_guid(),
            wgpu_texture,
            width,
            height,
            sampler: sampler.unwrap_or_default(),
            is_destroyed: false,
        }
    }

    /// Returns the unique identifier.
    pub fn id(&self) -> &str { &self.id }

    /// Returns the face width.
    pub fn width(&self) -> u32 { self.width }

    /// Returns the face height.
    pub fn height(&self) -> u32 { self.height }

    /// Returns the sampler.
    pub fn sampler(&self) -> &Sampler { &self.sampler }

    /// Returns a reference to the underlying wgpu texture.
    pub fn wgpu_texture(&self) -> &wgpu::Texture { &self.wgpu_texture }

    /// Returns whether this cube map has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys the cube map.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}
