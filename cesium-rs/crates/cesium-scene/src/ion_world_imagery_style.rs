//! Ported from `packages/engine/Source/Scene/IonWorldImageryStyle.js`.

/// Ion world imagery style.
pub struct IonWorldImageryStyle {
    _private: (),
}

impl IonWorldImageryStyle {
    /// Creates a new IonWorldImageryStyle.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for IonWorldImageryStyle {
    fn default() -> Self { Self::new() }
}
