//! WKT（Well-Known Text）矢量加载器：从待处理队列取出实体的 WKT
//! 字符串，解析为几何后直接构造为 Bevy mesh 并回插到实体。
//!
//! 支持全部 OGC 几何类：Point / LineString / Polygon（含内环）
//! 及各自的多重形式与 GeometryCollection。多边形三角化采用
//! 简单的扇形划分（见 [`triangulate_ring`]），足够用于展示。
//!
//! 几何到 mesh 的映射：点/多点→PointList，线/多线→LineList，
//! 面/多面→TriangleList；颜色为固定展示色，不读入样式。
//! 一个 GeometryCollection 会把各子几何回插到同一实体。
//!
//! 使用方式：向 [`WktLoadQueue::pending`] 推入 (实体, WKT)，
//! 系统会在下一帧处理并发出 [`WktLoaded`]。

use bevy::prelude::*;
use cesium_vector::{parse_wkt, WktGeometry};
use glam::DVec2;

/// 待加载 WKT 的队列资源：每项为 (目标实体, WKT 字符串)。
#[derive(Resource, Debug, Clone, Default)]
pub struct WktLoadQueue {
    /// 待处理项，系统会一次性取空并逐条处理。
    pub pending: Vec<(Entity, String)>,
}

/// WKT 加载完成事件。
#[derive(Event)]
pub struct WktLoaded {
    /// 已完成几何构建的实体。
    pub entity: Entity,
}

/// WKT 插件：注册队列资源、事件与加载系统。
pub struct CesiumWktPlugin;

impl Plugin for CesiumWktPlugin {
    /// 初始化队列与事件，在 `Update` 阶段添加 [`wkt_load_system`]。
    ///
    /// # 参数
    /// - `app`：待配置的 Bevy App
    fn build(&self, app: &mut App) {
        app.init_resource::<WktLoadQueue>()
            .add_event::<WktLoaded>()
            .add_systems(Update, wkt_load_system);
    }
}

/// 加载系统：取空队列，逐条解析 WKT 并为对应实体生成网格。
///
/// # 参数
/// - `queue`：待处理队列（会被取空）
/// - `commands`：实体命令写入器
/// - `meshes`/`materials`：资产写入器
/// - `events`：发送 [`WktLoaded`] 完成事件
pub fn wkt_load_system(
    mut queue: ResMut<WktLoadQueue>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut events: EventWriter<WktLoaded>,
) {
    // 取空队列，避免处理中向同一资源写入。
    let pending = std::mem::take(&mut queue.pending);

    for (entity, wkt_str) in pending {
        if let Ok(geometry) = parse_wkt(&wkt_str) {
            // 解析成功才建几何；失败静默丢弃（不阻断其他项）。
            create_entity_from_wkt(entity, &geometry, &mut commands, &mut meshes, &mut materials);
            events.send(WktLoaded { entity });
        }
    }
}

/// 按几何变体分派，为实体生成对应的 mesh 与材质。
///
/// 颜色约定：点系=青色、线系=黄色、面系=蓝色（均为固定展示色）。
///
/// # 参数
/// - `entity`：目标实体
/// - `geometry`：已解析的 WKT 几何
/// - `commands`/`meshes`/`materials`：写入渲染组件与资产
fn create_entity_from_wkt(
    entity: Entity,
    geometry: &WktGeometry,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    match geometry {
        WktGeometry::Point(p) => {
            // 单点：一个顶点的 PointList。
            let positions = vec![[p.x as f32, p.y as f32, 0.0f32]];
            // 面均位于 z=0 平面（2.5D 展示，不处理高程）。
            let mut mesh = Mesh::new(
                bevy::render::mesh::PrimitiveTopology::PointList,
                bevy::render::render_asset::RenderAssetUsages::default(),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            let handle = meshes.add(mesh);
            let mat = materials.add(StandardMaterial {
                base_color: Color::linear_rgb(0.0, 1.0, 1.0),
                unlit: true,
                ..default()
            });
            commands.entity(entity).insert(Mesh3d(handle)).insert(MeshMaterial3d(mat));
        }
        WktGeometry::LineString(coords) => {
            // 线串：逐相邻顶点成对索引。
            // 经纬度直接当作平面坐标，z 固定为 0。
            let positions: Vec<[f32; 3]> = coords
                .iter()
                .map(|c| [c.x as f32, c.y as f32, 0.0f32])
                .collect();
            let mut indices = Vec::new();
            // 逐相邻顶点生成线段索引（saturating_sub 避免空/单点下溢）。
            for i in 0..(positions.len().saturating_sub(1)) {
                // 每个线段引用相邻两个顶点索引。
                indices.push(i as u32);
                indices.push(i as u32 + 1);
            }
            let mut mesh = Mesh::new(
                bevy::render::mesh::PrimitiveTopology::LineList,
                bevy::render::render_asset::RenderAssetUsages::default(),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
            let handle = meshes.add(mesh);
            let mat = materials.add(StandardMaterial {
                base_color: Color::linear_rgb(1.0, 1.0, 0.0),
                unlit: true,
                ..default()
            });
            commands.entity(entity).insert(Mesh3d(handle)).insert(MeshMaterial3d(mat));
        }
        WktGeometry::Polygon { exterior, interiors } => {
            // 多边形：先外环三角化，再逐内环累加并施以索引偏移。
            // 面系采用受光照蓝色（区别于无光照的点/线）。
            let mut positions: Vec<[f32; 3]> = exterior
                .iter()
                .map(|c| [c.x as f32, c.y as f32, 0.0f32])
                .collect();
            // 外环三角化基于当前顶点数（此时只有外环）。
            let mut indices = triangulate_ring(exterior, positions.len());
            // 把内环顶点拼接到同一位置数组尾部。
            positions.extend(
                interiors
                    .iter()
                    .flat_map(|ring| ring.iter().map(|c| [c.x as f32, c.y as f32, 0.0f32])),
            );
            for (i, ring) in interiors.iter().enumerate() {
                // 内环三角化后需按已累加顶点数重新偏移索引。
                let interior_indices = triangulate_ring(ring, ring.len());
                // offset = 外环长度 + 之前各内环长度之和。
                let offset = exterior.len() as u32
                    + interiors.iter().take(i).map(|r| r.len() as u32).sum::<u32>();
                indices.extend(interior_indices.iter().map(|idx| idx + offset));
            }
            let mut mesh = Mesh::new(
                bevy::render::mesh::PrimitiveTopology::TriangleList,
                bevy::render::render_asset::RenderAssetUsages::default(),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
            let handle = meshes.add(mesh);
            let mat = materials.add(StandardMaterial {
                base_color: Color::linear_rgb(0.0, 0.5, 1.0),
                ..default()
            });
            commands.entity(entity).insert(Mesh3d(handle)).insert(MeshMaterial3d(mat));
        }
        WktGeometry::MultiPoint(points) => {
            // 多点：所有点归入一个 PointList。
            // 复用与单点相同的青色，便于视觉区分维度。
            let positions: Vec<[f32; 3]> = points
                .iter()
                .map(|p| [p.x as f32, p.y as f32, 0.0f32])
                .collect();
            let mut mesh = Mesh::new(
                bevy::render::mesh::PrimitiveTopology::PointList,
                bevy::render::render_asset::RenderAssetUsages::default(),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            let handle = meshes.add(mesh);
            let mat = materials.add(StandardMaterial {
                base_color: Color::linear_rgb(0.0, 1.0, 1.0),
                unlit: true,
                ..default()
            });
            commands.entity(entity).insert(Mesh3d(handle)).insert(MeshMaterial3d(mat));
        }
        WktGeometry::MultiLineString(lines) => {
            // 多线：逐线展平顶点，offset 记录每条线在共享数组中的基准。
            // 与单线区别在于需跨线累加索引偏移。
            let mut positions: Vec<[f32; 3]> = Vec::new();
            let mut indices: Vec<u32> = Vec::new();
            let mut offset = 0u32;
            for line in lines {
                // 先把本线所有顶点入数组，再成对生成段索引。
                for coord in line {
                    positions.push([coord.x as f32, coord.y as f32, 0.0f32]);
                }
                for i in 0..(line.len().saturating_sub(1)) {
                    // 全局索引 = 本线基准 offset + 线内局部索引。
                    indices.push(offset + i as u32);
                    indices.push(offset + i as u32 + 1);
                }
                offset += line.len() as u32;
            }
            let mut mesh = Mesh::new(
                bevy::render::mesh::PrimitiveTopology::LineList,
                bevy::render::render_asset::RenderAssetUsages::default(),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
            let handle = meshes.add(mesh);
            let mat = materials.add(StandardMaterial {
                base_color: Color::linear_rgb(1.0, 1.0, 0.0),
                unlit: true,
                ..default()
            });
            commands.entity(entity).insert(Mesh3d(handle)).insert(MeshMaterial3d(mat));
        }
        WktGeometry::MultiPolygon(polygons) => {
            // 多面：逐子面累加，统一维护全局顶点基准与内环偏移。
            // 与单面区别在于需跨子面累加位置与索引基准。
            let mut all_positions: Vec<[f32; 3]> = Vec::new();
            let mut all_indices: Vec<u32> = Vec::new();
            for polygon in polygons {
                // 本子面在全局位置数组中的起始基准。
                let offset = all_positions.len() as u32;
                if let WktGeometry::Polygon {
                    ref exterior,
                    ref interiors,
                } = polygon
                {
                    all_positions.extend(
                        exterior.iter().map(|c| [c.x as f32, c.y as f32, 0.0f32]),
                    );
                    let tri = triangulate_ring(exterior, exterior.len());
                    all_indices.extend(tri.iter().map(|idx| idx + offset));

                    // 内环基准：偏移至本子面外环尾部之后。
                    let hole_offset = exterior.len() as u32;
                    for (i, ring) in interiors.iter().enumerate() {
                        // 内环全局基准 = 子面基准 + 外环长 + 前面内环长之和。
                        let off = offset
                            + hole_offset
                            + interiors.iter().take(i).map(|r| r.len() as u32).sum::<u32>();
                        all_positions
                            .extend(ring.iter().map(|c| [c.x as f32, c.y as f32, 0.0f32]));
                        let hole_tri = triangulate_ring(ring, ring.len());
                        all_indices.extend(hole_tri.iter().map(|idx| idx + off));
                    }
                }
            }
            let mut mesh = Mesh::new(
                bevy::render::mesh::PrimitiveTopology::TriangleList,
                bevy::render::render_asset::RenderAssetUsages::default(),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, all_positions);
            mesh.insert_indices(bevy::render::mesh::Indices::U32(all_indices));
            let handle = meshes.add(mesh);
            let mat = materials.add(StandardMaterial {
                base_color: Color::linear_rgb(0.0, 0.5, 1.0),
                ..default()
            });
            commands.entity(entity).insert(Mesh3d(handle)).insert(MeshMaterial3d(mat));
        }
        WktGeometry::GeometryCollection(geoms) => {
            // 几何集合：递归处理每个子几何（共享同一实体）。
            for geom in geoms {
                // 递归分派：每个子几何各自插入一份 mesh/材质。
                create_entity_from_wkt(entity, geom, commands, meshes, materials);
            }
        }
    }
}

/// 对闭合环作简单扇形三角化：以顶点 0 为公共顶点与相邻两点组三角。
///
/// # 参数
/// - `ring`：环顶点（假定为凸多边形，不适用于凹多边形）
/// - `_ring_len`：保留参数（当前未使用）
///
/// # 返回
/// 三角形索引列表；顶点不足 3 时返回空。
fn triangulate_ring(ring: &[DVec2], _ring_len: usize) -> Vec<u32> {
    if ring.len() < 3 {
        return Vec::new();
    }
    // 从第二个顶点起，每步与首点构成一个三角形。
    let mut indices = Vec::new();
    for i in 1..(ring.len() - 1) {
        indices.push(0);
        indices.push(i as u32);
        indices.push(i as u32 + 1);
    }
    indices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 验证 POINT 解析为带坐标的点几何。
    fn test_wkt_point_to_entity() {
        let geom = parse_wkt("POINT (10 20)").unwrap();
        assert!(matches!(geom, WktGeometry::Point(_)));
        if let WktGeometry::Point(p) = &geom {
            assert_eq!(p.x, 10.0);
            assert_eq!(p.y, 20.0);
        }
    }

    #[test]
    /// 验证 LINESTRING 解析为线几何。
    fn test_wkt_linestring_to_entity() {
        let geom = parse_wkt("LINESTRING (0 0, 10 10, 20 0)").unwrap();
        assert!(matches!(geom, WktGeometry::LineString(_)));
    }

    #[test]
    /// 验证 POLYGON 解析为面几何。
    fn test_wkt_polygon_to_entity() {
        let geom = parse_wkt("POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0))").unwrap();
        assert!(matches!(geom, WktGeometry::Polygon { .. }));
    }

    #[test]
    /// 验证 MULTIPOINT 包含预期点数。
    fn test_wkt_multipoint_to_entity() {
        let geom = parse_wkt("MULTIPOINT ((0 0), (10 10), (20 20))").unwrap();
        match geom {
            WktGeometry::MultiPoint(points) => assert_eq!(points.len(), 3),
            _ => panic!("Expected MultiPoint"),
        }
    }

    #[test]
    /// 验证 MULTILINESTRING 包含预期线数。
    fn test_wkt_multilinestring_to_entity() {
        let geom =
            parse_wkt("MULTILINESTRING ((0 0, 10 10), (20 20, 30 30))").unwrap();
        match geom {
            WktGeometry::MultiLineString(lines) => assert_eq!(lines.len(), 2),
            _ => panic!("Expected MultiLineString"),
        }
    }

    #[test]
    /// 验证 GEOMETRYCOLLECTION 包含预期子几何数。
    fn test_wkt_geometry_collection() {
        let geom =
            parse_wkt("GEOMETRYCOLLECTION (POINT (4 6), LINESTRING (4 6, 7 10))").unwrap();
        match geom {
            WktGeometry::GeometryCollection(geoms) => assert_eq!(geoms.len(), 2),
            _ => panic!("Expected GeometryCollection"),
        }
    }

    #[test]
    /// 验证三角形环三角化恰为自身三索引。
    fn test_triangulate_ring_triangle() {
        let ring = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(0.0, 1.0),
        ];
        let indices = triangulate_ring(&ring, 3);
        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    /// 验证四边形环三角化为两个三角形（6 索引）。
    fn test_triangulate_ring_quad() {
        let ring = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(1.0, 1.0),
            DVec2::new(0.0, 1.0),
        ];
        let indices = triangulate_ring(&ring, 4);
        assert_eq!(indices.len(), 6);
    }
}
