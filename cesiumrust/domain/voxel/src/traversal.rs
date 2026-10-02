//! 体素 LOD 遍历系统。
//!
//! 为体素网格实现基于屏幕空间误差（SSE）的 LOD 八叉树遍历。
//! 自根节点递归下探，按相机距离与视场角决定渲染当前节点还是细化到子节点。

use crate::shape::{OrientedBoundingBox, VoxelShape, VoxelShapeType};

/// 体素八叉树中的空间节点。
///
/// 以层级与整数坐标定位瓦片，dimensions 为每轴采样数。
#[derive(Debug, Clone, PartialEq)]
pub struct SpatialNode {
    /// 八叉树中的层级（0 = 根）。
    pub level: u32,
    /// 该层级下的 X 坐标。
    pub x: u32,
    /// 该层级下的 Y 坐标。
    pub y: u32,
    /// 该层级下的 Z 坐标。
    pub z: u32,
    /// 瓦片尺寸（每轴采样数，填充前）。
    pub dimensions: [u32; 3],
}

impl SpatialNode {
    /// 创建新的空间节点。
    ///
    /// 记录层级、该层下的整数坐标与每轴采样尺寸。
    pub fn new(level: u32, x: u32, y: u32, z: u32, dimensions: [u32; 3]) -> Self {
        Self { level, x, y, z, dimensions }
    }

    /// 创建根节点。
    ///
    /// 层级为 0、各轴坐标全零，尺寸为整棵八叉树的初始瓦片。
    pub fn root(dimensions: [u32; 3]) -> Self {
        Self::new(0, 0, 0, 0, dimensions)
    }

    /// 获取子节点数量（八叉树恒为 8）。
    ///
    /// 三维空间每轴各二分一次，故子块数为 2³ = 8。
    pub fn child_count(&self) -> u32 {
        8
    }

    /// 按索引获取子节点（0-7）。
    ///
    /// index 的低三位决定子块在各轴上的偏移（0 或 1）。
    pub fn child(&self, index: u32) -> Self {
        // 子节点层级比父节点深一层
        let child_level = self.level + 1;
        // 将 index 低 3 位按位分配给三轴最低位：bit0→x、bit1→y、bit2→z
        let child_x = self.x * 2 + (index & 1);
        let child_y = self.y * 2 + ((index >> 1) & 1);
        let child_z = self.z * 2 + ((index >> 2) & 1);
        Self::new(child_level, child_x, child_y, child_z, self.dimensions)
    }

    /// 获取父节点，若为根节点则返回 None。
    ///
    /// 与 child 互逆，丢弃各坐标最低的一位。
    pub fn parent(&self) -> Option<Self> {
        // 根节点（层级 0）没有父节点
        if self.level == 0 {
            None
        } else {
            // 各坐标整除 2 即回到上一层的父节点
            Some(Self::new(
                self.level - 1,
                self.x / 2,
                self.y / 2,
                self.z / 2,
                self.dimensions,
            ))
        }
    }

    /// 获取此节点中的总采样数（包含填充）。
    ///
    /// 用于估算该瓦片纹理所需的采样容量。
    pub fn sample_count(&self, padding: u32) -> u32 {
        // 每轴两侧各补 padding 个采样，再取三轴乘积即填充后总采样数
        let dx = self.dimensions[0] + padding * 2;
        let dy = self.dimensions[1] + padding * 2;
        let dz = self.dimensions[2] + padding * 2;
        dx * dy * dz
    }

    /// 获取此节点的 Morton 索引。
    ///
    /// 相同空间位置得到相同 Morton 码，可作空间哈希键。
    pub fn morton_index(&self) -> u64 {
        // 将三轴坐标交织为单一 Z 序码
        morton_encode(self.x as u64, self.y as u64, self.z as u64)
    }
}

/// 一次遍历操作的结果。
///
/// 汇总本次遍历选中的渲染/细化节点与访问统计。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TraversalResult {
    /// 被选用于渲染的节点（满足 SSE 阈值）。
    pub render_nodes: Vec<SpatialNode>,
    /// 需要更多细节的节点（应加载其子节点）。
    pub refine_nodes: Vec<SpatialNode>,
    /// 已访问节点总数。
    pub nodes_visited: u32,
    /// 达到的最大深度。
    pub max_depth: u32,
}

/// 体素遍历配置。
///
/// 控制形状类型、SSE 阈值、层级上限与瓦片尺寸等遍历参数。
#[derive(Debug, Clone)]
pub struct VoxelTraversalConfig {
    /// 体素网格的形状类型。
    pub shape_type: VoxelShapeType,
    /// 以像素计的屏幕空间误差阈值。
    pub screen_space_error: f64,
    /// 遍历的最大层级数。
    pub max_level: u32,
    /// 瓦片尺寸（每轴采样数）。
    pub tile_dimensions: [u32; 3],
    /// 每个瓦片周围的填充。
    pub padding: u32,
    /// 是否跳过不可用的层级。
    pub skip_level_of_detail: bool,
    /// 跳过 LOD 的因子（跳过多少层级）。
    pub skip_levels: u32,
}

impl Default for VoxelTraversalConfig {
    /// 默认配置：Box 形状、SSE 阈值 16 像素、最大 10 层、8³ 瓦片、填充 1、不跳过 LOD。
    fn default() -> Self {
        Self {
            shape_type: VoxelShapeType::Box,
            screen_space_error: 16.0,
            max_level: 10,
            tile_dimensions: [8, 8, 8],
            padding: 1,
            skip_level_of_detail: false,
            skip_levels: 1,
        }
    }
}

/// 体素 LOD 遍历引擎。
///
/// 执行带基于屏幕空间误差细化的八叉树遍历。
///
/// 通过层级可用性表跳过缺失数据的层。
#[derive(Debug, Clone)]
pub struct VoxelTraversal {
    /// 遍历配置。
    pub config: VoxelTraversalConfig,
    /// 每一层的数据是否可用（层级 -> 可用）。
    level_availability: Vec<bool>,
}

impl Default for VoxelTraversal {
    /// 默认遍历器：采用默认配置，前 11 层（0..=10）全部标记为可用。
    fn default() -> Self {
        Self {
            config: VoxelTraversalConfig::default(),
            level_availability: vec![true; 11],
        }
    }
}

impl VoxelTraversal {
    /// 使用给定配置创建新的遍历。
    ///
    /// 初始所有层级均可用，后续通过 set_level_available 关闭缺失层。
    pub fn new(config: VoxelTraversalConfig) -> Self {
        // 层级 0..=max_level 共 max_level+1 层，初始全部可用
        let max_levels = (config.max_level + 1) as usize;
        Self {
            config,
            level_availability: vec![true; max_levels],
        }
    }

    /// 设置特定层级的可用性。
    pub fn set_level_available(&mut self, level: u32, available: bool) {
        // 越界层级忽略，避免数组越界
        if (level as usize) < self.level_availability.len() {
            self.level_availability[level as usize] = available;
        }
    }

    /// 检查某层级是否有可用数据。
    pub fn is_level_available(&self, level: u32) -> bool {
        // 超出记录范围的层级一律视为不可用
        if (level as usize) < self.level_availability.len() {
            self.level_availability[level as usize]
        } else {
            false
        }
    }

    /// 计算节点的屏幕空间误差。
    ///
    /// 结果越大表示该节点在当前视角下误差越明显。
    ///
    /// SSE = (geometric_error * viewport_height) / (distance * 2 * tan(fov/2))
    pub fn compute_screen_space_error(
        &self,
        node: &SpatialNode,
        shape: &dyn VoxelShape,
        camera_position: glam::DVec3,
        viewport_height: f64,
        fov_y: f64,
    ) -> f64 {
        // 取该瓦片 OBB 到相机的最近距离作为视距，钳制下限避免除零
        let obb = shape.compute_obb_for_tile(node.level, node.x, node.y, node.z);
        let distance = obb.distance_to(camera_position).max(1e-7);

        // 几何误差随层级递减
        let geometric_error = self.compute_geometric_error(node, shape);

        // 分母 2·tan(fov/2) 将世界尺度误差换算为屏幕像素误差
        let sse_denominator = 2.0 * (fov_y * 0.5).tan();
        (geometric_error * viewport_height) / (distance * sse_denominator)
    }

    /// 计算节点的几何误差（体素单元的大小）。
    ///
    /// 尺度越大、层级越浅，几何误差越大。
    fn compute_geometric_error(&self, node: &SpatialNode, shape: &dyn VoxelShape) -> f64 {
        // 以瓦片 OBB 的包围球半径代表其空间尺度
        let obb = shape.compute_obb_for_tile(node.level, node.x, node.y, node.z);
        let size = obb.bounding_sphere_radius();
        // 几何误差大致为单个采样的大小
        let max_dim = self.config.tile_dimensions.iter().max().copied().unwrap_or(8) as f64;
        size / max_dim
    }

    /// 执行遍历并返回选中的节点。
    ///
    /// 结果区分满足精度的渲染节点与需继续加载的细化节点。
    pub fn traverse(
        &self,
        shape: &dyn VoxelShape,
        camera_position: glam::DVec3,
        viewport_height: f64,
        fov_y: f64,
    ) -> TraversalResult {
        // 从根节点出发递归遍历，结果累积到 result
        let mut result = TraversalResult::default();
        let root = SpatialNode::root(self.config.tile_dimensions);
        self.traverse_node(
            &root,
            shape,
            camera_position,
            viewport_height,
            fov_y,
            &mut result,
        );
        // 返回遍历累积的渲染/细化节点与访问统计
        result
    }

    /// 递归遍历一个节点。
    ///
    /// 按最大层级、数据可用性与 SSE 阈值三种情形决定渲染、剪枝或细化。
    fn traverse_node(
        &self,
        node: &SpatialNode,
        shape: &dyn VoxelShape,
        camera_position: glam::DVec3,
        viewport_height: f64,
        fov_y: f64,
        result: &mut TraversalResult,
    ) {
        // 统计访问数并更新已遍历到的最大深度
        result.nodes_visited += 1;
        result.max_depth = result.max_depth.max(node.level);

        // 到达最大层级：无可再细化，直接渲染
        if node.level >= self.config.max_level {
            result.render_nodes.push(node.clone());
            return;
        }

        // 该层数据缺失：若启用跳过 LOD 则跨级下探子节点，否则剪枝返回
        if !self.is_level_available(node.level) {
            if self.config.skip_level_of_detail && node.level + self.config.skip_levels <= self.config.max_level {
                // 跨级取子节点继续遍历
                for i in 0..8 {
                    let child = node.child(i);
                    self.traverse_node(
                        &child,
                        shape,
                        camera_position,
                        viewport_height,
                        fov_y,
                        result,
                    );
                }
            }
            return;
        }

        // 计算该节点的屏幕空间误差
        let sse = self.compute_screen_space_error(
            node,
            shape,
            camera_position,
            viewport_height,
            fov_y,
        );

        if sse <= self.config.screen_space_error {
            // SSE 在阈值内：精度已足够，直接渲染该节点
            result.render_nodes.push(node.clone());
        } else {
            // SSE 超阈：精度不足，标记细化并逐个下探八个子节点
            result.refine_nodes.push(node.clone());
            for i in 0..8 {
                let child = node.child(i);
                self.traverse_node(
                    &child,
                    shape,
                    camera_position,
                    viewport_height,
                    fov_y,
                    result,
                );
            }
        }
    }

    /// 计算给定层级下的瓦片总数。
    ///
    /// 用于容量预估：随层级呈 8 的幂增长。
    pub fn tiles_at_level(level: u32) -> u64 {
        // 每轴随层级二分，该层瓦片总数为 (2^level)³
        let tiles_per_axis = 2u64.pow(level);
        tiles_per_axis * tiles_per_axis * tiles_per_axis
    }

    /// 获取特定瓦片的 OBB。
    pub fn tile_obb(
        &self,
        shape: &dyn VoxelShape,
        level: u32,
        x: u32,
        y: u32,
        z: u32,
    ) -> OrientedBoundingBox {
        // 委托给形状接口，按层级与索引计算该瓦片的空间包围盒
        shape.compute_obb_for_tile(level, x, y, z)
    }
}

/// 将 3D 坐标编码为 Morton 码（Z 序曲线）。
fn morton_encode(x: u64, y: u64, z: u64) -> u64 {
    // 将三轴各 21 位按 x/y/z 顺序逐位交织进结果整数的 3i、3i+1、3i+2 位
    let mut result = 0u64;
    for i in 0..21 {
        // 每轮将 x/y/z 的第 i 位依次放入 3i、3i+1、3i+2 位
        result |= ((x >> i) & 1) << (3 * i);
        result |= ((y >> i) & 1) << (3 * i + 1);
        result |= ((z >> i) & 1) << (3 * i + 2);
    }
    result
}

/// 将 Morton 码解码为 3D 坐标。
pub fn morton_decode(code: u64) -> (u64, u64, u64) {
    // morton_encode 的逆过程：从 3i、3i+1、3i+2 位分别还原 x/y/z 的第 i 位
    let mut x = 0u64;
    let mut y = 0u64;
    let mut z = 0u64;
    for i in 0..21 {
        // 每轮从对应位段取回一位并重新拼接到各轴
        x |= ((code >> (3 * i)) & 1) << i;
        y |= ((code >> (3 * i + 1)) & 1) << i;
        z |= ((code >> (3 * i + 2)) & 1) << i;
    }
    (x, y, z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::box_shape::VoxelBoxShape;

    #[test]
    fn test_spatial_node_root() {
        let root = SpatialNode::root([8, 8, 8]);
        assert_eq!(root.level, 0);
        assert_eq!(root.x, 0);
        assert_eq!(root.y, 0);
        assert_eq!(root.z, 0);
        assert_eq!(root.dimensions, [8, 8, 8]);
    }

    #[test]
    fn test_spatial_node_children() {
        let root = SpatialNode::root([8, 8, 8]);
        assert_eq!(root.child_count(), 8);

        let child0 = root.child(0);
        assert_eq!(child0.level, 1);
        assert_eq!((child0.x, child0.y, child0.z), (0, 0, 0));

        let child7 = root.child(7);
        assert_eq!(child7.level, 1);
        assert_eq!((child7.x, child7.y, child7.z), (1, 1, 1));

        let child5 = root.child(5);
        assert_eq!((child5.x, child5.y, child5.z), (1, 0, 1));
    }

    #[test]
    fn test_spatial_node_parent() {
        let root = SpatialNode::root([8, 8, 8]);
        assert!(root.parent().is_none());

        let child = root.child(3);
        let parent = child.parent().unwrap();
        assert_eq!(parent.level, 0);
        assert_eq!((parent.x, parent.y, parent.z), (0, 0, 0));
    }

    #[test]
    fn test_spatial_node_sample_count() {
        let node = SpatialNode::root([8, 8, 8]);
        // padding=1 时：(8+2)^3 = 1000
        assert_eq!(node.sample_count(1), 1000);
        // padding=0 时：8^3 = 512
        assert_eq!(node.sample_count(0), 512);
    }

    #[test]
    fn test_morton_encode_decode() {
        let code = morton_encode(1, 2, 3);
        let (x, y, z) = morton_decode(code);
        assert_eq!((x, y, z), (1, 2, 3));

        let code2 = morton_encode(0, 0, 0);
        assert_eq!(code2, 0);

        let code3 = morton_encode(7, 7, 7);
        let (x3, y3, z3) = morton_decode(code3);
        assert_eq!((x3, y3, z3), (7, 7, 7));
    }

    #[test]
    fn test_traversal_config_default() {
        let config = VoxelTraversalConfig::default();
        assert_eq!(config.shape_type, VoxelShapeType::Box);
        assert_eq!(config.screen_space_error, 16.0);
        assert_eq!(config.max_level, 10);
        assert_eq!(config.tile_dimensions, [8, 8, 8]);
        assert_eq!(config.padding, 1);
    }

    #[test]
    fn test_traversal_basic() {
        let mut shape = VoxelBoxShape::new();
        shape.update(
            glam::DMat4::IDENTITY,
            crate::box_shape::BOX_DEFAULT_MIN_BOUNDS,
            crate::box_shape::BOX_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );

        let config = VoxelTraversalConfig {
            max_level: 2,
            screen_space_error: 1000.0, // 高阈值 = 更少细化
            ..Default::default()
        };
        let traversal = VoxelTraversal::new(config);

        let result = traversal.traverse(
            &shape,
            glam::DVec3::new(0.0, 0.0, 10.0),
            1080.0,
            std::f64::consts::FRAC_PI_3,
        );

        assert!(result.nodes_visited > 0);
        assert!(!result.render_nodes.is_empty());
    }

    #[test]
    fn test_traversal_max_level() {
        let mut shape = VoxelBoxShape::new();
        shape.update(
            glam::DMat4::IDENTITY,
            crate::box_shape::BOX_DEFAULT_MIN_BOUNDS,
            crate::box_shape::BOX_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );

        let config = VoxelTraversalConfig {
            max_level: 0,
            screen_space_error: 0.001, // 极低阈值 = 总是细化
            ..Default::default()
        };
        let traversal = VoxelTraversal::new(config);

        let result = traversal.traverse(
            &shape,
            glam::DVec3::new(0.0, 0.0, 100.0),
            1080.0,
            std::f64::consts::FRAC_PI_3,
        );

        // 在 max_level=0 时，根节点应直接渲染
        assert_eq!(result.render_nodes.len(), 1);
        assert_eq!(result.max_depth, 0);
    }

    #[test]
    fn test_traversal_level_availability() {
        let mut traversal = VoxelTraversal::default();
        assert!(traversal.is_level_available(0));
        assert!(traversal.is_level_available(5));

        traversal.set_level_available(3, false);
        assert!(!traversal.is_level_available(3));
        assert!(traversal.is_level_available(2));
    }

    #[test]
    fn test_tiles_at_level() {
        assert_eq!(VoxelTraversal::tiles_at_level(0), 1);
        assert_eq!(VoxelTraversal::tiles_at_level(1), 8);
        assert_eq!(VoxelTraversal::tiles_at_level(2), 64);
        assert_eq!(VoxelTraversal::tiles_at_level(3), 512);
    }

    #[test]
    fn test_screen_space_error_computation() {
        let mut shape = VoxelBoxShape::new();
        shape.update(
            glam::DMat4::IDENTITY,
            crate::box_shape::BOX_DEFAULT_MIN_BOUNDS,
            crate::box_shape::BOX_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );

        let traversal = VoxelTraversal::default();
        let root = SpatialNode::root([8, 8, 8]);

        // 靠近的相机 = 高 SSE
        let sse_close = traversal.compute_screen_space_error(
            &root,
            &shape,
            glam::DVec3::new(0.0, 0.0, 2.0),
            1080.0,
            std::f64::consts::FRAC_PI_3,
        );

        // 远离的相机 = 低 SSE
        let sse_far = traversal.compute_screen_space_error(
            &root,
            &shape,
            glam::DVec3::new(0.0, 0.0, 1000.0),
            1080.0,
            std::f64::consts::FRAC_PI_3,
        );

        assert!(sse_close > sse_far);
    }
}
