//! 引用属性：指向其他实体上属性的透明链接。
//!
//! 映射到 CesiumJS `DataSources/ReferenceProperty.js`。

use crate::property_system::property::DynProperty;
use crate::property_system::value::{PropertyValue, ReferenceFrame};
use cesium_time::JulianDate;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

/// 将一个属性引用解析为一个具体属性。
///
/// 这将 `ReferenceProperty` 与任何具体的实体集合类型解耦。
/// 实现者将目标实体 id 与一条属性名路径映射到
/// 被引用的属性。
pub trait PropertyResolver: Send + Sync {
    /// 解析由 `target_id` 与 `property_names` 标识的属性。
    /// 若无法找到目标实体或属性路径则返回 `None`。
    fn resolve(&self, target_id: &str, property_names: &[String])
        -> Option<Arc<dyn DynProperty>>;
}

/// 解析形如 `"objectId#foo.bar"` 的引用字符串，其中 `#`
/// 分隔 id 与属性路径，`.` 分隔子属性。
/// `#`、`.` 和 `\` 字符可用反斜杠转义。
///
/// 返回 `(identifier, property_names)`。
fn parse_reference_string(reference_string: &str) -> (String, Vec<String>) {
    let mut identifier = String::new();
    let mut values: Vec<String> = Vec::new();

    let mut in_identifier = true;
    let mut is_escaped = false;
    let mut token = String::new();

    for c in reference_string.chars() {
        if is_escaped {
            token.push(c);
            is_escaped = false;
        } else if c == '\\' {
            is_escaped = true;
        } else if in_identifier && c == '#' {
            identifier = token.clone();
            in_identifier = false;
            token.clear();
        } else if !in_identifier && c == '.' {
            values.push(token.clone());
            token.clear();
        } else {
            token.push(c);
        }
    }
    values.push(token);

    (identifier, values)
}

/// 一种透明地链接到所提供对象上另一个属性的
/// 属性。
///
/// 映射到 CesiumJS `DataSources/ReferenceProperty.js`。
#[derive(Clone)]
pub struct ReferenceProperty {
    resolver: Arc<dyn PropertyResolver>,
    target_id: String,
    target_property_names: Vec<String>,
}

impl ReferenceProperty {
    /// 创建新的引用属性。
    /// 映射到 `new ReferenceProperty(targetCollection, targetId, targetPropertyNames)`。
    pub fn new(
        resolver: Arc<dyn PropertyResolver>,
        target_id: &str,
        target_property_names: Vec<String>,
    ) -> Self {
        Self {
            resolver,
            target_id: target_id.to_string(),
            target_property_names,
        }
    }

    /// 从形如 `"objectId#foo.bar"` 的引用字符串
    /// 创建新实例。
    /// 映射到 `ReferenceProperty.fromString`。
    pub fn from_string(resolver: Arc<dyn PropertyResolver>, reference_string: &str) -> Self {
        let (identifier, values) = parse_reference_string(reference_string);
        Self::new(resolver, &identifier, values)
    }

    /// 被引用实体的 id。映射到 `targetId`。
    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    /// 用于获取被引用属性的属性名数组。
    /// 映射到 `targetPropertyNames`。
    pub fn target_property_names(&self) -> &[String] {
        &self.target_property_names
    }

    /// 底层被引用属性的已解析实例，若无法解析
    /// 则为 `None`。映射到 `resolvedProperty`。
    pub fn resolved_property(&self) -> Option<Arc<dyn DynProperty>> {
        self.resolver
            .resolve(&self.target_id, &self.target_property_names)
    }
}

impl DynProperty for ReferenceProperty {
    fn is_constant(&self) -> bool {
        // CesiumJS 的 `Property.isConstant(resolve(this))` 在目标
        // 无法解析时为 true。
        match self.resolved_property() {
            None => true,
            Some(p) => p.is_constant(),
        }
    }

    fn get_value(&self, time: &JulianDate) -> PropertyValue {
        match self.resolved_property() {
            Some(p) => p.get_value(time),
            None => PropertyValue::Undefined,
        }
    }

    fn type_name(&self) -> &'static str {
        "ReferenceProperty"
    }

    fn equals(&self, other: &dyn DynProperty) -> bool {
        match other.as_any().downcast_ref::<ReferenceProperty>() {
            Some(o) => {
                Arc::ptr_eq(&self.resolver, &o.resolver)
                    && self.target_id == o.target_id
                    && self.target_property_names == o.target_property_names
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn reference_frame(&self) -> Option<ReferenceFrame> {
        self.resolved_property().and_then(|p| p.reference_frame())
    }

    fn get_value_in_reference_frame(
        &self,
        time: &JulianDate,
        frame: ReferenceFrame,
    ) -> Option<PropertyValue> {
        self.resolved_property()
            .and_then(|p| p.get_value_in_reference_frame(time, frame))
    }

    fn get_type(&self, time: &JulianDate) -> Option<String> {
        self.resolved_property().and_then(|p| p.get_type(time))
    }
}

/// 一个由映射支持的简单 `PropertyResolver`，键为
/// `"targetId#name1.name2..."`。适用于测试与简单场景。
#[derive(Default, Clone)]
pub struct MapPropertyResolver {
    entries: Arc<HashMap<String, Arc<dyn DynProperty>>>,
}

impl MapPropertyResolver {
    /// 创建一个空解析器。
    pub fn new() -> Self {
        Self {
            entries: Arc::new(HashMap::new()),
        }
    }

    /// 在由 `target_id` 与 `property_names` 构成的键下
    /// 插入一个属性。
    pub fn insert(
        &mut self,
        target_id: &str,
        property_names: &[String],
        property: Arc<dyn DynProperty>,
    ) {
        let key = format!("{}#{}", target_id, property_names.join("."));
        if let Some(map) = Arc::get_mut(&mut self.entries) {
            map.insert(key, property);
        }
    }
}

impl PropertyResolver for MapPropertyResolver {
    fn resolve(
        &self,
        target_id: &str,
        property_names: &[String],
    ) -> Option<Arc<dyn DynProperty>> {
        let key = format!("{}#{}", target_id, property_names.join("."));
        self.entries.get(&key).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::property_system::property::ConstantProperty;

    fn names(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_parse_reference_string_simple() {
        let (id, props) = parse_reference_string("object1#billboard.scale");
        assert_eq!(id, "object1");
        assert_eq!(props, names(&["billboard", "scale"]));
    }

    #[test]
    fn test_parse_reference_string_single_property() {
        let (id, props) = parse_reference_string("obj#position");
        assert_eq!(id, "obj");
        assert_eq!(props, names(&["position"]));
    }

    #[test]
    fn test_parse_reference_string_escaped() {
        // "\#object\.4#billboard.scale" -> id "#object.4"，props [billboard, scale]。
        let (id, props) = parse_reference_string("\\#object\\.4#billboard.scale");
        assert_eq!(id, "#object.4");
        assert_eq!(props, names(&["billboard", "scale"]));
    }

    #[test]
    fn test_parse_reference_string_escaped_backslash() {
        let (id, props) = parse_reference_string("a\\\\b#c");
        assert_eq!(id, "a\\b");
        assert_eq!(props, names(&["c"]));
    }

    #[test]
    fn test_reference_property_resolves_value() {
        let mut resolver = MapPropertyResolver::new();
        let target: Arc<dyn DynProperty> =
            Arc::new(ConstantProperty::new(PropertyValue::Number(2.0)));
        resolver.insert("object1", &names(&["billboard", "scale"]), target);
        let resolver = Arc::new(resolver);

        let prop = ReferenceProperty::new(
            Arc::clone(&resolver) as Arc<dyn PropertyResolver>,
            "object1",
            names(&["billboard", "scale"]),
        );

        assert!(prop.is_constant());
        assert_eq!(
            prop.get_value(&JulianDate::now()),
            PropertyValue::Number(2.0)
        );
        assert!(prop.resolved_property().is_some());
    }

    #[test]
    fn test_reference_property_unresolved() {
        let resolver = Arc::new(MapPropertyResolver::new());
        let prop = ReferenceProperty::new(
            resolver as Arc<dyn PropertyResolver>,
            "missing",
            names(&["foo"]),
        );
        // 未解析的引用为常量且产生 undefined。
        assert!(prop.is_constant());
        assert_eq!(
            prop.get_value(&JulianDate::now()),
            PropertyValue::Undefined
        );
        assert!(prop.resolved_property().is_none());
    }

    #[test]
    fn test_reference_property_from_string() {
        let mut resolver = MapPropertyResolver::new();
        let target: Arc<dyn DynProperty> =
            Arc::new(ConstantProperty::new(PropertyValue::Number(5.0)));
        resolver.insert("object1", &names(&["billboard", "scale"]), target);
        let resolver = Arc::new(resolver);

        let prop = ReferenceProperty::from_string(
            resolver as Arc<dyn PropertyResolver>,
            "object1#billboard.scale",
        );
        assert_eq!(prop.target_id(), "object1");
        assert_eq!(prop.target_property_names(), names(&["billboard", "scale"]));
        assert_eq!(
            prop.get_value(&JulianDate::now()),
            PropertyValue::Number(5.0)
        );
    }

    #[test]
    fn test_reference_property_equals() {
        let resolver = Arc::new(MapPropertyResolver::new());
        let a = ReferenceProperty::new(
            Arc::clone(&resolver) as Arc<dyn PropertyResolver>,
            "obj",
            names(&["x", "y"]),
        );
        let b = ReferenceProperty::new(
            Arc::clone(&resolver) as Arc<dyn PropertyResolver>,
            "obj",
            names(&["x", "y"]),
        );
        assert!(a.equals(&b));

        let c = ReferenceProperty::new(
            Arc::clone(&resolver) as Arc<dyn PropertyResolver>,
            "obj",
            names(&["x", "z"]),
        );
        assert!(!a.equals(&c));

        // 不同的解析器 -> 不相等。
        let other_resolver = Arc::new(MapPropertyResolver::new());
        let d = ReferenceProperty::new(
            other_resolver as Arc<dyn PropertyResolver>,
            "obj",
            names(&["x", "y"]),
        );
        assert!(!a.equals(&d));
    }
}
