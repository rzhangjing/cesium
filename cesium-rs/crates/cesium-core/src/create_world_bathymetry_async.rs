//! Ported from `packages/engine/Source/Core/createWorldBathymetryAsync.js`.
//!
//! # DEVIATION (Ion/Scene dependency, deferred)
//!
//! Same as [`create_world_terrain_async`](crate::create_world_terrain_async) —
//! the full port requires async Ion resource resolution and a
//! `ResourceBackend` implementation. Registered in `docs/deferred.md`.

/// Creates world bathymetry data asynchronously.
///
/// DEVIATION: stub — see module-level documentation.
pub fn create_world_bathymetry_async() -> Option<()> {
    None
}
