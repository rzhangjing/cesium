//! Ported from `packages/engine/Source/DataSources/KmlLookAt.js`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::heading_pitch_range::HeadingPitchRange;

/// Representation of `<LookAt>` from KML (mirror of `KmlLookAt`).
#[derive(Clone, Debug)]
pub struct KmlLookAt {
    /// The look-at view point.
    pub position: Cartesian3,
    /// The heading/pitch/range relative to the view point.
    pub heading_pitch_range: HeadingPitchRange,
}

impl KmlLookAt {
    /// Creates a new KML LookAt.
    pub fn new(position: Cartesian3, heading_pitch_range: HeadingPitchRange) -> Self {
        Self {
            position,
            heading_pitch_range,
        }
    }
}
