//! PropertyBag - a dynamic key-value property container.
//!
//! Maps to CesiumJS `DataSources/PropertyBag.js`

use crate::property_system::property::{ConstantProperty, DynProperty};
use crate::property_system::value::PropertyValue;
use cesium_time::JulianDate;
use std::collections::HashMap;
use std::sync::Arc;

/// A property whose value is a key-value mapping of property names to computed
/// values of other properties.
///
/// Maps to CesiumJS `DataSources/PropertyBag.js`
#[derive(Clone)]
pub struct PropertyBag {
    /// Ordered property names.
    property_names: Vec<String>,
    /// Property values indexed by name.
    properties: HashMap<String, Arc<dyn DynProperty>>,
}

impl PropertyBag {
    /// Creates a new empty PropertyBag.
    pub fn new() -> Self {
        Self {
            property_names: Vec::new(),
            properties: HashMap::new(),
        }
    }

    /// Creates a PropertyBag from a set of key-value pairs where values are
    /// raw values that get wrapped in ConstantProperty.
    ///
    /// Maps to `new PropertyBag({a: 1, b: 2})`
    pub fn from_values(values: &[(&str, PropertyValue)]) -> Self {
        let mut bag = Self::new();
        for (name, value) in values {
            let prop = ConstantProperty::new(value.clone());
            bag.add_property_with(name, Arc::new(prop));
        }
        bag
    }

    /// Creates a PropertyBag from existing properties.
    pub fn from_properties(props: &[(&str, Arc<dyn DynProperty>)]) -> Self {
        let mut bag = Self::new();
        for (name, prop) in props {
            bag.add_property_with(name, Arc::clone(prop));
        }
        bag
    }

    /// Gets the names of all properties registered on this instance.
    /// Maps to `PropertyBag.prototype.propertyNames`
    pub fn property_names(&self) -> &[String] {
        &self.property_names
    }

    /// Returns true if this property is constant (all members are constant).
    /// Maps to `PropertyBag.prototype.isConstant`
    pub fn is_constant(&self) -> bool {
        self.property_names.iter().all(|name| {
            match self.properties.get(name) {
                Some(prop) => prop.is_constant(),
                None => true,
            }
        })
    }

    /// Determines if this object has defined a property with the given name.
    /// Maps to `PropertyBag.prototype.hasProperty`
    pub fn has_property(&self, property_name: &str) -> bool {
        self.property_names.contains(&property_name.to_string())
    }

    /// Adds a property with no value.
    /// Maps to `PropertyBag.prototype.addProperty(name)`
    pub fn add_property(&mut self, property_name: &str) {
        assert!(
            !property_name.is_empty(),
            "propertyName is required."
        );
        assert!(
            !self.property_names.contains(&property_name.to_string()),
            "{property_name} is already a registered property."
        );
        self.property_names.push(property_name.to_string());
    }

    /// Adds a property with a property value.
    /// Maps to `PropertyBag.prototype.addProperty(name, value)`
    pub fn add_property_with(&mut self, property_name: &str, value: Arc<dyn DynProperty>) {
        assert!(
            !property_name.is_empty(),
            "propertyName is required."
        );
        assert!(
            !self.property_names.contains(&property_name.to_string()),
            "{property_name} is already a registered property."
        );
        self.property_names.push(property_name.to_string());
        self.properties.insert(property_name.to_string(), value);
    }

    /// Adds a property with a raw value (wrapped in ConstantProperty).
    /// Maps to `PropertyBag.prototype.addProperty(name, rawValue)`
    pub fn add_property_value(&mut self, property_name: &str, value: PropertyValue) {
        let prop = ConstantProperty::new(value);
        self.add_property_with(property_name, Arc::new(prop));
    }

    /// Removes a property previously added with addProperty.
    /// Maps to `PropertyBag.prototype.removeProperty`
    pub fn remove_property(&mut self, property_name: &str) {
        assert!(
            !property_name.is_empty(),
            "propertyName is required."
        );
        let index = self
            .property_names
            .iter()
            .position(|n| n == property_name);
        assert!(
            index.is_some(),
            "{property_name} is not a registered property."
        );
        let index = index.unwrap();
        self.property_names.remove(index);
        self.properties.remove(property_name);
    }

    /// Gets the property with the given name.
    pub fn get_property(&self, property_name: &str) -> Option<&Arc<dyn DynProperty>> {
        self.properties.get(property_name)
    }

    /// Sets the property value for an existing property name.
    pub fn set_property(&mut self, property_name: &str, value: Arc<dyn DynProperty>) {
        assert!(
            self.property_names.contains(&property_name.to_string()),
            "{property_name} is not a registered property."
        );
        self.properties.insert(property_name.to_string(), value);
    }

    /// Gets the value of this property at the given time.
    /// Each contained property is evaluated at the given time, and the overall
    /// result is a mapping of property names to those values.
    ///
    /// Maps to `PropertyBag.prototype.getValue`
    pub fn get_value(&self, time: &JulianDate) -> HashMap<String, PropertyValue> {
        let mut result = HashMap::new();
        for name in &self.property_names {
            let value = match self.properties.get(name) {
                Some(prop) => prop.get_value(time),
                None => PropertyValue::Undefined,
            };
            result.insert(name.clone(), value);
        }
        result
    }

    /// Gets the value, merging into an existing result map.
    /// Properties in result that are not part of this PropertyBag are left as-is.
    pub fn get_value_with_result(
        &self,
        time: &JulianDate,
        result: &mut HashMap<String, PropertyValue>,
    ) {
        for name in &self.property_names {
            let value = match self.properties.get(name) {
                Some(prop) => prop.get_value(time),
                None => PropertyValue::Undefined,
            };
            result.insert(name.clone(), value);
        }
    }

    /// Assigns each unassigned property on this object from the source.
    /// Maps to `PropertyBag.prototype.merge`
    pub fn merge(&mut self, source: &PropertyBag) {
        for name in &source.property_names {
            if !self.property_names.contains(name) {
                self.property_names.push(name.clone());
            }
            if let Some(prop) = source.properties.get(name) {
                self.properties.insert(name.clone(), Arc::clone(prop));
            }
        }
    }

    /// Compares this property to the provided property.
    /// Maps to `PropertyBag.prototype.equals`
    pub fn equals(&self, other: &PropertyBag) -> bool {
        if self.property_names.len() != other.property_names.len() {
            return false;
        }
        for name in &self.property_names {
            if !other.property_names.contains(name) {
                return false;
            }
            let self_prop = self.properties.get(name);
            let other_prop = other.properties.get(name);
            match (self_prop, other_prop) {
                (Some(a), Some(b)) => {
                    if !a.equals(b.as_ref()) {
                        return false;
                    }
                }
                (None, None) => {}
                _ => return false,
            }
        }
        true
    }
}

impl Default for PropertyBag {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for PropertyBag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PropertyBag")
            .field("property_names", &self.property_names)
            .finish()
    }
}
