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

// M4.3: +2 params (ImageryCache, ImageryLayerManager) for imagery draping;
// Bevy system params are positional — cannot bundle without losing system param
// desugaring. Localized allow per deferred.md #18 pattern.
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

    // Spawn render entities for tiles that finished decoding (`Ready`) and are
    // not yet rendered. This scans component state instead of draining
    // `selection.tiles_to_load` — the loader owns that queue — so the previous
    // drain race (whoever ran first emptied it) is structurally impossible.
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

        // RTC center = decoded mesh bounding-sphere center (ECEF, metres, f64).
        // Vertices are recentered on it in f64 before the f32 cast (precision
        // bridge), and the entity is placed back at `center` in render units.
        let center = terrain_mesh.bounding_sphere.center;

        let bevy_mesh = crate::terrain_mesh_to_bevy(terrain_mesh, Some(center));
        let mesh_handle = meshes.add(bevy_mesh);

        // M4.3: simple imagery draping — look up the primary visible layer's
        // texture for this tile coordinate. When found, use it as
        // base_color_texture with WHITE base_color (unmodulated albedo).
        // When absent (layer not yet loaded / no imagery configured), fall
        // back to the neutral gray placeholder (pre-M4.3 behaviour).
        // DEVIATION: simple single-layer draping, not full multi-layer blend;
        //   see docs/deviations.md#dev-011
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

        // Place the tile-local mesh at its ECEF center, scaled into render units.
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
            // Recursive: takes the render entity's own children with it.
            commands.entity(render_entity).try_despawn_recursive();
        }

        // Retire the container entity even when it never reached `Ready`: a
        // `Loading` placeholder has no render-map entry yet, so gating the
        // despawn on the `if let Some(render_entity)` branch above would leak it
        // forever — and once its in-flight task resolves, the loader would spawn
        // a mesh that is immediately unloaded ("spawn-then-destroy" flicker).
        // `try_despawn` is a no-op if the entity is already gone, and pairs with
        // the loader's `try_insert` to stay race-free. Mirrors tileset
        // render_system.
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
