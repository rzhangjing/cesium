//! Ported from `packages/engine/Source/Renderer/CubeMapFace.js`.
//!
//! Enumerates the six faces of a cube map.

/// The six faces of a cube map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CubeMapFace {
    /// Positive X face.
    PositiveX = 0,
    /// Negative X face.
    NegativeX = 1,
    /// Positive Y face.
    PositiveY = 2,
    /// Negative Y face.
    NegativeY = 3,
    /// Positive Z face.
    PositiveZ = 4,
    /// Negative Z face.
    NegativeZ = 5,
}

impl CubeMapFace {
    /// Returns the number of faces.
    pub const COUNT: usize = 6;

    /// Returns all face values.
    pub fn all() -> [CubeMapFace; 6] {
        [
            CubeMapFace::PositiveX,
            CubeMapFace::NegativeX,
            CubeMapFace::PositiveY,
            CubeMapFace::NegativeY,
            CubeMapFace::PositiveZ,
            CubeMapFace::NegativeZ,
        ]
    }
}
