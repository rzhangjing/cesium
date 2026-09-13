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

#[derive(Resource, Default)]
pub struct PendingPick {
    pub screen_x: f32,
    pub screen_y: f32,
    pub active: bool,
}

pub fn handle_mouse_click(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut pending: ResMut<PendingPick>,
) {
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

pub fn ray_cast_tiles(
    mut pending: ResMut<PendingPick>,
    camera: Query<(&Camera, &GlobalTransform)>,
    render_scale: Res<RenderScale>,
    tiles: Query<(Entity, &CesiumTileNode)>,
    mut pick_events: EventWriter<TilePickEvent>,
) {
    if !pending.active {
        return;
    }
    pending.active = false;

    let Ok((camera, cam_transform)) = camera.get_single() else {
        return;
    };

    let screen_pos = Vec2::new(pending.screen_x, pending.screen_y);
    let Ok(ray_3d) = camera.viewport_to_world(cam_transform, screen_pos) else {
        return;
    };

    let origin = ray_3d.origin.as_dvec3();
    let direction = ray_3d.direction.as_dvec3();

    let scale = render_scale.0;

    let ray_origin_ecf = origin * scale;
    let ray_dir = direction;

    let geospatial_ray = Ray::new(ray_origin_ecf, ray_dir);

    let mut best: Option<(f64, &CesiumTileNode, Entity)> = None;

    for (entity, node) in tiles.iter() {
        let (Some(center), Some(radius)) = (node.bounding_sphere_center, node.bounding_sphere_radius)
        else {
            continue;
        };

        let sphere = BoundingSphere::new(center, radius);
        if let Some((t_min, _t_max)) = ray_sphere(&geospatial_ray, &sphere) {
            if best.map_or(true, |(best_t, _, _)| t_min < best_t && t_min >= 0.0) {
                best = Some((t_min, node, entity));
            }
        }
    }

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
    fn test_ray_sphere_intersection_miss() {
        let sphere = BoundingSphere::new(DVec3::new(0.0, 0.0, 10.0), 1.0);
        let ray = Ray::new(DVec3::ZERO, DVec3::new(0.0, 1.0, 0.0));
        let result = ray_sphere(&ray, &sphere);
        assert!(result.is_none());
    }

    #[test]
    fn test_ray_sphere_intersection_behind() {
        let sphere = BoundingSphere::new(DVec3::new(0.0, 0.0, -10.0), 1.0);
        let ray = Ray::new(DVec3::ZERO, DVec3::new(0.0, 0.0, 1.0));
        let result = ray_sphere(&ray, &sphere);
        assert!(result.is_none());
    }
}
