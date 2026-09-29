//! cesium-scene-mode：场景模式与形态变换。
//!
//! 领域层 —— 纯 Rust，f64 精度。
//!
//! CesiumJS 映射：
//! - `Scene/SceneMode.js` → scene_mode

pub mod scene_mode;

pub use scene_mode::{
    compute_camera_for_mode, morph_position, project_to_2d, project_to_columbus_view,
    smoothstep, unproject_from_2d, MapProjection2D, MorphState, SceneMode,
};
