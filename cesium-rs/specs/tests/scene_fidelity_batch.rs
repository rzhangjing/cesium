//! Scene fidelity batch: one-to-one Rust mirrors of the CesiumJS Jasmine
//! scene specs that are runnable headless (Track A7 geocoder batch).
//!
//! Mirrors:
//! - `packages/engine/Specs/Scene/computeFlyToLocationForRectangleSpec.js`
//!   -> `scene_fidelity/compute_fly_to_location_for_rectangle_spec.rs`
//!
//! Additional Scene type specs (substantiated from stubs):
//! - Light/DirectionalLight/SunLight
//! - TileDiscardPolicy family
//! - EllipsoidSurfaceAppearance
//! - Particle/ParticleEmitter + BoxEmitter/CircleEmitter/ConeEmitter/SphereEmitter
//! - FrustumCommands
//! - MetadataType/MetadataEnumValue/MetadataComponentType

#[path = "scene_fidelity/compute_fly_to_location_for_rectangle_spec.rs"]
mod compute_fly_to_location_for_rectangle_spec;

#[path = "scene_fidelity/scene_types_spec.rs"]
mod scene_types_spec;

#[path = "scene_fidelity/scene_types_batch2_spec.rs"]
mod scene_types_batch2_spec;
