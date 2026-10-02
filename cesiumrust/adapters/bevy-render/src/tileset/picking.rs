//! 瓦片拾取插件：定义拾取事件并挂载鼠标→射线→报告三段系统。
use bevy::prelude::*;
use glam::DVec3;

use crate::resources::RenderScale;

use super::picking_system::{handle_mouse_click, ray_cast_tiles, report_pick_results, PendingPick};

/// 一次瓦片拾取请求事件（携屏幕坐标与可选目标）。
#[derive(Event)]
pub struct TilePickEvent {
    /// 屏幕像素 x 坐标。
    pub screen_x: f32,
    /// 屏幕像素 y 坐标。
    pub screen_y: f32,
    /// 目标瓦片集根实体。
    pub tileset_entity: Entity,
    /// 预选的瓦片路径（从根到瓦片的子索引）。
    pub tile_path: Vec<usize>,
    /// 拾取命中位置（世界坐标，可选）。
    pub position: Option<DVec3>,
}

/// 注册拾取资源/事件并链式挂载拾取三段系统的 Bevy 插件。
pub struct TilePickingPlugin;

impl Plugin for TilePickingPlugin {
    /// 初始化待拾取与渲染缩放资源、注册事件，并以 `.chain()` 固定处理→射线→报告顺序。
    ///
    /// # 参数
    /// - `app`：Bevy 应用
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingPick>()
            .init_resource::<RenderScale>()
            .add_event::<TilePickEvent>()
            .add_systems(
                Update,
                (handle_mouse_click, ray_cast_tiles, report_pick_results).chain(),
            );
    }
}
