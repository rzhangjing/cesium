//! 实体 → Bevy Mesh 转换系统。
//!
//! 将领域 Entity 的图形映射为 Bevy 的 mesh、材质与变换。
//! 处理 Point（四边形）、Polyline（挤出线串）、Polygon（扇形
//! 三角剖分）、Billboard（面向相机的四边形）与 Model（glTF 加载）。
//!
//! 设计要点：
//! - 事件驱动重建：仅当实体带 [`NeedsVisualUpdate`] 标记时才重建
//!   可视化，处操作后取走该标记，避免每帧无谓地重生 mesh。
//! - 时变属性统一以 `current_time` 采样，缺省值回退到常见默认。
//! - 几何网格生成委托给 `entity_render` 中的共享函数，保持与直接
//!   渲染路径一致；本模块只负责组件装配与子实体生成。
//! - 测试构建下不加载外部资产（贴图/glTF），以免依赖真实资源。

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

/// 将领域实体转为 Bevy 可渲染组件的系统。
///
/// 仅处理带 [`NeedsVisualUpdate`] 标记的实体：逐图形（点/线/面/
/// billboard/model）解析时变属性、插入对应的图形组件，并为
/// 几何生成子实体（mesh + 材质）。
///
/// # 参数
/// - `commands`：实体命令写入器，用于插入组件与生成子实体
/// - `domain_entities`：带领域包被与可选刷新标记的实体查询
/// - `ellipsoid`：地球椭球，供经纬度→地心坐标转换
/// - `time`：仿真时钟，提供属性采样的当前时刻
/// - `meshes`/`materials`：mesh 与材质资产写入器
/// - `asset_server`：（非测试）加载 billboard 贴图与 glTF 场景
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
    // 当前仿真时刻，用于对所有时变属性采样。
    let current_time = time.elapsed_secs_f64();

    for (bevy_entity, entity_wrapper, needs_update) in domain_entities.iter() {
        // 无刷新标记→几何已是最新，直接跳过。
        if needs_update.is_none() {
            continue;
        }

        let domain_entity = &entity_wrapper.0;
        let mut entity_cmd = commands.entity(bevy_entity);

        // 取走刷新标记，避免下一帧重复重建。
        entity_cmd.remove::<NeedsVisualUpdate>();

        // 点图形：把像素尺寸/颜色/描边写入组件，再生成一个单位四边形 mesh。
        if let Some(ref point) = domain_entity.point {
            // 主色/描边色不随时变，用默认回退；尺寸与描边宽则按时采样。
            let color = resolve_color(&point.color, Color::WHITE);
            let outline_color = resolve_color(&point.outline_color, Color::BLACK);
            // 像素尺寸与描边宽按时变属性采样，无值时分别回退 1.0 / 0.0。
            let pixel_size = point.pixel_size.get_value(current_time).copied().unwrap_or(1.0);
            // 描边宽默认 0.0，即不绘外轮廓。
            let outline_width = point.outline_width.get_value(current_time).copied().unwrap_or(0.0);

            // 把采样后的点参数烘入渲染组件（均为 f32，适配 shader 读取）。
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

            // 点 quad 居中于实体地表坐标，采用无光照材质以避免光照依赖。
            let mesh = create_point_quad_mesh(pixel_size as f32);
            let mesh_handle = meshes.add(mesh);
            // 实体变换由领域位置（经纬高）经椭球投影得出。
            let transform = entity_position_to_transform(domain_entity, &ellipsoid.0, current_time)
                .unwrap_or_default();
            let color_bevy = domain_color_to_bevy(&Color::new(
                color.red, color.green, color.blue, color.alpha,
            ));
            // 无光照材质：点颜色直接作为发射色，不受场景光照影响。
            let mat_handle = materials.add(StandardMaterial {
                base_color: color_bevy,
                unlit: true,
                ..default()
            });

            // 作为子实体挂入，携带共享的可视化标记。
            entity_cmd.with_children(|parent| {
                parent.spawn((
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(mat_handle),
                    transform,
                    VisualizationBuilt,
                ));
            });
        }

        // 折线图形：解析宽度/颜色/贴地标志，把经纬度顶点转为地心坐标。
        if let Some(ref polyline) = domain_entity.polyline {
            let width =
                polyline.width.get_value(current_time).copied().unwrap_or(1.0) as f32;
            let color = resolve_color(&polyline.color, Color::WHITE);
            // 贴地标志影响后续几何是否强制落在地表。
            let clamp_to_ground =
                polyline.clamp_to_ground.get_value(current_time).copied().unwrap_or(false);

            // 逐顶点把弧度 (lon,lat,alt) 转为椭球地心直角坐标。
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

            // 折线组件保留展平顶点列表以便下游重建段拓扑。
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

            // 复用 entity_render 的挤出线串网格生成；成功则附子实体。
            if let Some(mesh) =
                render_create_polyline_mesh(polyline, &ellipsoid.0, current_time)
            {
                let mesh_handle = meshes.add(mesh);
                let color_bevy = domain_color_to_bevy(&Color::new(
                    color.red, color.green, color.blue, color.alpha,
                ));
                // 折线采用受光照的标准材质（区别于无光照的点）。
                let mat_handle = materials.add(StandardMaterial {
                    base_color: color_bevy,
                    ..default()
                });
                entity_cmd.with_children(|parent| {
                    // 折线子实体不携带 transform（顶点已在世界坐标）。
                    parent.spawn((
                        Mesh3d(mesh_handle),
                        MeshMaterial3d(mat_handle),
                        VisualizationBuilt,
                    ));
                });
            }
        }

        // 多边形图形：高度、拉伸高度、材质与描边；顶点同折线转地心坐标。
        if let Some(ref polygon) = domain_entity.polygon {
            // 高度与可选拉伸高度共同决定多边形的立体形态。
            let height =
                polygon.height.get_value(current_time).copied().unwrap_or(0.0);
            // 拉伸高度为 None 时是多边形平铺，Some 时挤成立体棱柱。
            let extruded_height =
                polygon.extruded_height.get_value(current_time).copied();
            let color = resolve_color(&polygon.material, Color::WHITE);
            // 描边开关与描边颜色控制多边形轮廓是否可见。
            let outline = polygon.outline.get_value(current_time).copied().unwrap_or(false);
            let outline_color = resolve_color(&polygon.outline_color, Color::BLACK);

            // 多边形顶点在高度上额外叠加 height 偏移。
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

            // 多边形组件：暂不支持内洞（holes 置空），高度/拉伸原样存储。
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

            // 复用 entity_render 的多边形扇形三角化网格。
            if let Some(mesh) =
                render_create_polygon_mesh(polygon, &ellipsoid.0, current_time)
            {
                let mesh_handle = meshes.add(mesh);
                let color_bevy = domain_color_to_bevy(&Color::new(
                    color.red, color.green, color.blue, color.alpha,
                ));
                // 多边形同样用受光照材质，与折线保持一致的表面表现。
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

        // Billboard 图形：带贴图则走 AssetServer 加载纹理，否则回退为纯色 quad。
        if let Some(ref billboard) = domain_entity.billboard {
            // 图地址、缩放与调制颜色；图地址可能缺失则降级为纯色。
            let image_url = billboard.image.get_value(current_time).cloned();
            let scale =
                billboard.scale.get_value(current_time).copied().unwrap_or(1.0) as f32;
            // 颜色作为乘性调制叠加到贴图/回退色上。
            let color = resolve_color(&billboard.color, Color::WHITE);

            // 先写入图形组件（即使无贴图也让下游可见其缩放/颜色）。
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
                // 有图片：加载纹理并以混合模式渲染，标记为 billboard 以参与朝向相机。
                let mesh = create_billboard_quad_mesh();
                let mesh_handle = meshes.add(mesh);
                let texture: Handle<Image> = asset_server.load(url);
                // 贴图材质：启用 Alpha 混合以支持透明背景。
                let mat_handle = materials.add(StandardMaterial {
                    base_color_texture: Some(texture),
                    base_color: domain_color_to_bevy(&Color::new(
                        color.red, color.green, color.blue, color.alpha,
                    )),
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                });
                entity_cmd.with_children(|parent| {
                    // BillboardTag 标记使子实体被朝向相机系统捕获。
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
                // 无图片：用不透明白 quad 占位，仍保留朝向相机行为。
                let mesh = create_billboard_quad_mesh();
                let mesh_handle = meshes.add(mesh);
                let color_bevy = domain_color_to_bevy(&Color::new(
                    color.red, color.green, color.blue, color.alpha,
                ));
                // 无贴图回退：不透明纯色 quad，保证可见性。
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

        // Model 图形：按 URI 加载 glTF 场景，作为子 SceneRoot 挂入并施加缩放。
        if let Some(ref model) = domain_entity.model {
            // URI 缺失时降级为空路径；缩放与最小像素阈值控制可见性。
            let uri = model.uri.get_value(current_time).cloned().unwrap_or_default();
            let scale =
                model.scale.get_value(current_time).copied().unwrap_or(1.0) as f32;
            let min_pixel_size = model
                .minimum_pixel_size
                .get_value(current_time)
                .copied()
                .unwrap_or(0.0) as f32;

            // 模型参数先入组件，供拾取/UI 读取；实际渲染走子 SceneRoot。
            entity_cmd.insert(ModelGraphicsComponent {
                uri: uri.clone(),
                scale,
                minimum_pixel_size: min_pixel_size,
            });

            #[cfg(not(test))]
            {
                // `#Scene0` 后缀指向 glTF 资产的第一个场景。
                let scene_path = format!("{}#Scene0", uri);
                let scene_handle: Handle<Scene> = asset_server.load(&scene_path);
                // 子实体仅携 SceneRoot 与缩放，模型自身拓扑由 glTF 决定。
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

/// 每帧更新 billboard 变换以朝向相机。
///
/// 取唯一相机的位置作为参考：把每个 billboard 的旋转对准
/// “指向相机”方向，使其始终正面面向观察。
pub fn billboard_face_camera_system(
    camera_query: Query<&Transform, (With<Camera>, Without<BillboardTag>)>,
    mut billboard_query: Query<&mut Transform, With<BillboardTag>>,
) {
    // 无相机（或多于一个）时不做处理。
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };

    for mut billboard_tf in billboard_query.iter_mut() {
        // 从 billboard 指向相机的向量；零向量时跳过避免退化朝向。
        let direction = camera_transform.translation - billboard_tf.translation;
        if direction.length_squared() > f32::EPSILON {
            billboard_tf.look_to(-direction, camera_transform.up().as_vec3());
        }
    }
}

/// 为点渲染创建一个单位四边形 mesh。
///
/// # 参数
/// - `pixel_size`：四边形半边长基准（实际半边 = pixel_size * 0.5）
///
/// # 返回
/// 带位置/法线/UV 与两个三角形的 `Mesh`。
fn create_point_quad_mesh(pixel_size: f32) -> Mesh {
    // 以中心为原点、半边长 = pixel_size/2 的 XY 平面正方形。
    let half = pixel_size * 0.5;
    let vertices = vec![
        [-half, -half, 0.0f32],
        [half, -half, 0.0],
        [half, half, 0.0],
        [-half, half, 0.0],
    ];
    // 法线统一朝 +Z；UV 与四角一一对应，便于后续贴图。
    let normals = vec![[0.0f32, 0.0, 1.0]; 4];
    let uvs = vec![[0.0f32, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    // 两个三角形 (0,1,2) 与 (0,2,3) 拼成四边形。
    let indices = vec![0u32, 1, 2, 0, 2, 3];

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    // 依次写入位置/法线/UV 与索引，构成可渲染的双三角四边形。
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    mesh
}

/// 为 billboard 渲染创建一个单位四边形 mesh。
///
/// 尺寸固定为半宽 50_000 米（世界坐标尺度），朝向由 billboard
/// 面向相机系统逐帧修正。
fn create_billboard_quad_mesh() -> Mesh {
    // 固定半宽 50_000 米：足够大以保证近相机时不露边界。
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

/// 在给定时间解析一个颜色属性。
///
/// 颜色不随时变化，故固定以 0.0 采样；无值时回退到 `default`。
fn resolve_color(prop: &Property<Color>, default: Color) -> Color {
    // 无值时以 unwrap_or 回退到传入默认色。
    prop.get_value(0.0).copied().unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 验证点 quad 具备位置与 UV 属性。
    fn test_create_point_quad_mesh() {
        let mesh = create_point_quad_mesh(10.0);
        assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_UV_0).is_some());
    }

    #[test]
    /// 验证 billboard quad 具备位置与法线属性。
    fn test_create_billboard_quad_mesh() {
        let mesh = create_billboard_quad_mesh();
        assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
    }
}
