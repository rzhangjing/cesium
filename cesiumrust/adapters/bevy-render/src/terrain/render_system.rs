use std::collections::HashMap;

use bevy::prelude::*;

use crate::components::CesiumTerrainTile;
use crate::resources::METERS_PER_RENDER_UNIT;

use super::lod_system::TerrainSelection;
use super::tile_loader::TerrainTileReady;

#[derive(Resource, Default)]
pub struct TerrainRenderMap {
    pub render_entities: HashMap<(u32, u32, u32), Entity>,
}

pub fn terrain_render_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut selection: ResMut<TerrainSelection>,
    mut render_map: ResMut<TerrainRenderMap>,
    tile_query: Query<(Entity, &CesiumTerrainTile, &TerrainTileReady)>,
) {
    let render_scale_inv = 1.0 / METERS_PER_RENDER_UNIT;

    for (x, y, level) in selection.tiles_to_load.drain(..) {
        for (entity, tile, ready) in tile_query.iter() {
            if tile.x != x || tile.y != y || tile.level != level {
                continue;
            }

            if render_map.render_entities.contains_key(&(x, y, level)) {
                continue;
            }

            let terrain_mesh = match &ready.terrain_mesh {
                Some(m) => m,
                None => continue,
            };

            let bevy_mesh = crate::terrain_mesh_to_bevy(terrain_mesh);

            let mesh_handle = meshes.add(bevy_mesh);

            let material_handle = materials.add(StandardMaterial {
                base_color: Color::srgb(0.6, 0.7, 0.55),
                unlit: false,
                ..default()
            });

            let render_entity = commands
                .spawn((
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(material_handle),
                    Transform::default(),
                    Visibility::default(),
                ))
                .id();

            commands.entity(entity).add_child(render_entity);

            render_map
                .render_entities
                .insert((x, y, level), render_entity);
        }
    }

    for (x, y, level) in selection.tiles_to_unload.drain(..) {
        if let Some(render_entity) = render_map.render_entities.remove(&(x, y, level)) {
            commands.entity(render_entity).try_despawn_recursive();

            for (entity, tile, _) in tile_query.iter() {
                if tile.x == x && tile.y == y && tile.level == level {
                    commands.entity(entity).try_despawn();
                    break;
                }
            }
        }
    }

    let _ = render_scale_inv;
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
