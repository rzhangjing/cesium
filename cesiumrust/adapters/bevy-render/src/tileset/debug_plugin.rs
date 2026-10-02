//! 瓦片集调试插件：注册调试配置并挂载包围体/统计等可视化系统。
use bevy::prelude::*;

use super::debug_system::{
    debug_toggle_system, draw_bounding_volumes, spawn_stats_overlay, update_tile_stats,
    DebugConfig,
};

/// 开启包围体/统计信息叠加等调试可视化的 Bevy 插件。
pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    /// 初始化调试配置，在 Startup 生成统计叠加，在 Update 挂载切换/绘制/统计系统。
    ///
    /// # 参数
    /// - `app`：Bevy 应用
    fn build(&self, app: &mut App) {
        // 叠加仅生成一次（Startup），其余逐帧响应按键与刷新。
        app.init_resource::<DebugConfig>()
            .add_systems(Startup, spawn_stats_overlay)
            .add_systems(
                Update,
                (debug_toggle_system, draw_bounding_volumes, update_tile_stats),
            );
    }
}
