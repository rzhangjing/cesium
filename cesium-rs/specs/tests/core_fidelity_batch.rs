//! Core fidelity batch: one-to-one Rust mirrors of the CesiumJS Jasmine
//! specs for the Track A1/A5 fidelity backfill (Color / Ellipsoid / Resource).
//!
//! This is a *new* top-level test entry; the `tests/core.rs` aggregator is
//! intentionally left untouched.
//!
//! Mirrors:
//! - `packages/engine/Specs/Core/ColorSpec.js`      -> `core_fidelity/color_fidelity_spec.rs`
//! - `packages/engine/Specs/Core/EllipsoidSpec.js`  -> `core_fidelity/ellipsoid_fidelity_spec.rs`
//! - `packages/engine/Specs/Core/ResourceSpec.js`   -> `core_fidelity/resource_fidelity_spec.rs`
//!
//! Track A4 (terrain):
//! - `packages/engine/Specs/Core/HeightmapTerrainDataSpec.js`      -> `core_fidelity/terrain_fidelity_spec.rs`
//! - `packages/engine/Specs/Core/ApproximateTerrainHeightsSpec.js` -> `core_fidelity/terrain_fidelity_spec.rs`
//! - `packages/engine/Specs/Core/QuantizedMeshTerrainDataSpec.js`  -> `core_fidelity/terrain_fidelity_spec.rs`
//!
//! Track A1/A4-A7 fidelity-fix regressions:
//! - `packages/engine/Specs/Core/Matrix4Spec.js` (multiplyByPoint*) -> `core_fidelity/matrix_obb_fidelity_spec.rs`
//! - `packages/engine/Specs/Core/PerspectiveOffCenterFrustumSpec.js` -> `core_fidelity/matrix_obb_fidelity_spec.rs`
//! - `packages/engine/Specs/Core/OrientedBoundingBoxSpec.js` (fromRectangle) -> `core_fidelity/matrix_obb_fidelity_spec.rs`
//!
//! Track A6 (misc):
//! - `packages/engine/Specs/Core/FullscreenSpec.js` -> `core_fidelity/fullscreen_fidelity_spec.rs`
//!
//! Track A7 (geocoder):
//! - `packages/engine/Specs/Core/PeliasGeocoderServiceSpec.js`        -> `core_fidelity/geocoder_fidelity_spec.rs`
//! - `packages/engine/Specs/Core/IonGeocoderServiceSpec.js`           -> `core_fidelity/geocoder_fidelity_spec.rs`
//! - `packages/engine/Specs/Core/CartographicGeocoderServiceSpec.js`  -> `core_fidelity/geocoder_fidelity_spec.rs`
//! - `packages/engine/Specs/Core/BingMapsGeocoderServiceSpec.js`      -> `core_fidelity/geocoder_fidelity_spec.rs`
//! - `packages/engine/Specs/Core/GoogleGeocoderServicesSpec.js`       -> `core_fidelity/geocoder_fidelity_spec.rs`
//! - `packages/engine/Specs/Core/OpenCageGeocoderServiceSpec.js`      -> `core_fidelity/geocoder_fidelity_spec.rs`

#[path = "core_fidelity/color_fidelity_spec.rs"]
mod color_fidelity_spec;
#[path = "core_fidelity/ellipsoid_fidelity_spec.rs"]
mod ellipsoid_fidelity_spec;
#[path = "core_fidelity/resource_fidelity_spec.rs"]
mod resource_fidelity_spec;
#[path = "core_fidelity/terrain_fidelity_spec.rs"]
mod terrain_fidelity_spec;
#[path = "core_fidelity/matrix_obb_fidelity_spec.rs"]
mod matrix_obb_fidelity_spec;
#[path = "core_fidelity/fullscreen_fidelity_spec.rs"]
mod fullscreen_fidelity_spec;
#[path = "core_fidelity/geocoder_fidelity_spec.rs"]
mod geocoder_fidelity_spec;
