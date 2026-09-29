//! 实体可视化渲染。
//!
//! 将领域 Entity 图形转换为 Bevy mesh 与材质。
//! 对应 CesiumJS `DataSources/GeometryVisualizer.js`

use bevy::prelude::*;
use cesium_datasource::entity::{Entity, PolygonGraphics, PolylineGraphics};
use cesium_datasource::property::{Color, Property};
use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;

/// 标记一个实体可视化的组件。
#[derive(Component)]
pub struct EntityVisual {
    /// 此可视化所表示的实体 ID。
    pub entity_id: String,
}

/// 将领域 Color 转换为 Bevy Color。
pub fn domain_color_to_bevy(color: &Color) -> bevy::prelude::Color {
    bevy::prelude::Color::srgba(
        color.red as f32,
        color.green as f32,
        color.blue as f32,
        color.alpha as f32,
    )
}

/// 在时刻 0 解析一个颜色属性。
fn resolve_color(prop: &Property<Color>, default: Color) -> Color {
    prop.get_value(0.0).copied().unwrap_or(default)
}

/// 从椭球上的位置创建一条 polyline mesh。
///
/// 沿线以给定宽度生成一条 triangle strip。
pub fn create_polyline_mesh(
    polyline: &PolylineGraphics,
    ellipsoid: &Ellipsoid,
    time: f64,
) -> Option<Mesh> {
    let positions = polyline.positions.get_value(time)?;
    if positions.len() < 2 {
        return None;
    }

    let width = polyline.width.get_value(time).copied().unwrap_or(1.0);
    // 将像素宽度转换为近似的世界宽度（粗略启发式）
    let world_width = width * 1000.0; // 中等缩放下的近似米/像素

    // 将测绘学位置转换为 ECEF
    let ecef_points: Vec<glam::DVec3> = positions
        .iter()
        .map(|p| {
            let carto = Cartographic::from_radians(p[0], p[1], p[2]);
            ellipsoid.cartographic_to_cartesian(&carto)
        })
        .collect();

    // 沿线生成一条扁平带状体（triangle strip）
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for i in 0..ecef_points.len() {
        let point = ecef_points[i];
        let normal = point.normalize(); // 表面法线（近似）

        // 计算切线方向
        let tangent = if i < ecef_points.len() - 1 {
            (ecef_points[i + 1] - point).normalize()
        } else {
            (point - ecef_points[i - 1]).normalize()
        };

        // 侧向量（垂直于切线与法线）
        let side = tangent.cross(normal).normalize();

        // 每点两个顶点（中心的左与右）
        let half_width = world_width / 2.0;
        let left = point + side * half_width;
        let right = point - side * half_width;

        vertices.push([left.x as f32, left.y as f32, left.z as f32]);
        vertices.push([right.x as f32, right.y as f32, right.z as f32]);

        let n = [normal.x as f32, normal.y as f32, normal.z as f32];
        normals.push(n);
        normals.push(n);

        // 生成三角形索引
        if i < ecef_points.len() - 1 {
            let base = (i * 2) as u32;
            indices.extend_from_slice(&[base, base + 1, base + 2]);
            indices.extend_from_slice(&[base + 1, base + 3, base + 2]);
        }
    }

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    Some(mesh)
}

/// 从椭球上的位置创建一个 polygon mesh。
///
/// 对凸多边形使用简单的扇形三角剖分。
pub fn create_polygon_mesh(
    polygon: &PolygonGraphics,
    ellipsoid: &Ellipsoid,
    time: f64,
) -> Option<Mesh> {
    let positions = polygon.positions.get_value(time)?;
    if positions.len() < 3 {
        return None;
    }

    let height = polygon.height.get_value(time).copied().unwrap_or(0.0);

    // 转换为 ECEF
    let ecef_points: Vec<glam::DVec3> = positions
        .iter()
        .map(|p| {
            let carto = Cartographic::from_radians(p[0], p[1], height);
            ellipsoid.cartographic_to_cartesian(&carto)
        })
        .collect();

    // 为扇形三角剖分计算质心
    let centroid = ecef_points.iter().fold(glam::DVec3::ZERO, |acc, p| acc + *p)
        / ecef_points.len() as f64;
    let centroid_normal = centroid.normalize();

    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // 添加质心顶点
    vertices.push([centroid.x as f32, centroid.y as f32, centroid.z as f32]);
    normals.push([centroid_normal.x as f32, centroid_normal.y as f32, centroid_normal.z as f32]);

    // 添加环顶点
    for point in &ecef_points {
        vertices.push([point.x as f32, point.y as f32, point.z as f32]);
        let n = point.normalize();
        normals.push([n.x as f32, n.y as f32, n.z as f32]);
    }

    // 从质心做扇形三角剖分
    let n_points = ecef_points.len();
    for i in 0..n_points {
        let next = (i + 1) % n_points;
        indices.extend_from_slice(&[0, (i + 1) as u32, (next + 1) as u32]);
    }

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    Some(mesh)
}

/// 从实体图形创建一个 Bevy 材质。
pub fn create_entity_material(entity: &Entity, _time: f64) -> StandardMaterial {
    // 尝试从不同图形类型获取颜色
    let color = if let Some(ref point) = entity.point {
        resolve_color(&point.color, Color::WHITE)
    } else if let Some(ref polyline) = entity.polyline {
        resolve_color(&polyline.color, Color::WHITE)
    } else if let Some(ref polygon) = entity.polygon {
        resolve_color(&polygon.material, Color::WHITE)
    } else {
        Color::WHITE
    };

    StandardMaterial {
        base_color: domain_color_to_bevy(&color),
        ..default()
    }
}

/// 将实体的位置转换为椭球上的一个 Bevy Transform。
pub fn entity_position_to_transform(
    entity: &Entity,
    ellipsoid: &Ellipsoid,
    time: f64,
) -> Option<Transform> {
    let pos = entity.position.get_value(time)?;
    let carto = Cartographic::from_radians(pos[0], pos[1], pos[2]);
    let ecef = ellipsoid.cartographic_to_cartesian(&carto);

    Some(Transform::from_xyz(ecef.x as f32, ecef.y as f32, ecef.z as f32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_datasource::entity::{PointGraphics, PolylineGraphics, PolygonGraphics};
    use cesium_datasource::property::Property;

    #[test]
    fn test_domain_color_to_bevy() {
        let color = Color::new(1.0, 0.5, 0.25, 1.0);
        let bevy_color = domain_color_to_bevy(&color);
        if let bevy::prelude::Color::Srgba(srgba) = bevy_color {
            assert!((srgba.red - 1.0).abs() < 1e-5);
            assert!((srgba.green - 0.5).abs() < 1e-5);
            assert!((srgba.blue - 0.25).abs() < 1e-5);
        } else {
            panic!("Expected Srgba color");
        }
    }

    #[test]
    fn test_create_polyline_mesh() {
        let polyline = PolylineGraphics {
            positions: Property::Constant(vec![
                [0.0, 0.0, 0.0],
                [0.01, 0.0, 0.0],
                [0.02, 0.0, 0.0],
            ]),
            width: Property::Constant(2.0),
            ..Default::default()
        };

        let mesh = create_polyline_mesh(&polyline, &Ellipsoid::WGS84, 0.0);
        assert!(mesh.is_some());

        let mesh = mesh.unwrap();
        assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
    }

    #[test]
    fn test_create_polygon_mesh() {
        let polygon = PolygonGraphics {
            positions: Property::Constant(vec![
                [0.0, 0.0, 0.0],
                [0.01, 0.0, 0.0],
                [0.01, 0.01, 0.0],
                [0.0, 0.01, 0.0],
            ]),
            ..Default::default()
        };

        let mesh = create_polygon_mesh(&polygon, &Ellipsoid::WGS84, 0.0);
        assert!(mesh.is_some());

        let mesh = mesh.unwrap();
        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        if let bevy::render::mesh::VertexAttributeValues::Float32x3(pos) = positions {
            // 质心 + 4 个环顶点 = 5
            assert_eq!(pos.len(), 5);
        }
    }

    #[test]
    fn test_create_entity_material() {
        let entity = Entity::new("test")
            .with_point(PointGraphics {
                color: Property::Constant(Color::RED),
                ..Default::default()
            });

        let material = create_entity_material(&entity, 0.0);
        if let bevy::prelude::Color::Srgba(srgba) = material.base_color {
            assert!((srgba.red - 1.0).abs() < 1e-5);
            assert!((srgba.green - 0.0).abs() < 1e-5);
        }
    }

    #[test]
    fn test_entity_position_to_transform() {
        let entity = Entity::new("test")
            .with_position(0.0, 0.0, 0.0); // lon=0, lat=0, h=0

        let transform = entity_position_to_transform(&entity, &Ellipsoid::WGS84, 0.0);
        assert!(transform.is_some());

        let t = transform.unwrap();
        // 在 lon=0, lat=0 处，位置应位于 X 轴上（约 6378137m）
        assert!(t.translation.x > 6_000_000.0);
        assert!(t.translation.y.abs() < 1.0);
        assert!(t.translation.z.abs() < 1.0);
    }

    #[test]
    fn test_polyline_too_few_points() {
        let polyline = PolylineGraphics {
            positions: Property::Constant(vec![[0.0, 0.0, 0.0]]),
            ..Default::default()
        };

        let mesh = create_polyline_mesh(&polyline, &Ellipsoid::WGS84, 0.0);
        assert!(mesh.is_none());
    }
}
