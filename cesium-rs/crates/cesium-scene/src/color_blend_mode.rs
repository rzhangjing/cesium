//! Ported from `packages/engine/Source/Scene/ColorBlendMode.js`.

/// Color blend mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ColorBlendMode {
    /// Highlight.
    Highlight = 0,
    /// Replace.
    Replace = 1,
    /// Mix.
    Mix = 2,
}
