//! Reference properties: transparent links to properties on other entities.
//!
//! Maps to CesiumJS `DataSources/ReferenceProperty.js`.

use crate::property_system::property::DynProperty;
use crate::property_system::value::{PropertyValue, ReferenceFrame};
use cesium_time::JulianDate;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

/// Resolves a property reference to a concrete property.
///
/// This decouples `ReferenceProperty` from any concrete entity-collection
/// type. Implementors map a target entity id and a path of property names to
/// the referenced property.
pub trait PropertyResolver: Send + Sync {
    /// Resolves the property identified by `target_id` and `property_names`.
    /// Returns `None` if the target entity or property path cannot be found.
    fn resolve(&self, target_id: &str, property_names: &[String])
        -> Option<Arc<dyn DynProperty>>;
}

/// Parses a reference string of the form `"objectId#foo.bar"`, where `#`
/// separates the id from the property path and `.` separates sub-properties.
/// The `#`, `.` and `\` characters may be escaped with a backslash.
///
/// Returns `(identifier, property_names)`.
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

/// A property which transparently links to another property on a provided
/// object.
///
/// Maps to CesiumJS `DataSources/ReferenceProperty.js`.
#[derive(Clone)]
pub struct ReferenceProperty {
    resolver: Arc<dyn PropertyResolver>,
    target_id: String,
    target_property_names: Vec<String>,
}

impl ReferenceProperty {
    /// Creates a new reference property.
    /// Maps to `new ReferenceProperty(targetCollection, targetId, targetPropertyNames)`.
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

    /// Creates a new instance from a reference string of the form
    /// `"objectId#foo.bar"`.
    /// Maps to `ReferenceProperty.fromString`.
    pub fn from_string(resolver: Arc<dyn PropertyResolver>, reference_string: &str) -> Self {
        let (identifier, values) = parse_reference_string(reference_string);
        Self::new(resolver, &identifier, values)
    }

    /// The id of the entity being referenced. Maps to `targetId`.
    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    /// The array of property names used to retrieve the referenced property.
    /// Maps to `targetPropertyNames`.
    pub fn target_property_names(&self) -> &[String] {
        &self.target_property_names
    }

    /// The resolved instance of the underlying referenced property, or `None`
    /// if it cannot be resolved. Maps to `resolvedProperty`.
    pub fn resolved_property(&self) -> Option<Arc<dyn DynProperty>> {
        self.resolver
            .resolve(&self.target_id, &self.target_property_names)
    }
}

impl DynProperty for ReferenceProperty {
    fn is_constant(&self) -> bool {
        // CesiumJS `Property.isConstant(resolve(this))` is true when the
        // target cannot be resolved.
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

/// A simple `PropertyResolver` backed by a map keyed on
/// `"targetId#name1.name2..."`. Useful for testing and simple use cases.
#[derive(Default, Clone)]
pub struct MapPropertyResolver {
    entries: Arc<HashMap<String, Arc<dyn DynProperty>>>,
}

impl MapPropertyResolver {
    /// Creates an empty resolver.
    pub fn new() -> Self {
        Self {
            entries: Arc::new(HashMap::new()),
        }
    }

    /// Inserts a property under the key formed from `target_id` and
    /// `property_names`.
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
        // "\#object\.4#billboard.scale" -> id "#object.4", props [billboard, scale].
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
        // Unresolved reference is constant and yields undefined.
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

        // Different resolver -> not equal.
        let other_resolver = Arc::new(MapPropertyResolver::new());
        let d = ReferenceProperty::new(
            other_resolver as Arc<dyn PropertyResolver>,
            "obj",
            names(&["x", "y"]),
        );
        assert!(!a.equals(&d));
    }
}
