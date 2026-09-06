//! Ported from `packages/engine/Source/Scene/MetadataSemantic.js`.
//!
//! Well-known metadata semantic names from EXT_structural_metadata.

/// Well-known metadata semantic names.
///
/// Each constant corresponds to a semantic defined by the
/// EXT_structural_metadata / EXT_feature_metadata extensions.
/// Mirrors CesiumJS `MetadataSemantic` (~100 lines).
pub struct MetadataSemantic;

impl MetadataSemantic {
    // ── Tileset-level semantics ────────────────────────────────

    /// Unique identifier.
    pub const ID: &'static str = "ID";
    /// Human-readable name.
    pub const NAME: &'static str = "NAME";
    /// Description text.
    pub const DESCRIPTION: &'static str = "DESCRIPTION";
    /// Number of tiles in the tileset.
    pub const TILESET_TILE_COUNT: &'static str = "TILESET_TILE_COUNT";

    // ── Tile-level semantics ───────────────────────────────────

    /// Tile bounding box (12 floats: center + half-axes).
    pub const TILE_BOUNDING_BOX: &'static str = "TILE_BOUNDING_BOX";
    /// Tile bounding region (6 doubles: west/south/east/north/minH/maxH).
    pub const TILE_BOUNDING_REGION: &'static str = "TILE_BOUNDING_REGION";
    /// Tile bounding sphere (4 floats: center xyz + radius).
    pub const TILE_BOUNDING_SPHERE: &'static str = "TILE_BOUNDING_SPHERE";
    /// Tile minimum height.
    pub const TILE_MINIMUM_HEIGHT: &'static str = "TILE_MINIMUM_HEIGHT";
    /// Tile maximum height.
    pub const TILE_MAXIMUM_HEIGHT: &'static str = "TILE_MAXIMUM_HEIGHT";
    /// Tile horizon occlusion point (VEC3).
    pub const TILE_HORIZON_OCCLUSION_POINT: &'static str = "TILE_HORIZON_OCCLUSION_POINT";
    /// Tile geometric error.
    pub const TILE_GEOMETRIC_ERROR: &'static str = "TILE_GEOMETRIC_ERROR";

    // ── Content-level semantics ────────────────────────────────

    /// Content bounding box.
    pub const CONTENT_BOUNDING_BOX: &'static str = "CONTENT_BOUNDING_BOX";
    /// Content bounding region.
    pub const CONTENT_BOUNDING_REGION: &'static str = "CONTENT_BOUNDING_REGION";
    /// Content bounding sphere.
    pub const CONTENT_BOUNDING_SPHERE: &'static str = "CONTENT_BOUNDING_SPHERE";
    /// Content minimum height.
    pub const CONTENT_MINIMUM_HEIGHT: &'static str = "CONTENT_MINIMUM_HEIGHT";
    /// Content maximum height.
    pub const CONTENT_MAXIMUM_HEIGHT: &'static str = "CONTENT_MAXIMUM_HEIGHT";
    /// Content horizon occlusion point.
    pub const CONTENT_HORIZON_OCCLUSION_POINT: &'static str = "CONTENT_HORIZON_OCCLUSION_POINT";

    /// Returns whether the given string is a known metadata semantic.
    pub fn is_known(semantic: &str) -> bool {
        matches!(
            semantic,
            Self::ID
                | Self::NAME
                | Self::DESCRIPTION
                | Self::TILESET_TILE_COUNT
                | Self::TILE_BOUNDING_BOX
                | Self::TILE_BOUNDING_REGION
                | Self::TILE_BOUNDING_SPHERE
                | Self::TILE_MINIMUM_HEIGHT
                | Self::TILE_MAXIMUM_HEIGHT
                | Self::TILE_HORIZON_OCCLUSION_POINT
                | Self::TILE_GEOMETRIC_ERROR
                | Self::CONTENT_BOUNDING_BOX
                | Self::CONTENT_BOUNDING_REGION
                | Self::CONTENT_BOUNDING_SPHERE
                | Self::CONTENT_MINIMUM_HEIGHT
                | Self::CONTENT_MAXIMUM_HEIGHT
                | Self::CONTENT_HORIZON_OCCLUSION_POINT
        )
    }

    /// Returns whether the semantic is tile-level.
    pub fn is_tile_semantic(semantic: &str) -> bool {
        semantic.starts_with("TILE_")
    }

    /// Returns whether the semantic is content-level.
    pub fn is_content_semantic(semantic: &str) -> bool {
        semantic.starts_with("CONTENT_")
    }
}
