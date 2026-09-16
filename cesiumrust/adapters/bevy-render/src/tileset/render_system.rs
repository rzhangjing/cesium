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

    // Spawn render entities for tiles whose content finished decoding (`Ready`
    // *and* carrying a mesh handle) and that are not rendered yet.
    //
    // This scans component state instead of draining `selection.tiles_to_load`:
    // the content loader is the sole consumer of that queue, so the previous
    // double drain (whoever ran first emptied it, and the render side could only
    // ever spawn an empty placeholder mesh) is structurally impossible.
    for (entity, node, content) in tile_query.iter() {
        if !matches!(node.state, TileContentState::Ready) {
            continue;
        }

        let Some(content) = content else { continue };
        // No mesh handle -> nothing to draw; never spawn an empty entity.
        let Some(mesh_handle) = content.mesh_handle.as_ref() else {
            continue;
        };

        if render_map.render_entities.contains_key(&node.path) {
            continue;
        }

        // Same RTC center the loader handed to `geometry_to_mesh`: the vertices
        // were recentered on it in f64 before the f32 cast, so the entity is
        // placed back at `center` expressed in render units (1 unit = earth
        // radius metres) and scaled to match.
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
            // Recursive: takes the render entity's own children with it.
            commands.entity(render_entity).try_despawn_recursive();
        }

        // Retire the tile node even when it never reached `Ready`: a `Loading`
        // placeholder has no render-map entry yet, and leaving it behind would
        // make the loader treat that path as already known forever. `try_*` is a
        // no-op if the in-flight task resolves after the despawn.
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
        // The render transform must undo the RTC recentering in render units and
        // shrink metre-space geometry by the shared scale factor.
        let center = glam::DVec3::new(1_000_000.0, -2_000_000.0, 3_000_000.0);
        let translation = (center / METERS_PER_RENDER_UNIT).as_vec3();
        let transform = Transform::from_translation(translation)
            * Transform::from_scale(Vec3::splat((1.0 / METERS_PER_RENDER_UNIT) as f32));

        assert!((transform.translation.x - (1_000_000.0 / METERS_PER_RENDER_UNIT) as f32).abs() < 1e-6);
        assert!((transform.scale.x - (1.0 / METERS_PER_RENDER_UNIT) as f32).abs() < 1e-9);
        // Scale must be uniform, otherwise tile-local geometry is sheared.
        assert_eq!(transform.scale.x, transform.scale.y);
        assert_eq!(transform.scale.y, transform.scale.z);
    }
}
