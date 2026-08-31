//! Mirrors `packages/engine/Specs/Scene/Model/Extensions/Gpm/`
//! (`GltfGpmLoaderSpec.js` and the pure-logic parts of
//! `GltfMeshPrimitiveGpmLoaderSpec.js`).

use serde_json::json;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::math::CesiumMath;
use cesium_core::matrix3::Matrix3;
use cesium_scene::model::extensions::gpm::gltf_gpm_loader;
use cesium_scene::model::extensions::gpm::gltf_mesh_primitive_gpm_loader as mesh_loader;
use cesium_scene::model::extensions::gpm::ppe_source::PpeSource;
use cesium_scene::model::extensions::gpm::storage_type::StorageType;
use cesium_scene::resource_loader_state::ResourceLoaderState;

// ============================================================================
// Scene/Model/Extensions/Gpm/GltfGpmLoader
// ============================================================================

#[test]
fn load_throws_with_invalid_storage_type() {
    let gltf_gpm_local_json = json!({
        "storageType": "INVALID",
    });
    assert!(gltf_gpm_loader::load(&gltf_gpm_local_json).is_err());
}

#[test]
fn load_throws_for_storage_type_direct_without_anchor_points_direct() {
    let gltf_gpm_local_json = json!({
        "storageType": "Direct",
    });
    assert!(gltf_gpm_loader::load(&gltf_gpm_local_json).is_err());
}

#[test]
fn load_throws_for_storage_type_direct_without_covariance_direct_upper_triangle() {
    let gltf_gpm_local_json = json!({
        "storageType": "Direct",
        "anchorPointsDirect": [
            {
                "position": [1.0, 2.0, 3.0],
                "adjustmentParams": [0.1, 0.2, 0.3],
            },
        ],
    });
    assert!(gltf_gpm_loader::load(&gltf_gpm_local_json).is_err());
}

#[test]
fn load_returns_result_for_valid_json_for_storage_type_direct() {
    let gltf_gpm_local_json = json!({
        "storageType": "Direct",
        "anchorPointsDirect": [
            {
                "position": [1.0, 2.0, 3.0],
                "adjustmentParams": [0.1, 0.2, 0.3],
            },
        ],
        "covarianceDirectUpperTriangle": [0.1, 0.2, 0.3, 0.4, 0.5, 0.6],
    });

    let result = gltf_gpm_loader::load(&gltf_gpm_local_json).expect("valid Direct JSON");
    let anchor_points_direct = result.anchor_points_direct().expect("anchorPointsDirect");
    assert_eq!(anchor_points_direct.len(), 1);

    let actual_anchor_point = &anchor_points_direct[0];

    let expected_position = Cartesian3::new(1.0, 2.0, 3.0);
    assert!(Cartesian3::equals_epsilon(
        Some(actual_anchor_point.position()),
        Some(&expected_position),
        Some(CesiumMath::EPSILON6),
        None,
    ));

    let expected_adjustment_params = Cartesian3::new(0.1, 0.2, 0.3);
    assert!(Cartesian3::equals_epsilon(
        Some(actual_anchor_point.adjustment_params()),
        Some(&expected_adjustment_params),
        Some(CesiumMath::EPSILON6),
        None,
    ));

    let expected_covariance_direct = Matrix3::from_array_new(
        &[0.1, 0.2, 0.4, 0.2, 0.3, 0.5, 0.4, 0.5, 0.6],
        0,
    );
    assert!(Matrix3::equals_epsilon(
        &result.covariance_direct().expect("covarianceDirect"),
        &expected_covariance_direct,
        CesiumMath::EPSILON6,
    ));
}

#[test]
fn load_throws_for_storage_type_indirect_without_anchor_points_indirect() {
    let gltf_gpm_local_json = json!({
        "storageType": "Indirect",
    });
    assert!(gltf_gpm_loader::load(&gltf_gpm_local_json).is_err());
}

#[test]
fn load_throws_for_storage_type_indirect_without_intra_tile_correlation_groups() {
    let gltf_gpm_local_json = json!({
        "storageType": "Indirect",
        "anchorPointsIndirect": [
            {
                "position": [1.0, 2.0, 3.0],
                "adjustmentParams": [0.1, 0.2, 0.3],
            },
        ],
    });
    assert!(gltf_gpm_loader::load(&gltf_gpm_local_json).is_err());
}

#[test]
fn load_returns_result_for_valid_json_for_storage_type_indirect() {
    let gltf_gpm_local_json = json!({
        "storageType": "Indirect",
        "anchorPointsIndirect": [
            {
                "position": [1.0, 2.0, 3.0],
                "adjustmentParams": [0.1, 0.2, 0.3],
                "covarianceMatrix": [0.1, 0.2, 0.3, 0.4, 0.5, 0.6],
            },
        ],
        "intraTileCorrelationGroups": [
            {
                "groupFlags": [true, true, true],
                "rotationThetas": [0.1, 0.2, 0.3],
                "params": [
                    {
                        "A": 0.1,
                        "alpha": 0.2,
                        "beta": 0.3,
                        "T": 0.4,
                    },
                ],
            },
        ],
    });

    let result = gltf_gpm_loader::load(&gltf_gpm_local_json).expect("valid Indirect JSON");
    assert_eq!(result.storage_type(), StorageType::Indirect);

    let anchor_points_indirect = result
        .anchor_points_indirect()
        .expect("anchorPointsIndirect");
    assert_eq!(anchor_points_indirect.len(), 1);

    let actual_anchor_point = &anchor_points_indirect[0];

    let expected_position = Cartesian3::new(1.0, 2.0, 3.0);
    assert!(Cartesian3::equals_epsilon(
        Some(actual_anchor_point.position()),
        Some(&expected_position),
        Some(CesiumMath::EPSILON6),
        None,
    ));

    let expected_adjustment_params = Cartesian3::new(0.1, 0.2, 0.3);
    assert!(Cartesian3::equals_epsilon(
        Some(actual_anchor_point.adjustment_params()),
        Some(&expected_adjustment_params),
        Some(CesiumMath::EPSILON6),
        None,
    ));

    let expected_covariance_matrix = Matrix3::from_array_new(
        &[0.1, 0.2, 0.4, 0.2, 0.3, 0.5, 0.4, 0.5, 0.6],
        0,
    );
    assert!(Matrix3::equals_epsilon(
        actual_anchor_point.covariance_matrix(),
        &expected_covariance_matrix,
        CesiumMath::EPSILON6,
    ));

    let intra_tile_correlation_groups = result
        .intra_tile_correlation_groups()
        .expect("intraTileCorrelationGroups");
    assert_eq!(intra_tile_correlation_groups.len(), 1);

    let correlation_group = &intra_tile_correlation_groups[0];
    let group_flags = correlation_group.group_flags();
    assert_eq!(group_flags, &[true, true, true]);

    let expected_rotation_thetas = Cartesian3::new(0.1, 0.2, 0.3);
    assert!(Cartesian3::equals_epsilon(
        Some(correlation_group.rotation_thetas()),
        Some(&expected_rotation_thetas),
        Some(CesiumMath::EPSILON6),
        None,
    ));

    let params = correlation_group.params();
    assert_eq!(params.len(), 1);
    let param = &params[0];
    assert_eq!(param.a(), 0.1);
    assert_eq!(param.alpha(), 0.2);
    assert_eq!(param.beta(), 0.3);
    assert_eq!(param.t(), 0.4);
}

// ============================================================================
// Scene/Model/Extensions/Gpm/GltfMeshPrimitiveGpmLoader (pure-logic part)
// ============================================================================

/// The JSON representation of the NGA_gpm_local extension object that
/// will be inserted into the mesh primitive (mirror of the spec
/// `ngaGpmLocalExtension`).
fn nga_gpm_local_extension() -> serde_json::Value {
    json!({
        "ppeTextures": [
            {
                "traits": {
                    "source": "SIGZ",
                    "min": 0.0,
                    "max": 16.0,
                },
                "index": 0,
                "noData": 255,
                "offset": 0.0,
                "scale": 0.06274509803921569,
                "texCoord": 0,
            },
        ],
    })
}

#[test]
fn loads_mesh_primitive_gpm_extension_data() {
    let extension = nga_gpm_local_extension();
    let mut loader = mesh_loader::GltfMeshPrimitiveGpmLoader::new(extension, None, None);

    loader.load();
    assert_eq!(loader.state(), ResourceLoaderState::Loaded);
    assert!(loader.process().expect("process succeeds"));
    assert_eq!(loader.state(), ResourceLoaderState::Ready);

    let gpm_data = loader.mesh_primitive_gpm_local().expect("gpm data");

    let ppe_textures = gpm_data.ppe_textures();
    assert_eq!(ppe_textures.len(), 1);

    let ppe_texture = &ppe_textures[0];
    assert_eq!(ppe_texture.index(), 0);
    assert_eq!(ppe_texture.tex_coord(), Some(0));
    assert_eq!(ppe_texture.no_data(), Some(255.0));
    assert_eq!(ppe_texture.offset(), Some(0.0));
    assert_eq!(ppe_texture.scale(), Some(0.06274509803921569));

    let traits = ppe_texture.traits();
    assert_eq!(traits.min(), Some(0.0));
    assert_eq!(traits.max(), Some(16.0));
    assert_eq!(traits.source(), PpeSource::Sigz);
}

#[test]
fn gathers_used_texture_ids() {
    let extension = nga_gpm_local_extension();
    let texture_ids = mesh_loader::gather_used_texture_ids(&extension);
    assert_eq!(texture_ids.len(), 1);
    assert!(texture_ids.contains_key(&0));
}

#[test]
fn creates_ppe_texture_class_json_with_normalization_factor() {
    let extension = nga_gpm_local_extension();
    let ppe_textures = mesh_loader::parse_ppe_textures(&extension).expect("parse");
    let class_json = mesh_loader::create_ppe_texture_class_json(&ppe_textures[0], 0);

    assert_eq!(class_json["name"], "PPE texture class 0");
    let property = &class_json["properties"]["SIGZ"];
    assert_eq!(property["name"], "PPE");
    assert_eq!(property["type"], "SCALAR");
    assert_eq!(property["componentType"], "UINT8");
    assert_eq!(property["normalized"], true);
    assert_eq!(property["offset"], 0.0);
    // The scale is multiplied by 255 to cancel out the normalization.
    let scale = property["scale"].as_f64().unwrap();
    assert!((scale - 0.06274509803921569 * 255.0).abs() < 1e-12);
    assert_eq!(property["min"], 0.0);
    assert_eq!(property["max"], 16.0);
}

#[test]
fn obtains_ppe_textures_metadata_schema_json() {
    let extension = nga_gpm_local_extension();
    let ppe_textures = mesh_loader::parse_ppe_textures(&extension).expect("parse");
    let local = cesium_scene::model::extensions::gpm::mesh_primitive_gpm_local::MeshPrimitiveGpmLocal::new(ppe_textures);

    let schema_json = mesh_loader::obtain_ppe_textures_metadata_schema_json(&local, 0);
    assert_eq!(schema_json["id"], "PPE_TEXTURE_SCHEMA_0");
    assert!(schema_json["classes"]["ppeTexture_0"].is_object());

    let identifiers = mesh_loader::collect_ppe_texture_property_identifiers(&local);
    assert_eq!(identifiers.len(), 1);
    assert!(identifiers[0].contains("PPE texture class 0"));
}

#[test]
fn unload_clears_the_extension_data() {
    let extension = nga_gpm_local_extension();
    let mut loader = mesh_loader::GltfMeshPrimitiveGpmLoader::new(extension, None, None);
    loader.load();
    loader.process().expect("process succeeds");
    assert!(loader.mesh_primitive_gpm_local().is_some());

    loader.unload();
    assert!(loader.mesh_primitive_gpm_local().is_none());
    assert!(loader.texture_ids().is_empty());
}
