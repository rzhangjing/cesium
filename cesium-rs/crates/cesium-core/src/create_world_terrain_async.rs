//! Ported from `packages/engine/Source/Core/createWorldTerrainAsync.js`.
//!
//! # DEVIATION (Ion/Scene dependency, deferred)
//!
//! `createWorldTerrainAsync` in JS creates a `CesiumTerrainProvider`
//! configured with Cesium World Terrain URLs from Ion. The full port
//! requires:
//! 1. `IonResource` (async Ion asset resolution) — partially ported in
//!    `ion_resource.rs` but needs async HTTP backend
//! 2. `CesiumTerrainProvider.fromUrl` (async factory) — available but
//!    requires `ResourceBackend` at call site
//!
//! Since this function is primarily a convenience wrapper that delegates
//! to `CesiumTerrainProvider.fromUrl`, the full async port is deferred
//! until the application layer provides a `ResourceBackend` implementation.
//!
//! Registered in `docs/deferred.md`.

/// Options for [`create_world_terrain_async`].
///
/// Mirrors `createWorldTerrainAsync` `options`.
#[derive(Debug, Clone, Default)]
pub struct CreateWorldTerrainOptions {
    /// Whether to request vertex normals from the terrain server.
    pub request_vertex_normals: Option<bool>,
    /// Whether to request water mask from the terrain server.
    pub request_water_mask: Option<bool>,
}

/// Creates a [`CesiumTerrainProvider`](crate::cesium_terrain_provider::CesiumTerrainProvider)
/// configured for Cesium World Terrain.
///
/// DEVIATION: the JS function is async and returns
/// `Promise<CesiumTerrainProvider>`. This Rust port is a stub — the full
/// async factory requires `ResourceBackend` which belongs to the
/// application layer. See module-level DEVIATION notes.
///
/// Returns `None` to indicate the factory is not yet fully ported.
pub fn create_world_terrain_async(
    _options: Option<CreateWorldTerrainOptions>,
) -> Option<()> {
    // DEVIATION: stub — full implementation requires async Ion resource
    // resolution and CesiumTerrainProvider.fromUrl with a ResourceBackend.
    None
}

// Note: `create_world_bathymetry_async` is in its own module.
