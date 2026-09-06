//! Ported from `packages/engine/Source/Scene/ITwinData.js`.

/// iTwin data.
///
/// Manages data from Bentley iTwin digital twin platform.
pub struct ITwinData {
    /// The iTwin ID.
    pub itwin_id: String,
    /// Whether the data is loaded.
    pub loaded: bool,
}

impl ITwinData {
    /// Creates a new ITwinData.
    pub fn new() -> Self { Self { itwin_id: String::new(), loaded: false } }
}

impl Default for ITwinData {
    fn default() -> Self { Self::new() }
}
