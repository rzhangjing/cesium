//! Ported from `packages/engine/Source/Scene/BatchTexture.js`.
//!
//! A texture that encodes per-feature show/alpha and color data for
//! batch ID rendering.

/// A texture used for batch ID rendering.
///
/// Stores per-feature show/alpha properties and RGBA color overrides.
/// Mirrors CesiumJS `BatchTexture` (~450 lines).
pub struct BatchTexture {
    /// The number of features in this batch.
    pub features_length: u32,
    /// Per-feature show (1 byte) + alpha (1 byte) data.
    /// Layout: [show_0, alpha_0, show_1, alpha_1, ...].
    show_alpha_properties: Vec<u8>,
    /// Per-feature RGBA color data (4 bytes each).
    /// Layout: [r_0, g_0, b_0, a_0, r_1, ...].
    batch_values: Vec<u8>,
    /// Whether the batch values have been modified since last upload.
    pub batch_values_dirty: bool,
    /// The number of translucent features.
    pub translucent_features_length: u32,
}

impl BatchTexture {
    /// Creates a new `BatchTexture`.
    pub fn new(features_length: u32) -> Self {
        let fl = features_length as usize;
        Self {
            features_length,
            show_alpha_properties: vec![0xFF; 2 * fl], // all visible, full alpha
            batch_values: vec![0xFF; 4 * fl], // all white
            batch_values_dirty: false,
            translucent_features_length: 0,
        }
    }

    /// Gets the show state for a feature.
    pub fn get_show(&self, batch_id: usize) -> bool {
        self.show_alpha_properties
            .get(batch_id * 2)
            .map_or(true, |&v| v != 0)
    }

    /// Sets the show state for a feature.
    pub fn set_show(&mut self, batch_id: usize, show: bool) {
        if let Some(slot) = self.show_alpha_properties.get_mut(batch_id * 2) {
            *slot = if show { 0xFF } else { 0x00 };
            self.batch_values_dirty = true;
        }
    }

    /// Sets all features' show state.
    pub fn set_all_show(&mut self, show: bool) {
        let val = if show { 0xFF } else { 0x00 };
        for i in (0..self.show_alpha_properties.len()).step_by(2) {
            self.show_alpha_properties[i] = val;
        }
        self.batch_values_dirty = true;
    }

    /// Gets the RGBA color for a feature.
    pub fn get_color(&self, batch_id: usize) -> [u8; 4] {
        let offset = batch_id * 4;
        if offset + 3 < self.batch_values.len() {
            [
                self.batch_values[offset],
                self.batch_values[offset + 1],
                self.batch_values[offset + 2],
                self.batch_values[offset + 3],
            ]
        } else {
            [0xFF, 0xFF, 0xFF, 0xFF]
        }
    }

    /// Sets the RGBA color for a feature.
    pub fn set_color(&mut self, batch_id: usize, color: [u8; 4]) {
        let offset = batch_id * 4;
        if offset + 3 < self.batch_values.len() {
            self.batch_values[offset..offset + 4].copy_from_slice(&color);
            self.batch_values_dirty = true;
        }
    }

    /// Returns the total byte length of the batch data.
    pub fn byte_length(&self) -> usize {
        self.show_alpha_properties.len() + self.batch_values.len()
    }
}

impl Default for BatchTexture {
    fn default() -> Self { Self::new(0) }
}
