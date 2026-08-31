//! Ported from `packages/engine/Source/Scene/Model/Extensions/Gpm/StorageType.js`.

/// An enum of storage types for covariance information.
///
/// This reflects the `gltfGpmLocal.storageType` definition of the
/// NGA_gpm_local glTF extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StorageType {
    /// Store the full error covariance of the anchor points, to include
    /// the cross-covariance terms.
    Direct,
    /// A full covariance matrix is stored for each of the anchor points.
    /// However, in this case the cross-covariance terms are not directly
    /// stored, but can be computed by a set of spatial correlation
    /// function parameters which are stored in the metadata.
    Indirect,
}

impl StorageType {
    /// The string value used inside the glTF extension JSON (`"Direct"`).
    pub const DIRECT: &'static str = "Direct";
    /// The string value used inside the glTF extension JSON (`"Indirect"`).
    pub const INDIRECT: &'static str = "Indirect";

    /// Returns the string representation of this storage type, mirroring
    /// the frozen string constants of the JS enum.
    pub fn as_str(self) -> &'static str {
        match self {
            StorageType::Direct => Self::DIRECT,
            StorageType::Indirect => Self::INDIRECT,
        }
    }

    /// Parses a storage type from its glTF JSON string representation.
    /// Returns `None` for any other value (the loader raises a
    /// `RuntimeError` in that case, mirroring `GltfGpmLoader.load`).
    pub fn from_str(value: &str) -> Option<StorageType> {
        match value {
            Self::DIRECT => Some(StorageType::Direct),
            Self::INDIRECT => Some(StorageType::Indirect),
            _ => None,
        }
    }
}

impl Default for StorageType {
    fn default() -> Self {
        StorageType::Direct
    }
}
