//! 用于 Bevy 的 Cesium 实体插件。
//!
//! 提供实体渲染、时间动态更新与 visualizer 系统。

pub mod components;
pub mod time_system;
pub mod visualizer;

use bevy::prelude::*;

use self::components::GlobeEllipsoid;
use self::time_system::{entity_visibility_system, time_dynamic_update_system, AnimationClock};
use self::visualizer::{billboard_face_camera_system, entity_visualizer_system};

pub struct CesiumEntityPlugin;

impl Plugin for CesiumEntityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GlobeEllipsoid>()
            // AnimationClock 也由 CesiumCorePlugin（lib.rs）注册。
            // init_resource 是幂等的 —— 双重注册无害，且
            // 确保 CesiumEntityPlugin 无需 CesiumCorePlugin 即可独立工作。
            .init_resource::<AnimationClock>()
            .add_systems(
                Update,
                (
                    time_dynamic_update_system,
                    entity_visualizer_system,
                    entity_visibility_system,
                    billboard_face_camera_system,
                ),
            );
    }
}
