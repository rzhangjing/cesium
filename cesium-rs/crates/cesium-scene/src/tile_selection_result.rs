//! Ported from `packages/engine/Source/Scene/TileSelectionResult.js`.

/// Indicates what happened the last time this tile was visited for selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TileSelectionResult {
    /// There was no selection result.
    None = 0,
    /// This tile was deemed not visible and culled.
    Culled = 1,
    /// The tile was selected for rendering.
    Rendered = 2,
    /// This tile did not meet the required screen-space error and was refined.
    Refined = 3,
    /// This tile was originally rendered, but got kicked out in favor of an ancestor.
    RenderedAndKicked = 6,  // 2 | 4
    /// This tile was originally refined, but its descendants got kicked out.
    RefinedAndKicked = 7,   // 3 | 4
    /// This tile was culled but still needs to be loaded.
    CulledButNeeded = 9,    // 1 | 8
}

impl TileSelectionResult {
    /// Returns the integer value.
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    /// Determines if a selection result indicates that this tile or its
    /// descendants were rendered (or refined, which implies rendering).
    pub fn was_rendered(self) -> bool {
        matches!(
            self,
            Self::Rendered | Self::Refined | Self::RenderedAndKicked | Self::RefinedAndKicked
        )
    }

    /// Determines if a selection result indicates that this tile was kicked
    /// out of the render list.
    pub fn was_kicked(self) -> bool {
        matches!(self, Self::RenderedAndKicked | Self::RefinedAndKicked)
    }
}
