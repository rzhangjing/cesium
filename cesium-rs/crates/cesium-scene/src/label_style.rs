//! Ported from `packages/engine/Source/Scene/LabelStyle.js`.

/// The style of a label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum LabelStyle {
    /// Fill only.
    Fill = 0,
    /// Outline only.
    Outline = 1,
    /// Fill and outline.
    FillAndOutline = 2,
}
