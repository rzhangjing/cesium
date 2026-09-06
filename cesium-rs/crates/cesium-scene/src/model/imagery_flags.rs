//! Ported from `packages/engine/Source/Scene/Model/ImageryFlags.js`.

/// A class containing a set of flags indicating which parts of an
/// `ImageryLayer` need to be processed.
///
/// This is used in the `ImageryPipelineStage` to decide the structure
/// of the function that blends the imagery texture information with
/// the previous pixels.
#[derive(Debug, Clone, Default)]
pub struct ImageryFlags {
    /// Whether any imagery layer has a non-default alpha.
    pub alpha: bool,
    /// Whether any imagery layer has a non-default brightness.
    pub brightness: bool,
    /// Whether any imagery layer has a non-default contrast.
    pub contrast: bool,
    /// Whether any imagery layer has a non-default hue.
    pub hue: bool,
    /// Whether any imagery layer has a non-default saturation.
    pub saturation: bool,
    /// Whether any imagery layer has a non-default gamma.
    pub gamma: bool,
    /// Whether any imagery layer has a non-default color-to-alpha.
    pub color_to_alpha: bool,
}
