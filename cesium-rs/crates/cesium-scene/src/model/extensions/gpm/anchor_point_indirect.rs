//! Ported from `packages/engine/Source/Scene/Model/Extensions/Gpm/AnchorPointIndirect.js`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::matrix3::Matrix3;

/// Metadata for one stored anchor point.
///
/// This reflects the `anchronPointIndirect` definition of the
/// NGA_gpm_local glTF extension.
#[derive(Clone, Debug, PartialEq)]
pub struct AnchorPointIndirect {
    /// Anchor point geographic coordinates in meters as
    /// X/Easting, Y/Northing, Z/HAE.
    position: Cartesian3,
    /// The delta-x delta-y delta-z adjustment values in meters per
    /// anchor point.
    adjustment_params: Cartesian3,
    /// The 3x3 covariance matrix.
    covariance_matrix: Matrix3,
}

impl AnchorPointIndirect {
    /// Creates a new `AnchorPointIndirect`.
    ///
    /// Port of the `AnchorPointIndirect(options)` constructor.
    pub fn new(
        position: Cartesian3,
        adjustment_params: Cartesian3,
        covariance_matrix: Matrix3,
    ) -> Self {
        Self {
            position,
            adjustment_params,
            covariance_matrix,
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

    /// The 3x3 covariance matrix (port of the `covarianceMatrix` getter).
    pub fn covariance_matrix(&self) -> &Matrix3 {
        &self.covariance_matrix
    }
}
