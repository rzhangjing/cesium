use std::collections::HashMap;

use bevy::prelude::*;

use crate::components::{CesiumTileNode, TileContent, TileContentState};

use super::traversal_system::TileSelection;

#[derive(Resource, Default)]
pub struct TileRenderMap {
    pub render_entities: HashMap<Vec<usize>, Entity>,
}

pub fn tile_render_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut selection: ResMut<TileSelection>,
    mut render_map: ResMut<TileRenderMap>,
    tile_query: Query<(Entity, &CesiumTileNode, Option<&TileContent>)>,
    children_query: Query<&Children>,
) {
    for path in selection.tiles_to_load.drain(..) {
        for (entity, node, _content) in tile_query.iter() {
            if node.path != path {
                continue;
            }

            if !matches!(node.state, TileContentState::Ready) {
                continue;
            }

            if render_map.render_entities.contains_key(&path) {
                continue;
            }

            let mesh_handle = meshes.add(Mesh::new(
                bevy::render::mesh::PrimitiveTopology::TriangleList,
                bevy::render::render_asset::RenderAssetUsages::default(),
            ));

            let material_handle = materials.add(StandardMaterial {
                base_color: Color::srgb(0.8, 0.8, 0.8),
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
                .insert(path.clone(), render_entity);
        }
    }

    for path in selection.tiles_to_unload.drain(..) {
        if let Some(render_entity) = render_map.render_entities.remove(&path) {
            if let Ok(children) = children_query.get(render_entity) {
                for &child in children.iter() {
                    commands.entity(child).try_despawn();
                }
            }

            for (entity, node, _) in tile_query.iter() {
                if node.path == path {
                    commands.entity(entity).try_despawn();
                    break;
                }
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
}
