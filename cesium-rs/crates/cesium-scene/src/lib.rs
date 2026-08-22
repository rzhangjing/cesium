//! One-to-one port of `packages/engine/Source/Scene`.
//!
//! Scene graph of CesiumJS: Scene, Camera, Globe, primitives, imagery &
//! terrain providers, 3D Tiles tilesets, models, materials. Depends on
//! `cesium-core` (math/domain) and `cesium-renderer` (GPU commands).

#![forbid(unsafe_code)]
