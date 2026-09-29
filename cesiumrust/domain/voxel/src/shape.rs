//! 体素形状特质与形状类型枚举。
//!
//! 映射到 CesiumJS `Scene/VoxelShape.js` 与 `Scene/VoxelShapeType.js`。

use glam::{DMat4, DVec3};
use serde::{Deserialize, Serialize};

/// 3D 空间中的有向包围盒。
#[derive(Debug, Clone, PartialEq)]
pub struct OrientedBoundingBox {
    /// 包围盒的中心。
    pub center: DVec3,
    /// 半轴作为 3x3 矩阵的列（存储为 DMat4 的左上部分）。
    pub half_axes: glam::DMat3,
}

impl Default for OrientedBoundingBox {
    fn default() -> Self {
        Self {
            center: DVec3::ZERO,
            half_axes: glam::DMat3::IDENTITY,
        }
    }
}

impl OrientedBoundingBox {
    /// 由中心和半轴创建新的 OBB。
    pub fn new(center: DVec3, half_axes: glam::DMat3) -> Self {
        Self { center, half_axes }
    }

    /// 由半轴计算包围球半径。
    pub fn bounding_sphere_radius(&self) -> f64 {
        let col0 = self.half_axes.col(0);
        let col1 = self.half_axes.col(1);
        let col2 = self.half_axes.col(2);
        (col0.length_squared() + col1.length_squared() + col2.length_squared()).sqrt()
    }

    /// 测试点是否位于 OBB 内部。
    pub fn contains(&self, point: DVec3) -> bool {
        let offset = point - self.center;
        // 投影到每个轴
        for i in 0..3 {
            let axis = self.half_axes.col(i);
            let half_len = axis.length();
            if half_len < 1e-15 {
                continue;
            }
            let dir = axis / half_len;
            let proj = offset.dot(dir);
            if proj.abs() > half_len {
                return false;
            }
        }
        true
    }

    /// 计算点到 OBB 表面的距离（若在内部则为 0）。
    pub fn distance_to(&self, point: DVec3) -> f64 {
        let offset = point - self.center;
        let mut dist_sq = 0.0;
        for i in 0..3 {
            let axis = self.half_axes.col(i);
            let half_len = axis.length();
            if half_len < 1e-15 {
                continue;
            }
            let dir = axis / half_len;
            let proj = offset.dot(dir);
            let excess = proj.abs() - half_len;
            if excess > 0.0 {
                dist_sq += excess * excess;
            }
        }
        dist_sq.sqrt()
    }
}

/// 3D 空间中的包围球。
#[derive(Debug, Clone, PartialEq)]
pub struct BoundingSphere {
    /// 球心。
    pub center: DVec3,
    /// 球的半径。
    pub radius: f64,
}

impl Default for BoundingSphere {
    fn default() -> Self {
        Self {
            center: DVec3::ZERO,
            radius: 0.0,
        }
    }
}

impl BoundingSphere {
    /// 由有向包围盒创建。
    pub fn from_obb(obb: &OrientedBoundingBox) -> Self {
        Self {
            center: obb.center,
            radius: obb.bounding_sphere_radius(),
        }
    }

    /// 测试点是否位于球内部。
    pub fn contains(&self, point: DVec3) -> bool {
        (point - self.center).length() <= self.radius
    }

    /// 计算点到球表面的距离。
    pub fn distance_to(&self, point: DVec3) -> f64 {
        ((point - self.center).length() - self.radius).max(0.0)
    }
}

/// 体素形状的类型，控制体素网格如何映射到 3D 空间。
///
/// 映射到 CesiumJS `VoxelShapeType`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VoxelShapeType {
    /// 边界位于 [-1, 1]^3 的长方体形状。
    Box,
    /// 边界位于 [lon, lat, height] 的椭球体形状。
    Ellipsoid,
    /// 边界位于 [radius, angle, height] 的圆柱体形状。
    Cylinder,
}

impl VoxelShapeType {
    /// 获取此形状类型的默认最小边界。
    pub fn default_min_bounds(&self) -> DVec3 {
        match self {
            Self::Box => DVec3::new(-1.0, -1.0, -1.0),
            Self::Ellipsoid => DVec3::new(-std::f64::consts::PI, -std::f64::consts::FRAC_PI_2, -1.0),
            Self::Cylinder => DVec3::new(0.0, -std::f64::consts::PI, -1.0),
        }
    }

    /// 获取此形状类型的默认最大边界。
    pub fn default_max_bounds(&self) -> DVec3 {
        match self {
            Self::Box => DVec3::new(1.0, 1.0, 1.0),
            Self::Ellipsoid => DVec3::new(std::f64::consts::PI, std::f64::consts::FRAC_PI_2, 1.0),
            Self::Cylinder => DVec3::new(1.0, std::f64::consts::PI, 1.0),
        }
    }
}

/// 体素形状的特质，控制体素网格的剔除与渲染。
///
/// 映射到 CesiumJS `VoxelShape` 接口。
pub trait VoxelShape {
    /// 获取包含有界形状的有向包围盒。
    fn oriented_bounding_box(&self) -> &OrientedBoundingBox;

    /// 获取包含有界形状的包围球。
    fn bounding_sphere(&self) -> &BoundingSphere;

    /// 获取包含有界形状的变换矩阵。
    fn bound_transform(&self) -> DMat4;

    /// 获取包含形状的变换矩阵，忽略边界。
    fn shape_transform(&self) -> DMat4;

    /// 获取任意方向上射线-形状相交的最大数量。
    fn maximum_intersections_length(&self) -> u32;

    /// 更新形状状态。返回形状是否可见。
    fn update(
        &mut self,
        model_matrix: DMat4,
        min_bounds: DVec3,
        max_bounds: DVec3,
        clip_min_bounds: Option<DVec3>,
        clip_max_bounds: Option<DVec3>,
    ) -> bool;

    /// 将局部坐标转换为形状的 UV 空间。
    fn convert_local_to_shape_uv_space(&self, position_local: DVec3) -> DVec3;

    /// 为指定瓦片计算有向包围盒。
    fn compute_obb_for_tile(
        &self,
        tile_level: u32,
        tile_x: u32,
        tile_y: u32,
        tile_z: u32,
    ) -> OrientedBoundingBox;
}

/// 线性插值。
#[inline]
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// 在 min 与 max 向量之间逐分量钳制值。
#[inline]
pub fn clamp_vec3(v: DVec3, min: DVec3, max: DVec3) -> DVec3 {
    DVec3::new(
        v.x.clamp(min.x, max.x),
        v.y.clamp(min.y, max.y),
        v.z.clamp(min.z, max.z),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shape_type_default_bounds() {
        let box_min = VoxelShapeType::Box.default_min_bounds();
        let box_max = VoxelShapeType::Box.default_max_bounds();
        assert_eq!(box_min, DVec3::new(-1.0, -1.0, -1.0));
        assert_eq!(box_max, DVec3::new(1.0, 1.0, 1.0));

        let cyl_min = VoxelShapeType::Cylinder.default_min_bounds();
        let cyl_max = VoxelShapeType::Cylinder.default_max_bounds();
        assert_eq!(cyl_min.x, 0.0);
        assert!((cyl_min.y + std::f64::consts::PI).abs() < 1e-10);
        assert_eq!(cyl_max.x, 1.0);
        assert!((cyl_max.y - std::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn test_obb_contains() {
        let obb = OrientedBoundingBox::new(
            DVec3::ZERO,
            glam::DMat3::from_cols(
                DVec3::new(2.0, 0.0, 0.0),
                DVec3::new(0.0, 2.0, 0.0),
                DVec3::new(0.0, 0.0, 2.0),
            ),
        );
        assert!(obb.contains(DVec3::new(1.0, 1.0, 1.0)));
        assert!(obb.contains(DVec3::new(-1.5, 0.0, 0.0)));
        assert!(!obb.contains(DVec3::new(2.5, 0.0, 0.0)));
    }

    #[test]
    fn test_obb_distance() {
        let obb = OrientedBoundingBox::new(
            DVec3::ZERO,
            glam::DMat3::from_cols(
                DVec3::new(1.0, 0.0, 0.0),
                DVec3::new(0.0, 1.0, 0.0),
                DVec3::new(0.0, 0.0, 1.0),
            ),
        );
        assert_eq!(obb.distance_to(DVec3::ZERO), 0.0);
        assert!((obb.distance_to(DVec3::new(2.0, 0.0, 0.0)) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_bounding_sphere_from_obb() {
        let obb = OrientedBoundingBox::new(
            DVec3::new(1.0, 2.0, 3.0),
            glam::DMat3::from_cols(
                DVec3::new(1.0, 0.0, 0.0),
                DVec3::new(0.0, 1.0, 0.0),
                DVec3::new(0.0, 0.0, 1.0),
            ),
        );
        let bs = BoundingSphere::from_obb(&obb);
        assert_eq!(bs.center, DVec3::new(1.0, 2.0, 3.0));
        assert!((bs.radius - 3.0_f64.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_lerp() {
        assert!((lerp(0.0, 10.0, 0.5) - 5.0).abs() < 1e-10);
        assert!((lerp(-1.0, 1.0, 0.0) - (-1.0)).abs() < 1e-10);
        assert!((lerp(-1.0, 1.0, 1.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_clamp_vec3() {
        let v = DVec3::new(-2.0, 0.5, 3.0);
        let min = DVec3::new(-1.0, -1.0, -1.0);
        let max = DVec3::new(1.0, 1.0, 1.0);
        let result = clamp_vec3(v, min, max);
        assert_eq!(result, DVec3::new(-1.0, 0.5, 1.0));
    }
}
