use bevy::prelude::*;
use bevy::render::camera::Projection;

use crate::camera::components::CesiumCamera;
use crate::METERS_PER_RENDER_UNIT;

/// Per-frame camera update: reads the domain Camera and applies its
/// view/projection state to Bevy's Transform and Projection components.
pub fn camera_update_system(
    mut cameras: Query<(&CesiumCamera, &mut Transform, &mut Projection)>,
) {
    for (cesium_cam, mut transform, mut projection) in cameras.iter_mut() {
        let cam = &cesium_cam.camera;

        let scale = 1.0 / METERS_PER_RENDER_UNIT;
        let position_f32 = cam.position.as_vec3() * scale as f32;

        let forward = -cam.direction.as_vec3();
        let up = cam.up.as_vec3();

        *transform = Transform::from_translation(position_f32)
            .looking_to(forward, up);

        match &cam.frustum {
            cesium_camera::Frustum::Perspective(f) => {
                *projection = Projection::Perspective(PerspectiveProjection {
                    fov: f.fov as f32,
                    aspect_ratio: f.aspect_ratio as f32,
                    near: f.near as f32 * scale as f32,
                    far: f.far as f32 * scale as f32,
                });
            }
            cesium_camera::Frustum::Orthographic(f) => {
                let hw = (f.width * 0.5 * scale) as f32;
                let hh = (f.height() * 0.5 * scale) as f32;
                *projection = Projection::Orthographic(OrthographicProjection {
                    near: f.near as f32 * scale as f32,
                    far: f.far as f32 * scale as f32,
                    scale: 1.0,
                    area: Rect::new(-hw, -hh, hw, hh),
                    ..OrthographicProjection::default_3d()
                });
            }
        }
    }
}
