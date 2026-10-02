//! 3D Tiles 包围体定义。
//!
//! 支持三种类型：Box（定向包围盒 OBB）、Region（地理区域）和 Sphere（包围球），
//! 提供中心、转换包围球与点到体距离等几何计算。

use cesium_geospatial::bounding::BoundingSphere;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::rectangle::Rectangle;
use glam::DVec3;
use serde::{Deserialize, Serialize};

/// 一个 3D Tile 的包围体。
///
/// 映射到 tileset.json 中的 `boundingVolume` 属性。
/// 根据 3D Tiles 规范支持三种类型：
/// - `box`：定向包围盒（中心 + 3 个半轴向量）
/// - `region`：地理区域 [west, south, east, north, minHeight, maxHeight]
/// - `sphere`：包围球 [centerX, centerY, centerZ, radius]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BoundingVolume {
    /// 定向包围盒：[cx, cy, cz, xDirX, xDirY, xDirZ, yDirX, yDirY, yDirZ, zDirX, zDirY, zDirZ]
    Box([f64; 12]),
    /// 地理区域：[west, south, east, north, minHeight, maxHeight]（弧度 + 米）
    Region([f64; 6]),
    /// 包围球：[centerX, centerY, centerY, radius]
    Sphere([f64; 4]),
}

impl BoundingVolume {
    /// 由中心和半轴向量创建包围盒。
    pub fn from_box(center: DVec3, half_x: DVec3, half_y: DVec3, half_z: DVec3) -> Self {
        // 按 [中心 3 + 半轴 x/y/z 各 3] 顺序展平为 12 元组
        BoundingVolume::Box([
            center.x, center.y, center.z,
            half_x.x, half_x.y, half_x.z,
            half_y.x, half_y.y, half_y.z,
            half_z.x, half_z.y, half_z.z,
        ])
    }

    /// 由中心和半径创建包围球。
    pub fn from_sphere(center: DVec3, radius: f64) -> Self {
        // 包围球存为 [cx, cy, cz, radius]
        BoundingVolume::Sphere([center.x, center.y, center.z, radius])
    }

    /// 创建一个地理区域包围体。
    pub fn from_region(west: f64, south: f64, east: f64, north: f64, min_height: f64, max_height: f64) -> Self {
        // region 存为 [west, south, east, north, minH, maxH]（弧度+米）
        BoundingVolume::Region([west, south, east, north, min_height, max_height])
    }

    /// 获取包围体在 ECEF 坐标系中的中心。
    pub fn center(&self, ellipsoid: &Ellipsoid) -> DVec3 {
        // Box/Sphere 直接取前三分量作中心；Region 需由经纬高投影
        match self {
            BoundingVolume::Box(data) => DVec3::new(data[0], data[1], data[2]),
            BoundingVolume::Sphere(data) => DVec3::new(data[0], data[1], data[2]),
            BoundingVolume::Region(data) => {
                // region 中心取经纬与高度的中点再投影回 ECEF
                let lon = (data[0] + data[2]) / 2.0;
                let lat = (data[1] + data[3]) / 2.0;
                let height = (data[4] + data[5]) / 2.0;
                ellipsoid.cartographic_to_cartesian(
                    &cesium_geospatial::cartographic::Cartographic::from_radians(lon, lat, height),
                )
            }
        }
    }

    /// 将该包围体转换为 BoundingSphere，用于距离计算。
    pub fn to_bounding_sphere(&self, ellipsoid: &Ellipsoid) -> BoundingSphere {
        match self {
            BoundingVolume::Sphere(data) => {
                // 球体本身就是包围球，直接取中心与半径
                BoundingSphere::new(DVec3::new(data[0], data[1], data[2]), data[3])
            }
            BoundingVolume::Box(data) => {
                // 盒转球：中心取前三项，三组半轴向量取后续九项
                let center = DVec3::new(data[0], data[1], data[2]);
                let half_x = DVec3::new(data[3], data[4], data[5]);
                let half_y = DVec3::new(data[6], data[7], data[8]);
                let half_z = DVec3::new(data[9], data[10], data[11]);
                // 半径是最长对角线的长度
                let radius = (half_x.length_squared()
                    + half_y.length_squared()
                    + half_z.length_squared())
                .sqrt();
                BoundingSphere::new(center, radius)
            }
            BoundingVolume::Region(data) => {
                // region 转球：先取经纬矩形与高度范围再用球近似
                let rect = Rectangle::new(data[0], data[1], data[2], data[3]);
                let min_h = data[4];
                let max_h = data[5];
                // 用球体近似
                let center_carto = cesium_geospatial::cartographic::Cartographic::from_radians(
                    (rect.west + rect.east) / 2.0,
                    (rect.south + rect.north) / 2.0,
                    (min_h + max_h) / 2.0,
                );
                let center = ellipsoid.cartographic_to_cartesian(&center_carto);

                // 由角点距离计算半径
                let sw = ellipsoid.cartographic_to_cartesian(
                    &cesium_geospatial::cartographic::Cartographic::from_radians(
                        rect.west, rect.south, min_h,
                    ),
                );
                let ne = ellipsoid.cartographic_to_cartesian(
                    &cesium_geospatial::cartographic::Cartographic::from_radians(
                        rect.east, rect.north, max_h,
                    ),
                );
                let radius = center.distance(sw).max(center.distance(ne));
                BoundingSphere::new(center, radius)
            }
        }
    }

    /// 计算一个点到包围体的距离。
    ///
    /// 若点位于体内则返回 0。
    pub fn distance_to(&self, point: DVec3, ellipsoid: &Ellipsoid) -> f64 {
        match self {
            BoundingVolume::Sphere(data) => {
                // 球面距离：点距减半径，体内为负时钳 0
                let center = DVec3::new(data[0], data[1], data[2]);
                let radius = data[3];
                (point.distance(center) - radius).max(0.0)
            }
            BoundingVolume::Box(data) => {
                // Box：从 12 元组拆出中心与前三个半轴向量
                let center = DVec3::new(data[0], data[1], data[2]);
                let half_x = DVec3::new(data[3], data[4], data[5]);
                let half_y = DVec3::new(data[6], data[7], data[8]);
                let half_z = DVec3::new(data[9], data[10], data[11]);

                // 将点变换到 box 局部坐标系
                // 先求点相对中心的偏移，再投影到各归一化半轴方向
                let offset = point - center;
                let dx = offset.dot(half_x.normalize_or_zero());
                let dy = offset.dot(half_y.normalize_or_zero());
                let dz = offset.dot(half_z.normalize_or_zero());

                let ex = (dx.abs() - half_x.length()).max(0.0);
                let ey = (dy.abs() - half_y.length()).max(0.0);
                let ez = (dz.abs() - half_z.length()).max(0.0);

                // 各轴超出半长的分量按勾股合成外部距离
                (ex * ex + ey * ey + ez * ez).sqrt()
            }
            BoundingVolume::Region(_) => {
                // 对 region 使用包围球近似
                // 先转为包围球再算球面距离，作为区域距离的保守估计
                let sphere = self.to_bounding_sphere(ellipsoid);
                (point.distance(sphere.center) - sphere.radius).max(0.0)
            }
        }
    }

    /// 若这是 region 体则获取其地理矩形。
    pub fn as_region(&self) -> Option<Rectangle> {
        // 仅 Region 变体可回取地理矩形，其余返 None
        match self {
            BoundingVolume::Region(data) => {
                Some(Rectangle::new(data[0], data[1], data[2], data[3]))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 验证从中心与半径构造包围球的展平存储。
    fn test_bounding_sphere_creation() {
        let bv = BoundingVolume::from_sphere(DVec3::new(1.0, 2.0, 3.0), 10.0);
        assert_eq!(bv, BoundingVolume::Sphere([1.0, 2.0, 3.0, 10.0]));
    }

    #[test]
    /// 验证包围盒按中心+半轴顺序填充 12 元组。
    fn test_bounding_box_creation() {
        let bv = BoundingVolume::from_box(
            DVec3::ZERO,
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
            DVec3::new(0.0, 0.0, 1.0),
        );
        if let BoundingVolume::Box(data) = bv {
            assert_eq!(data[0], 0.0); // 中心 x
            assert_eq!(data[3], 1.0); // half_x x
        } else {
            panic!("Expected Box variant");
        }
    }

    #[test]
    /// 验证包围球中心直接取前三分量。
    fn test_sphere_center() {
        let bv = BoundingVolume::from_sphere(DVec3::new(100.0, 200.0, 300.0), 50.0);
        let center = bv.center(&Ellipsoid::WGS84);
        assert!((center.x - 100.0).abs() < 1e-10);
        assert!((center.y - 200.0).abs() < 1e-10);
        assert!((center.z - 300.0).abs() < 1e-10);
    }

    #[test]
    /// 验证球外点到球面的距离为点距减半径。
    fn test_sphere_distance() {
        let bv = BoundingVolume::from_sphere(DVec3::ZERO, 10.0);
        let point = DVec3::new(20.0, 0.0, 0.0);
        let dist = bv.distance_to(point, &Ellipsoid::WGS84);
        assert!((dist - 10.0).abs() < 1e-10);
    }

    #[test]
    /// 验证球内点距离钳制为 0。
    fn test_sphere_distance_inside() {
        let bv = BoundingVolume::from_sphere(DVec3::ZERO, 10.0);
        let point = DVec3::new(5.0, 0.0, 0.0);
        let dist = bv.distance_to(point, &Ellipsoid::WGS84);
        assert!((dist - 0.0).abs() < 1e-10);
    }

    #[test]
    /// 验证轴向盒外点的距离计算。
    fn test_box_distance() {
        let bv = BoundingVolume::from_box(
            DVec3::ZERO,
            DVec3::new(5.0, 0.0, 0.0),
            DVec3::new(0.0, 5.0, 0.0),
            DVec3::new(0.0, 0.0, 5.0),
        );
        // X 轴上方的外部点
        let point = DVec3::new(10.0, 0.0, 0.0);
        let dist = bv.distance_to(point, &Ellipsoid::WGS84);
        assert!((dist - 5.0).abs() < 1e-10);
    }

    #[test]
    /// 验证盒转包围球半径为半轴平方和的开方。
    fn test_to_bounding_sphere_from_box() {
        let bv = BoundingVolume::from_box(
            DVec3::ZERO,
            DVec3::new(3.0, 0.0, 0.0),
            DVec3::new(0.0, 4.0, 0.0),
            DVec3::new(0.0, 0.0, 0.0),
        );
        let sphere = bv.to_bounding_sphere(&Ellipsoid::WGS84);
        assert!((sphere.radius - 5.0).abs() < 1e-10); // sqrt(9 + 16) = 5
    }

    #[test]
    /// 验证 region 体可回取为地理矩形。
    fn test_region_as_rectangle() {
        let bv = BoundingVolume::from_region(-1.0, -0.5, 1.0, 0.5, 0.0, 100.0);
        let rect = bv.as_region().unwrap();
        assert!((rect.west - (-1.0)).abs() < 1e-10);
        assert!((rect.east - 1.0).abs() < 1e-10);
    }

    #[test]
    /// 验证包围体的 serde 序列化往返一致。
    fn test_serde_roundtrip() {
        let bv = BoundingVolume::from_sphere(DVec3::new(1.0, 2.0, 3.0), 10.0);
        let json = serde_json::to_string(&bv).unwrap();
        let parsed: BoundingVolume = serde_json::from_str(&json).unwrap();
        assert_eq!(bv, parsed);
    }
}
