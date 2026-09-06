//! Ported from `packages/engine/Source/Scene/Model/ModelType.js`.

/// An enum to distinguish the different uses for `Model`,
/// which include individual glTF models, and various 3D Tiles formats
/// (including glTF via `3DTILES_content_gltf`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelType {
    /// An individual glTF model.
    Gltf,
    /// A glTF model used as tile content in a 3D Tileset via
    /// `3DTILES_content_gltf`.
    TileGltf,
    /// A 3D Tiles 1.0 Batched 3D Model.
    TileB3dm,
    /// A 3D Tiles 1.0 Instanced 3D Model.
    TileI3dm,
    /// A 3D Tiles 1.0 Point Cloud.
    TilePnts,
    /// GeoJSON content for `MAXAR_content_geojson` extension.
    TileGeojson,
}

impl ModelType {
    /// Returns the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Gltf => "GLTF",
            Self::TileGltf => "TILE_GLTF",
            Self::TileB3dm => "B3DM",
            Self::TileI3dm => "I3DM",
            Self::TilePnts => "PNTS",
            Self::TileGeojson => "TILE_GEOJSON",
        }
    }

    /// Parses from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "GLTF" => Some(Self::Gltf),
            "TILE_GLTF" => Some(Self::TileGltf),
            "B3DM" => Some(Self::TileB3dm),
            "I3DM" => Some(Self::TileI3dm),
            "PNTS" => Some(Self::TilePnts),
            "TILE_GEOJSON" => Some(Self::TileGeojson),
            _ => None,
        }
    }

    /// Check if a model is used for 3D Tiles.
    pub fn is_3d_tiles(&self) -> bool {
        matches!(
            self,
            Self::TileGltf
                | Self::TileB3dm
                | Self::TileI3dm
                | Self::TilePnts
                | Self::TileGeojson
        )
    }
}
