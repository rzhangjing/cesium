use std::collections::HashMap;

use bevy::prelude::*;

use crate::components::{CesiumTerrainTile, TileContentState};
use crate::imagery::{ImageryCache, ImageryLayerManager};
use crate::resources::METERS_PER_RENDER_UNIT;

use super::lod_system::TerrainSelection;
use super::tile_loader::TerrainTileReady;

#[derive(Resource, Default)]
pub struct TerrainRenderMap {
    pub render_entities: HashMap<(u32, u32, u32), Entity>,
}

// M4.3：+2 参数（ImageryCache, ImageryLayerManager）用于 imagery 垂辖；
// Bevy 系统参数是位置性的——不加系统参数
// desugaring 就无法打包。按 deferred.md #18 模式进
// 行局部 allow。
#[allow(clippy::too_many_arguments)]
pub fn terrain_render_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut selection: ResMut<TerrainSelection>,
    mut render_map: ResMut<TerrainRenderMap>,
    tile_query: Query<(Entity, &CesiumTerrainTile, &TerrainTileReady)>,
    imagery_cache: Res<ImageryCache>,
    layer_manager: Res<ImageryLayerManager>,
) {
    let render_scale_inv = 1.0 / METERS_PER_RENDER_UNIT;

    // 为已完成解码（`Ready`）且尚未渲染的 tile spawn 渲染 entity。
    // 这 scan 组件状态而非 drain
    // `selection.tiles_to_load`——那个队列归 loader 所有——因此之前
    // 的 drain 竞态（谁先运行谁就清空它）从结构上不可能发生。
    for (entity, tile, ready) in tile_query.iter() {
        if ready.state != TileContentState::Ready {
            continue;
        }

        let key = (tile.x, tile.y, tile.level);
        if render_map.render_entities.contains_key(&key) {
            continue;
        }

        let terrain_mesh = match &ready.terrain_mesh {
            Some(m) => m,
            None => continue,
        };

        // RTC 中心 = 解码 mesh 包围球中心（ECEF，米，f64）。
        // 顶点在 f32 转换前以 f64 重新居中于它（精度
        // 桥），且 entity 以 render 单位被放回 `center`。
        let center = terrain_mesh.bounding_sphere.center;

        let bevy_mesh = crate::terrain_mesh_to_bevy(terrain_mesh, Some(center));
        let mesh_handle = meshes.add(bevy_mesh);

        // M4.3：简单 imagery 垂辖——查找主可见图层针对
        // 该 tile 坐标的贴图。找到时，把它用作
        // base_color_texture，base_color 为 WHITE（未调制的光照）。
        // 缺失时（图层尚未加载 / 未配置 imagery），回退
        // 到中性灰 placeholder（M4.3 之前的行为）。
        // DEVIATION：简单的单层垂辖，不是完整的多层融合；
        //   见 docs/deviations.md#dev-011
        let primary_layer_id = layer_manager.visible_layers().next().map(|l| l.id);
        let material_handle = match primary_layer_id
            .and_then(|lid| imagery_cache.textures.get(&(lid, tile.x, tile.y, tile.level)).cloned())
        {
            Some(texture_handle) => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(texture_handle),
                unlit: false,
                ..default()
            }),
            None => materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.5, 0.5),
                unlit: false,
                ..default()
            }),
        };

        // 把 tile-local mesh 放到其 ECEF 中心，并缩放到 render 单位。
        let transform = Transform::from_translation(
            (center / METERS_PER_RENDER_UNIT).as_vec3(),
        ) * Transform::from_scale(Vec3::splat(render_scale_inv as f32));

        let render_entity = commands
            .spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(material_handle),
                transform,
                Visibility::default(),
            ))
            .id();

        commands.entity(entity).add_child(render_entity);

        render_map.render_entities.insert(key, render_entity);
    }

    for (x, y, level) in selection.tiles_to_unload.drain(..) {
        if let Some(render_entity) = render_map.render_entities.remove(&(x, y, level)) {
            // 递归：把渲染 entity 自己的子节点一并带走。
            commands.entity(render_entity).try_despawn_recursive();
        }

        // 即使容器 entity 从未到达 `Ready` 也要退役它：一个
        // `Loading` placeholder 在 render-map 中还没有条目，所以把
        // despawn 门控在上面那个 `if let Some(render_entity)` 分支会永远泄露它
        // ——一旦它的飞行 task 解析，loader 就会 spawn
        // 一个立即被卸载的 mesh（“spawn-then-destroy”闪烁）。
        // `try_despawn` 在 entity 已消失时是空操作，且与
        // loader 的 `try_insert` 配对以保持无竞态。镜像 tileset
        // render_system。
        for (entity, tile, _) in tile_query.iter() {
            if tile.x == x && tile.y == y && tile.level == level {
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
    fn test_terrain_render_map_insert_remove() {
        let mut map = TerrainRenderMap::default();
        let entity = Entity::from_raw(42);
        map.render_entities.insert((0, 0, 0), entity);
        assert!(map.render_entities.contains_key(&(0, 0, 0)));
        let removed = map.render_entities.remove(&(0, 0, 0));
        assert_eq!(removed, Some(entity));
    }

    #[test]
    fn test_terrain_render_map_multiple_keys() {
        let mut map = TerrainRenderMap::default();
        map.render_entities.insert((0, 0, 0), Entity::from_raw(1));
        map.render_entities.insert((1, 0, 0), Entity::from_raw(2));
        map.render_entities.insert((0, 1, 1), Entity::from_raw(3));
        assert_eq!(map.render_entities.len(), 3);
    }
}
