//! Ported from `packages/engine/Source/Scene/Cesium3DTileContentType.js`.
//!
//! An enum to indicate the different types of `Cesium3DTileContent`. For
//! binary files, the enum value is the magic number of the binary file
//! unless otherwise noted. For JSON files, the enum value is a unique name
//! for internal use (the Rust analogue of the JS string constants).

/// The type of 3D tile content (JS `Cesium3DTileContentType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cesium3DTileContentType {
    /// A Batched 3D Model. Binary format with magic number `b3dm`.
    Batched3DModel,
    /// An Instanced 3D Model. Binary format with magic number `i3dm`.
    Instanced3DModel,
    /// A Composite model. Binary format with magic number `cmpt`.
    Composite,
    /// A Point Cloud model. Binary format with magic number `pnts`.
    PointCloud,
    /// Vector tiles. Binary format with magic number `vctr`.
    Vector,
    /// Geometry tiles. Binary format with magic number `geom`.
    Geometry,
    /// A glTF model in JSON + external BIN form (treated as JSON).
    Gltf,
    /// The binary form of a glTF file (magic `glTF`, stored as `glb`).
    GltfBinary,
    /// Implicit tiling availability bitstreams (binary subtree, `subt`).
    ImplicitSubtree,
    /// Implicit tiling subtree represented as JSON.
    ImplicitSubtreeJson,
    /// Content referencing another tileset.json (JSON-based).
    ExternalTileset,
    /// Multiple contents (handled separately for request scheduling).
    MultipleContent,
    /// GeoJSON content for the `MAXAR_content_geojson` extension.
    GeoJson,
    /// Binary voxel content for `3DTILES_content_voxels` (`voxl`).
    VoxelBinary,
    /// JSON voxel content for `3DTILES_content_voxels`.
    VoxelJson,
}

impl Cesium3DTileContentType {
    /// The JS string constant for this content type.
    pub fn as_str(self) -> &'static str {
        match self {
            Cesium3DTileContentType::Batched3DModel => "b3dm",
            Cesium3DTileContentType::Instanced3DModel => "i3dm",
            Cesium3DTileContentType::Composite => "cmpt",
            Cesium3DTileContentType::PointCloud => "pnts",
            Cesium3DTileContentType::Vector => "vctr",
            Cesium3DTileContentType::Geometry => "geom",
            Cesium3DTileContentType::Gltf => "gltf",
            Cesium3DTileContentType::GltfBinary => "glb",
            Cesium3DTileContentType::ImplicitSubtree => "subt",
            Cesium3DTileContentType::ImplicitSubtreeJson => "subtreeJson",
            Cesium3DTileContentType::ExternalTileset => "externalTileset",
            Cesium3DTileContentType::MultipleContent => "multipleContent",
            Cesium3DTileContentType::GeoJson => "geoJson",
            Cesium3DTileContentType::VoxelBinary => "voxl",
            Cesium3DTileContentType::VoxelJson => "voxelJson",
        }
    }

    /// Looks up a content type by its JS string constant.
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "b3dm" => Some(Cesium3DTileContentType::Batched3DModel),
            "i3dm" => Some(Cesium3DTileContentType::Instanced3DModel),
            "cmpt" => Some(Cesium3DTileContentType::Composite),
            "pnts" => Some(Cesium3DTileContentType::PointCloud),
            "vctr" => Some(Cesium3DTileContentType::Vector),
            "geom" => Some(Cesium3DTileContentType::Geometry),
            "gltf" => Some(Cesium3DTileContentType::Gltf),
            "glb" => Some(Cesium3DTileContentType::GltfBinary),
            "subt" => Some(Cesium3DTileContentType::ImplicitSubtree),
            "subtreeJson" => Some(Cesium3DTileContentType::ImplicitSubtreeJson),
            "externalTileset" => Some(Cesium3DTileContentType::ExternalTileset),
            "multipleContent" => Some(Cesium3DTileContentType::MultipleContent),
            "geoJson" => Some(Cesium3DTileContentType::GeoJson),
            "voxl" => Some(Cesium3DTileContentType::VoxelBinary),
            "voxelJson" => Some(Cesium3DTileContentType::VoxelJson),
            _ => None,
        }
    }

    /// Checks if a content is one of the supported binary formats.
    /// Otherwise, the caller can assume a JSON format.
    ///
    /// JS `Cesium3DTileContentType.isBinaryFormat`.
    pub fn is_binary_format(content_type: Cesium3DTileContentType) -> bool {
        matches!(
            content_type,
            Cesium3DTileContentType::Batched3DModel
                | Cesium3DTileContentType::Instanced3DModel
                | Cesium3DTileContentType::Composite
                | Cesium3DTileContentType::PointCloud
                | Cesium3DTileContentType::Vector
                | Cesium3DTileContentType::Geometry
                | Cesium3DTileContentType::ImplicitSubtree
                | Cesium3DTileContentType::VoxelBinary
                | Cesium3DTileContentType::GltfBinary
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_binary_format_matches_js_switch() {
        // Binary formats (JS switch cases).
        assert!(Cesium3DTileContentType::is_binary_format(Cesium3DTileContentType::Batched3DModel));
        assert!(Cesium3DTileContentType::is_binary_format(Cesium3DTileContentType::Instanced3DModel));
        assert!(Cesium3DTileContentType::is_binary_format(Cesium3DTileContentType::Composite));
        assert!(Cesium3DTileContentType::is_binary_format(Cesium3DTileContentType::PointCloud));
        assert!(Cesium3DTileContentType::is_binary_format(Cesium3DTileContentType::Vector));
        assert!(Cesium3DTileContentType::is_binary_format(Cesium3DTileContentType::Geometry));
        assert!(Cesium3DTileContentType::is_binary_format(Cesium3DTileContentType::ImplicitSubtree));
        assert!(Cesium3DTileContentType::is_binary_format(Cesium3DTileContentType::VoxelBinary));
        assert!(Cesium3DTileContentType::is_binary_format(Cesium3DTileContentType::GltfBinary));
        // JSON formats (JS default branch).
        assert!(!Cesium3DTileContentType::is_binary_format(Cesium3DTileContentType::Gltf));
        assert!(!Cesium3DTileContentType::is_binary_format(Cesium3DTileContentType::ImplicitSubtreeJson));
        assert!(!Cesium3DTileContentType::is_binary_format(Cesium3DTileContentType::ExternalTileset));
        assert!(!Cesium3DTileContentType::is_binary_format(Cesium3DTileContentType::MultipleContent));
        assert!(!Cesium3DTileContentType::is_binary_format(Cesium3DTileContentType::GeoJson));
        assert!(!Cesium3DTileContentType::is_binary_format(Cesium3DTileContentType::VoxelJson));
    }

    #[test]
    fn string_constants_round_trip() {
        let all = [
            Cesium3DTileContentType::Batched3DModel,
            Cesium3DTileContentType::Instanced3DModel,
            Cesium3DTileContentType::Composite,
            Cesium3DTileContentType::PointCloud,
            Cesium3DTileContentType::Vector,
            Cesium3DTileContentType::Geometry,
            Cesium3DTileContentType::Gltf,
            Cesium3DTileContentType::GltfBinary,
            Cesium3DTileContentType::ImplicitSubtree,
            Cesium3DTileContentType::ImplicitSubtreeJson,
            Cesium3DTileContentType::ExternalTileset,
            Cesium3DTileContentType::MultipleContent,
            Cesium3DTileContentType::GeoJson,
            Cesium3DTileContentType::VoxelBinary,
            Cesium3DTileContentType::VoxelJson,
        ];
        for content_type in all {
            assert_eq!(
                Cesium3DTileContentType::from_str(content_type.as_str()),
                Some(content_type)
            );
        }
        assert_eq!(Cesium3DTileContentType::from_str("unknown"), None);
    }
}
