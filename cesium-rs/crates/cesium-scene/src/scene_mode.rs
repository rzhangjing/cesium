//! Ported from `packages/engine/Source/Scene/SceneMode.js`.
//!
//! The type itself now lives in `cesium-core` (see
//! [`cesium_core::scene_mode`]) because the core-level picking path
//! (`TerrainPicker`, `TerrainMesh`) branches on it and `cesium-core` cannot
//! depend on `cesium-scene`. This module is kept as a re-export so the
//! historical `cesium_scene::scene_mode::SceneMode` path still resolves.

pub use cesium_core::scene_mode::SceneMode;
