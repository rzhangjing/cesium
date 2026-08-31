//! Ported from `packages/engine/Source/DataSources/KmlTourFlyTo.js`.

use crate::kml_tour::KmlTourView;

/// A KML tour entry that flies the camera to a specified viewpoint
/// (mirror of `KmlTourFlyTo`).
///
/// DEVIATION (playback): the JS `play`/`stop` camera tweening against the
/// scene is not materialized; only the parsed value model is kept.
#[derive(Clone, Debug)]
pub struct KmlTourFlyTo {
    /// The duration in seconds.
    pub duration: Option<f64>,
    /// The fly-to mode (`bounce` or `smooth`), if provided.
    pub fly_to_mode: Option<String>,
    /// The target view (LookAt preferred over Camera, mirroring the JS
    /// `t.kml.lookAt || t.kml.camera` selection).
    pub view: Option<KmlTourView>,
}

impl KmlTourFlyTo {
    /// Creates a new fly-to tour entry.
    pub fn new(
        duration: Option<f64>,
        fly_to_mode: Option<String>,
        view: Option<KmlTourView>,
    ) -> Self {
        Self {
            duration,
            fly_to_mode,
            view,
        }
    }
}
