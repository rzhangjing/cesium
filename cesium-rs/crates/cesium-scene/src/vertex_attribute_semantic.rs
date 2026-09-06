//! Ported from `packages/engine/Source/Scene/VertexAttributeSemantic.js`.
//!
//! Semantic meaning of a vertex attribute.

/// The semantic meaning of a vertex attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VertexAttributeSemantic {
    /// Per-vertex position.
    Position,
    /// Per-vertex normal.
    Normal,
    /// Per-vertex tangent.
    Tangent,
    /// Per-vertex texture coordinates.
    TexCoord,
    /// Per-vertex color.
    Color,
    /// Per-vertex joint IDs for skinning.
    Joints,
    /// Per-vertex joint weights for skinning.
    Weights,
    /// Per-vertex feature ID.
    FeatureId,
    /// Gaussian splat scale.
    Scale,
    /// Gaussian splat rotation.
    Rotation,
    /// Per-vertex cumulative distance for line patterning.
    CumulativeDistance,
}

impl VertexAttributeSemantic {
    /// Returns whether the semantic can have a set index.
    ///
    /// Mirrors `VertexAttributeSemantic.hasSetIndex(semantic)`.
    pub fn has_set_index(&self) -> bool {
        matches!(
            self,
            Self::TexCoord
                | Self::Color
                | Self::Joints
                | Self::Weights
                | Self::FeatureId
                | Self::Scale
                | Self::Rotation
        )
    }

    /// Gets the semantic matching the glTF semantic string.
    ///
    /// Mirrors `VertexAttributeSemantic.fromGltfSemantic(gltfSemantic)`.
    pub fn from_gltf_semantic(gltf_semantic: &str) -> Option<Self> {
        // Strip the set index from the semantic (e.g. TEXCOORD_0 → TEXCOORD).
        let base = if let Some(pos) = gltf_semantic.rfind('_') {
            let suffix = &gltf_semantic[pos + 1..];
            if suffix.chars().all(|c| c.is_ascii_digit()) {
                &gltf_semantic[..pos]
            } else {
                gltf_semantic
            }
        } else {
            gltf_semantic
        };

        match base {
            "POSITION" => Some(Self::Position),
            "NORMAL" => Some(Self::Normal),
            "TANGENT" => Some(Self::Tangent),
            "TEXCOORD" => Some(Self::TexCoord),
            "COLOR" => Some(Self::Color),
            "JOINTS" => Some(Self::Joints),
            "WEIGHTS" => Some(Self::Weights),
            "_FEATURE_ID" => Some(Self::FeatureId),
            "KHR_gaussian_splatting:SCALE" | "_SCALE" => Some(Self::Scale),
            "KHR_gaussian_splatting:ROTATION" | "_ROTATION" => Some(Self::Rotation),
            "BENTLEY_materials_line_style:CUMULATIVE_DISTANCE" => {
                Some(Self::CumulativeDistance)
            }
            _ => None,
        }
    }

    /// Gets the GLSL type for the given semantic.
    ///
    /// Mirrors `VertexAttributeSemantic.getGlslType(semantic)`.
    pub fn get_glsl_type(&self) -> &'static str {
        match self {
            Self::Position | Self::Normal | Self::Tangent | Self::Scale => "vec3",
            Self::TexCoord => "vec2",
            Self::Color | Self::Weights | Self::Rotation => "vec4",
            Self::Joints => "ivec4",
            Self::FeatureId => "int",
            Self::CumulativeDistance => "float",
        }
    }

    /// Gets the variable name for the given semantic and optional set index.
    ///
    /// Mirrors `VertexAttributeSemantic.getVariableName(semantic, setIndex)`.
    pub fn get_variable_name(&self, set_index: Option<u32>) -> String {
        let base = match self {
            Self::Position => "positionMC",
            Self::Normal => "normalMC",
            Self::Tangent => "tangentMC",
            Self::TexCoord => "texCoord",
            Self::Color => "color",
            Self::Joints => "joints",
            Self::Weights => "weights",
            Self::FeatureId => "featureId",
            Self::Scale => "scale",
            Self::Rotation => "rotation",
            Self::CumulativeDistance => "cumulativeDistance",
        };
        match set_index {
            Some(idx) => format!("{base}_{idx}"),
            None => base.to_string(),
        }
    }
}
