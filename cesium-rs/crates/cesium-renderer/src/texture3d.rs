//! Ported from `packages/engine/Source/Renderer/Texture3D.js`.
//!
//! A 3D texture.

use cesium_core::create_guid::create_guid;
use crate::sampler::Sampler;

/// A 3D texture on the GPU.
pub struct Texture3D {
    id: String,
    wgpu_texture: wgpu::Texture,
    width: u32,
    height: u32,
    depth: u32,
    sampler: Sampler,
    is_destroyed: bool,
}

impl Texture3D {
    /// Creates a new 3D texture.
    pub fn new(device: &wgpu::Device, width: u32, height: u32, depth: u32) -> Self {
        let wgpu_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d { width, height, depth_or_array_layers: depth },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        Self {
            id: create_guid(),
            wgpu_texture,
            width,
            height,
            depth,
            sampler: Sampler::default(),
            is_destroyed: false,
        }
    }

    /// Returns the unique identifier.
    pub fn id(&self) -> &str { &self.id }
    /// Returns the width.
    pub fn width(&self) -> u32 { self.width }
    /// Returns the height.
    pub fn height(&self) -> u32 { self.height }
    /// Returns the depth.
    pub fn depth(&self) -> u32 { self.depth }
    /// Returns the sampler.
    pub fn sampler(&self) -> &Sampler { &self.sampler }
    /// Returns whether this texture has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }
    /// Destroys the texture.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}
