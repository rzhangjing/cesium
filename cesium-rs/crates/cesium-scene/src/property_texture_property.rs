//! Ported from `packages/engine/Source/Scene/PropertyTextureProperty.js`.
//!
//! A property within a property texture, reading metadata from specific
//! texture channels with optional value transforms.

use serde_json::Value;

/// A property within a property texture.
///
/// Reads metadata values from texture channels, applying optional
/// offset/scale transforms and noData/default handling.
/// Mirrors CesiumJS `PropertyTextureProperty` (~200 lines).
pub struct PropertyTextureProperty {
    /// The texture index this property reads from.
    pub texture_index: usize,
    /// The channel indices to read (e.g. [0, 1] for "rg").
    pub channels: Vec<u32>,
    /// Offset transform value.
    pub offset: Option<f64>,
    /// Scale transform value.
    pub scale: Option<f64>,
    /// Whether this property has offset/scale transforms.
    pub has_value_transform: bool,
    /// Minimum value constraint.
    pub min: Option<Value>,
    /// Maximum value constraint.
    pub max: Option<Value>,
    /// No-data sentinel value.
    pub no_data: Option<Value>,
    /// Default value when noData is encountered.
    pub default: Option<Value>,
    /// Extra user-defined data.
    pub extras: Option<Value>,
    /// Extension data.
    pub extensions: Option<Value>,
}

impl PropertyTextureProperty {
    /// Creates a new `PropertyTextureProperty`.
    pub fn new() -> Self {
        Self {
            texture_index: 0,
            channels: vec![0],
            offset: None,
            scale: None,
            has_value_transform: false,
            min: None,
            max: None,
            no_data: None,
            default: None,
            extras: None,
            extensions: None,
        }
    }

    /// Returns the GLSL swizzle string for the channel indices.
    ///
    /// e.g. `[0]` → `"r"`, `[0, 1]` → `"rg"`, `[0, 1, 2, 3]` → `"rgba"`.
    pub fn reformat_channels(&self) -> String {
        let swizzle = ['r', 'g', 'b', 'a'];
        self.channels
            .iter()
            .map(|&c| swizzle.get(c as usize).copied().unwrap_or('r'))
            .collect()
    }
}

impl Default for PropertyTextureProperty {
    fn default() -> Self { Self::new() }
}
