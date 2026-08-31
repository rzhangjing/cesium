//! Ported from `packages/engine/Source/Scene/Model/Extensions/Gpm/PpeMetadata.js`.

use crate::model::extensions::gpm::ppe_source::PpeSource;

/// Metadata related to the stored PPE (Per-Point Error) data.
///
/// This reflects the `ppeMetadata` definition of the NGA_gpm_local
/// glTF extension.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PpeMetadata {
    /// The source of the error data.
    source: PpeSource,
    /// Minimum allowed value for the property.
    min: Option<f64>,
    /// Maximum allowed value for the property.
    max: Option<f64>,
}

impl PpeMetadata {
    /// Creates a new `PpeMetadata`.
    ///
    /// Port of the `PpeMetadata(options)` constructor.
    pub fn new(source: PpeSource, min: Option<f64>, max: Option<f64>) -> Self {
        Self { source, min, max }
    }

    /// Minimum allowed value for the property. This is the minimum of
    /// all values after the transforms based on the offset and scale
    /// properties have been applied (port of the `min` getter).
    pub fn min(&self) -> Option<f64> {
        self.min
    }

    /// Maximum allowed value for the property. This is the maximum of
    /// all values after the transforms based on the offset and scale
    /// properties have been applied (port of the `max` getter).
    pub fn max(&self) -> Option<f64> {
        self.max
    }

    /// Possible error source contents (port of the `source` getter).
    pub fn source(&self) -> PpeSource {
        self.source
    }
}
