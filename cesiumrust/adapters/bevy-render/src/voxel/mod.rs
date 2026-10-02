//! 体元（voxel）基本体渲染：把一个三维网格按形状类型（盒/柱/
//! 椭球）采样，仅对位于体内的格子发射小立方体，拼接为
//! 可渲染 mesh。适用于 Cesium 的 VoxelPrimitive 风格体积可视化。
//!
//! 主入口 [`voxel_render_system`] 监听 [`VoxelPrimitiveComponent`] 的变化，
//! 调用 [`generate_voxel_mesh`] 重建几何，几何本身由 [`add_cube_mesh`]
//! 逐格累加。

use bevy::prelude::*;
use glam::DVec3;

/// 体元基本体的形状类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoxelPrimitiveType {
    /// 长方体：网格内所有格子均视为内部。
    Box,
    /// 圆柱：以 XY 平面单位圆判定内部。
    Cylinder,
    /// 椭球：归一化坐标平方和 ≤ 1 判定内部。
    Ellipsoid,
}

/// 单个体元实体的渲染参数组件。
#[derive(Component, Debug, Clone)]
pub struct VoxelPrimitiveComponent {
    /// 形状类型。
    pub shape_type: VoxelPrimitiveType,
    /// 每个轴向的网格分辨率。
    pub grid_resolution: u32,
    /// 包围盒最小角（局部坐标）。
    pub bounds_min: DVec3,
    /// 包围盒最大角（局部坐标）。
    pub bounds_max: DVec3,
    /// 作用于局部坐标的模型变换矩阵。
    pub model_matrix: glam::DMat4,
    /// 基础 RGBA 颜色（逐格会据距中心衰减）。
    pub color_base: [f32; 4],
    /// 可见标志；为假时移除 mesh 不渲染。
    pub visible: bool,
}

impl Default for VoxelPrimitiveComponent {
    /// 默认盒体、分辨率 16、[-1,1] 包围盒、半兰紫色、可见。
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

/// 体元渲染的全局默认配置（用作新建实体的模板）。
#[derive(Resource, Debug, Clone)]
pub struct VoxelConfig {
    /// 默认网格分辨率。
    pub grid_resolution: u32,
    /// 默认形状类型。
    pub shape_type: VoxelPrimitiveType,
    /// 默认包围盒最小角。
    pub bounds_min: DVec3,
    /// 默认包围盒最大角。
    pub bounds_max: DVec3,
    /// 总开关：关闭时渲染系统直接返回。
    pub enabled: bool,
}

impl Default for VoxelConfig {
    /// 默认启用，分辨率 16、盒形、[-1,1] 包围盒。
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

/// 体元插件：注册配置资源与渲染系统。
pub struct CesiumVoxelPlugin;

impl Plugin for CesiumVoxelPlugin {
    /// 初始化 [`VoxelConfig`] 并在 `Update` 阶段添加 [`voxel_render_system`]。
    ///
    /// # 参数
    /// - `app`：待配置的 Bevy App
    fn build(&self, app: &mut App) {
        app.init_resource::<VoxelConfig>()
            .add_systems(Update, voxel_render_system);
    }
}

/// 渲染系统：对变化的体元组件重建 mesh 并回插 `Mesh3d`/材质；
/// 不可见时移除 `Mesh3d`。
///
/// # 参数
/// - `config`：总开关，关闭时直接返回
/// - `commands`：实体命令，用于增删渲染组件
/// - `voxel_query`：仅匹配发生变化的 [`VoxelPrimitiveComponent`]
/// - `meshes`/`materials`：资产写入器
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
            // 不可见：取走已有 mesh，下帧不会参与渲染。
            commands.entity(entity).remove::<Mesh3d>();
            continue;
        }

        // 可见：根据形状/分辨率/包围盒重建体元网格。
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

/// 生成体元网格：逐格采样并判断是否在体内，内部格子发射为小立方体。
///
/// # 参数
/// - `shape_type`：形状类型（决定内部判定方式）
/// - `resolution`：每轴格子数（下限 2）
/// - `bounds_min`/`bounds_max`：局部坐标包围盒
/// - `model_matrix`：将局部坐标变到世界坐标的矩阵
/// - `color_base`：基础颜色，逐格按距中心比例衰减
///
/// # 返回
/// 合并了所有小立方体的顶点/法线/颜色/索引 `Mesh`。
fn generate_voxel_mesh(
    shape_type: VoxelPrimitiveType,
    resolution: u32,
    bounds_min: DVec3,
    bounds_max: DVec3,
    model_matrix: glam::DMat4,
    color_base: [f32; 4],
) -> Mesh {
    // 保证至少 2 格避免除零；step 为单格边长。
    let res = resolution.max(2);
    let step = (bounds_max - bounds_min) / res as f64;

    // 累加目标：四个平行数组逐格 append，最后打包为 mesh。
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // center/extents 用于颜色衰减归一化与椭球判定。
    let center = (bounds_min + bounds_max) * 0.5;
    let extents = (bounds_max - bounds_min) * 0.5;

    for ix in 0..res {
        for iy in 0..res {
            for iz in 0..res {
                // 体中心：取当前格子几何中心（+0.5 对齐到格中）。
                let local_center = DVec3::new(
                    bounds_min.x + (ix as f64 + 0.5) * step.x,
                    bounds_min.y + (iy as f64 + 0.5) * step.y,
                    bounds_min.z + (iz as f64 + 0.5) * step.z,
                );

                // 逐形状判断格中心是否位于体内。
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
                    // 体外格子不发射几何。
                    continue;
                }

                // 将局部中心变到世界坐标（考虑模型变换）。
                let world_center = model_matrix.transform_point3(local_center);

                // 按距包围盒中心的归一化距离 t 对颜色作衰减，营造体积阴影。
                let dist_from_center = local_center.distance(center);
                let max_dist = extents.length();
                let t = (dist_from_center / max_dist).clamp(0.0, 1.0) as f32;

                let cell_color = [
                    color_base[0] * (1.0 - t * 0.5),
                    color_base[1] * (1.0 - t * 0.5),
                    color_base[2] * (1.0 - t * 0.5),
                    color_base[3],
                ];

                // 半轴 = 格长一半，并按模型列向量缩放补偿非均匀缩放。
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

    // 将累加的体元几何写入一个三角列表 mesh。
    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    // 空网格时不写索引，避免 Bevy 断言失败。
    if !indices.is_empty() {
        mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    }
    // 仅在含颜色体元时附加逐顶点颜色属性。
    if !colors.is_empty() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }
    mesh
}

/// 向累加数组追加一个小立方体（6 面、每面 4 顶点 + 2 三角）。
///
/// # 参数
/// - `center`：立方体世界中心
/// - `half`：三轴半轴长
/// - `color`：逐顶点共享颜色
/// - `positions`/`normals`/`colors`/`indices`：累加目标
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

    // 预先算出 8 个角点，再按面索引引用。
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

    // base：本立方体首个顶点在累加 positions 中的基索引。
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

    // 逐面写入 4 个同法线顶点与共 6 个索引（两个三角形）。
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
    /// 验证体元配置默认值。
    fn test_voxel_config_default() {
        let config = VoxelConfig::default();
        assert!(config.enabled);
        assert_eq!(config.grid_resolution, 16);
        assert_eq!(config.shape_type, VoxelPrimitiveType::Box);
    }

    #[test]
    /// 验证盒形体元网格包含位置与法线。
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
    /// 验证圆柱体元网格包含位置。
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
    /// 验证椭球体元网格包含位置。
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
