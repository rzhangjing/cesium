//! cesium-ports-driven: Driven ports (Domain → External)
//! Trait contracts for external services that the domain depends on.
//!
//! In hexagonal architecture, driven ports define how the domain
//! communicates with external systems (adapters implement these traits).

use cesium_geospatial::{GeometryData, Rectangle};
use std::future::Future;
use std::pin::Pin;

/// Error type for port operations.
#[derive(Debug, thiserror::Error)]
pub enum PortError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("Decode error: {0}")]
    Decode(String),
    #[error("Cache error: {0}")]
    Cache(String),
    #[error("GPU error: {0}")]
    Gpu(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Cancelled")]
    Cancelled,
}

/// Result type for port operations.
pub type PortResult<T> = Result<T, PortError>;

// ============================================================================
// Data Fetching Ports
// ============================================================================

/// Fetches raw bytes from a URL.
/// Implemented by HTTP adapters (reqwest, browser fetch, etc.)
pub trait TileFetcher: Send + Sync {
    /// Fetches bytes from the given URL.
    fn fetch<'a>(
        &'a self,
        url: &'a str,
        priority: f64,
    ) -> Pin<Box<dyn Future<Output = PortResult<Vec<u8>>> + Send + 'a>>;

    /// Cancels a pending fetch.
    fn cancel(&self, url: &str);
}

/// Fetches imagery tiles.
pub trait ImageryProvider: Send + Sync {
    /// Gets the rectangle covered by this imagery provider.
    fn rectangle(&self) -> Rectangle;

    /// Gets the minimum zoom level.
    fn minimum_level(&self) -> u32;

    /// Gets the maximum zoom level.
    fn maximum_level(&self) -> u32;

    /// Gets the tile width in pixels.
    fn tile_width(&self) -> u32;

    /// Gets the tile height in pixels.
    fn tile_height(&self) -> u32;

    /// Requests an imagery tile.
    fn request_image<'a>(
        &'a self,
        x: u32,
        y: u32,
        level: u32,
    ) -> Pin<Box<dyn Future<Output = PortResult<Vec<u8>>> + Send + 'a>>;
}

/// Fetches terrain tiles.
pub trait TerrainProvider: Send + Sync {
    /// Gets the rectangle covered by this terrain provider.
    fn rectangle(&self) -> Rectangle;

    /// Gets the maximum zoom level.
    fn maximum_level(&self) -> u32;

    /// Requests a terrain tile.
    fn request_tile_geometry<'a>(
        &'a self,
        x: u32,
        y: u32,
        level: u32,
    ) -> Pin<Box<dyn Future<Output = PortResult<GeometryData>> + Send + 'a>>;

    /// Gets the availability of terrain data at a position.
    fn get_availability(&self, x: u32, y: u32, level: u32) -> bool;
}

// ============================================================================
// GPU/Rendering Ports
// ============================================================================

/// A handle to a GPU resource (texture, buffer, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GpuHandle(pub u64);

/// Sends geometry and textures to the GPU.
/// Implemented by rendering adapters (Bevy, wgpu, etc.)
pub trait GpuSink: Send + Sync {
    /// Uploads geometry data to the GPU, returns a handle.
    fn upload_geometry(&mut self, geometry: &GeometryData) -> PortResult<GpuHandle>;

    /// Uploads texture data to the GPU, returns a handle.
    fn upload_texture(
        &mut self,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> PortResult<GpuHandle>;

    /// Updates an existing texture.
    fn update_texture(
        &mut self,
        handle: GpuHandle,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> PortResult<()>;

    /// Deletes a GPU resource.
    fn delete(&mut self, handle: GpuHandle);
}

// ============================================================================
// Decoding Ports
// ============================================================================

/// Decodes compressed/encoded data formats.
pub trait Decoder: Send + Sync {
    /// Decodes Draco-compressed geometry.
    fn decode_draco(&self, data: &[u8]) -> PortResult<GeometryData>;

    /// Decodes an image (PNG, JPEG, WebP, etc.)
    fn decode_image(&self, data: &[u8]) -> PortResult<DecodedImage>;

    /// Decodes gzip-compressed data.
    fn decode_gzip(&self, data: &[u8]) -> PortResult<Vec<u8>>;
}

/// A decoded image with raw pixel data.
#[derive(Debug, Clone)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub channels: u32,
    pub data: Vec<u8>,
}

// ============================================================================
// Caching Ports
// ============================================================================

/// A cache for storing fetched/decoded data.
pub trait Cache: Send + Sync {
    /// Gets a cached value by key.
    fn get(&self, key: &str) -> Option<Vec<u8>>;

    /// Stores a value in the cache.
    fn set(&self, key: &str, value: Vec<u8>);

    /// Removes a value from the cache.
    fn remove(&self, key: &str) -> bool;

    /// Clears all cached data.
    fn clear(&self);

    /// Gets the current cache size in bytes.
    fn size(&self) -> usize;

    /// Gets the maximum cache size in bytes.
    fn max_size(&self) -> usize;
}

// ============================================================================
// Time/Clock Ports
// ============================================================================

/// Provides the current system time.
pub trait SystemClock: Send + Sync {
    /// Gets the current time in seconds since Unix epoch.
    fn now_secs(&self) -> f64;

    /// Gets the elapsed time since the last call (for frame timing).
    fn delta_secs(&mut self) -> f64;
}

// ============================================================================
// Scene/Rendering Ports
// ============================================================================

/// Provides access to the rendering context.
pub trait RenderContext: Send + Sync {
    /// Gets the drawing buffer width.
    fn drawing_buffer_width(&self) -> u32;

    /// Gets the drawing buffer height.
    fn drawing_buffer_height(&self) -> u32;

    /// Gets the device pixel ratio.
    fn device_pixel_ratio(&self) -> f64;
}
