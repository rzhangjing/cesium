//! Ported from `packages/engine/Source/Scene/ColorBlendMode.js`.
//!
//! Determines how feature colors are blended with the base color.

/// Color blend mode for 3D Tiles features.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ColorBlendMode {
    /// Highlight — the feature color is multiplied by the blend amount.
    Highlight = 0,
    /// Replace — the feature color replaces the base color entirely.
    Replace = 1,
    /// Mix — the feature color is linearly interpolated with the base color.
    Mix = 2,
}

impl ColorBlendMode {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Highlight),
            1 => Some(Self::Replace),
            2 => Some(Self::Mix),
            _ => None,
        }
    }

    /// Returns the integer value.
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    /// Computes the blend factor for the given mode and amount.
    ///
    /// - `Highlight` → 0.0 (no replacement)
    /// - `Replace` → 1.0 (full replacement)
    /// - `Mix` → clamp(amount, EPSILON, 1.0)
    pub fn get_color_blend(amount: f64) -> f64 {
        // Default to MIX behavior; the caller selects the mode.
        amount.clamp(1e-14, 1.0)
    }

    /// Computes the blend factor for this specific mode and the given amount.
    pub fn blend_factor(&self, amount: f64) -> f64 {
        match self {
            Self::Highlight => 0.0,
            Self::Replace => 1.0,
            Self::Mix => amount.clamp(1e-14, 1.0),
        }
    }
}

impl Default for ColorBlendMode {
    fn default() -> Self {
        Self::Highlight
    }
}
