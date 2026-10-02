//! 基于 clipmap（层级同心网格）的地形渲染 Bevy 插件。
//!
//! 将无限延伸的 LOD 地形拆分为若干环带网格：最内层覆盖最高精度，
//! 每向外一层覆盖面积翻倍。网格随相机目标“吸附”平移，并配合
//! 缝合（stitch）与裁剪（trim）图元消除层级间的裂缝。
//!
//! 主要类型：
//! - [`Clipmap`]：组件，描述单个 clipmap 的参数（宽度/层级/贴图）。
//! - [`ClipmapGrid`]：某一 LOD 层级的网格实体。
//! - [`GridMaterial`]：驱动顶点位移与地平线剔除的 Bevy 材质扩展。

use std::{
    collections::HashMap,
    f32::consts::{FRAC_PI_2, PI},
};

use bevy::{
    asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path},
    camera::{primitives::Aabb, visibility::NoAutoAabb},
    light::NotShadowCaster,
    mesh::{Indices, PrimitiveTopology},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};

/// clipmap 地形插件：注册内嵌着色器与材质，并挂载初始化/更新系统。
pub struct ClipmapPlugin;

/// clipmap 网格的一个组成部件（已上传的 mesh + 其轴对齐包围盒）。
struct ClipmapPart {
    /// 已注册到资产库的网格 handle。
    handle: Handle<Mesh>,
    /// 该部件的轴对齐包围盒（Aabb）。
    aabb: Aabb,
}

impl ClipmapPart {
    /// 由 [`MeshBuilder`] 构建网格部件：上传 mesh 并根据顶点计算 min/max 包围盒。
    ///
    /// # 参数
    /// - `meshes`：网格资产库。
    /// - `builder`：待消费的网格构建器。
    fn build(meshes: &mut ResMut<Assets<Mesh>>, builder: MeshBuilder) -> Self {
        let mut min = Vec3::from_slice(&builder.vertices[0]);
        let mut max = min;
        for v in builder.vertices.iter().map(|v| Vec3::from_slice(v)) {
            min = min.min(v);
            max = max.max(v);
        }
        Self {
            handle: meshes.add(builder.build()),
            aabb: Aabb::from_min_max(min, max),
        }
    }
}

#[derive(Component)]
struct ClipmapParts {
    /// 主方形网格（每层复用的基本图块）。
    square: ClipmapPart,
    /// 填补奇偶对齐间隙的填充网格。
    filler: ClipmapPart,
    /// 仅 level 0 使用的中心网格。
    center: ClipmapPart,
    /// 最外圈的裁剪（环带）网格。
    trim: ClipmapPart,
    /// 用于缝合层级间裂缝的网格。
    stitch: ClipmapPart,
}

impl Plugin for ClipmapPlugin {
    /// 插件装配入口：嵌入 terrain 着色器、注册扩展材质，并添加预更新/更新系统。
    ///
    /// # 参数
    /// - `app`：Bevy 应用。
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "terrain.wgsl");

        app.add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, GridMaterial>,
        >::default())
            .add_systems(PreUpdate, (init_clipmaps, init_grids))
            .add_systems(Update, update_grids);
    }
}

/// 增量式网格构建器：按 (x, y) 格点去重顶点并收集三角形索引。
struct MeshBuilder {
    /// 格点坐标 → 顶点索引 的去重映射。
    unique_vertices: HashMap<(i32, i32), u32>,
    /// 已收集的顶点位置（[x, 0, y]）。
    vertices: Vec<[f32; 3]>,
    /// 已收集的三角形索引（u32）。
    indices: Vec<u32>,
}

impl MeshBuilder {
    /// 创建一个空的构建器。
    fn new() -> Self {
        Self {
            unique_vertices: HashMap::new(),
            vertices: vec![],
            indices: vec![],
        }
    }

    /// 添加（或复用）一个格点顶点，返回其在顶点列表中的索引。
    ///
    /// # 参数
    /// - `x`：格点 x 坐标。
    /// - `y`：格点 y 坐标。
    fn add_vertex(&mut self, x: i32, y: i32) -> u32 {
        if let Some(index) = self.unique_vertices.get(&(x, y)) {
            *index
        } else {
            let index = self.vertices.len() as u32;
            self.vertices.push([x as f32, 0.0, y as f32]);
            self.unique_vertices.insert((x, y), index);
            index
        }
    }

    /// 由三个格点坐标添加一个三角形（自动去重复用顶点）。
    ///
    /// # 参数
    /// - `x1`/`y1`、`x2`/`y2`、`x3`/`y3`：三角形三个顶点的格点坐标。
    fn add_triangle(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, x3: i32, y3: i32) {
        let p1 = self.add_vertex(x1, y1);
        let p2 = self.add_vertex(x2, y2);
        let p3 = self.add_vertex(x3, y3);
        self.indices.extend([p1, p2, p3]);
    }

    /// 在格点 (x, y) 处添加一个由两个三角形组成的单位正方形。
    ///
    /// # 参数
    /// - `x`：正方形左上角格点 x 坐标。
    /// - `y`：正方形左上角格点 y 坐标。
    fn add_square(&mut self, x: i32, y: i32) {
        let p1 = self.add_vertex(x, y);
        let p2 = self.add_vertex(x, y + 1);
        let p3 = self.add_vertex(x + 1, y + 1);
        let p4 = self.add_vertex(x + 1, y);
        self.indices.extend([p1, p2, p3]);
        self.indices.extend([p1, p3, p4]);
    }

    /// 消费构建器，生成带位置属性与 u32 索引的三角形列表 [`Mesh`]。
    fn build(self) -> Mesh {
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.vertices)
            .with_inserted_indices(Indices::U32(self.indices))
    }
}

/// 定义单个 clipmap 的组件（层级同心网格地形）。
/// 参 https://hhoppe.com/gpugcm.pdf
#[derive(Component)]
pub struct Clipmap {
    /// 网格的半宽。
    /// 存为半宽是因为全宽必须为偶数。
    pub half_width: u32,

    /// 生成的 LOD 层级数。
    /// 每向外一层覆盖面积翻倍。
    pub levels: u32,

    /// 最内层 LOD 方块的基准尺度（世界单位）。
    pub base_scale: f32,

    /// 一个 texel 的物理尺寸（米）。
    pub texel_size: f32,

    /// 需要跟随的目标实体。
    pub target: Entity,

    /// 颜色贴图。
    pub color: Handle<Image>,

    /// 高程图贴图。
    pub heightmap: Handle<Image>,

    /// FFT 压缩的地平线贴图。
    pub horizon: Handle<Image>,

    /// FFT 系数数量。
    pub horizon_coeffs: u32,

    /// 高程范围下限。
    pub min: f32,
    /// 高程范围上限。
    pub max: f32,

    /// 是否启用线框。
    pub wireframe: bool,
}

#[derive(Component)]
struct ClipmapGrid {
    /// 该网格所属的 LOD 层级（0 为最内/最高精度）。
    level: u32,
    /// 环带裁剪图元的实体（需逐帧重新定位）。
    trim: Entity,
}

impl ClipmapGrid {
    /// 返回该层级的世界尺度：base_scale × 2^level。
    ///
    /// # 参数
    /// - `base_scale`：最内层的基础尺度。
    fn scale(&self, base_scale: f32) -> f32 {
        base_scale * 2u32.pow(self.level) as f32
    }
}

/// 预更新系统：为新加入的 [`Clipmap`] 构建各组成网格部件并生成对应层级的子网格实体。
///
/// # 参数
/// - `commands`：实体命令生成器。
/// - `meshes`：网格资产库。
/// - `clipmaps`：本帧新增的 clipmap 查询。
fn init_clipmaps(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    clipmaps: Query<(Entity, &Clipmap), Added<Clipmap>>,
) {
    for (entity, clipmap) in clipmaps {
        // 构建器总宽度为 half_width×2；filler_width 用于把宽度补成偶数以对齐中心。
        let builder_width = clipmap.half_width as i32 * 2;
        let filler_width = 2 - clipmap.half_width as i32 % 2;
        let square_width = (clipmap.half_width as i32 - filler_width) / 2;

        let mut square = MeshBuilder::new();
        let mut filler = MeshBuilder::new();
        let mut center = MeshBuilder::new();
        let mut trim = MeshBuilder::new();
        let mut stitch = MeshBuilder::new();

        // 逐格点遍历，按所在区域把正方形分派给 square/center/filler/trim 四类部件。
        for xy in 0..builder_width.pow(2) {
            let x = xy % builder_width;
            let y = xy / builder_width;
            // 左上 square_width×square_width 区域为可复用的基本方形图块。
            if x < square_width && y < square_width {
                square.add_square(x, y);
            }
            // 命中中心十字区域者归入 center；其中不在内部实心区的补入 filler。
            let range = square_width * 2..square_width * 2 + filler_width;
            if (range.contains(&x) || range.contains(&y))
                && x < builder_width - filler_width
                && y < builder_width - filler_width
            {
                center.add_square(x, y);
                let range = square_width..builder_width - square_width - filler_width;
                if !range.contains(&x) || !range.contains(&y) {
                    filler.add_square(x, y);
                }
            }
            // 最外一行/一列归入 trim（裁剪环带）。
            if x >= builder_width - filler_width || y >= builder_width - filler_width {
                trim.add_square(x, y);
            }
        }

        // 沿四条边生成退化三角形，用于缝合相邻 LOD 层级之间的裂缝。
        for x in 0..builder_width / 2 {
            let x = x * 2;
            stitch.add_triangle(x, 0, x + 1, 0, x + 2, 0);
            stitch.add_triangle(x + 2, builder_width, x + 1, builder_width, x, builder_width);
            stitch.add_triangle(0, x + 2, 0, x + 1, 0, x);
            stitch.add_triangle(builder_width, x, builder_width, x + 1, builder_width, x + 2);
        }

        commands.entity(entity).insert((
            Transform::default(),
            Visibility::default(),
            ClipmapParts {
                square: ClipmapPart::build(&mut meshes, square),
                filler: ClipmapPart::build(&mut meshes, filler),
                center: ClipmapPart::build(&mut meshes, center),
                trim: ClipmapPart::build(&mut meshes, trim),
                stitch: ClipmapPart::build(&mut meshes, stitch),
            },
        ));

        // 为每个层级生成一个 ClipmapGrid 子实体（trim 实体稍后在 init_grids 中回填）。
        for level in 0..clipmap.levels {
            commands.entity(entity).with_child(ClipmapGrid {
                level,
                trim: Entity::PLACEHOLDER,
            });
        }
    }
}

/// 预更新系统：为新增的 [`ClipmapGrid`] 创建地形材质并按层级布局 square/center/filler/stitch 图元。
///
/// # 参数
/// - `commands`：实体命令生成器。
/// - `materials`：扩展材质资产库。
/// - `clipmaps`：clipmap 与其网格部件的查询。
/// - `grids`：本帧新增的层级网格查询。
fn init_grids(
    mut commands: Commands,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, GridMaterial>>>,
    clipmaps: Query<(&Clipmap, &ClipmapParts)>,
    mut grids: Query<(Entity, &mut ClipmapGrid, &ChildOf), Added<ClipmapGrid>>,
) {
    for (entity, mut grid, clipmap) in &mut grids {
        let (clipmap, parts) = clipmaps.get(clipmap.parent()).unwrap();

        let filler_width = 2 - clipmap.half_width as i32 % 2;
        let square_width = (clipmap.half_width as i32 - filler_width) / 2;

        commands.entity(entity).insert((
            Transform::from_scale(Vec3::splat(grid.scale(clipmap.base_scale))),
            Visibility::default(),
        ));

        // 为当前层级创建两份地形材质：一份实体渲染，一份（_w）专用于线框叠加。
        let terrain_material = materials.add(ExtendedMaterial {
            base: StandardMaterial::default(),
            extension: GridMaterial {
                color: clipmap.color.clone(),
                heightmap: clipmap.heightmap.clone(),
                horizon: clipmap.horizon.clone(),
                horizon_coeffs: clipmap.horizon_coeffs,
                lod: grid.level,
                texel_size: clipmap.texel_size,
                minmax: Vec2 {
                    x: clipmap.min,
                    y: clipmap.max,
                },
                translation: Vec2::ZERO,
                wireframe: 0,
            },
        });

        let terrain_material_w = materials.add(ExtendedMaterial {
            base: StandardMaterial::default(),
            extension: GridMaterial {
                color: clipmap.color.clone(),
                heightmap: clipmap.heightmap.clone(),
                horizon: clipmap.horizon.clone(),
                horizon_coeffs: clipmap.horizon_coeffs,
                lod: grid.level,
                texel_size: clipmap.texel_size,
                minmax: Vec2 {
                    x: clipmap.min,
                    y: clipmap.max,
                },
                translation: Vec2::ZERO,
                wireframe: 1,
            },
        });

        // 以 4×4 分块环绕中心布局方形图元；level≠​0 时跳过被更高层覆盖的中心 2×2。
        for xy in 0..4 * 4 {
            let x = xy % 4;
            let y = xy / 4;

            if grid.level != 0 && (x == 1 || x == 2) && (y == 1 || y == 2) {
                continue;
            }

            // 右/下半区的图元需额外偏移 filler_width 以补偿奇偶间隙。
            let offset_x = if x >= 2 { filler_width as f32 } else { 0.0 };
            let offset_y = if y >= 2 { filler_width as f32 } else { 0.0 };

            commands.entity(entity).with_children(|c| {
                let mut e = c.spawn((
                    Mesh3d(parts.square.handle.clone()),
                    MeshMaterial3d(terrain_material.clone()),
                    NotShadowCaster,
                    Transform::from_xyz(
                        (x - 2) as f32 * square_width as f32 + offset_x,
                        0.0,
                        (y - 2) as f32 * square_width as f32 + offset_y,
                    ),
                    NoAutoAabb,
                    parts.square.aabb.clone(),
                ));
                if clipmap.wireframe {
                    e.with_child((
                        Mesh3d(parts.square.handle.clone()),
                        MeshMaterial3d(terrain_material_w.clone()),
                        NoAutoAabb,
                        parts.square.aabb.clone(),
                    ));
                }
            });
        }

        // level 0 用中心图元覆盖最内区；更高层级改用 filler + stitch 处理层级衔接。
        if grid.level == 0 {
            commands.entity(entity).with_children(|c| {
                let mut e = c.spawn((
                    Mesh3d(parts.center.handle.clone()),
                    MeshMaterial3d(terrain_material.clone()),
                    NotShadowCaster,
                    Transform::from_xyz(
                        -2.0 * square_width as f32,
                        0.0,
                        -2.0 * square_width as f32,
                    ),
                    NoAutoAabb,
                    parts.center.aabb,
                ));
                if clipmap.wireframe {
                    e.with_child((
                        Mesh3d(parts.center.handle.clone()),
                        MeshMaterial3d(terrain_material_w.clone()),
                        NoAutoAabb,
                        parts.center.aabb,
                    ));
                }
            });
        } else {
            commands.entity(entity).with_children(|c| {
                let mut e = c.spawn((
                    Mesh3d(parts.filler.handle.clone()),
                    MeshMaterial3d(terrain_material.clone()),
                    NotShadowCaster,
                    Transform::from_xyz(
                        -2.0 * square_width as f32,
                        0.0,
                        -2.0 * square_width as f32,
                    ),
                    NoAutoAabb,
                    parts.filler.aabb,
                ));
                if clipmap.wireframe {
                    e.with_child((
                        Mesh3d(parts.filler.handle.clone()),
                        MeshMaterial3d(terrain_material_w.clone()),
                        NoAutoAabb,
                        parts.filler.aabb,
                    ));
                }
            });
            commands.entity(entity).with_children(|c| {
                let mut e = c.spawn((
                    Mesh3d(parts.stitch.handle.clone()),
                    MeshMaterial3d(terrain_material.clone()),
                    NotShadowCaster,
                    Transform::from_xyz(-square_width as f32, 0.0, -square_width as f32)
                        .with_scale(Vec3::splat(0.5)),
                    NoAutoAabb,
                    parts.stitch.aabb,
                ));
                if clipmap.wireframe {
                    e.with_child((
                        Mesh3d(parts.stitch.handle.clone()),
                        MeshMaterial3d(terrain_material_w.clone()),
                        NoAutoAabb,
                        parts.stitch.aabb,
                    ));
                }
            });
        }

        // 最后拼接最外圈裁剪（trim）图元，并记录其实体供 update 阶段逐帧定位。
        let mut trim = commands.spawn((
            Mesh3d(parts.trim.handle.clone()),
            MeshMaterial3d(terrain_material.clone()),
            NotShadowCaster,
            Transform::from_xyz(-2.0 * square_width as f32, 0.0, -2.0 * square_width as f32),
            NoAutoAabb,
            parts.trim.aabb,
        ));
        if clipmap.wireframe {
            trim.with_child((
                Mesh3d(parts.trim.handle.clone()),
                MeshMaterial3d(terrain_material_w.clone()),
                NoAutoAabb,
                parts.trim.aabb,
            ));
        }
        grid.trim = trim.id();
        commands.entity(entity).add_child(grid.trim);
    }
}

/// 更新系统：根据目标位置将各层级网格“吸附”到格点，同步裁剪图元的偏移/旋转，
/// 并更新子图元的材质平移与包围盒高度。
///
/// # 参数
/// - `transforms`：变换组件查询。
/// - `aabbs`：包围盒组件查询。
/// - `terrain_materials`：地形扩展材质资产库。
/// - `terrain_material_handles`：网格上的材质 handle 查询。
/// - `clipmaps`：clipmap 参数查询。
/// - `children`：子实体遗启查询。
/// - `grids`：已有变换的层级网格查询。
fn update_grids(
    mut transforms: Query<&mut Transform>,
    mut aabbs: Query<&mut Aabb>,
    mut terrain_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, GridMaterial>>>,
    terrain_material_handles: Query<
        &MeshMaterial3d<ExtendedMaterial<StandardMaterial, GridMaterial>>,
    >,
    clipmaps: Query<&Clipmap>,
    children: Query<&Children>,
    grids: Query<(Entity, &ClipmapGrid, &ChildOf), With<Transform>>,
) {
    for (entity, grid, clipmap) in grids {
        let clipmap = clipmaps.get(clipmap.parent()).unwrap();
        let filler_width = 2 - clipmap.half_width as i32 % 2;
        let snap_scale = grid.scale(clipmap.base_scale) * filler_width as f32;
        // 目标位置除以吸附尺度取整，得到对齐到格点的整数坐标与吸附后位置。
        let target_pos = transforms.get(clipmap.target).unwrap().translation;
        let snap_factor = (target_pos / snap_scale).floor().as_ivec3().xz();
        let snap_pos = snap_factor.as_vec2() * snap_scale;
        transforms.get_mut(entity).unwrap().translation = snap_pos.extend(0.0).xzy();

        // 吸附坐标模 2 决定裁剪环带的偏移与旋转（消除奇偶对齐产生的缝隙）。
        let snap_mod2 = ((snap_factor % 2) + 2) % 2;
        let mut trim_transform = transforms.get_mut(grid.trim).unwrap();
        trim_transform.translation = {
            let offset_0 = filler_width as f32 - clipmap.half_width as f32;
            let offset_1 = clipmap.half_width as f32;
            Vec3 {
                x: if snap_mod2.x == 0 { offset_0 } else { offset_1 },
                y: 0.0,
                z: if snap_mod2.y == 0 { offset_0 } else { offset_1 },
            }
        };
        // 根据吸附奇偶选择四种旋转，使裁剪环带始终朝向最外侧。
        trim_transform.rotation = Quat::from_rotation_y(match snap_mod2 {
            IVec2 { x: 0, y: 0 } => 0.0,
            IVec2 { x: 0, y: 1 } => FRAC_PI_2,
            IVec2 { x: 1, y: 0 } => -FRAC_PI_2,
            IVec2 { x: 1, y: 1 } => PI,
            _ => unreachable!(),
        });

        // 遍历层级网格的后代图元，写入材质平移并按层级修正包围盒高度。
        let grid_pos = (snap_pos.extend(0.0).xzy() + trim_transform.translation * snap_scale).xz();
        let aabb_scale = 2u32.pow(1 + grid.level) as f32;
        for child in children.iter_descendants(entity) {
            // 仅处理携带地形材质的后代；其余（如容器实体）跳过。
            let Ok(material) = terrain_material_handles.get(child) else {
                continue;
            };
            // 取到可变的材质扩展后，写入当前层级的格点平移。
            let Some(mut material) = terrain_materials.get_mut(material) else {
                continue;
            };
            let Ok(mut aabb) = aabbs.get_mut(child) else {
                continue;
            };
            material.extension.translation = grid_pos;
            // 按层级尺度缩放包围盒的高度中心与半高，使其贴合实际高程范围。
            aabb.center.y = (clipmap.max + clipmap.min) / aabb_scale;
            aabb.half_extents.y = (clipmap.max - clipmap.min) / aabb_scale;
        }
    }
}

#[repr(C)]
#[derive(Eq, PartialEq, Hash, Copy, Clone)]
struct WireframeKey {
    /// 是否以线框（polygon_mode=Line）模式特化管线。
    wireframe: bool,
}

impl From<&GridMaterial> for WireframeKey {
    /// 从材质的 wireframe 标志推导绑定组特化键。
    ///
    /// # 参数
    /// - `material`：源网格材质。
    fn from(material: &GridMaterial) -> Self {
        Self {
            wireframe: material.wireframe != 0,
        }
    }
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[bind_group_data(WireframeKey)]
struct GridMaterial {
    /// 地表颜色贴图。
    #[texture(100)]
    #[sampler(101)]
    color: Handle<Image>,
    /// 高程图贴图（驱动顶点位移）。
    #[texture(102)]
    #[sampler(103)]
    heightmap: Handle<Image>,
    /// FFT 压缩的地平线贴图（2d_array）。
    #[texture(104, dimension = "2d_array")]
    #[sampler(105)]
    horizon: Handle<Image>,
    /// FFT 系数数量。
    #[uniform(106)]
    horizon_coeffs: u32,
    /// 当前网格的 LOD 层级。
    #[uniform(107)]
    lod: u32,
    /// 一个 texel 的物理尺寸（米）。
    #[uniform(108)]
    texel_size: f32,
    /// 高程范围（x=min, y=max）。
    #[uniform(109)]
    minmax: Vec2,
    /// 网格在格点空间的平移（用于贴图采样）。
    #[uniform(110)]
    translation: Vec2,
    /// 线框开关（非 0 则渲染线框）。
    #[uniform(111)]
    wireframe: u32,
}

impl MaterialExtension for GridMaterial {
    /// 前向渲染顶点着色器（指向内嵌的 terrain.wgsl）。
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("terrain.wgsl")).with_source("embedded"),
        )
    }

    /// 延迟渲染顶点着色器。
    fn deferred_vertex_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("terrain.wgsl")).with_source("embedded"),
        )
    }

    /// 前向渲染片元着色器。
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("terrain.wgsl")).with_source("embedded"),
        )
    }

    /// 延迟渲染片元着色器。
    fn deferred_fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("terrain.wgsl")).with_source("embedded"),
        )
    }

    /// 管线特化：当启用线框时切换到 Line 多边形模式并调整深度偏差。
    ///
    /// # 参数
    /// - `descriptor`：待修改的渲染管线描述符。
    /// - `key`：由 [`WireframeKey`] 驱动的绑定组特化键。
    fn specialize(
        _: &bevy::pbr::MaterialExtensionPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _: &bevy::mesh::MeshVertexBufferLayoutRef,
        key: bevy::pbr::MaterialExtensionKey<Self>,
    ) -> std::result::Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        if key.bind_group_data.wireframe {
            descriptor.primitive.polygon_mode = bevy::render::render_resource::PolygonMode::Line;
            descriptor.depth_stencil.as_mut().unwrap().bias.slope_scale = 1.0;
        }
        Ok(())
    }
}
