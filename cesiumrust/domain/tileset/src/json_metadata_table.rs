//! JsonMetadataTable - 基于 JSON 的 3D Tiles 元数据表。
//!
//! 镜像 CesiumJS `Scene/JsonMetadataTable.js`

use serde_json::Value;
use std::collections::HashMap;

/// 由 JSON 值支撑的元数据表。
/// 镜像 CesiumJS `Scene/JsonMetadataTable.js`
#[derive(Debug, Clone)]
pub struct JsonMetadataTable {
    count: usize,
    properties: HashMap<String, Vec<Value>>,
}

impl JsonMetadataTable {
    /// 创建一个新的 JsonMetadataTable。
    ///
    /// # 参数
    /// * `count` - 表中 feature 的数量。
    /// * `properties` - 属性 ID 到值数组的映射。
    pub fn new(count: usize, properties: HashMap<String, Vec<Value>>) -> Self {
        Self { count, properties }
    }

    /// 返回表中 feature 的数量。
    pub fn count(&self) -> usize {
        self.count
    }

    /// 若表含有给定属性则返回 true。
    pub fn has_property(&self, property_id: &str) -> bool {
        self.properties.contains_key(property_id)
    }

    /// 返回已排序的属性 ID 列表。
    pub fn get_property_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.properties.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// 获取给定索引处的属性值。
    /// 若属性不存在或索引越界则返回 None。
    pub fn get_property(&self, index: usize, property_id: &str) -> Option<Value> {
        if index >= self.count {
            return None;
        }
        let values = self.properties.get(property_id)?;
        values.get(index).cloned()
    }

    /// 设置给定索引处的属性值。
    /// 若属性不存在则创建它。
    pub fn set_property(&mut self, index: usize, property_id: &str, value: Value) {
        if index >= self.count {
            return;
        }
        let values = self
            .properties
            .entry(property_id.to_string())
            .or_insert_with(|| vec![Value::Null; self.count]);
        if index < values.len() {
            values[index] = value;
        }
    }
}
