//! 3D Tiles 与地形的分类（classification）系统。
//!
//! 分类图元涵盖：
//! - 分类图元本体（ClassificationPrimitive）
//! - 分类类型（ClassificationType）
//! - 基于 Feature ID 的分类

/// 分类类型决定受影响的几何体。
///
/// 取值与 3D Tiles 规范的 `classificationType` 一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClassificationType {
    /// 同时分类地形和 3D Tiles。
    #[default]
    Both,
    /// 仅分类地形。
    Terrain,
    /// 仅分类 3D Tiles。
    Cesium3DTile,
}

/// 针对 feature 的分类定义。
#[derive(Debug, Clone)]
pub struct Classification {
    /// 唯一标识符。
    pub id: String,
    /// 分类类型。
    pub classification_type: ClassificationType,
    /// 分类是否显示。
    pub show: bool,
    /// 要应用的颜色 [r, g, b, a]。
    pub color: [f64; 4],
    /// 要分类的 Feature ID（空 = 全部）。
    pub feature_ids: Vec<u64>,
    /// 要分类的 Batch ID（用于 b3dm）。
    pub batch_ids: Vec<u32>,
}

impl Classification {
    /// 创建一个新的分类。
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            classification_type: ClassificationType::Both,
            show: true,
            color: [1.0, 1.0, 0.0, 0.5],
            feature_ids: Vec::new(),
            batch_ids: Vec::new(),
        }
    }

    /// 设置分类类型。
    pub fn with_type(mut self, classification_type: ClassificationType) -> Self {
        self.classification_type = classification_type;
        self
    }

    /// 设置颜色。
    pub fn with_color(mut self, color: [f64; 4]) -> Self {
        self.color = color;
        self
    }

    /// 添加要分类的 feature ID。
    pub fn with_feature_ids(mut self, ids: Vec<u64>) -> Self {
        self.feature_ids = ids;
        self
    }

    /// 添加要分类的 batch ID。
    pub fn with_batch_ids(mut self, ids: Vec<u32>) -> Self {
        self.batch_ids = ids;
        self
    }

    /// 检查某个 feature ID 是否被分类。
    pub fn contains_feature(&self, feature_id: u64) -> bool {
        self.feature_ids.is_empty() || self.feature_ids.contains(&feature_id)
    }

    /// 检查某个 batch ID 是否被分类。
    pub fn contains_batch(&self, batch_id: u32) -> bool {
        self.batch_ids.is_empty() || self.batch_ids.contains(&batch_id)
    }
}

/// 分类的集合。
#[derive(Debug, Default)]
pub struct ClassificationCollection {
    /// 按 ID 存储的分类。
    classifications: Vec<Classification>,
}

impl ClassificationCollection {
    /// 创建一个新的空集合。
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加一个分类。
    pub fn add(&mut self, classification: Classification) {
        self.classifications.push(classification);
    }

    /// 按 ID 移除一个分类。
    pub fn remove(&mut self, id: &str) -> Option<Classification> {
        if let Some(pos) = self.classifications.iter().position(|c| c.id == id) {
            Some(self.classifications.remove(pos))
        } else {
            None
        }
    }

    /// 按 ID 获取一个分类。
    pub fn get(&self, id: &str) -> Option<&Classification> {
        self.classifications.iter().find(|c| c.id == id)
    }

    /// 返回分类的数量。
    pub fn len(&self) -> usize {
        self.classifications.len()
    }

    /// 若集合为空则返回 true。
    pub fn is_empty(&self) -> bool {
        self.classifications.is_empty()
    }

    /// 获取影响某个 feature 的所有分类。
    pub fn get_for_feature(&self, feature_id: u64) -> Vec<&Classification> {
        self.classifications
            .iter()
            .filter(|c| c.show && c.contains_feature(feature_id))
            .collect()
    }

    /// 获取影响某个 batch 的所有分类。
    pub fn get_for_batch(&self, batch_id: u32) -> Vec<&Classification> {
        self.classifications
            .iter()
            .filter(|c| c.show && c.contains_batch(batch_id))
            .collect()
    }

    /// 计算某个 feature 的混合颜色。
    pub fn compute_feature_color(&self, feature_id: u64, base_color: [f64; 4]) -> [f64; 4] {
        let classifications = self.get_for_feature(feature_id);
        Self::blend_colors(base_color, &classifications)
    }

    /// 将多个分类颜色与基础颜色混合。
    fn blend_colors(base_color: [f64; 4], classifications: &[&Classification]) -> [f64; 4] {
        let mut result = base_color;

        for classification in classifications {
            let c = classification.color;
            let alpha = c[3];

            // Alpha 混合：result = base * (1 - alpha) + overlay * alpha
            result[0] = result[0] * (1.0 - alpha) + c[0] * alpha;
            result[1] = result[1] * (1.0 - alpha) + c[1] * alpha;
            result[2] = result[2] * (1.0 - alpha) + c[2] * alpha;
            result[3] = result[3].max(alpha);
        }

        result
    }
}

/// 用于分类的 feature 元数据。
#[derive(Debug, Clone, Default)]
pub struct FeatureMetadata {
    /// Feature ID。
    pub feature_id: u64,
    /// Batch ID（用于 b3dm）。
    pub batch_id: Option<u32>,
    /// 属性表索引。
    pub property_table: Option<u32>,
    /// 自定义属性。
    pub properties: Vec<(String, MetadataValue)>,
}

/// 元数据值类型。
#[derive(Debug, Clone, PartialEq)]
pub enum MetadataValue {
    /// 布尔值。
    Bool(bool),
    /// 整数值。
    Int(i64),
    /// 浮点值。
    Float(f64),
    /// 字符串值。
    String(String),
    /// 浮点数组。
    FloatArray(Vec<f64>),
}

impl FeatureMetadata {
    /// 创建新的 feature 元数据。
    pub fn new(feature_id: u64) -> Self {
        Self {
            feature_id,
            ..Default::default()
        }
    }

    /// 按名称获取属性值。
    pub fn get_property(&self, name: &str) -> Option<&MetadataValue> {
        self.properties
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v)
    }

    /// 设置属性值。
    pub fn set_property(&mut self, name: impl Into<String>, value: MetadataValue) {
        let name = name.into();
        if let Some(prop) = self.properties.iter_mut().find(|(n, _)| *n == name) {
            prop.1 = value;
        } else {
            self.properties.push((name, value));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证链式构造器正确写入 id/类型/颜色。
    #[test]
    fn test_classification_creation() {
        let classification = Classification::new("test")
            .with_type(ClassificationType::Terrain)
            .with_color([1.0, 0.0, 0.0, 0.5]);

        assert_eq!(classification.id, "test");
        assert_eq!(classification.classification_type, ClassificationType::Terrain);
        assert_eq!(classification.color, [1.0, 0.0, 0.0, 0.5]);
    }

    /// 验证显式 feature ID 列表的包含/不包含判定。
    #[test]
    fn test_classification_contains() {
        let classification = Classification::new("test")
            .with_feature_ids(vec![1, 2, 3]);

        assert!(classification.contains_feature(1));
        assert!(classification.contains_feature(2));
        assert!(!classification.contains_feature(4));
    }

    /// 验证空集合默认分类任意 feature（全量语义）。
    #[test]
    fn test_classification_empty_ids() {
        let classification = Classification::new("test");

        // 空的 feature_ids 表示全部 feature
        assert!(classification.contains_feature(999));
    }

    #[test]
    fn test_classification_collection() {
        let mut collection = ClassificationCollection::new();

        collection.add(Classification::new("c1").with_feature_ids(vec![1, 2]));
        collection.add(Classification::new("c2").with_feature_ids(vec![2, 3]));

        assert_eq!(collection.len(), 2);

        let for_feature_2 = collection.get_for_feature(2);
        assert_eq!(for_feature_2.len(), 2);

        let for_feature_1 = collection.get_for_feature(1);
        assert_eq!(for_feature_1.len(), 1);
    }

    /// 验证按 ID 移除分类后集合长度递减。
    #[test]
    fn test_classification_removal() {
        let mut collection = ClassificationCollection::new();
        collection.add(Classification::new("c1"));
        collection.add(Classification::new("c2"));

        let removed = collection.remove("c1");
        assert!(removed.is_some());
        assert_eq!(collection.len(), 1);
    }

    /// 验证 alpha 混合公式在蓝底叠加半透明红得紫。
    #[test]
    fn test_color_blending() {
        let mut collection = ClassificationCollection::new();
        collection.add(
            Classification::new("c1")
                .with_color([1.0, 0.0, 0.0, 0.5])
                .with_feature_ids(vec![1]),
        );

        let base = [0.0, 0.0, 1.0, 1.0]; // 蓝色
        let result = collection.compute_feature_color(1, base);

        // 在蓝色基底上叠加 50% alpha 的红色
        // result = blue * 0.5 + red * 0.5 = [0.5, 0.0, 0.5, 1.0]
        assert!((result[0] - 0.5).abs() < 0.01);
        assert!((result[1] - 0.0).abs() < 0.01);
        assert!((result[2] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_feature_metadata() {
        let mut metadata = FeatureMetadata::new(42);
        metadata.set_property("height", MetadataValue::Float(100.0));
        metadata.set_property("type", MetadataValue::String("building".to_string()));

        assert_eq!(metadata.feature_id, 42);
        assert_eq!(
            metadata.get_property("height"),
            Some(&MetadataValue::Float(100.0))
        );
        assert_eq!(
            metadata.get_property("type"),
            Some(&MetadataValue::String("building".to_string()))
        );
        assert_eq!(metadata.get_property("missing"), None);
    }

    #[test]
    fn test_classification_type_default() {
        assert_eq!(ClassificationType::default(), ClassificationType::Both);
    }

    #[test]
    fn test_batch_classification() {
        let classification = Classification::new("test")
            .with_batch_ids(vec![0, 5, 10]);

        assert!(classification.contains_batch(0));
        assert!(classification.contains_batch(5));
        assert!(!classification.contains_batch(3));
    }
}
