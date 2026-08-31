//! Ported from `packages/engine/Source/Scene/Model/Extensions/Gpm/GltfGpmLoader.js`.
//!
//! Loads glTF `NGA_gpm_local` data from the root of a glTF extension
//! object (given as `serde_json::Value`).

use serde_json::Value;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::matrix3::Matrix3;
use cesium_core::runtime_error::RuntimeError;

use crate::model::extensions::gpm::anchor_point_direct::AnchorPointDirect;
use crate::model::extensions::gpm::anchor_point_indirect::AnchorPointIndirect;
use crate::model::extensions::gpm::correlation_group::CorrelationGroup;
use crate::model::extensions::gpm::gltf_gpm_local::{GltfGpmLocal, GltfGpmLocalOptions};
use crate::model::extensions::gpm::spdcf::Spdcf;
use crate::model::extensions::gpm::storage_type::StorageType;

/// Reads a JSON number array field.
fn f64_array(json: &Value, field: &str) -> Result<Vec<f64>, RuntimeError> {
    let array = json
        .get(field)
        .and_then(|v| v.as_array())
        .ok_or_else(|| RuntimeError::new(Some(&format!("{} is not an array", field))))?;
    array
        .iter()
        .map(|v| {
            v.as_f64().ok_or_else(|| {
                RuntimeError::new(Some(&format!("{} contains a non-number element", field)))
            })
        })
        .collect()
}

/// Reads a JSON boolean array field.
fn bool_array(json: &Value, field: &str) -> Result<Vec<bool>, RuntimeError> {
    let array = json
        .get(field)
        .and_then(|v| v.as_array())
        .ok_or_else(|| RuntimeError::new(Some(&format!("{} is not an array", field))))?;
    array
        .iter()
        .map(|v| {
            v.as_bool().ok_or_else(|| {
                RuntimeError::new(Some(&format!("{} contains a non-boolean element", field)))
            })
        })
        .collect()
}

/// Reads a JSON number field.
fn f64_field(json: &Value, field: &str) -> Result<f64, RuntimeError> {
    json.get(field)
        .and_then(|v| v.as_f64())
        .ok_or_else(|| RuntimeError::new(Some(&format!("{} is not a number", field))))
}

/// Creates a `Matrix3` that describes a covariance matrix (which is
/// symmetric) from the array containing the upper triangle, in
/// column-major order.
///
/// Port of `createCovarianceMatrixFromUpperTriangle(array)`.
pub fn create_covariance_matrix_from_upper_triangle(array: &[f64]) -> Matrix3 {
    Matrix3::new(
        array[0], array[1], array[3], // column 0
        array[1], array[2], array[4], // column 1
        array[3], array[4], array[5], // column 2
    )
}

/// Creates an `AnchorPointDirect` from the given JSON representation.
///
/// Port of `createAnchorPointDirect(anchorPointDirectJson)`.
pub fn create_anchor_point_direct(json: &Value) -> Result<AnchorPointDirect, RuntimeError> {
    let position = Cartesian3::from_array_new(&f64_array(json, "position")?, Some(0));
    let adjustment_params =
        Cartesian3::from_array_new(&f64_array(json, "adjustmentParams")?, Some(0));
    Ok(AnchorPointDirect::new(position, adjustment_params))
}

/// Creates an `AnchorPointIndirect` from the given JSON representation.
///
/// Port of `createAnchorPointIndirect(anchorPointIndirectJson)`.
pub fn create_anchor_point_indirect(json: &Value) -> Result<AnchorPointIndirect, RuntimeError> {
    let position = Cartesian3::from_array_new(&f64_array(json, "position")?, Some(0));
    let adjustment_params =
        Cartesian3::from_array_new(&f64_array(json, "adjustmentParams")?, Some(0));
    let covariance_matrix = create_covariance_matrix_from_upper_triangle(
        &f64_array(json, "covarianceMatrix")?,
    );
    Ok(AnchorPointIndirect::new(
        position,
        adjustment_params,
        covariance_matrix,
    ))
}

/// Creates a `CorrelationGroup` from the given JSON representation.
///
/// Port of `createCorrelationGroup(correlationGroupJson)`.
pub fn create_correlation_group(json: &Value) -> Result<CorrelationGroup, RuntimeError> {
    let group_flags = bool_array(json, "groupFlags")?;
    let rotation_thetas = Cartesian3::from_array_new(&f64_array(json, "rotationThetas")?, Some(0));
    let params_json = json
        .get("params")
        .and_then(|v| v.as_array())
        .ok_or_else(|| RuntimeError::new(Some("params is not an array")))?;
    let mut params = Vec::with_capacity(params_json.len());
    for param_json in params_json {
        let param = Spdcf::new(
            f64_field(param_json, "A")?,
            f64_field(param_json, "alpha")?,
            f64_field(param_json, "beta")?,
            f64_field(param_json, "T")?,
        );
        params.push(param);
    }
    Ok(CorrelationGroup::new(group_flags, rotation_thetas, params))
}

/// Loads the GPM data from the given JSON that was found as the
/// `NGA_gpm_local` extension object in the root of the glTF.
///
/// Port of `GltfGpmLoader.load(gltfGpmLocalJson)`.
///
/// # Errors
/// Returns a `RuntimeError` when the given object contains invalid
/// storage types or malformed contents.
pub fn load(gltf_gpm_local_json: &Value) -> Result<GltfGpmLocal, RuntimeError> {
    let storage_type_string = gltf_gpm_local_json
        .get("storageType")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    match StorageType::from_str(storage_type_string) {
        Some(StorageType::Direct) => load_direct(gltf_gpm_local_json),
        Some(StorageType::Indirect) => load_indirect(gltf_gpm_local_json),
        None => Err(RuntimeError::new(Some(&format!(
            "Invalid storage type in NGA_gpm_local - expected 'Direct' or 'Indirect', but found {}",
            serde_json::to_string(&gltf_gpm_local_json.get("storageType")).unwrap_or_default()
        )))),
    }
}

/// Loads the GPM data assuming that the `storageType` of the given
/// object is `StorageType.Direct`.
///
/// Port of `GltfGpmLoader.loadDirect(gltfGpmLocalJson)`.
///
/// DEVIATION: the JS debug-only `Check.typeOf.object` preconditions are
/// surfaced as `RuntimeError` results here (Rust cannot silently index a
/// missing field like JS can); observable error behavior is preserved.
pub fn load_direct(gltf_gpm_local_json: &Value) -> Result<GltfGpmLocal, RuntimeError> {
    let anchor_points_direct_json = gltf_gpm_local_json
        .get("anchorPointsDirect")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            RuntimeError::new(Some(
                "The anchorPointsDirect are required for 'Direct' storage",
            ))
        })?;
    let covariance_upper_triangle = f64_array(gltf_gpm_local_json, "covarianceDirectUpperTriangle")
        .map_err(|_| {
            RuntimeError::new(Some(
                "The covarianceDirectUpperTriangle is required for 'Direct' storage",
            ))
        })?;

    let mut anchor_points_direct = Vec::with_capacity(anchor_points_direct_json.len());
    for anchor_point_direct_json in anchor_points_direct_json {
        anchor_points_direct.push(create_anchor_point_direct(anchor_point_direct_json)?);
    }
    let covariance_direct =
        create_covariance_matrix_from_upper_triangle(&covariance_upper_triangle);

    Ok(GltfGpmLocal::new(GltfGpmLocalOptions {
        storage_type: StorageType::Direct,
        anchor_points_direct: Some(anchor_points_direct),
        covariance_direct: Some(covariance_direct),
        ..Default::default()
    }))
}

/// Loads the GPM data assuming that the `storageType` of the given
/// object is `StorageType.Indirect`.
///
/// Port of `GltfGpmLoader.loadIndirect(gltfGpmLocalJson)`.
///
/// DEVIATION: the JS debug-only `Check.typeOf.object` preconditions are
/// surfaced as `RuntimeError` results here; observable error behavior is
/// preserved.
pub fn load_indirect(gltf_gpm_local_json: &Value) -> Result<GltfGpmLocal, RuntimeError> {
    let anchor_points_indirect_json = gltf_gpm_local_json
        .get("anchorPointsIndirect")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            RuntimeError::new(Some(
                "The anchorPointsIndirect are required for 'Indirect' storage",
            ))
        })?;
    let intra_tile_correlation_groups_json = gltf_gpm_local_json
        .get("intraTileCorrelationGroups")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            RuntimeError::new(Some(
                "The intraTileCorrelationGroups are required for 'Indirect' storage",
            ))
        })?;

    let mut anchor_points_indirect = Vec::with_capacity(anchor_points_indirect_json.len());
    for anchor_point_indirect_json in anchor_points_indirect_json {
        anchor_points_indirect.push(create_anchor_point_indirect(anchor_point_indirect_json)?);
    }

    let mut intra_tile_correlation_groups =
        Vec::with_capacity(intra_tile_correlation_groups_json.len());
    for correlation_group_json in intra_tile_correlation_groups_json {
        intra_tile_correlation_groups.push(create_correlation_group(correlation_group_json)?);
    }

    Ok(GltfGpmLocal::new(GltfGpmLocalOptions {
        storage_type: StorageType::Indirect,
        anchor_points_indirect: Some(anchor_points_indirect),
        intra_tile_correlation_groups: Some(intra_tile_correlation_groups),
        ..Default::default()
    }))
}
