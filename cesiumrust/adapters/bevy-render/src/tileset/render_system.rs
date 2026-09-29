use std::collections::HashMap;

use bevy::prelude::*;

use crate::components::{CesiumTileNode, TileContent, TileContentState};
use crate::resources::METERS_PER_RENDER_UNIT;

use super::traversal_system::TileSelection;

#[derive(Resource, Default)]
pub struct TileRenderMap {
    pub render_entities: HashMap<Vec<usize>, Entity>,
}

pub fn tile_render_system(
    mut commands: Commands,
    mut selection: ResMut<TileSelection>,
    mut render_map: ResMut<TileRenderMap>,
    tile_query: Query<(Entity, &CesiumTileNode, Option<&TileContent>)>,
) {
    let render_scale_inv = 1.0 / METERS_PER_RENDER_UNIT;

    // 为那些 content 已完成解码（`Ready` *且*携带一个 mesh handle）
    // 且尚未被渲染的 tile spawn 渲染 entity。
    //
    // 这扫描组件状态而非 drain `selection.tiles_to_load`：
    // content loader 是那个队列的唯一消费者，所以此前的
    // 双重 drain（谁先跑谁就把它清空，而渲染侧永远只能
    // spawn 一个空占位 mesh）在结构上已不可能。
    for (entity, node, content) in tile_query.iter() {
        if !matches!(node.state, TileContentState::Ready) {
            continue;
        }

        let Some(content) = content else { continue };
        // 无 mesh handle -> 无可绘之物；绝不 spawn 一个空 entity。
        let Some(mesh_handle) = content.mesh_handle.as_ref() else {
            continue;
        };

        if render_map.render_entities.contains_key(&node.path) {
            continue;
        }

        // 与 loader 交给 `geometry_to_mesh` 的同一 RTC 中心：顶点在 f32 转换
        // 之前已在 f64 中以它重新居中，所以该 entity 被放回以 render unit
        // 表示的 `center`（1 单位 = 地球半径米）并相应缩放。
        let center_render = node
            .bounding_sphere_center
            .map(|center| (center / METERS_PER_RENDER_UNIT).as_vec3())
            .unwrap_or(Vec3::ZERO);

        let transform = Transform::from_translation(center_render)
            * Transform::from_scale(Vec3::splat(render_scale_inv as f32));

        let mut entity_commands = commands.spawn((
            Mesh3d(mesh_handle.clone()),
            transform,
            Visibility::default(),
        ));

        if let Some(material_handle) = content.material_handle.clone() {
            entity_commands.insert(MeshMaterial3d(material_handle));
        }

        let render_entity = entity_commands.id();

        commands.entity(entity).add_child(render_entity);

        render_map
            .render_entities
            .insert(node.path.clone(), render_entity);
    }

    for path in selection.tiles_to_unload.drain(..) {
        if let Some(render_entity) = render_map.render_entities.remove(&path) {
            // 递归：把渲染 entity 自身的子节点一并带走。
            commands.entity(render_entity).try_despawn_recursive();
        }

        // 即使那个 tile 从未达到 `Ready` 也退役它：一个 `Loading`
        // 占位尚没有 render-map 条目，而留着它会使 loader 永远把那个
        // 路径当作已知。若飞行中的 task 在 despawn 之后才解析，`try_*` 就是一个
        // 空操作。
        for (entity, node, _) in tile_query.iter() {
            if node.path == path {
                commands.entity(entity).try_despawn();
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_map_insert_and_remove() {
        let mut map = TileRenderMap::default();
        let entity = Entity::from_raw(42);
        map.render_entities.insert(vec![0, 1], entity);
        assert!(map.render_entities.contains_key(&vec![0, 1]));
        let removed = map.render_entities.remove(&vec![0, 1]);
        assert_eq!(removed, Some(entity));
        assert!(map.render_entities.is_empty());
    }

    #[test]
    fn test_render_map_multiple_paths() {
        let mut map = TileRenderMap::default();
        map.render_entities.insert(vec![0], Entity::from_raw(1));
        map.render_entities.insert(vec![1], Entity::from_raw(2));
        assert_eq!(map.render_entities.len(), 2);
        assert!(map.render_entities.contains_key(&vec![0]));
        assert!(map.render_entities.contains_key(&vec![1]));
    }

    #[test]
    fn test_rtc_transform_matches_render_scale() {
        // 渲染变换必须以 render unit 撤销 RTC 重新居中，并
        // 按共享的缩放因子将米空间几何缩小。
        let center = glam::DVec3::new(1_000_000.0, -2_000_000.0, 3_000_000.0);
        let translation = (center / METERS_PER_RENDER_UNIT).as_vec3();
        let transform = Transform::from_translation(translation)
            * Transform::from_scale(Vec3::splat((1.0 / METERS_PER_RENDER_UNIT) as f32));

        assert!((transform.translation.x - (1_000_000.0 / METERS_PER_RENDER_UNIT) as f32).abs() < 1e-6);
        assert!((transform.scale.x - (1.0 / METERS_PER_RENDER_UNIT) as f32).abs() < 1e-9);
        // 缩放必须是均匀的，否则 tile 局部几何会被错切。
        assert_eq!(transform.scale.x, transform.scale.y);
        assert_eq!(transform.scale.y, transform.scale.z);
    }
}
