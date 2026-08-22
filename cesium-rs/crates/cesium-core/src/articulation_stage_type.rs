//! Ported from `packages/engine/Source/Core/ArticulationStageType.js`.

/// An enum describing the type of motion that is defined by an articulation stage
/// in the AGI_articulations extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArticulationStageType {
    XTranslate,
    YTranslate,
    ZTranslate,
    XRotate,
    YRotate,
    ZRotate,
    XScale,
    YScale,
    ZScale,
    UniformScale,
}

impl ArticulationStageType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::XTranslate => "xTranslate",
            Self::YTranslate => "yTranslate",
            Self::ZTranslate => "zTranslate",
            Self::XRotate => "xRotate",
            Self::YRotate => "yRotate",
            Self::ZRotate => "zRotate",
            Self::XScale => "xScale",
            Self::YScale => "yScale",
            Self::ZScale => "zScale",
            Self::UniformScale => "uniformScale",
        }
    }
}
