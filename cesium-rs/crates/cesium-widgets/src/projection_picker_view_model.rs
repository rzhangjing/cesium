//! Ported from `packages/widgets/Source/ProjectionPicker/ProjectionPickerViewModel.js`.

/// The view model for the ProjectionPicker widget.
///
/// Allows switching between perspective and orthographic projections.
pub struct ProjectionPickerViewModel {
    /// Whether the current projection is perspective (true) or orthographic (false).
    pub is_perspective: bool,
    /// The tooltip text.
    pub tooltip: String,
}

impl ProjectionPickerViewModel {
    /// Creates a new projection picker view model.
    pub fn new() -> Self {
        Self {
            is_perspective: true,
            tooltip: String::from("Projection"),
        }
    }

    /// Switches to perspective projection.
    pub fn switch_to_perspective(&mut self) { self.is_perspective = true; }

    /// Switches to orthographic projection.
    pub fn switch_to_orthographic(&mut self) { self.is_perspective = false; }
}

impl Default for ProjectionPickerViewModel {
    fn default() -> Self { Self::new() }
}
