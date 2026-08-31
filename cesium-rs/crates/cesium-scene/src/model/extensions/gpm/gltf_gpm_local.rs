//! Ported from `packages/engine/Source/Scene/Model/Extensions/Gpm/GltfGpmLocal.js`.

use cesium_core::matrix3::Matrix3;

use crate::model::extensions::gpm::anchor_point_direct::AnchorPointDirect;
use crate::model::extensions::gpm::anchor_point_indirect::AnchorPointIndirect;
use crate::model::extensions::gpm::correlation_group::CorrelationGroup;
use crate::model::extensions::gpm::storage_type::StorageType;

/// Initialization options for the `GltfGpmLocal` constructor.
///
/// The presence requirements of the optional fields depend on the
/// storage type, exactly like the JS `ConstructorOptions`:
/// - `Indirect`: `anchor_points_indirect` and
///   `intra_tile_correlation_groups` must be present, the direct fields
///   must be `None`.
/// - `Direct`: `anchor_points_direct` and `covariance_direct` must be
///   present, the indirect fields must be `None`.
#[derive(Clone, Debug, Default)]
pub struct GltfGpmLocalOptions {
    /// The storage type: `StorageType::Direct` or `StorageType::Indirect`.
    pub storage_type: StorageType,
    /// The indirect anchor points.
    pub anchor_points_indirect: Option<Vec<AnchorPointIndirect>>,
    /// The intra-tile correlation groups.
    pub intra_tile_correlation_groups: Option<Vec<CorrelationGroup>>,
    /// The direct anchor points.
    pub anchor_points_direct: Option<Vec<AnchorPointDirect>>,
    /// The covariance of anchor point parameters.
    pub covariance_direct: Option<Matrix3>,
}

/// The GPM metadata for a Ground-Space Indirect implementation stored
/// locally (i.e. a tile and/or leaf node).
///
/// This reflects the root extension object of the NGA_gpm_local glTF
/// extension. The storage type determines the presence of the optional
/// properties:
/// - `StorageType::Indirect`: `anchor_points_indirect` and
///   `intra_tile_correlation_groups` are present.
/// - `StorageType::Direct`: `anchor_points_direct` and
///   `covariance_direct` are present.
#[derive(Clone, Debug)]
pub struct GltfGpmLocal {
    storage_type: StorageType,
    anchor_points_indirect: Option<Vec<AnchorPointIndirect>>,
    anchor_points_direct: Option<Vec<AnchorPointDirect>>,
    intra_tile_correlation_groups: Option<Vec<CorrelationGroup>>,
    covariance_direct: Option<Matrix3>,
}

impl GltfGpmLocal {
    /// Creates a new `GltfGpmLocal`.
    ///
    /// Port of the `GltfGpmLocal(options)` constructor. The presence
    /// validation of the optional fields is debug-only, mirroring
    /// `includeStart('debug', pragmas.debug)`; it panics with the JS
    /// `RuntimeError` messages when the fields are inconsistent with the
    /// storage type.
    pub fn new(options: GltfGpmLocalOptions) -> Self {
        let gltf_gpm_local = Self {
            storage_type: options.storage_type,
            anchor_points_indirect: options.anchor_points_indirect,
            anchor_points_direct: options.anchor_points_direct,
            intra_tile_correlation_groups: options.intra_tile_correlation_groups,
            covariance_direct: options.covariance_direct,
        };

        #[cfg(debug_assertions)]
        {
            if gltf_gpm_local.storage_type == StorageType::Indirect {
                if gltf_gpm_local.anchor_points_indirect.is_none() {
                    panic!(
                        "RuntimeError: The anchorPointsIndirect are required for 'Indirect' storage"
                    );
                }
                if gltf_gpm_local.intra_tile_correlation_groups.is_none() {
                    panic!(
                        "RuntimeError: The intraTileCorrelationGroups are required for 'Indirect' storage"
                    );
                }
                if gltf_gpm_local.anchor_points_direct.is_some() {
                    panic!(
                        "RuntimeError: The anchorPointsDirect must be omitted for 'Indirect' storage"
                    );
                }
                if gltf_gpm_local.covariance_direct.is_some() {
                    panic!(
                        "RuntimeError: The covarianceDirect must be omitted for 'Indirect' storage"
                    );
                }
            } else {
                // Direct storage
                if gltf_gpm_local.anchor_points_direct.is_none() {
                    panic!(
                        "RuntimeError: The anchorPointsDirect are required for 'Direct' storage"
                    );
                }
                if gltf_gpm_local.covariance_direct.is_none() {
                    panic!("RuntimeError: The covarianceDirect is required for 'Direct' storage");
                }
                if gltf_gpm_local.anchor_points_indirect.is_some() {
                    panic!(
                        "RuntimeError: The anchorPointsIndirect must be omitted for 'Direct' storage"
                    );
                }
                if gltf_gpm_local.intra_tile_correlation_groups.is_some() {
                    panic!(
                        "RuntimeError: The intraTileCorrelationGroups must be omitted for 'Direct' storage"
                    );
                }
            }
        }

        gltf_gpm_local
    }

    /// Specifies if covariance storage is indirect or direct
    /// (port of the `storageType` getter).
    pub fn storage_type(&self) -> StorageType {
        self.storage_type
    }

    /// Array of stored indirect anchor points
    /// (port of the `anchorPointsIndirect` getter).
    pub fn anchor_points_indirect(&self) -> Option<&[AnchorPointIndirect]> {
        self.anchor_points_indirect.as_deref()
    }

    /// Array of stored direct anchor points
    /// (port of the `anchorPointsDirect` getter).
    pub fn anchor_points_direct(&self) -> Option<&[AnchorPointDirect]> {
        self.anchor_points_direct.as_deref()
    }

    /// Metadata identifying parameters using same correlation modeling
    /// and associated correlation parameters
    /// (port of the `intraTileCorrelationGroups` getter).
    pub fn intra_tile_correlation_groups(&self) -> Option<&[CorrelationGroup]> {
        self.intra_tile_correlation_groups.as_deref()
    }

    /// The full covariance of anchor point parameters
    /// (port of the `covarianceDirect` getter).
    pub fn covariance_direct(&self) -> Option<&Matrix3> {
        self.covariance_direct.as_ref()
    }
}
