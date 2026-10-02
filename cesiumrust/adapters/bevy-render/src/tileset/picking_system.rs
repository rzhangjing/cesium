// legacy CesiumJS-port style debt (deferred.md #18); revisit at M13 lint-cleanup 或本文件在其里程碑被重写时
//! 3D Tiles 拾取系统：把鼠标点击转为射线，与瓦片包围球求交。
//!
//! [`handle_mouse_click`] 捕获左键并记录屏幕坐标；[`ray_cast_tiles`] 把
//! 视口射线变到 ECEF 米空间，对所有瓦片包围球求最近命中并发出
//! [`TilePickEvent`]；[`report_pick_results`] 负责日志回显。
#![allow(clippy::unnecessary_map_or)]
use bevy::prelude::*;
use bevy::input::ButtonInput;
use bevy::window::PrimaryWindow;
use cesium_geospatial::ray::{ray_sphere, Ray};
use cesium_geospatial::bounding::BoundingSphere;
use glam::Vec2;
#[cfg(test)]
use glam::DVec3;

use crate::components::CesiumTileNode;
use crate::resources::RenderScale;
use super::picking::TilePickEvent;

/// 待处理的拾取请求（缓存本次点击的屏幕坐标）。
#[derive(Resource, Default)]
pub struct PendingPick {
    /// 点击处屏幕 X 坐标。
    pub screen_x: f32,
    /// 点击处屏幕 Y 坐标。
    pub screen_y: f32,
    /// 是否有待处理的拾取。
    pub active: bool,
}

/// 点击捕获系统：左键按下时记录光标位置并置位待处理标志。
///
/// # 参数
/// - `mouse`：鼠标按键输入
/// - `window`：主窗口（提供光标位置）
/// - `pending`：待处理拾取（可写）
pub fn handle_mouse_click(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut pending: ResMut<PendingPick>,
) {
    // 仅响应左键刚按下，且需有光标位置。
    if mouse.just_pressed(MouseButton::Left) {
        if let Ok(win) = window.get_single() {
            if let Some(pos) = win.cursor_position() {
                pending.screen_x = pos.x;
                pending.screen_y = pos.y;
                pending.active = true;
            }
        }
    }
}

/// 射线拾取系统：对当前待处理点击做视域射线与瓦片求交，取最近命中。
///
/// # 参数
/// - `pending`：待处理拾取（处理后清位）
/// - `camera`：单相机与全局变换
/// - `render_scale`：渲染缩放（渲染→米）
/// - `tiles`：瓦片节点查询
/// - `pick_events`：拾取事件写出器
pub fn ray_cast_tiles(
    mut pending: ResMut<PendingPick>,
    camera: Query<(&Camera, &GlobalTransform)>,
    render_scale: Res<RenderScale>,
    tiles: Query<(Entity, &CesiumTileNode)>,
    mut pick_events: EventWriter<TilePickEvent>,
) {
    // 无待处理拾取时直接返回。
    if !pending.active {
        return;
    }
    // 消费本次请求，避免重复射线投射。
    pending.active = false;

    let Ok((camera, cam_transform)) = camera.get_single() else {
        return;
    };

    // 由屏幕坐标反投影为世界射线（渲染单位空间）。
    let screen_pos = Vec2::new(pending.screen_x, pending.screen_y);
    let Ok(ray_3d) = camera.viewport_to_world(cam_transform, screen_pos) else {
        return;
    };

    let origin = ray_3d.origin.as_dvec3();
    let direction = ray_3d.direction.as_dvec3();

    let scale = render_scale.0;

    // 把射线起点乘以缩放转回 ECEF 米（方向无需缩放）。
    let ray_origin_ecf = origin * scale;
    let ray_dir = direction;

    let geospatial_ray = Ray::new(ray_origin_ecf, ray_dir);

    // 保留最近（t 最小且非负）的命中瓦片。
    let mut best: Option<(f64, &CesiumTileNode, Entity)> = None;

    for (entity, node) in tiles.iter() {
        // 无包围球信息的瓦片跳过。
        let (Some(center), Some(radius)) = (node.bounding_sphere_center, node.bounding_sphere_radius)
        else {
            continue;
        };

        // 射线与包围球求交，命中则比较取最近。
        let sphere = BoundingSphere::new(center, radius);
        if let Some((t_min, _t_max)) = ray_sphere(&geospatial_ray, &sphere) {
            if best.map_or(true, |(best_t, _, _)| t_min < best_t && t_min >= 0.0) {
                best = Some((t_min, node, entity));
            }
        }
    }

    // 命中时计算 ECEF 交点并发出拾取事件。
    if let Some((t, node, entity)) = best {
        let hit_pos = if t >= 0.0 {
            Some(geospatial_ray.point_at(t))
        } else {
            None
        };
        pick_events.send(TilePickEvent {
            screen_x: pending.screen_x,
            screen_y: pending.screen_y,
            tileset_entity: entity,
            tile_path: node.path.clone(),
            position: hit_pos,
        });
    }
}

/// 结果上报系统：消费 [`TilePickEvent`] 并回显拾取信息。
///
/// # 参数
/// - `events`：拾取事件读取器
pub fn report_pick_results(mut events: EventReader<TilePickEvent>) {
    for event in events.read() {
        info!(
            "Tile picked at screen ({:.1}, {:.1}): path {:?}, ECEF {:?}",
            event.screen_x, event.screen_y, event.tile_path, event.position
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_geospatial::ray::Ray;

    #[test]
    /// 验证射线正对球时命中且 t_min 等于球心距减半径。
    fn test_ray_sphere_intersection_hit() {
        let sphere = BoundingSphere::new(DVec3::new(0.0, 0.0, 10.0), 1.0);
        let ray = Ray::new(DVec3::ZERO, DVec3::new(0.0, 0.0, 1.0));
        let result = ray_sphere(&ray, &sphere);
        assert!(result.is_some());
        let (t_min, t_max) = result.unwrap();
        assert!(t_min > 0.0);
        assert!(t_max > t_min);
        assert!((t_min - 9.0).abs() < 1e-6);
    }

    #[test]
    /// 验证射线与球错开时无交点。
    fn test_ray_sphere_intersection_miss() {
        let sphere = BoundingSphere::new(DVec3::new(0.0, 0.0, 10.0), 1.0);
        let ray = Ray::new(DVec3::ZERO, DVec3::new(0.0, 1.0, 0.0));
        let result = ray_sphere(&ray, &sphere);
        assert!(result.is_none());
    }

    #[test]
    /// 验证球在射线后方时不算命中。
    fn test_ray_sphere_intersection_behind() {
        let sphere = BoundingSphere::new(DVec3::new(0.0, 0.0, -10.0), 1.0);
        let ray = Ray::new(DVec3::ZERO, DVec3::new(0.0, 0.0, 1.0));
        let result = ray_sphere(&ray, &sphere);
        assert!(result.is_none());
    }
}
