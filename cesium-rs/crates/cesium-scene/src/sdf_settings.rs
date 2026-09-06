//! Ported from `packages/engine/Source/Scene/SdfSettings.js`.

/// Settings for the generation of signed distance field glyphs.
pub struct SdfSettings;

impl SdfSettings {
    /// The font size in pixels.
    pub const FONT_SIZE: f64 = 48.0;
    /// Whitespace padding around glyphs.
    pub const PADDING: f64 = 10.0;
    /// How many pixels around the glyph shape to use for encoding distance.
    pub const RADIUS: f64 = 8.0;
    /// The cutoff value for the SDF.
    pub const CUTOFF: f64 = 0.5;
}
