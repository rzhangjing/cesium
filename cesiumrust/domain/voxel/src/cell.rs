//! 体素单元元数据访问。
//!
//! 提供一个体素单元的空间定位（瓦片/采样索引、有向包围盒）与按名称访问的
//! 多类型元数据属性集合。

use std::collections::HashMap;

use crate::shape::OrientedBoundingBox;

/// 体素单元的元数据值类型。
///
/// 以枚举封装各分类型的标量与向量属性，便于统一存储与按需解包。
#[derive(Debug, Clone, PartialEq)]
pub enum VoxelMetadataValue {
    /// 单个 f32 值。
    Float(f32),
    /// 单个 f64 值。
    Double(f64),
    /// 单个 i32 值。
    Int(i32),
    /// 单个 u32 值。
    Uint(u32),
    /// f32 值向量。
    VecF32(Vec<f32>),
    /// f64 值向量。
    VecF64(Vec<f64>),
    /// 字符串值。
    String(String),
}

/// 来自体素图元的单元，提供对元数据和空间信息的访问。
///
/// 瓦片与采样索引定位单元，包围盒描述其空间范围，元数据表携带属性值。
#[derive(Debug, Clone)]
pub struct VoxelCell {
    /// 包含此单元的瓦片索引。
    tile_index: u32,
    /// 瓦片内采样点索引。
    sample_index: u32,
    /// 元数据属性映射（名称 -> 值）。
    metadata: HashMap<String, VoxelMetadataValue>,
    /// 单元的有向包围盒。
    oriented_bounding_box: OrientedBoundingBox,
}

impl VoxelCell {
    /// 创建一个新的体素单元。
    pub fn new(tile_index: u32, sample_index: u32) -> Self {
        Self {
            tile_index,
            sample_index,
            metadata: HashMap::new(),
            oriented_bounding_box: OrientedBoundingBox::default(),
        }
    }

    /// 创建带元数据和包围盒的单元。
    pub fn with_data(
        tile_index: u32,
        sample_index: u32,
        metadata: HashMap<String, VoxelMetadataValue>,
        obb: OrientedBoundingBox,
    ) -> Self {
        // 直接采用调用方提供的元数据表与包围盒，适用于从拾取结果重建
        Self {
            tile_index,
            sample_index,
            metadata,
            oriented_bounding_box: obb,
        }
    }

    /// 获取瓦片索引。
    pub fn tile_index(&self) -> u32 {
        self.tile_index
    }

    /// 获取瓦片内的采样点索引。
    pub fn sample_index(&self) -> u32 {
        self.sample_index
    }

    /// 获取有向包围盒。
    pub fn oriented_bounding_box(&self) -> &OrientedBoundingBox {
        &self.oriented_bounding_box
    }

    /// 检查单元是否具有给定名称的属性。
    pub fn has_property(&self, name: &str) -> bool {
        // 仅判断键是否存在，不区分值的类型
        self.metadata.contains_key(name)
    }

    /// 获取所有属性名称。
    pub fn get_names(&self) -> Vec<&str> {
        // 以借用形式返回键列表，避免拷贝字符串
        self.metadata.keys().map(|s| s.as_str()).collect()
    }

    /// 按名称获取属性值。
    pub fn get_property(&self, name: &str) -> Option<&VoxelMetadataValue> {
        self.metadata.get(name)
    }

    /// 获取浮点属性值。
    pub fn get_float(&self, name: &str) -> Option<f64> {
        // Float 与 Double 均可读作 f64，其余类型返回 None
        match self.metadata.get(name) {
            Some(VoxelMetadataValue::Float(v)) => Some(*v as f64),
            Some(VoxelMetadataValue::Double(v)) => Some(*v),
            _ => None,
        }
    }

    /// 获取整数属性值。
    pub fn get_int(&self, name: &str) -> Option<i64> {
        // Int 与 Uint 均提升为 i64，其余类型返回 None
        match self.metadata.get(name) {
            Some(VoxelMetadataValue::Int(v)) => Some(*v as i64),
            Some(VoxelMetadataValue::Uint(v)) => Some(*v as i64),
            _ => None,
        }
    }

    /// 设置属性值。
    pub fn set_property(&mut self, name: String, value: VoxelMetadataValue) {
        // 同名键会被新值覆盖
        self.metadata.insert(name, value);
    }

    /// 获取元数据属性数量。
    pub fn property_count(&self) -> usize {
        // 返回内部 HashMap 的键数量
        self.metadata.len()
    }

    /// 在给定填充维度时，将采样点索引转换为 3D 瓦片坐标。
    ///
    /// 返回填充瓦片内的 (x, y, z) 索引。
    pub fn sample_index_to_tile_coordinate(
        sample_index: u32,
        padded_dim_x: u32,
        padded_dim_y: u32,
    ) -> (u32, u32, u32) {
        // 采样索引按行主序展平：先除片层面积得 z，再依次解出 y、x
        let slice_size = padded_dim_x * padded_dim_y;
        let z = sample_index / slice_size;
        let index_in_slice = sample_index - z * slice_size;
        let y = index_in_slice / padded_dim_x;
        let x = index_in_slice - y * padded_dim_x;
        (x, y, z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voxel_cell_new() {
        let cell = VoxelCell::new(5, 42);
        assert_eq!(cell.tile_index(), 5);
        assert_eq!(cell.sample_index(), 42);
        assert_eq!(cell.property_count(), 0);
    }

    #[test]
    fn test_voxel_cell_metadata() {
        let mut cell = VoxelCell::new(0, 0);
        cell.set_property("temperature".to_string(), VoxelMetadataValue::Float(25.5));
        cell.set_property("density".to_string(), VoxelMetadataValue::Double(1.225));
        cell.set_property("class_id".to_string(), VoxelMetadataValue::Int(3));

        assert!(cell.has_property("temperature"));
        assert!(cell.has_property("density"));
        assert!(!cell.has_property("pressure"));

        assert_eq!(cell.property_count(), 3);
        assert!((cell.get_float("temperature").unwrap() - 25.5).abs() < 1e-5);
        assert!((cell.get_float("density").unwrap() - 1.225).abs() < 1e-10);
        assert_eq!(cell.get_int("class_id"), Some(3));
    }

    #[test]
    fn test_voxel_cell_get_names() {
        let mut cell = VoxelCell::new(0, 0);
        cell.set_property("a".to_string(), VoxelMetadataValue::Int(1));
        cell.set_property("b".to_string(), VoxelMetadataValue::Int(2));

        let names = cell.get_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
    }

    #[test]
    fn test_voxel_cell_with_data() {
        let mut metadata = HashMap::new();
        metadata.insert("value".to_string(), VoxelMetadataValue::Float(42.0));

        let obb = OrientedBoundingBox::default();
        let cell = VoxelCell::with_data(1, 10, metadata, obb);

        assert_eq!(cell.tile_index(), 1);
        assert_eq!(cell.sample_index(), 10);
        assert!(cell.has_property("value"));
    }

    #[test]
    fn test_sample_index_to_tile_coordinate() {
        // 4x4x4 填充维度
        let (x, y, z) = VoxelCell::sample_index_to_tile_coordinate(0, 4, 4);
        assert_eq!((x, y, z), (0, 0, 0));

        let (x, y, z) = VoxelCell::sample_index_to_tile_coordinate(5, 4, 4);
        assert_eq!((x, y, z), (1, 1, 0));

        let (x, y, z) = VoxelCell::sample_index_to_tile_coordinate(63, 4, 4);
        assert_eq!((x, y, z), (3, 3, 3));
    }

    #[test]
    fn test_voxel_metadata_value_types() {
        let f = VoxelMetadataValue::Float(1.5);
        let d = VoxelMetadataValue::Double(2.5);
        let i = VoxelMetadataValue::Int(-3);
        let u = VoxelMetadataValue::Uint(4);
        let s = VoxelMetadataValue::String("hello".to_string());
        let v = VoxelMetadataValue::VecF32(vec![1.0, 2.0, 3.0]);

        assert_eq!(f, VoxelMetadataValue::Float(1.5));
        assert_eq!(d, VoxelMetadataValue::Double(2.5));
        assert_eq!(i, VoxelMetadataValue::Int(-3));
        assert_eq!(u, VoxelMetadataValue::Uint(4));
        assert_eq!(s, VoxelMetadataValue::String("hello".to_string()));
        assert_eq!(v, VoxelMetadataValue::VecF32(vec![1.0, 2.0, 3.0]));
    }
}
