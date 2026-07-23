//! cesium-scene-mode: Scene modes and morphing.
//!
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping:
//! - `Scene/SceneMode.js` → scene_mode

pub mod scene_mode;

pub use scene_mode::{
    compute_camera_for_mode, morph_position, project_to_2d, project_to_columbus_view,
    smoothstep, unproject_from_2d, MapProjection2D, MorphState, SceneMode,
};
