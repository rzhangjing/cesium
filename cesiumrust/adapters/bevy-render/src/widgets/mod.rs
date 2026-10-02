//! 小部件模块聚合：动画时钟、地理编码、场景模式选择等交互控件。
//!
//! [`CesiumWidgetPlugin`] 统一注册各控件资源并挂载其响应系统。
pub mod animation;
pub mod geocoder;
pub mod scene_mode_picker;

pub use animation::{
    animation_widget_system, setup_animation_widget, AnimationWidget,
};
pub use geocoder::{
    geocoder_widget_system, setup_geocoder_widget, GeocoderWidget,
};
pub use scene_mode_picker::{
    scene_mode_picker_system, setup_scene_mode_picker, SceneModeWidget,
};

use bevy::prelude::*;

/// 统一注册各交互控件资源与系统的 Bevy 插件。
pub struct CesiumWidgetPlugin;

impl Plugin for CesiumWidgetPlugin {
    /// 初始化三个控件资源并在 Update 阶段挂载其系统。
    ///
    /// # 参数
    /// - `app`：Bevy 应用
    fn build(&self, app: &mut App) {
        // 先注册资源，再一次性注册三个系统。
        app.init_resource::<AnimationWidget>()
            .init_resource::<GeocoderWidget>()
            .init_resource::<SceneModeWidget>()
            .add_systems(
                Update,
                (
                    // 三个控件系统彼此独立，同一阶段并行执行即可。
                    animation_widget_system,
                    geocoder_widget_system,
                    scene_mode_picker_system,
                ),
            );
    }
}
