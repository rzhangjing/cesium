//! Ported from `packages/engine/Source/Core/CustomHeightmapTerrainProvider.js`.
//!
//! A terrain provider that uses custom heightmap callbacks.

use crate::rectangle::Rectangle;

/// A terrain provider that uses user-provided heightmap callbacks.
/// Skeleton: requires network I/O.
pub struct CustomHeightmapTerrainProvider;

impl CustomHeightmapTerrainProvider {
    /// Whether the provider is ready.
    pub fn ready(&self) -> bool { false }
    /// Returns the rectangle.
    pub fn rectangle(&self) -> Rectangle { Rectangle::default() }
    /// Whether the provider has vertex normals.
    pub fn has_vertex_normals(&self) -> bool { false }
    /// Whether the provider has water mask.
    pub fn has_water_mask(&self) -> bool { false }
}
