use bevy::prelude::*;
use glam::DVec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoxelPrimitiveType {
    Box,
    Cylinder,
    Ellipsoid,
}

#[derive(Component, Debug, Clone)]
pub struct VoxelPrimitiveComponent {
    pub shape_type: VoxelPrimitiveType,
    pub grid_resolution: u32,
    pub bounds_min: DVec3,
    pub bounds_max: DVec3,
    pub model_matrix: glam::DMat4,
    pub color_base: [f32; 4],
    pub visible: bool,
}

impl Default for VoxelPrimitiveComponent {
    fn default() -> Self {
        Self {
            shape_type: VoxelPrimitiveType::Box,
            grid_resolution: 16,
            bounds_min: DVec3::splat(-1.0),
            bounds_max: DVec3::splat(1.0),
            model_matrix: glam::DMat4::IDENTITY,
            color_base: [0.5, 0.5, 0.8, 1.0],
            visible: true,
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct VoxelConfig {
    pub grid_resolution: u32,
    pub shape_type: VoxelPrimitiveType,
    pub bounds_min: DVec3,
    pub bounds_max: DVec3,
    pub enabled: bool,
}

impl Default for VoxelConfig {
    fn default() -> Self {
        Self {
            grid_resolution: 16,
            shape_type: VoxelPrimitiveType::Box,
            bounds_min: DVec3::splat(-1.0),
            bounds_max: DVec3::splat(1.0),
            enabled: true,
        }
    }
}

pub struct CesiumVoxelPlugin;

impl Plugin for CesiumVoxelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VoxelConfig>()
            .add_systems(Update, voxel_render_system);
    }
}

pub fn voxel_render_system(
    config: Res<VoxelConfig>,
    mut commands: Commands,
    voxel_query: Query<(Entity, &VoxelPrimitiveComponent), Changed<VoxelPrimitiveComponent>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !config.enabled {
        return;
    }

    for (entity, comp) in voxel_query.iter() {
        if !comp.visible {
            commands.entity(entity).remove::<Mesh3d>();
            continue;
        }

        let mesh = generate_voxel_mesh(
            comp.shape_type,
            comp.grid_resolution,
            comp.bounds_min,
            comp.bounds_max,
            comp.model_matrix,
            comp.color_base,
        );

        let mesh_handle = meshes.add(mesh);
        let mat_handle = materials.add(StandardMaterial {
            base_color: Color::linear_rgb(
                comp.color_base[0],
                comp.color_base[1],
                comp.color_base[2],
            ),
            alpha_mode: AlphaMode::Opaque,
            ..default()
        });

        commands
            .entity(entity)
            .insert(Mesh3d(mesh_handle))
            .insert(MeshMaterial3d(mat_handle));
    }
}

fn generate_voxel_mesh(
    shape_type: VoxelPrimitiveType,
    resolution: u32,
    bounds_min: DVec3,
    bounds_max: DVec3,
    model_matrix: glam::DMat4,
    color_base: [f32; 4],
) -> Mesh {
    let res = resolution.max(2);
    let step = (bounds_max - bounds_min) / res as f64;

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let center = (bounds_min + bounds_max) * 0.5;
    let extents = (bounds_max - bounds_min) * 0.5;

    for ix in 0..res {
        for iy in 0..res {
            for iz in 0..res {
                let local_center = DVec3::new(
                    bounds_min.x + (ix as f64 + 0.5) * step.x,
                    bounds_min.y + (iy as f64 + 0.5) * step.y,
                    bounds_min.z + (iz as f64 + 0.5) * step.z,
                );

                let inside = match shape_type {
                    VoxelPrimitiveType::Box => true,
                    VoxelPrimitiveType::Cylinder => {
                        let r = (local_center.x * local_center.x
                            + local_center.y * local_center.y)
                            .sqrt();
                        r <= 1.0
                    }
                    VoxelPrimitiveType::Ellipsoid => {
                        let nx = local_center.x / extents.x;
                        let ny = local_center.y / extents.y;
                        let nz = local_center.z / extents.z;
                        nx * nx + ny * ny + nz * nz <= 1.0
                    }
                };

                if !inside {
                    continue;
                }

                let world_center = model_matrix.transform_point3(local_center);

                let dist_from_center = local_center.distance(center);
                let max_dist = extents.length();
                let t = (dist_from_center / max_dist).clamp(0.0, 1.0) as f32;

                let cell_color = [
                    color_base[0] * (1.0 - t * 0.5),
                    color_base[1] * (1.0 - t * 0.5),
                    color_base[2] * (1.0 - t * 0.5),
                    color_base[3],
                ];

                let half = step * 0.5;
                let scale = DVec3::new(
                    model_matrix.col(0).truncate().length(),
                    model_matrix.col(1).truncate().length(),
                    model_matrix.col(2).truncate().length(),
                );
                let half_world = DVec3::new(
                    half.x * scale.x,
                    half.y * scale.y,
                    half.z * scale.z,
                );

                add_cube_mesh(
                    world_center.as_vec3(),
                    [half_world.x as f32, half_world.y as f32, half_world.z as f32],
                    cell_color,
                    &mut positions,
                    &mut normals,
                    &mut colors,
                    &mut indices,
                );
            }
        }
    }

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    if !indices.is_empty() {
        mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    }
    if !colors.is_empty() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }
    mesh
}

fn add_cube_mesh(
    center: Vec3,
    half: [f32; 3],
    color: [f32; 4],
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
) {
    let hx = half[0];
    let hy = half[1];
    let hz = half[2];

    let corners: [[f32; 3]; 8] = [
        [center.x - hx, center.y - hy, center.z - hz],
        [center.x + hx, center.y - hy, center.z - hz],
        [center.x + hx, center.y + hy, center.z - hz],
        [center.x - hx, center.y + hy, center.z - hz],
        [center.x - hx, center.y - hy, center.z + hz],
        [center.x + hx, center.y - hy, center.z + hz],
        [center.x + hx, center.y + hy, center.z + hz],
        [center.x - hx, center.y + hy, center.z + hz],
    ];

    let base = positions.len() as u32;

    let face_normals: [[f32; 3]; 6] = [
        [0.0, 0.0, -1.0],
        [0.0, 0.0, 1.0],
        [0.0, -1.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [-1.0, 0.0, 0.0],
    ];

    let face_quads: [[u32; 4]; 6] = [
        [0, 1, 2, 3], // front
        [5, 4, 7, 6], // back
        [4, 5, 1, 0], // bottom
        [1, 5, 6, 2], // right
        [3, 2, 6, 7], // top
        [4, 0, 3, 7], // left
    ];

    for (fi, &[a, b, c, d]) in face_quads.iter().enumerate() {
        let n = face_normals[fi];
        positions.push(corners[a as usize]);
        positions.push(corners[b as usize]);
        positions.push(corners[c as usize]);
        positions.push(corners[d as usize]);
        normals.push(n);
        normals.push(n);
        normals.push(n);
        normals.push(n);
        colors.push(color);
        colors.push(color);
        colors.push(color);
        colors.push(color);
        let i0 = base + fi as u32 * 4;
        indices.push(i0);
        indices.push(i0 + 1);
        indices.push(i0 + 2);
        indices.push(i0);
        indices.push(i0 + 2);
        indices.push(i0 + 3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voxel_config_default() {
        let config = VoxelConfig::default();
        assert!(config.enabled);
        assert_eq!(config.grid_resolution, 16);
        assert_eq!(config.shape_type, VoxelPrimitiveType::Box);
    }

    #[test]
    fn test_generate_voxel_mesh_box() {
        let mesh = generate_voxel_mesh(
            VoxelPrimitiveType::Box,
            4,
            DVec3::splat(-1.0),
            DVec3::splat(1.0),
            glam::DMat4::IDENTITY,
            [0.5, 0.5, 0.8, 1.0],
        );
        assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
    }

    #[test]
    fn test_generate_voxel_mesh_cylinder() {
        let mesh = generate_voxel_mesh(
            VoxelPrimitiveType::Cylinder,
            4,
            DVec3::new(-1.0, -1.0, -1.0),
            DVec3::new(1.0, 1.0, 1.0),
            glam::DMat4::IDENTITY,
            [0.8, 0.2, 0.2, 1.0],
        );
        assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
    }

    #[test]
    fn test_generate_voxel_mesh_ellipsoid() {
        let mesh = generate_voxel_mesh(
            VoxelPrimitiveType::Ellipsoid,
            4,
            DVec3::splat(-1.0),
            DVec3::splat(1.0),
            glam::DMat4::IDENTITY,
            [0.2, 0.8, 0.2, 1.0],
        );
        assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
    }
}
