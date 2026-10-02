// legacy CesiumJS-port style debt (deferred.md #18); revisit at M13 lint-cleanup 或本文件在其里程碑被重写时
//! 3D Tiles 调试可视化：开关式展示包围球、瓦片统计与线框模式。
//!
//! [`debug_toggle_system`] 监听 F1/F2/F3 切换 [`DebugConfig`] 开关；
//! [`draw_bounding_volumes`] 用 gizmos 沿三个坐标平面画包围圆；
//! [`update_tile_stats`] 按瓦片状态计数并刷新叠层文本。
#![allow(clippy::derivable_impls)]
use bevy::prelude::*;
use bevy::input::ButtonInput;
use bevy::gizmos::gizmos::Gizmos;

use crate::components::CesiumTileNode;
use crate::resources::RenderScale;

/// 调试叠层的开关配置资源。
#[derive(Resource)]
pub struct DebugConfig {
    /// 是否绘制包围体（F1）。
    pub show_bounding_volumes: bool,
    /// 是否显示瓦片统计文本（F2）。
    pub show_tile_stats: bool,
    /// 是否启用线框模式（F3）。
    pub wireframe_mode: bool,
}

impl Default for DebugConfig {
    /// 默认：全部关闭。
    fn default() -> Self {
        Self {
            show_bounding_volumes: false,
            show_tile_stats: false,
            wireframe_mode: false,
        }
    }
}

/// 标记瓦片统计叠层文本实体。
#[derive(Component)]
pub struct TilesetStatsText;

/// 切换系统：F1/F2/F3 分别翻转三个调试开关并回显状态。
///
/// # 参数
/// - `keys`：按键输入状态
/// - `config`：调试配置（可写）
pub fn debug_toggle_system(keys: Res<ButtonInput<KeyCode>>, mut config: ResMut<DebugConfig>) {
    // F1：切换包围体显示。
    if keys.just_pressed(KeyCode::F1) {
        config.show_bounding_volumes = !config.show_bounding_volumes;
        info!("Bounding volumes: {}", config.show_bounding_volumes);
    }
    // F2：切换瓦片统计显示。
    if keys.just_pressed(KeyCode::F2) {
        config.show_tile_stats = !config.show_tile_stats;
        info!("Tile stats: {}", config.show_tile_stats);
    }
    // F3：切换线框模式。
    if keys.just_pressed(KeyCode::F3) {
        config.wireframe_mode = !config.wireframe_mode;
        info!("Wireframe mode: {}", config.wireframe_mode);
    }
}

/// 绘制包围体：对每个瓦片节点沿三平面画圆，颜色随 SSE 从绿到红。
///
/// # 参数
/// - `config`：调试配置（总开关）
/// - `tiles`：瓦片节点查询
/// - `render_scale`：渲染缩放（世界→渲染坐标）
/// - `gizmos`：线条绘制器
pub fn draw_bounding_volumes(
    config: Res<DebugConfig>,
    tiles: Query<&CesiumTileNode>,
    render_scale: Res<RenderScale>,
    mut gizmos: Gizmos,
) {
    // 开关关闭时直接返回，不产生任何绘制。
    if !config.show_bounding_volumes {
        return;
    }

    let scale = render_scale.0;
    let segments = 32u32;

    for node in tiles.iter() {
        // 无包围球信息的节点跳过。
        let (Some(center), Some(radius)) = (node.bounding_sphere_center, node.bounding_sphere_radius)
        else {
            continue;
        };

        // 世界坐标→渲染坐标（除以缩放）。
        let center_render = (center / scale).as_vec3();
        let radius_render = (radius / scale) as f32;

        // 用屏幕空间误差决定颜色：低误差偏绿、高误差偏红。
        let sse = node.screen_space_error as f32;
        let t = (sse / 32.0).clamp(0.0, 1.0);
        let color = Color::hsl(120.0 * (1.0 - t), 1.0, 0.5);

        // 分别在 XY/XZ/YZ 三个平面画圆，组成线框球。
        for plane in 0..3u32 {
            for i in 0..segments {
                // 当前段的两端角度（均匀采样圆周一圈）。
                let angle0 = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
                let angle1 =
                    2.0 * std::f32::consts::PI * (i + 1) as f32 / segments as f32;
                let (sin0, cos0) = angle0.sin_cos();
                let (sin1, cos1) = angle1.sin_cos();

                // 按平面选择圆所在的两个轴（第三个轴固定为 0）。
                let (p0, p1) = match plane {
                    0 => (
                        Vec3::new(cos0 * radius_render, sin0 * radius_render, 0.0),
                        Vec3::new(cos1 * radius_render, sin1 * radius_render, 0.0),
                    ),
                    1 => (
                        Vec3::new(cos0 * radius_render, 0.0, sin0 * radius_render),
                        Vec3::new(cos1 * radius_render, 0.0, sin1 * radius_render),
                    ),
                    _ => (
                        Vec3::new(0.0, cos0 * radius_render, sin0 * radius_render),
                        Vec3::new(0.0, cos1 * radius_render, sin1 * radius_render),
                    ),
                };

                // 把圆上相邻两点平移到球心后画为线段。
                gizmos.line(center_render + p0, center_render + p1, color);
            }
        }
    }
}

/// 统计系统：按瓦片内容状态计数并刷新叠层文本（开关控制显隐）。
///
/// # 参数
/// - `config`：调试配置
/// - `tiles`：瓦片节点查询
/// - `stats_query`：叠层文本与可见性
pub fn update_tile_stats(
    config: Res<DebugConfig>,
    tiles: Query<&CesiumTileNode>,
    mut stats_query: Query<(&mut Text, &mut Visibility), With<TilesetStatsText>>,
) {
    // 无叠层实体时直接返回。
    let Ok((mut text, mut visibility)) = stats_query.get_single_mut() else {
        return;
    };

    if config.show_tile_stats {
        *visibility = Visibility::Visible;

        // 分类计数：总数与就绪/加载中/未加载三种状态。
        let total = tiles.iter().count();
        let ready = tiles
            .iter()
            .filter(|t| matches!(t.state, crate::components::TileContentState::Ready))
            .count();
        let loading = tiles
            .iter()
            .filter(|t| matches!(t.state, crate::components::TileContentState::Loading))
            .count();
        let unloaded = tiles
            .iter()
            .filter(|t| matches!(t.state, crate::components::TileContentState::Unloaded))
            .count();

        // 拼接统计文本（就绪/加载中/待加载）。
        text.0 = format!(
            "Tiles: {} total | {} ready | {} loading | {} pending",
            total, ready, loading, unloaded
        );
    } else {
        // 开关关闭：隐藏叠层。
        *visibility = Visibility::Hidden;
    }
}

/// 生成瓦片统计叠层（左上角绝对定位，初始隐藏）。
///
/// # 参数
/// - `commands`：实体命令写入器
pub fn spawn_stats_overlay(mut commands: Commands) {
    commands.spawn((
        Text::new("Tiles: 0 total | 0 ready | 0 loading | 0 pending"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::WHITE),
        TilesetStatsText,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        Visibility::Hidden,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 验证默认下三个开关均为关闭。
    fn test_debug_config_default() {
        let config = DebugConfig::default();
        assert!(!config.show_bounding_volumes);
        assert!(!config.show_tile_stats);
        assert!(!config.wireframe_mode);
    }

    #[test]
    /// 验证逐个翻转开关后再翻转可回到原状态。
    fn test_debug_config_toggle() {
        let mut config = DebugConfig::default();

        config.show_bounding_volumes = !config.show_bounding_volumes;
        assert!(config.show_bounding_volumes);

        config.show_tile_stats = !config.show_tile_stats;
        assert!(config.show_tile_stats);

        config.wireframe_mode = !config.wireframe_mode;
        assert!(config.wireframe_mode);

        config.show_bounding_volumes = !config.show_bounding_volumes;
        assert!(!config.show_bounding_volumes);

        config.show_tile_stats = !config.show_tile_stats;
        assert!(!config.show_tile_stats);

        config.wireframe_mode = !config.wireframe_mode;
        assert!(!config.wireframe_mode);
    }
}
