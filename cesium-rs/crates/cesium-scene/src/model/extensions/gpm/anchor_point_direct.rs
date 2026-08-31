//! Ported from `packages/engine/Source/Scene/Model/Extensions/Gpm/AnchorPointDirect.js`.

use cesium_core::cartesian3::Cartesian3;

/// Metadata for one stored anchor point using direct storage.
///
/// This reflects the `anchronPointDirect` definition of the
/// NGA_gpm_local glTF extension.
#[derive(Clone, Debug, PartialEq)]
pub struct AnchorPointDirect {
    /// Anchor point geographic coordinates in meters as
    /// X/Easting, Y/Northing, Z/HAE.
    position: Cartesian3,
    /// The delta-x delta-y delta-z adjustment values in meters per
    /// anchor point.
    adjustment_params: Cartesian3,
}

impl AnchorPointDirect {
    /// Creates a new `AnchorPointDirect`.
    ///
    /// Port of the `AnchorPointDirect(options)` constructor.
    pub fn new(position: Cartesian3, adjustment_params: Cartesian3) -> Self {
        Self {
            position,
            adjustment_params,
        }
    }

    /// Anchor point geographic coordinates in meters as
    /// X/Easting, Y/Northing, Z/HAE (port of the `position` getter).
    pub fn position(&self) -> &Cartesian3 {
        &self.position
    }

    /// The delta-x delta-y delta-z adjustment values in meters per
    /// anchor point (port of the `adjustmentParams` getter).
    pub fn adjustment_params(&self) -> &Cartesian3 {
        &self.adjustment_params
    }
}
