//! Ported from `packages/engine/Source/DataSources/NodeTransformationProperty.js`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::quaternion::Quaternion;
use crate::property::{Property, PropertyResult};

/// A property that defines a transformation for a model node.
pub struct NodeTransformationProperty {
    /// The translation of the node.
    pub translation: Option<Cartesian3>,
    /// The rotation of the node.
    pub rotation: Option<Quaternion>,
    /// The scale of the node.
    pub scale: Option<Cartesian3>,
}

impl NodeTransformationProperty {
    /// Creates a new node transformation property.
    pub fn new() -> Self {
        Self { translation: None, rotation: None, scale: None }
    }
}

impl Default for NodeTransformationProperty {
    fn default() -> Self { Self::new() }
}

impl Property for NodeTransformationProperty {
    fn get_value(&self, _time: f64) -> PropertyResult {
        PropertyResult::None
    }

    fn is_constant(&self) -> bool { true }
    fn is_destroyed(&self) -> bool { false }
}
