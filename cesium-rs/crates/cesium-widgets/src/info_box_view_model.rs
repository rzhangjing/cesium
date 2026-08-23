//! Ported from `packages/widgets/Source/InfoBox/InfoBoxViewModel.js`.

/// The view model for the InfoBox widget.
///
/// Displays information about the selected entity.
pub struct InfoBoxViewModel {
    /// The title text.
    pub title: String,
    /// The description (HTML).
    pub description: String,
    /// Whether the info box is visible.
    pub is_visible: bool,
}

impl InfoBoxViewModel {
    /// Creates a new info box view model.
    pub fn new() -> Self {
        Self {
            title: String::new(),
            description: String::new(),
            is_visible: false,
        }
    }
}

impl Default for InfoBoxViewModel {
    fn default() -> Self { Self::new() }
}
