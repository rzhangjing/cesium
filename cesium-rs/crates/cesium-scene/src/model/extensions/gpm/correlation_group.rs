//! Ported from `packages/engine/Source/Scene/Model/Extensions/Gpm/CorrelationGroup.js`.

use cesium_core::cartesian3::Cartesian3;

use crate::model::extensions::gpm::spdcf::Spdcf;

/// Metadata identifying parameters using same correlation modeling and
/// associated correlation parameters.
///
/// This reflects the `correlationGroup` definition of the
/// NGA_gpm_local glTF extension.
#[derive(Clone, Debug, PartialEq)]
pub struct CorrelationGroup {
    /// Array of 3 booleans indicating if parameters delta-x delta-y
    /// delta-z are used in the correlation group.
    group_flags: Vec<bool>,
    /// Rotations in milliradians about X, Y, Z axes, respectively.
    rotation_thetas: Cartesian3,
    /// Array of 3 sets of SPDCF parameters, for the U, V, W directions,
    /// respectively.
    params: Vec<Spdcf>,
}

impl CorrelationGroup {
    /// Creates a new `CorrelationGroup`.
    ///
    /// Port of the `CorrelationGroup(options)` constructor.
    pub fn new(group_flags: Vec<bool>, rotation_thetas: Cartesian3, params: Vec<Spdcf>) -> Self {
        Self {
            group_flags,
            rotation_thetas,
            params,
        }
    }

    /// Array of 3 booleans indicating if parameters delta-x delta-y
    /// delta-z are used in the correlation group
    /// (port of the `groupFlags` getter).
    pub fn group_flags(&self) -> &[bool] {
        &self.group_flags
    }

    /// Rotations in milliradians about X, Y, Z axes, respectively
    /// (port of the `rotationThetas` getter).
    pub fn rotation_thetas(&self) -> &Cartesian3 {
        &self.rotation_thetas
    }

    /// Array of 3 sets of SPDCF parameters, for the U, V, W directions,
    /// respectively (port of the `params` getter).
    pub fn params(&self) -> &[Spdcf] {
        &self.params
    }
}
