//! Ported from `packages/widgets/Source/VRButton/VRButtonViewModel.js`.

/// The view model for the VRButton widget.
pub struct VrButtonViewModel {
    /// Whether VR mode is currently active.
    pub is_vr_enabled: bool,
    /// Whether VR is supported.
    pub is_vr_supported: bool,
    /// The tooltip text.
    pub tooltip: String,
}

impl VrButtonViewModel {
    /// Creates a new VR button view model.
    pub fn new() -> Self {
        Self {
            is_vr_enabled: false,
            is_vr_supported: false,
            tooltip: String::from("VR"),
        }
    }

    /// Toggles VR mode.
    pub fn toggle_vr(&mut self) {
        if self.is_vr_supported {
            self.is_vr_enabled = !self.is_vr_enabled;
        }
    }
}

impl Default for VrButtonViewModel {
    fn default() -> Self { Self::new() }
}
