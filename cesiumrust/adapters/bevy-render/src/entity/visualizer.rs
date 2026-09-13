//! Entity → Bevy Mesh conversion system.
//!
//! Maps domain Entity graphics to Bevy meshes, materials, and transforms.
//! Handles Point (quad), Polyline (extruded line strip), Polygon (fan
//! triangulation), Billboard (camera-facing quad), and Model (glTF loading).

use bevy::prelude::*;
use cesium_datasource::property::{Color, Property};
use cesium_geospatial::cartographic::Cartographic;

use super::components::{
    BillboardGraphicsComponent, BillboardTag, EntityWrapper, GlobeEllipsoid, ModelGraphicsComponent,
    NeedsVisualUpdate, PointGraphicsComponent, PolygonGraphicsComponent,
    PolylineGraphicsComponent, VisualizationBuilt,
};
use crate::entity_render::{
    create_polygon_mesh as render_create_polygon_mesh,
    create_polyline_mesh as render_create_polyline_mesh, domain_color_to_bevy,
    entity_position_to_transform,
};

/// System that converts domain entities to Bevy renderable components.
#[allow(clippy::too_many_arguments)]
pub fn entity_visualizer_system(
    mut commands: Commands,
    domain_entities: Query<(Entity, &EntityWrapper, Option<&NeedsVisualUpdate>)>,
    ellipsoid: Res<GlobeEllipsoid>,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    #[cfg(not(test))] asset_server: Res<AssetServer>,
) {
    let current_time = time.elapsed_secs_f64();

    for (bevy_entity, entity_wrapper, needs_update) in domain_entities.iter() {
        if needs_update.is_none() {
            continue;
        }

        let domain_entity = &entity_wrapper.0;
        let mut entity_cmd = commands.entity(bevy_entity);

        entity_cmd.remove::<NeedsVisualUpdate>();

        if let Some(ref point) = domain_entity.point {
            let color = resolve_color(&point.color, Color::WHITE);
            let outline_color = resolve_color(&point.outline_color, Color::BLACK);
            let pixel_size = point.pixel_size.get_value(current_time).copied().unwrap_or(1.0);
            let outline_width = point.outline_width.get_value(current_time).copied().unwrap_or(0.0);

            entity_cmd.insert(PointGraphicsComponent {
                pixel_size: pixel_size as f32,
                color: [
                    color.red as f32,
                    color.green as f32,
                    color.blue as f32,
                    color.alpha as f32,
                ],
                outline_color: [
                    outline_color.red as f32,
                    outline_color.green as f32,
                    outline_color.blue as f32,
                    outline_color.alpha as f32,
                ],
                outline_width: outline_width as f32,
            });

            let mesh = create_point_quad_mesh(pixel_size as f32);
            let mesh_handle = meshes.add(mesh);
            let transform = entity_position_to_transform(domain_entity, &ellipsoid.0, current_time)
                .unwrap_or_default();
            let color_bevy = domain_color_to_bevy(&Color::new(
                color.red, color.green, color.blue, color.alpha,
            ));
            let mat_handle = materials.add(StandardMaterial {
                base_color: color_bevy,
                unlit: true,
                ..default()
            });

            entity_cmd.with_children(|parent| {
                parent.spawn((
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(mat_handle),
                    transform,
                    VisualizationBuilt,
                ));
            });
        }

        if let Some(ref polyline) = domain_entity.polyline {
            let width =
                polyline.width.get_value(current_time).copied().unwrap_or(1.0) as f32;
            let color = resolve_color(&polyline.color, Color::WHITE);
            let clamp_to_ground =
                polyline.clamp_to_ground.get_value(current_time).copied().unwrap_or(false);

            let positions: Vec<glam::DVec3> = match polyline.positions.get_value(current_time) {
                Some(pts) => pts
                    .iter()
                    .map(|p| {
                        let carto = Cartographic::from_radians(p[0], p[1], p[2]);
                        ellipsoid.0.cartographic_to_cartesian(&carto)
                    })
                    .collect(),
                None => Vec::new(),
            };

            entity_cmd.insert(PolylineGraphicsComponent {
                width,
                material_color: [
                    color.red as f32,
                    color.green as f32,
                    color.blue as f32,
                    color.alpha as f32,
                ],
                clamp_to_ground,
                positions: positions.clone(),
            });

            if let Some(mesh) =
                render_create_polyline_mesh(polyline, &ellipsoid.0, current_time)
            {
                let mesh_handle = meshes.add(mesh);
                let color_bevy = domain_color_to_bevy(&Color::new(
                    color.red, color.green, color.blue, color.alpha,
                ));
                let mat_handle = materials.add(StandardMaterial {
                    base_color: color_bevy,
                    ..default()
                });
                entity_cmd.with_children(|parent| {
                    parent.spawn((
                        Mesh3d(mesh_handle),
                        MeshMaterial3d(mat_handle),
                        VisualizationBuilt,
                    ));
                });
            }
        }

        if let Some(ref polygon) = domain_entity.polygon {
            let height =
                polygon.height.get_value(current_time).copied().unwrap_or(0.0);
            let extruded_height =
                polygon.extruded_height.get_value(current_time).copied();
            let color = resolve_color(&polygon.material, Color::WHITE);
            let outline = polygon.outline.get_value(current_time).copied().unwrap_or(false);
            let outline_color = resolve_color(&polygon.outline_color, Color::BLACK);

            let positions: Vec<glam::DVec3> = match polygon.positions.get_value(current_time) {
                Some(pts) => pts
                    .iter()
                    .map(|p| {
                        let carto = Cartographic::from_radians(
                            p[0], p[1], p[2] + height,
                        );
                        ellipsoid.0.cartographic_to_cartesian(&carto)
                    })
                    .collect(),
                None => Vec::new(),
            };

            entity_cmd.insert(PolygonGraphicsComponent {
                positions: positions.clone(),
                holes: Vec::new(),
                height,
                extruded_height,
                material_color: [
                    color.red as f32,
                    color.green as f32,
                    color.blue as f32,
                    color.alpha as f32,
                ],
                outline,
                outline_color: [
                    outline_color.red as f32,
                    outline_color.green as f32,
                    outline_color.blue as f32,
                    outline_color.alpha as f32,
                ],
            });

            if let Some(mesh) =
                render_create_polygon_mesh(polygon, &ellipsoid.0, current_time)
            {
                let mesh_handle = meshes.add(mesh);
                let color_bevy = domain_color_to_bevy(&Color::new(
                    color.red, color.green, color.blue, color.alpha,
                ));
                let mat_handle = materials.add(StandardMaterial {
                    base_color: color_bevy,
                    ..default()
                });
                entity_cmd.with_children(|parent| {
                    parent.spawn((
                        Mesh3d(mesh_handle),
                        MeshMaterial3d(mat_handle),
                        VisualizationBuilt,
                    ));
                });
            }
        }

        if let Some(ref billboard) = domain_entity.billboard {
            let image_url = billboard.image.get_value(current_time).cloned();
            let scale =
                billboard.scale.get_value(current_time).copied().unwrap_or(1.0) as f32;
            let color = resolve_color(&billboard.color, Color::WHITE);

            entity_cmd.insert(BillboardGraphicsComponent {
                image_url: image_url.clone(),
                scale,
                color: [
                    color.red as f32,
                    color.green as f32,
                    color.blue as f32,
                    color.alpha as f32,
                ],
            });

            #[cfg(not(test))]
            if let Some(ref url) = image_url {
                let mesh = create_billboard_quad_mesh();
                let mesh_handle = meshes.add(mesh);
                let texture: Handle<Image> = asset_server.load(url);
                let mat_handle = materials.add(StandardMaterial {
                    base_color_texture: Some(texture),
                    base_color: domain_color_to_bevy(&Color::new(
                        color.red, color.green, color.blue, color.alpha,
                    )),
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                });
                entity_cmd.with_children(|parent| {
                    parent.spawn((
                        Mesh3d(mesh_handle),
                        MeshMaterial3d(mat_handle),
                        BillboardTag,
                        VisualizationBuilt,
                    ));
                });
            }
            #[cfg(not(test))]
            {
            }

            if image_url.is_none() {
                let mesh = create_billboard_quad_mesh();
                let mesh_handle = meshes.add(mesh);
                let color_bevy = domain_color_to_bevy(&Color::new(
                    color.red, color.green, color.blue, color.alpha,
                ));
                let mat_handle = materials.add(StandardMaterial {
                    base_color: color_bevy,
                    unlit: true,
                    ..default()
                });
                entity_cmd.with_children(|parent| {
                    parent.spawn((
                        Mesh3d(mesh_handle),
                        MeshMaterial3d(mat_handle),
                        BillboardTag,
                        VisualizationBuilt,
                    ));
                });
            }
        }

        if let Some(ref model) = domain_entity.model {
            let uri = model.uri.get_value(current_time).cloned().unwrap_or_default();
            let scale =
                model.scale.get_value(current_time).copied().unwrap_or(1.0) as f32;
            let min_pixel_size = model
                .minimum_pixel_size
                .get_value(current_time)
                .copied()
                .unwrap_or(0.0) as f32;

            entity_cmd.insert(ModelGraphicsComponent {
                uri: uri.clone(),
                scale,
                minimum_pixel_size: min_pixel_size,
            });

            #[cfg(not(test))]
            {
                let scene_path = format!("{}#Scene0", uri);
                let scene_handle: Handle<Scene> = asset_server.load(&scene_path);
                entity_cmd.with_children(|parent| {
                    parent.spawn((
                        SceneRoot(scene_handle),
                        Transform::from_scale(Vec3::splat(scale)),
                        VisualizationBuilt,
                    ));
                });
            }
        }
    }
}

/// Updates billboard transforms to face the camera each frame.
pub fn billboard_face_camera_system(
    camera_query: Query<&Transform, (With<Camera>, Without<BillboardTag>)>,
    mut billboard_query: Query<&mut Transform, With<BillboardTag>>,
) {
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };

    for mut billboard_tf in billboard_query.iter_mut() {
        let direction = camera_transform.translation - billboard_tf.translation;
        if direction.length_squared() > f32::EPSILON {
            billboard_tf.look_to(-direction, camera_transform.up().as_vec3());
        }
    }
}

/// Creates a unit quad mesh for point rendering.
fn create_point_quad_mesh(pixel_size: f32) -> Mesh {
    let half = pixel_size * 0.5;
    let vertices = vec![
        [-half, -half, 0.0f32],
        [half, -half, 0.0],
        [half, half, 0.0],
        [-half, half, 0.0],
    ];
    let normals = vec![[0.0f32, 0.0, 1.0]; 4];
    let uvs = vec![[0.0f32, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let indices = vec![0u32, 1, 2, 0, 2, 3];

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    mesh
}

/// Creates a unit quad mesh for billboard rendering.
fn create_billboard_quad_mesh() -> Mesh {
    let half = 50_000.0f32;
    let vertices = vec![
        [-half, -half, 0.0f32],
        [half, -half, 0.0],
        [half, half, 0.0],
        [-half, half, 0.0],
    ];
    let normals = vec![[0.0f32, 0.0, 1.0]; 4];
    let uvs = vec![[0.0f32, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let indices = vec![0u32, 1, 2, 0, 2, 3];

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    mesh
}

/// Resolves a color property at the given time.
fn resolve_color(prop: &Property<Color>, default: Color) -> Color {
    prop.get_value(0.0).copied().unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_point_quad_mesh() {
        let mesh = create_point_quad_mesh(10.0);
        assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_UV_0).is_some());
    }

    #[test]
    fn test_create_billboard_quad_mesh() {
        let mesh = create_billboard_quad_mesh();
        assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
    }
}
