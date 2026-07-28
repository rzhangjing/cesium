//! PropertyArray and PositionPropertyArray - properties whose values are arrays of other properties.
//!
//! Maps to CesiumJS `DataSources/PropertyArray.js` and `DataSources/PositionPropertyArray.js`.

use std::sync::Arc;

use cesium_time::JulianDate;
use glam::DVec3;

use crate::property_system::property::DynProperty;
use crate::property_system::value::PropertyValue;

/// A property whose value is an array whose items are the computed values
/// of other property instances.
///
/// Maps to CesiumJS `DataSources/PropertyArray`.
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
    /// Creates an empty PropertyArray.
    pub fn new() -> Self {
        Self { value: None }
    }

    /// Creates a PropertyArray with the given property array.
    pub fn with_value(value: Vec<Arc<dyn DynProperty>>) -> Self {
        Self {
            value: Some(value),
        }
    }

    /// Sets the value (array of properties).
    pub fn set_value(&mut self, value: Option<Vec<Arc<dyn DynProperty>>>) {
        self.value = value;
    }

    /// Gets the value at the given time. Returns None if no value is set.
    /// Undefined property values are filtered out.
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

    /// Returns true if all property items in the array are constant.
    pub fn is_constant(&self) -> bool {
        match &self.value {
            None => true,
            Some(arr) => arr.iter().all(|p| p.is_constant()),
        }
    }

    /// Compares this property to another for equality.
    pub fn equals(&self, other: &Self) -> bool {
        match (&self.value, &other.value) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                // Compare by evaluating at epoch (simplified equality)
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

/// A property whose value is an array of position properties.
/// Similar to PropertyArray but specialized for Cartesian3 positions.
///
/// Maps to CesiumJS `DataSources/PositionPropertyArray`.
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
    /// Creates an empty PositionPropertyArray.
    pub fn new() -> Self {
        Self { value: None }
    }

    /// Creates a PositionPropertyArray with the given property array.
    pub fn with_value(value: Vec<Arc<dyn DynProperty>>) -> Self {
        Self {
            value: Some(value),
        }
    }

    /// Sets the value (array of position properties).
    pub fn set_value(&mut self, value: Option<Vec<Arc<dyn DynProperty>>>) {
        self.value = value;
    }

    /// Gets the value at the given time as an array of Cartesian3.
    /// Undefined property values are filtered out.
    pub fn get_value(&self, time: &JulianDate) -> Option<Vec<DVec3>> {
        let value = self.value.as_ref()?;
        let mut result = Vec::with_capacity(value.len());
        for prop in value {
            let item_value = prop.get_value(time);
            match item_value {
                PropertyValue::Cartesian3(v) => result.push(v),
                PropertyValue::Undefined => {} // skip
                _ => {}                        // skip non-position values
            }
        }
        Some(result)
    }

    /// Returns true if all property items in the array are constant.
    pub fn is_constant(&self) -> bool {
        match &self.value {
            None => true,
            Some(arr) => arr.iter().all(|p| p.is_constant()),
        }
    }

    /// Compares this property to another for equality.
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
