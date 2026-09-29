//! PropertyArray 与 PositionPropertyArray - 值为其子属性数组的属性。
//!
//! 映射到 CesiumJS `DataSources/PropertyArray.js` 和 `DataSources/PositionPropertyArray.js`。

use std::sync::Arc;

use cesium_time::JulianDate;
use glam::DVec3;

use crate::property_system::property::DynProperty;
use crate::property_system::value::PropertyValue;

/// 一个属性，其值是一个数组，数组中的各项是其他属性
/// 实例的计算值。
///
/// 映射到 CesiumJS `DataSources/PropertyArray`。
#[derive(Clone)]
pub struct PropertyArray {
    value: Option<Vec<Arc<dyn DynProperty>>>,
}

impl Default for PropertyArray {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyArray {
    /// 创建一个空的 PropertyArray。
    pub fn new() -> Self {
        Self { value: None }
    }

    /// 使用给定的属性数组创建 PropertyArray。
    pub fn with_value(value: Vec<Arc<dyn DynProperty>>) -> Self {
        Self {
            value: Some(value),
        }
    }

    /// 设置值（属性数组）。
    pub fn set_value(&mut self, value: Option<Vec<Arc<dyn DynProperty>>>) {
        self.value = value;
    }

    /// 获取给定时间处的值。若未设置值则返回 None。
    /// Undefined 属性值会被过滤掉。
    pub fn get_value(&self, time: &JulianDate) -> Option<Vec<PropertyValue>> {
        let value = self.value.as_ref()?;
        let mut result = Vec::with_capacity(value.len());
        for prop in value {
            let item_value = prop.get_value(time);
            if item_value != PropertyValue::Undefined {
                result.push(item_value);
            }
        }
        Some(result)
    }

    /// 若数组中所有属性项均为常量则返回 true。
    pub fn is_constant(&self) -> bool {
        match &self.value {
            None => true,
            Some(arr) => arr.iter().all(|p| p.is_constant()),
        }
    }

    /// 将此属性与另一个属性进行相等性比较。
    pub fn equals(&self, other: &Self) -> bool {
        match (&self.value, &other.value) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                // 通过在历元处求值来比较（简化的相等性）
                let time = JulianDate::new(0.0, 0.0);
                a.iter().zip(b.iter()).all(|(pa, pb)| {
                    let va = pa.get_value(&time);
                    let vb = pb.get_value(&time);
                    va == vb
                })
            }
            _ => false,
        }
    }
}

/// 一个属性，其值是位置属性的数组。
/// 类似于 PropertyArray，但专门用于 Cartesian3 位置。
///
/// 映射到 CesiumJS `DataSources/PositionPropertyArray`。
#[derive(Clone)]
pub struct PositionPropertyArray {
    value: Option<Vec<Arc<dyn DynProperty>>>,
}

impl Default for PositionPropertyArray {
    fn default() -> Self {
        Self::new()
    }
}

impl PositionPropertyArray {
    /// 创建一个空的 PositionPropertyArray。
    pub fn new() -> Self {
        Self { value: None }
    }

    /// 使用给定的属性数组创建 PositionPropertyArray。
    pub fn with_value(value: Vec<Arc<dyn DynProperty>>) -> Self {
        Self {
            value: Some(value),
        }
    }

    /// 设置值（位置属性的数组）。
    pub fn set_value(&mut self, value: Option<Vec<Arc<dyn DynProperty>>>) {
        self.value = value;
    }

    /// 获取给定时间处的值，以 Cartesian3 数组形式返回。
    /// Undefined 属性值会被过滤掉。
    pub fn get_value(&self, time: &JulianDate) -> Option<Vec<DVec3>> {
        let value = self.value.as_ref()?;
        let mut result = Vec::with_capacity(value.len());
        for prop in value {
            let item_value = prop.get_value(time);
            match item_value {
                PropertyValue::Cartesian3(v) => result.push(v),
                PropertyValue::Undefined => {} // 跳过
                _ => {}                        // 跳过非位置值
            }
        }
        Some(result)
    }

    /// 若数组中所有属性项均为常量则返回 true。
    pub fn is_constant(&self) -> bool {
        match &self.value {
            None => true,
            Some(arr) => arr.iter().all(|p| p.is_constant()),
        }
    }

    /// 将此属性与另一个属性进行相等性比较。
    pub fn equals(&self, other: &Self) -> bool {
        match (&self.value, &other.value) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                let time = JulianDate::new(0.0, 0.0);
                a.iter().zip(b.iter()).all(|(pa, pb)| {
                    let va = pa.get_value(&time);
                    let vb = pb.get_value(&time);
                    va == vb
                })
            }
            _ => false,
        }
    }
}
