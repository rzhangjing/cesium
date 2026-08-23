//! Ported from `packages/engine/Source/Scene/ImageryState.js`.
//!
//! The combined imagery state for a single terrain tile.

use crate::imagery::Imagery;

/// The combined imagery state for a single terrain tile.
///
/// Tracks all imagery layers' contributions to a single terrain tile,
/// including which layers are loaded, loading, or failed.
pub struct ImageryState {
    /// The imagery tiles for each layer.
    pub imagery_layers: Vec<Option<Imagery>>,
    /// Whether all required imagery layers are loaded.
    pub all_loaded: bool,
    /// Whether any imagery layer has failed.
    pub any_failed: bool,
}

impl ImageryState {
    /// Creates a new ImageryState.
    pub fn new() -> Self {
        Self {
            imagery_layers: Vec::new(),
            all_loaded: false,
            any_failed: false,
        }
    }

    /// Updates the combined state from the individual imagery tiles.
    pub fn update(&mut self) {
        self.all_loaded = true;
        self.any_failed = false;
        for layer in &self.imagery_layers {
            match layer {
                Some(imagery) => {
                    if !imagery.is_ready() {
                        self.all_loaded = false;
                    }
                    if imagery.failed {
                        self.any_failed = true;
                    }
                }
                None => {
                    self.all_loaded = false;
                }
            }
        }
    }

    /// Returns whether all imagery is ready.
    pub fn is_ready(&self) -> bool {
        self.all_loaded && !self.any_failed
    }
}

impl Default for ImageryState {
    fn default() -> Self { Self::new() }
}
