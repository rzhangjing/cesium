//! Ported from `packages/engine/Source/Scene/Model/CustomShaderMode.js`.

/// An enum describing how the `CustomShader` will be added to the
/// fragment shader. This determines how the shader interacts with the material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CustomShaderMode {
    /// The custom shader will be used to modify the results of the material stage
    /// before lighting is applied.
    ModifyMaterial,
    /// The custom shader will be used instead of the material stage. This is a hint
    /// to optimize out the material processing code.
    ReplaceMaterial,
}

impl CustomShaderMode {
    /// Returns the string representation (e.g. `"MODIFY_MATERIAL"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ModifyMaterial => "MODIFY_MATERIAL",
            Self::ReplaceMaterial => "REPLACE_MATERIAL",
        }
    }

    /// Parses from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "MODIFY_MATERIAL" => Some(Self::ModifyMaterial),
            "REPLACE_MATERIAL" => Some(Self::ReplaceMaterial),
            _ => None,
        }
    }

    /// Convert the shader mode to an uppercase identifier for use in GLSL `#define`
    /// directives. For example: `#define CUSTOM_SHADER_MODIFY_MATERIAL`.
    pub fn get_define_name(&self) -> String {
        format!("CUSTOM_SHADER_{}", self.as_str())
    }
}
