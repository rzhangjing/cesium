//! 矢量瓦片（MVT）渲染适配层：把 `cesium-vector` 解码出的
//! Mapbox Vector Tile 几何（点/线/面）转为 Bevy 组件，并在
//! `Update` 阶段重建为可渲染的 mesh。
//!
//! 处理流程分三步：[`decode_mvt_system`] 抽取几何与属性，
//! [`clamp_to_ground_system`] 可选地把高度压平到地面，
//! [`render_vector_system`] 生成最终的 `Mesh3d`/材质。

use bevy::prelude::*;
use cesium_vector::{
    decode_mvt_geometry, MvtFeature, MvtGeometryType, MvtLayer, MvtValue,
};
use glam::DVec3;

/// 矢量瓦片渲染的全局配置资源。
#[derive(Resource, Debug, Clone)]
pub struct VectorTileConfig {
    /// 总开关：关闭时所有矢量系统直接跳过。
    pub enabled: bool,
    /// 是否把顶点高度钳制到地面（z=0）。
    pub clamp_to_ground: bool,
    /// 点要素的默认像素尺寸。
    pub default_point_size: f64,
    /// 折线的默认宽度。
    pub default_polyline_width: f64,
}

impl Default for VectorTileConfig {
    /// 默认启用渲染、不贴地，点尺寸 5.0、线宽 2.0。
    fn default() -> Self {
        Self {
            enabled: true,
            clamp_to_ground: false,
            default_point_size: 5.0,
            default_polyline_width: 2.0,
        }
    }
}

/// 单个已解码瓦片的原始 MVT 数据及其瓦片坐标（z/x/y）。
#[derive(Component, Debug, Clone)]
pub struct MvtTileData {
    /// 瓦片包含的所有 MVT 图层。
    pub layers: Vec<MvtLayer>,
    /// 瓦片的 x 索引。
    pub tile_x: u32,
    /// 瓦片的 y 索引。
    pub tile_y: u32,
    /// 瓦片的缩放层级。
    pub tile_z: u32,
}

/// 点几何渲染组件：位置、逐点颜色与逐点尺寸一一对应。
#[derive(Component, Debug, Clone)]
pub struct PointGraphics {
    /// 各点的世界坐标。
    pub positions: Vec<DVec3>,
    /// 各点的 RGBA 颜色。
    pub colors: Vec<[f64; 4]>,
    /// 各点的像素尺寸。
    pub sizes: Vec<f64>,
}

/// 折线几何渲染组件：顶点展平存储，由 `polyline_counts` 分段。
#[derive(Component, Debug, Clone)]
pub struct PolylineGraphics {
    /// 所有折线段的顶点（按段拼接）。
    pub positions: Vec<DVec3>,
    /// 逐顶点的 RGBA 颜色。
    pub colors: Vec<[f64; 4]>,
    /// 逐段的宽度。
    pub widths: Vec<f64>,
    /// 每条折线包含的顶点数，用于重建段边界。
    pub polyline_counts: Vec<usize>,
}

/// 多边形几何渲染组件：预三角化后的顶点与索引。
#[derive(Component, Debug, Clone)]
pub struct PolygonGraphics {
    /// 三角化后的顶点。
    pub positions: Vec<DVec3>,
    /// 三角形索引列表。
    pub indices: Vec<u32>,
    /// 逐顶点的 RGBA 颜色。
    pub colors: Vec<[f64; 4]>,
}

/// 瓦片解码完成事件，携带已填充几何的目标实体。
#[derive(Event)]
pub struct MvtTileLoaded {
    /// 已加载数据对应的实体。
    pub entity: Entity,
}

/// 矢量瓦片插件：注册配置资源、事件与三个 `Update` 系统。
pub struct CesiumVectorTilePlugin;

impl Plugin for CesiumVectorTilePlugin {
    /// 初始化配置与事件，并按“解码→贴地→渲染”顺序注册系统。
    ///
    /// # 参数
    /// - `app`：待配置的 Bevy App
    fn build(&self, app: &mut App) {
        app.init_resource::<VectorTileConfig>()
            .add_event::<MvtTileLoaded>()
            .add_systems(
                Update,
                (decode_mvt_system, clamp_to_ground_system, render_vector_system),
            );
    }
}

/// 解码系统：遍历新加入的 [`MvtTileData`]，逐图层逐要素把 MVT
/// 几何解为环，按几何类型分别为实体插入点/线/面组件。
///
/// # 参数
/// - `config`：矢量渲染配置，关闭时直接返回
/// - `mvt_query`：仅匹配刚新增 `MvtTileData` 的实体
/// - `commands`：用于给实体插入图形组件
pub fn decode_mvt_system(
    config: Res<VectorTileConfig>,
    mvt_query: Query<(Entity, &MvtTileData), Added<MvtTileData>>,
    mut commands: Commands,
) {
    if !config.enabled {
        return;
    }

    for (entity, tile_data) in mvt_query.iter() {
        for layer in &tile_data.layers {
            for feature in &layer.features {
                // 把 MVT 整型坐标按图层 extent 解码为局部坐标环。
                let rings = decode_mvt_geometry(&feature.geometry, layer.extent.max(1));
                let properties = extract_properties(feature, &layer.keys, &layer.values);

                // 属性里的 color 优先，否则用不透明白。
                let color = properties
                    .get("color")
                    .and_then(|v| parse_color(v))
                    .unwrap_or([1.0, 1.0, 1.0, 1.0]);

                match feature.geometry_type {
                    MvtGeometryType::Point => {
                        // 点：每环首点即一个顶点，尺寸/颜色按默认值广播。
                        let positions: Vec<DVec3> = rings
                            .iter()
                            .filter_map(|r| r.first().copied())
                            .collect();

                        let sizes = vec![config.default_point_size; positions.len()];
                        let colors = vec![color; positions.len()];

                        commands.entity(entity).insert(PointGraphics {
                            positions,
                            colors,
                            sizes,
                        });
                    }
                    MvtGeometryType::LineString => {
                        // 线：逐环展平拼接，同时记录每环点数供后续分段。
                        let mut positions: Vec<DVec3> = Vec::new();
                        let mut polyline_counts: Vec<usize> = Vec::new();

                        for ring in &rings {
                            // 至少两点才能构成一条线段。
                            if ring.len() >= 2 {
                                polyline_counts.push(ring.len());
                                positions.extend_from_slice(ring);
                            }
                        }

                        if !positions.is_empty() {
                            let widths = vec![config.default_polyline_width; polyline_counts.len()];
                            let colors = vec![color; positions.len()];

                            commands.entity(entity).insert(PolylineGraphics {
                                positions,
                                colors,
                                widths,
                                polyline_counts,
                            });
                        }
                    }
                    MvtGeometryType::Polygon => {
                        // 面：用扇形三角化（首点为公共顶点）逐环展开。
                        let mut all_positions: Vec<DVec3> = Vec::new();
                        let mut all_indices: Vec<u32> = Vec::new();

                        for ring in &rings {
                            if ring.len() < 3 {
                                // 不足三点的环无法构成多边形，跳过。
                                continue;
                            }
                            // base：当前环在展平数组中的起始顶点索引。
                            let base = all_positions.len() as u32;
                            all_positions.extend_from_slice(ring);
                            for i in 1..(ring.len() - 1) {
                                all_indices.push(base);
                                all_indices.push(base + i as u32);
                                all_indices.push(base + i as u32 + 1);
                            }
                        }

                        if !all_positions.is_empty() {
                            let colors = vec![color; all_positions.len()];

                            commands.entity(entity).insert(PolygonGraphics {
                                positions: all_positions,
                                indices: all_indices,
                                colors,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// 从要素的 `tags`（交替的 key/value 索引）与图层的键值表还原为
/// 字符串哈希表。
///
/// # 返回
/// 属性名→字符串值的映射（无法解析的成对索引会被跳过）。
fn extract_properties(
    feature: &MvtFeature,
    keys: &[String],
    values: &[MvtValue],
) -> std::collections::HashMap<String, String> {
    let mut props = std::collections::HashMap::new();
    let mut i = 0;
    while i + 1 < feature.tags.len() {
        let key_idx = feature.tags[i] as usize;
        let val_idx = feature.tags[i + 1] as usize;
        if let (Some(key), Some(val)) = (keys.get(key_idx), values.get(val_idx)) {
            props.insert(key.clone(), mvt_value_to_string(val));
        }
        i += 2;
    }
    props
}

/// 把任意 `MvtValue` 变体格式化为字符串（数值走 `Display`）。
fn mvt_value_to_string(value: &MvtValue) -> String {
    match value {
        MvtValue::String(s) => s.clone(),
        MvtValue::Float(f) => format!("{}", f),
        MvtValue::Double(d) => format!("{}", d),
        MvtValue::Int(i) => format!("{}", i),
        MvtValue::Uint(u) => format!("{}", u),
        MvtValue::Sint(i) => format!("{}", i),
        MvtValue::Bool(b) => format!("{}", b),
    }
}

/// 解析逗号分隔的 rgba/rbg 颜串；不足三分量时返回 `None`，
/// 缺省 alpha 为 1.0。
fn parse_color(color_str: &str) -> Option<[f64; 4]> {
    let parts: Vec<f64> = color_str
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    if parts.len() >= 3 {
        let a = parts.get(3).copied().unwrap_or(1.0);
        Some([parts[0], parts[1], parts[2], a])
    } else {
        None
    }
}

/// 贴地系统：当配置开启 `clamp_to_ground` 时，把点/线/面所有
/// 顶点高度归零；否则直接返回。
///
/// # 参数
/// - `config`：总开关与贴地开关来源
/// - `point_query`/`polyline_query`/`polygon_query`：需修改高度的三类几何
pub fn clamp_to_ground_system(
    config: Res<VectorTileConfig>,
    mut point_query: Query<&mut PointGraphics>,
    mut polyline_query: Query<&mut PolylineGraphics>,
    mut polygon_query: Query<&mut PolygonGraphics>,
) {
    if !config.enabled || !config.clamp_to_ground {
        return;
    }

    for mut points in point_query.iter_mut() {
        for pos in &mut points.positions {
            // 把局部坐标的垂直分量强制归零。
            pos.z = 0.0;
        }
    }

    for mut polylines in polyline_query.iter_mut() {
        // 线：同样逐顶点归零高度。
        for pos in &mut polylines.positions {
            pos.z = 0.0;
        }
    }

    for mut polygons in polygon_query.iter_mut() {
        // 面：逐顶点归零高度。
        for pos in &mut polygons.positions {
            pos.z = 0.0;
        }
    }
}

/// 渲染系统：对新增的三类几何分别构造 `Mesh`（PointList/LineList/
/// TriangleList），写入位置与索引并附无光照标准材质。
///
/// # 参数
/// - `_config`：预留（当前渲染不受开关影响）
/// - `point_query`/`polyline_query`/`polygon_query`：各自仅匹配新增的几何
/// - `commands`：给实体回插 `Mesh3d`/`MeshMaterial3d`
/// - `meshes`/`materials`：资产写入器
pub fn render_vector_system(
    _config: Res<VectorTileConfig>,
    point_query: Query<(Entity, &PointGraphics), Added<PointGraphics>>,
    polyline_query: Query<(Entity, &PolylineGraphics), Added<PolylineGraphics>>,
    polygon_query: Query<(Entity, &PolygonGraphics), Added<PolygonGraphics>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, points) in point_query.iter() {
        // 点集拓扑：每个顶点独立渲染为点精灵。
        let mut mesh = Mesh::new(
            bevy::render::mesh::PrimitiveTopology::PointList,
            bevy::render::render_asset::RenderAssetUsages::default(),
        );
        // DVec3→[f32;3]：GPU 顶点属性需单精度浮点。
        let positions: Vec<[f32; 3]> = points
            .positions
            .iter()
            .map(|p| [p.x as f32, p.y as f32, p.z as f32])
            .collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        let handle = meshes.add(mesh);
        // 无光照白材质：颜色将由顶点属性而非光照计算决定。
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgb(1.0, 1.0, 1.0),
            unlit: true,
            ..default()
        });
        // 回插 mesh 与材质，令实体进入渲染管线。
        commands.entity(entity).insert(Mesh3d(handle)).insert(MeshMaterial3d(mat));
    }

    for (entity, polylines) in polyline_query.iter() {
        // 线集拓扑：成对索引构成线段。
        let mut mesh = Mesh::new(
            bevy::render::mesh::PrimitiveTopology::LineList,
            bevy::render::render_asset::RenderAssetUsages::default(),
        );
        let positions: Vec<[f32; 3]> = polylines
            .positions
            .iter()
            .map(|p| [p.x as f32, p.y as f32, p.z as f32])
            .collect();

        // 由每段点数还原为相邻顶点对的索引（offset 累加跨段基准）。
        let mut indices: Vec<u32> = Vec::new();
        let mut offset = 0u32;
        for &count in &polylines.polyline_counts {
            for i in 0..(count.saturating_sub(1)) {
                indices.push(offset + i as u32);
                indices.push(offset + i as u32 + 1);
            }
            offset += count as u32;
        }

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
        let handle = meshes.add(mesh);
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgb(1.0, 1.0, 1.0),
            unlit: true,
            ..default()
        });
        commands.entity(entity).insert(Mesh3d(handle)).insert(MeshMaterial3d(mat));
    }

    for (entity, polygons) in polygon_query.iter() {
        // 三角列表拓扑：索引已在解码阶段预先生成。
        let mut mesh = Mesh::new(
            bevy::render::mesh::PrimitiveTopology::TriangleList,
            bevy::render::render_asset::RenderAssetUsages::default(),
        );
        let positions: Vec<[f32; 3]> = polygons
            .positions
            .iter()
            .map(|p| [p.x as f32, p.y as f32, p.z as f32])
            .collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_indices(bevy::render::mesh::Indices::U32(polygons.indices.clone()));
        let handle = meshes.add(mesh);
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgb(0.8, 0.8, 0.8),
            ..default()
        });
        commands.entity(entity).insert(Mesh3d(handle)).insert(MeshMaterial3d(mat));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 验证配置默认值与设计一致。
    fn test_vector_tile_config_default() {
        let config = VectorTileConfig::default();
        assert!(config.enabled);
        assert!(!config.clamp_to_ground);
        assert_eq!(config.default_point_size, 5.0);
        assert_eq!(config.default_polyline_width, 2.0);
    }

    #[test]
    /// 验证颜色字符串解析（rgb/rgba/非法）。
    fn test_parse_color() {
        assert_eq!(parse_color("1.0,0.5,0.2"), Some([1.0, 0.5, 0.2, 1.0]));
        assert_eq!(parse_color("1.0,0.5,0.2,0.8"), Some([1.0, 0.5, 0.2, 0.8]));
        assert_eq!(parse_color("not a color"), None);
    }

    #[test]
    /// 验证从 tags 还原属性映射。
    fn test_extract_properties() {
        let feat = MvtFeature {
            id: Some(1),
            geometry_type: MvtGeometryType::Point,
            geometry: vec![],
            tags: vec![0, 0, 1, 1],
        };
        let keys = vec!["name".to_string(), "value".to_string()];
        let values = vec![
            MvtValue::String("test".to_string()),
            MvtValue::Float(42.0),
        ];
        let props = extract_properties(&feat, &keys, &values);
        assert_eq!(props.get("name").unwrap(), "test");
        assert_eq!(props.get("value").unwrap(), "42");
    }

    #[test]
    /// 验证单点 MoveTo 命令的几何解码。
    fn test_decode_mvt_geometry_basic() {
        let commands = vec![
            (1 << 3) | 1, // MoveTo, count=1
            24,           // zigzag(12)
            16,           // zigzag(8)
        ];
        let rings = decode_mvt_geometry(&commands, 4096);
        assert_eq!(rings.len(), 1);
        assert_eq!(rings[0].len(), 1);
    }
}
