//! NodeTransformationProperty - composite property for model node TRS transforms.
//!
//! Maps to CesiumJS `DataSources/NodeTransformationProperty.js`

use crate::property_system::property::DynProperty;
use crate::property_system::value::PropertyValue;
use cesium_time::JulianDate;
use glam::{DQuat, DVec3};
use std::sync::Arc;

/// The resolved value of a NodeTransformationProperty at a given time.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeTransformationValue {
    /// Translation offset.
    pub translation: DVec3,
    /// Rotation quaternion.
    pub rotation: DQuat,
    /// Scale factors.
    pub scale: DVec3,
}

impl Default for NodeTransformationValue {
    fn default() -> Self {
        Self {
            translation: DVec3::ZERO,
            rotation: DQuat::IDENTITY,
            scale: DVec3::ONE,
        }
    }
}

/// A property that represents a model node transformation composed of
/// translation, rotation, and scale sub-properties.
///
/// Maps to CesiumJS `DataSources/NodeTransformationProperty.js`
#[derive(Clone)]
pub struct NodeTransformationProperty {
    /// The translation property.
    translation: Option<Arc<dyn DynProperty>>,
    /// The rotation property.
    rotation: Option<Arc<dyn DynProperty>>,
    /// The scale property.
    scale: Option<Arc<dyn DynProperty>>,
}

impl NodeTransformationProperty {
    /// Creates a new NodeTransformationProperty with no sub-properties.
    pub fn new() -> Self {
        Self {
            translation: None,
            rotation: None,
            scale: None,
        }
    }

    /// Creates a NodeTransformationProperty with constant values.
    pub fn with_values(translation: DVec3, rotation: DQuat, scale: DVec3) -> Self {
        use crate::property_system::property::ConstantProperty;
        Self {
            translation: Some(Arc::new(ConstantProperty::new(PropertyValue::Cartesian3(
                translation,
            )))),
            rotation: Some(Arc::new(ConstantProperty::new(PropertyValue::Quaternion(
                rotation,
            )))),
            scale: Some(Arc::new(ConstantProperty::new(PropertyValue::Cartesian3(
                scale,
            )))),
        }
    }

    /// Gets whether this property is constant (all sub-properties are constant).
    pub fn is_constant(&self) -> bool {
        let t_const = self.translation.as_ref().map_or(true, |p| p.is_constant());
        let r_const = self.rotation.as_ref().map_or(true, |p| p.is_constant());
        let s_const = self.scale.as_ref().map_or(true, |p| p.is_constant());
        t_const && r_const && s_const
    }

    /// Gets the translation property.
    pub fn translation(&self) -> Option<&Arc<dyn DynProperty>> {
        self.translation.as_ref()
    }

    /// Sets the translation property.
    pub fn set_translation(&mut self, prop: Option<Arc<dyn DynProperty>>) {
        self.translation = prop;
    }

    /// Gets the rotation property.
    pub fn rotation(&self) -> Option<&Arc<dyn DynProperty>> {
        self.rotation.as_ref()
    }

    /// Sets the rotation property.
    pub fn set_rotation(&mut self, prop: Option<Arc<dyn DynProperty>>) {
        self.rotation = prop;
    }

    /// Gets the scale property.
    pub fn scale(&self) -> Option<&Arc<dyn DynProperty>> {
        self.scale.as_ref()
    }

    /// Sets the scale property.
    pub fn set_scale(&mut self, prop: Option<Arc<dyn DynProperty>>) {
        self.scale = prop;
    }

    /// Gets the resolved transformation value at the given time.
    ///
    /// Defaults: translation=ZERO, rotation=IDENTITY, scale=ONE.
    ///
    /// Maps to `NodeTransformationProperty.prototype.getValue`
    pub fn get_value(&self, time: &JulianDate) -> NodeTransformationValue {
        let translation = self
            .translation
            .as_ref()
            .and_then(|p| match p.get_value(time) {
                PropertyValue::Cartesian3(v) => Some(v),
                _ => None,
            })
            .unwrap_or(DVec3::ZERO);

        let rotation = self
            .rotation
            .as_ref()
            .and_then(|p| match p.get_value(time) {
                PropertyValue::Quaternion(q) => Some(q),
                _ => None,
            })
            .unwrap_or(DQuat::IDENTITY);

        let scale = self
            .scale
            .as_ref()
            .and_then(|p| match p.get_value(time) {
                PropertyValue::Cartesian3(v) => Some(v),
                _ => None,
            })
            .unwrap_or(DVec3::ONE);

        NodeTransformationValue {
            translation,
            rotation,
            scale,
        }
    }

    /// Compares this property to another.
    pub fn equals(&self, other: &NodeTransformationProperty) -> bool {
        prop_equals(&self.translation, &other.translation)
            && prop_equals(&self.rotation, &other.rotation)
            && prop_equals(&self.scale, &other.scale)
    }
}

fn prop_equals(
    a: &Option<Arc<dyn DynProperty>>,
    b: &Option<Arc<dyn DynProperty>>,
) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(_), None) | (None, Some(_)) => false,
        (Some(pa), Some(pb)) => pa.equals(pb.as_ref()),
    }
}

impl Default for NodeTransformationProperty {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for NodeTransformationProperty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeTransformationProperty")
            .field("has_translation", &self.translation.is_some())
            .field("has_rotation", &self.rotation.is_some())
            .field("has_scale", &self.scale.is_some())
            .finish()
    }
}
