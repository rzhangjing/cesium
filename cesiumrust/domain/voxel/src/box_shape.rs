//! 长方体体素形状实现。
//!
//! 长方体形状将体素数据映射到一个轴对齐的矩形区域，
//! 边界以最小与最大 XYZ 坐标指定，默认为 [-1, 1]^3。

use glam::{DMat3, DMat4, DVec3};

use crate::shape::{
    clamp_vec3, lerp, BoundingSphere, OrientedBoundingBox, VoxelShape,
};

/// 长方体形状的默认最小边界：(-1, -1, -1)。
pub const BOX_DEFAULT_MIN_BOUNDS: DVec3 = DVec3::new(-1.0, -1.0, -1.0);
/// 长方体形状的默认最大边界：(1, 1, 1)。
pub const BOX_DEFAULT_MAX_BOUNDS: DVec3 = DVec3::new(1.0, 1.0, 1.0);

/// 长方体形状的体素区域。
///
/// 长方体形状将体素数据映射到 3D 空间中的一个矩形区域。
/// 边界以最小和最大 XYZ 坐标指定。
/// 形状变换矩阵决定其在世界中的位置、朝向与大小。
/// 形状始终为轴对齐长方体，射线与其表面最多相交一次。
#[derive(Debug, Clone)]
pub struct VoxelBoxShape {
    /// 包含有界形状的有向包围盒。
    obb: OrientedBoundingBox,
    /// 包含有界形状的包围球。
    bounding_sphere: BoundingSphere,
    /// 有界形状的变换。
    bound_transform: DMat4,
    /// 形状的变换，忽略边界。
    shape_transform: DMat4,
    /// 最小边界。
    min_bounds: DVec3,
    /// 最大边界。
    max_bounds: DVec3,
    /// 最小渲染边界（裁剪后）。
    render_min_bounds: DVec3,
    /// 最大渲染边界（裁剪后）。
    render_max_bounds: DVec3,
    /// 用于局部到形状 UV 变换的 UV 缩放。
    local_to_shape_uv_scale: DVec3,
    /// 用于局部到形状 UV 变换的 UV 平移。
    local_to_shape_uv_translate: DVec3,
    /// 最大相交数量。
    max_intersections: u32,
}

impl Default for VoxelBoxShape {
    /// 默认长方体：单位包围盒与球、恒等变换、边界 [-1,1]^3、UV 缩放/平移 0.5、最大相交 1。
    fn default() -> Self {
        Self {
            obb: OrientedBoundingBox::default(),
            bounding_sphere: BoundingSphere::default(),
            bound_transform: DMat4::IDENTITY,
            shape_transform: DMat4::IDENTITY,
            min_bounds: BOX_DEFAULT_MIN_BOUNDS,
            max_bounds: BOX_DEFAULT_MAX_BOUNDS,
            render_min_bounds: BOX_DEFAULT_MIN_BOUNDS,
            render_max_bounds: BOX_DEFAULT_MAX_BOUNDS,
            // UV 缩放/平移 0.5 使 [-1,1] 区间映射到 [0,1]
            local_to_shape_uv_scale: DVec3::new(0.5, 0.5, 0.5),
            local_to_shape_uv_translate: DVec3::new(0.5, 0.5, 0.5),
            max_intersections: 1,
        }
    }
}

impl VoxelBoxShape {
    /// 创建具有默认边界的新长方体形状。
    ///
    /// 以默认边界构造，与 Default 实现一致。
    pub fn new() -> Self {
        // 委托默认构造
        Self::default()
    }

    /// 获取最小边界。
    ///
    /// 完整体素区域（未裁剪）的左下角。
    pub fn min_bounds(&self) -> DVec3 {
        self.min_bounds
    }

    /// 获取最大边界。
    ///
    /// 完整体素区域（未裁剪）的右上角。
    pub fn max_bounds(&self) -> DVec3 {
        self.max_bounds
    }

    /// 获取渲染最小边界。
    ///
    /// 裁剪后实际参与渲染范围的左下角。
    pub fn render_min_bounds(&self) -> DVec3 {
        self.render_min_bounds
    }

    /// 获取渲染最大边界。
    ///
    /// 裁剪后实际参与渲染范围的右上角。
    pub fn render_max_bounds(&self) -> DVec3 {
        self.render_max_bounds
    }

    /// 检查局部坐标中的点是否位于渲染边界内部。
    pub fn contains_local(&self, point: DVec3) -> bool {
        // 逐轴判断点是否落在渲染包围盒的 [min, max] 闭区间内
        point.x >= self.render_min_bounds.x
            && point.x <= self.render_max_bounds.x
            && point.y >= self.render_min_bounds.y
            && point.y <= self.render_max_bounds.y
            && point.z >= self.render_min_bounds.z
            && point.z <= self.render_max_bounds.z
    }

    /// 计算长方体某个子区域的 OBB。
    fn compute_chunk_obb(&self, min_b: DVec3, max_b: DVec3) -> OrientedBoundingBox {
        // 判断该子区域是否恰为默认全域 [-1,1]^3
        let is_default = (min_b - BOX_DEFAULT_MIN_BOUNDS).length() < 1e-10
            && (max_b - BOX_DEFAULT_MAX_BOUNDS).length() < 1e-10;

        if is_default {
            // 全域情形：中心即变换后的原点，半轴直接取变换矩阵的列向量
            let center = self.shape_transform.transform_point3(DVec3::ZERO);
            // 全域时半轴即变换矩阵的前三列（含缩放）
            let half_axes = DMat3::from_cols(
                self.shape_transform.col(0).truncate(),
                self.shape_transform.col(1).truncate(),
                self.shape_transform.col(2).truncate(),
            );
            OrientedBoundingBox::new(center, half_axes)
        } else {
            // 子区域情形：先从变换矩阵列向量长度还原各轴缩放
            let scale = DVec3::new(
                self.shape_transform.col(0).truncate().length(),
                self.shape_transform.col(1).truncate().length(),
                self.shape_transform.col(2).truncate().length(),
            );
            // 子区域中心的局部坐标 = 两边界中点，再变换到世界
            let local_center = (min_b + max_b) * 0.5;
            let center = self.shape_transform.transform_point3(local_center);

            // 半轴长 = 缩放 × 0.5 × 子区域在该轴的长度
            let half_scale = DVec3::new(
                scale.x * 0.5 * (max_b.x - min_b.x),
                scale.y * 0.5 * (max_b.y - min_b.y),
                scale.z * 0.5 * (max_b.z - min_b.z),
            );

            // 从形状变换中提取旋转
            let rotation = extract_rotation(&self.shape_transform);
            // 半轴 = 旋转方向列向量按对应轴半长缩放
            let half_axes = DMat3::from_cols(
                rotation.col(0) * half_scale.x,
                rotation.col(1) * half_scale.y,
                rotation.col(2) * half_scale.z,
            );
            OrientedBoundingBox::new(center, half_axes)
        }
    }
}

impl VoxelShape for VoxelBoxShape {
    /// 返回包含形状的有向包围盒引用。
    fn oriented_bounding_box(&self) -> &OrientedBoundingBox {
        &self.obb
    }

    /// 返回包含形状的包围球引用。
    fn bounding_sphere(&self) -> &BoundingSphere {
        &self.bounding_sphere
    }

    /// 返回边界变换矩阵。
    fn bound_transform(&self) -> DMat4 {
        self.bound_transform
    }

    /// 返回忽略边界的形状变换矩阵。
    fn shape_transform(&self) -> DMat4 {
        self.shape_transform
    }

    /// 返回射线-形状相交的最大数量。
    fn maximum_intersections_length(&self) -> u32 {
        self.max_intersections
    }

    /// 更新形状状态：设置边界、裁剪出渲染范围并重建包围体，返回是否可见。
    fn update(
        &mut self,
        model_matrix: DMat4,
        min_bounds: DVec3,
        max_bounds: DVec3,
        clip_min_bounds: Option<DVec3>,
        clip_max_bounds: Option<DVec3>,
    ) -> bool {
        // 未提供裁剪边界时退化为完整边界
        let clip_min = clip_min_bounds.unwrap_or(min_bounds);
        let clip_max = clip_max_bounds.unwrap_or(max_bounds);

        self.min_bounds = min_bounds;
        self.max_bounds = max_bounds;

        // 将完整边界钳制到裁剪区间，得到实际参与渲染的范围
        let render_min = clamp_vec3(min_bounds, clip_min, clip_max);
        let render_max = clamp_vec3(max_bounds, clip_min, clip_max);
        self.render_min_bounds = render_min;
        self.render_max_bounds = render_max;

        // 检查可见性：取模型矩阵各列长度作为逐轴缩放
        let scale = DVec3::new(
            model_matrix.col(0).truncate().length(),
            model_matrix.col(1).truncate().length(),
            model_matrix.col(2).truncate().length(),
        );

        let degenerate_count = (if (render_min.x - render_max.x).abs() < 1e-10 { 1 } else { 0 })
            + (if (render_min.y - render_max.y).abs() < 1e-10 { 1 } else { 0 })
            + (if (render_min.z - render_max.z).abs() < 1e-10 { 1 } else { 0 });

        // 任一缩放分量为零、范围反转或两轴以上退化时不可见
        // （重建旋转矩阵代价过高，直接判为不可见）
        if render_min.x > render_max.x
            || render_min.y > render_max.y
            || render_min.z > render_max.z
            || degenerate_count >= 2
            || scale.x == 0.0
            || scale.y == 0.0
            || scale.z == 0.0
        {
            // 命中上述任一退化/裁剪条件即判定不可见
            return false;
        }

        // 保存形状变换并据渲染范围重建 OBB 与包围球
        self.shape_transform = model_matrix;
        self.obb = self.compute_chunk_obb(render_min, render_max);
        // 包围球由 OBB 派生，用于更廉价的快速剔除
        self.bounding_sphere = BoundingSphere::from_obb(&self.obb);

        // 由 OBB 的半轴与中心组装边界变换矩阵
        // 半轴列向量扩展为旋转/缩放部分，平移分量取 OBB 中心
        self.bound_transform = DMat4::from_cols(
            self.obb.half_axes.col(0).extend(0.0),
            self.obb.half_axes.col(1).extend(0.0),
            self.obb.half_axes.col(2).extend(0.0),
            self.obb.center.extend(1.0),
        );

        // 计算 UV 缩放与平移：缩放使各轴归一到 [0,1]，平移对齐最小边界
        self.local_to_shape_uv_scale = DVec3::new(
            bound_scale(min_bounds.x, max_bounds.x),
            bound_scale(min_bounds.y, max_bounds.y),
            bound_scale(min_bounds.z, max_bounds.z),
        );
        // 平移使最小边界对齐到 UV 原点
        self.local_to_shape_uv_translate = -(self.local_to_shape_uv_scale * min_bounds);

        // 长方体的射线最多与表面相交一次
        self.max_intersections = 1;
        true
    }

    /// 将局部坐标线性映射到形状的 [0,1] UV 空间。
    fn convert_local_to_shape_uv_space(&self, position_local: DVec3) -> DVec3 {
        // 应用缩放再平移，等价于 (p - min) / (max - min)
        self.local_to_shape_uv_scale * position_local + self.local_to_shape_uv_translate
    }

    /// 为指定层级与索引的瓦片计算 OBB。
    fn compute_obb_for_tile(
        &self,
        tile_level: u32,
        tile_x: u32,
        tile_y: u32,
        tile_z: u32,
    ) -> OrientedBoundingBox {
        // 该层级下每个瓦片在归一化参数空间中的边长
        let size_at_level = 1.0 / (2.0_f64.powi(tile_level as i32));
        let min_b = self.min_bounds;
        let max_b = self.max_bounds;

        // 用线性插值把瓦片索引换算为边界坐标的最小角
        let tile_min = DVec3::new(
            lerp(min_b.x, max_b.x, size_at_level * tile_x as f64),
            lerp(min_b.y, max_b.y, size_at_level * tile_y as f64),
            lerp(min_b.z, max_b.z, size_at_level * tile_z as f64),
        );
        // 最大角对应索引 +1 的插值位置
        let tile_max = DVec3::new(
            lerp(min_b.x, max_b.x, size_at_level * (tile_x + 1) as f64),
            lerp(min_b.y, max_b.y, size_at_level * (tile_y + 1) as f64),
            lerp(min_b.z, max_b.z, size_at_level * (tile_z + 1) as f64),
        );

        // 复用子区域 OBB 计算，得到该瓦片的空间包围盒
        self.compute_chunk_obb(tile_min, tile_max)
    }
}

/// 计算用于 UV 映射的缩放因子。
///
/// 输入为单轴的最小与最大值。
fn bound_scale(min_bound: f64, max_bound: f64) -> f64 {
    // 区间退化时返回 1.0 避免除零，否则取区间长度的倒数
    if (min_bound - max_bound).abs() < 1e-7 {
        1.0
    } else {
        1.0 / (max_bound - min_bound)
    }
}

/// 从变换中提取旋转矩阵（对列归一化）。
///
/// 用于在存在缩放时仍得到纯旋转的半轴方向。
fn extract_rotation(matrix: &DMat4) -> DMat3 {
    // 取前三列并截去平移分量
    let col0 = matrix.col(0).truncate();
    let col1 = matrix.col(1).truncate();
    let col2 = matrix.col(2).truncate();
    // 用各列长度（钳制下限防除零）归一化，剥离缩放仅保留旋转
    let l0 = col0.length().max(1e-15);
    let l1 = col1.length().max(1e-15);
    let l2 = col2.length().max(1e-15);
    DMat3::from_cols(col0 / l0, col1 / l1, col2 / l2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_shape_default() {
        let shape = VoxelBoxShape::new();
        assert_eq!(shape.min_bounds(), BOX_DEFAULT_MIN_BOUNDS);
        assert_eq!(shape.max_bounds(), BOX_DEFAULT_MAX_BOUNDS);
        assert_eq!(shape.maximum_intersections_length(), 1);
    }

    #[test]
    fn test_box_shape_update_identity() {
        let mut shape = VoxelBoxShape::new();
        let visible = shape.update(
            DMat4::IDENTITY,
            BOX_DEFAULT_MIN_BOUNDS,
            BOX_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        assert!(visible);
        assert_eq!(shape.obb.center, DVec3::ZERO);
        assert!(shape.bounding_sphere.radius > 0.0);
    }

    #[test]
    fn test_box_shape_update_with_translation() {
        let mut shape = VoxelBoxShape::new();
        let matrix = DMat4::from_translation(DVec3::new(10.0, 20.0, 30.0));
        let visible = shape.update(
            matrix,
            BOX_DEFAULT_MIN_BOUNDS,
            BOX_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        assert!(visible);
        assert!((shape.obb.center - DVec3::new(10.0, 20.0, 30.0)).length() < 1e-10);
    }

    #[test]
    fn test_box_shape_update_with_scale() {
        let mut shape = VoxelBoxShape::new();
        let matrix = DMat4::from_scale(DVec3::new(2.0, 3.0, 4.0));
        let visible = shape.update(
            matrix,
            BOX_DEFAULT_MIN_BOUNDS,
            BOX_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        assert!(visible);
        // 包围球半径应反映缩放后的长方体
        assert!(shape.bounding_sphere.radius > 4.0);
    }

    #[test]
    fn test_box_shape_invisible_degenerate() {
        let mut shape = VoxelBoxShape::new();
        // 任一分量缩放为零 => 不可见（退化形状不渲染）
        let matrix = DMat4::from_scale(DVec3::new(0.0, 1.0, 1.0));
        let visible = shape.update(
            matrix,
            BOX_DEFAULT_MIN_BOUNDS,
            BOX_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        assert!(!visible);

        // 两个缩放为零 => 同样不可见
        let matrix2 = DMat4::from_scale(DVec3::new(0.0, 0.0, 1.0));
        let visible2 = shape.update(
            matrix2,
            BOX_DEFAULT_MIN_BOUNDS,
            BOX_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        assert!(!visible2);
    }

    #[test]
    fn test_box_shape_invisible_clipped_away() {
        let mut shape = VoxelBoxShape::new();
        // 裁剪边界排除了整个形状
        let visible = shape.update(
            DMat4::IDENTITY,
            BOX_DEFAULT_MIN_BOUNDS,
            BOX_DEFAULT_MAX_BOUNDS,
            Some(DVec3::new(5.0, 5.0, 5.0)),
            Some(DVec3::new(10.0, 10.0, 10.0)),
        );
        assert!(!visible);
    }

    #[test]
    fn test_box_shape_uv_transform() {
        let mut shape = VoxelBoxShape::new();
        shape.update(
            DMat4::IDENTITY,
            BOX_DEFAULT_MIN_BOUNDS,
            BOX_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        // [-1,1] 的中心应映射到 UV (0.5, 0.5, 0.5)
        let uv = shape.convert_local_to_shape_uv_space(DVec3::ZERO);
        assert!((uv.x - 0.5).abs() < 1e-10);
        assert!((uv.y - 0.5).abs() < 1e-10);
        assert!((uv.z - 0.5).abs() < 1e-10);

        // 最小角应映射到 (0, 0, 0)
        let uv_min = shape.convert_local_to_shape_uv_space(BOX_DEFAULT_MIN_BOUNDS);
        assert!(uv_min.x.abs() < 1e-10);
        assert!(uv_min.y.abs() < 1e-10);
        assert!(uv_min.z.abs() < 1e-10);
    }

    #[test]
    fn test_box_shape_tile_obb() {
        let mut shape = VoxelBoxShape::new();
        shape.update(
            DMat4::IDENTITY,
            BOX_DEFAULT_MIN_BOUNDS,
            BOX_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        // 级别 0，瓦片 (0,0,0) 应为完整长方体
        let obb = shape.compute_obb_for_tile(0, 0, 0, 0);
        assert!(obb.center.length() < 1e-10);

        // 级别 1，瓦片 (0,0,0) 应为第一挂限
        let obb_octant = shape.compute_obb_for_tile(1, 0, 0, 0);
        assert!((obb_octant.center - DVec3::new(-0.5, -0.5, -0.5)).length() < 1e-10);
    }

    #[test]
    fn test_box_shape_contains_local() {
        let mut shape = VoxelBoxShape::new();
        shape.update(
            DMat4::IDENTITY,
            BOX_DEFAULT_MIN_BOUNDS,
            BOX_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        assert!(shape.contains_local(DVec3::ZERO));
        assert!(shape.contains_local(DVec3::new(0.9, -0.9, 0.5)));
        assert!(!shape.contains_local(DVec3::new(1.5, 0.0, 0.0)));
    }

    #[test]
    fn test_box_shape_partial_bounds() {
        let mut shape = VoxelBoxShape::new();
        let min_b = DVec3::new(0.0, 0.0, 0.0);
        let max_b = DVec3::new(1.0, 1.0, 1.0);
        let visible = shape.update(DMat4::IDENTITY, min_b, max_b, None, None);
        assert!(visible);
        // (0,0,0) 的 UV 应为 (0,0,0)
        let uv = shape.convert_local_to_shape_uv_space(DVec3::ZERO);
        assert!(uv.x.abs() < 1e-10);
        // (1,1,1) 的 UV 应为 (1,1,1)
        let uv_max = shape.convert_local_to_shape_uv_space(DVec3::ONE);
        assert!((uv_max.x - 1.0).abs() < 1e-10);
    }
}
