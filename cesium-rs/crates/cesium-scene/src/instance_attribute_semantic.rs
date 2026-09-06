//! Ported from `packages/engine/Source/Scene/InstanceAttributeSemantic.js`.
//!
//! Instance attribute semantic for model instancing.

/// Instance attribute semantic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum InstanceAttributeSemantic {
    /// Position.
    Position = 0,
    /// Rotation.
    Rotation = 1,
    /// Scale.
    Scale = 2,
    /// Translation.
    Translation = 3,
}

impl InstanceAttributeSemantic {
    /// Converts from an integer value.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Position),
            1 => Some(Self::Rotation),
            2 => Some(Self::Scale),
            3 => Some(Self::Translation),
            _ => None,
        }
    }

    /// Returns the integer value.
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    /// Returns the CesiumJS string name.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Position => "POSITION",
            Self::Rotation => "ROTATION",
            Self::Scale => "SCALE",
            Self::Translation => "TRANSLATION",
        }
    }

    /// Parses from a glTF semantic string (e.g. `_FEATURE_ID_0` → None).
    ///
    /// CesiumJS `InstanceAttributeSemantic.fromGltfSemantic` strips the
    /// trailing index from glTF attribute semantics.
    pub fn from_gltf_semantic(gltf_semantic: &str) -> Option<Self> {
        // Strip trailing _N suffix
        let base = gltf_semantic.rsplit_once('_').map_or(gltf_semantic, |(b, _)| {
            // Check if the suffix is numeric
            if b.is_empty() { gltf_semantic } else { b }
        });
        match base {
            "TRANSLATION" => Some(Self::Translation),
            "ROTATION" => Some(Self::Rotation),
            "SCALE" => Some(Self::Scale),
            _ => None,
        }
    }
}

impl Default for InstanceAttributeSemantic {
    fn default() -> Self {
        Self::Position
    }
}
