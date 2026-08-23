//! Ported from `packages/widgets/Source/SelectionIndicator/SelectionIndicatorViewModel.js`.

/// The view model for the SelectionIndicator widget.
///
/// Shows a visual indicator at the position of the selected entity.
pub struct SelectionIndicatorViewModel {
    /// The title text.
    pub title: String,
    /// Whether the indicator is visible.
    pub is_visible: bool,
}

impl SelectionIndicatorViewModel {
    /// Creates a new selection indicator view model.
    pub fn new() -> Self {
        Self {
            title: String::new(),
            is_visible: false,
        }
    }

    /// Shows the indicator at the given entity position.
    pub fn animate_appear(&mut self) {
        self.is_visible = true;
    }

    /// Hides the indicator.
    pub fn animate_disappear(&mut self) {
        self.is_visible = false;
    }
}

impl Default for SelectionIndicatorViewModel {
    fn default() -> Self { Self::new() }
}
