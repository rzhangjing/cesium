//! ImageBasedLighting + CloudCollection 规范
//! 移植自 CesiumJS Scene/ImageBasedLightingSpec.js + Scene/CloudCollectionSpec.js

use cesium_effects::{
    default_spherical_harmonics, CloudCollection, CumulusCloud, ImageBasedLighting,
    SH_COEFFICIENT_COUNT,
};
use glam::DVec3;

// ==================== ImageBasedLighting ====================

#[test]
fn ibl_default_state() {
    let ibl = ImageBasedLighting::default();
    assert_eq!(ibl.image_based_lighting_factor, [1.0, 1.0]);
    assert!(!ibl.has_spherical_harmonics());
    assert!(!ibl.has_specular_environment_maps());
    assert!(!ibl.needs_shader_regeneration());
}

#[test]
fn ibl_set_factor_valid() {
    let mut ibl = ImageBasedLighting::new();
    ibl.set_factor(0.5, 0.8);
    assert_eq!(ibl.image_based_lighting_factor, [0.5, 0.8]);
}

#[test]
#[should_panic(expected = "diffuse factor must be in [0, 1]")]
fn ibl_set_factor_panics_on_invalid() {
    let mut ibl = ImageBasedLighting::new();
    ibl.set_factor(1.5, 0.5);
}

#[test]
fn ibl_set_spherical_harmonics() {
    let mut ibl = ImageBasedLighting::default();
    let sh = default_spherical_harmonics();
    ibl.set_spherical_harmonics(sh);

    assert!(ibl.has_spherical_harmonics());
    assert!(ibl.needs_shader_regeneration());
    assert!(!ibl.use_default_spherical_harmonics);
}

#[test]
fn ibl_specular_environment_maps() {
    let mut ibl = ImageBasedLighting::default();
    assert!(!ibl.has_specular_environment_maps());

    ibl.specular_environment_maps = Some("env.ktx2".to_string());
    assert!(ibl.has_specular_environment_maps());
    assert!(ibl.needs_shader_regeneration());
}

#[test]
fn ibl_compute_diffuse_no_coefficients_returns_zero() {
    let ibl = ImageBasedLighting::default();
    let result = ibl.compute_diffuse_ibl(DVec3::Y);
    assert_eq!(result, [0.0; 3]);
}

#[test]
fn ibl_compute_diffuse_with_coefficients() {
    let mut ibl = ImageBasedLighting::default();
    ibl.set_spherical_harmonics(default_spherical_harmonics());

    let result = ibl.compute_diffuse_ibl(DVec3::Y);
    assert!(result[0] > 0.0 || result[1] > 0.0 || result[2] > 0.0);
}

#[test]
fn ibl_compute_diffuse_zero_factor() {
    let mut ibl = ImageBasedLighting::default();
    ibl.set_spherical_harmonics(default_spherical_harmonics());
    ibl.set_factor(0.0, 1.0);

    let result = ibl.compute_diffuse_ibl(DVec3::Y);
    assert_eq!(result, [0.0; 3]);
}

#[test]
fn ibl_compute_specular_default() {
    // 精确固定当前占位实现的约定（见 DEVIATION 说明
    // 于 `compute_specular_ibl` / docs/deviations.md#dev-024）：一个固定的中性
    // 环境色调 `[0.1, 0.1, 0.12]`，按 specular 因子 (1.0) 缩放。
    // 有意不用旧的弱断言 `result[0] > 0.0`，占位实现
    // 会平凡通过，且无法区分占位实现与真正的
    // 预滤波。当延期的 #54 落地真正的粗糙度驱动预滤波时，
    // 下方被 `#[ignore]` 标记的依赖测试将接管，此固定会被替换。
    let ibl = ImageBasedLighting::default();
    let result = ibl.compute_specular_ibl(DVec3::Y, 0.5);
    assert_eq!(result, [0.1, 0.1, 0.12]);
}

#[test]
#[ignore = "awaiting deferred #54 real prefilter — placeholder returns a constant, so roughness/reflection dependence cannot hold yet"]
fn ibl_compute_specular_scales_linearly_with_specular_factor() {
    // 真正预滤波的采样与 specular IBL 因子成线性。
    let mut ibl = ImageBasedLighting::default();
    ibl.set_factor(1.0, 0.5);
    let half = ibl.compute_specular_ibl(DVec3::Y, 0.5);
    ibl.set_factor(1.0, 1.0);
    let full = ibl.compute_specular_ibl(DVec3::Y, 0.5);
    for c in 0..3 {
        assert!((full[c] - 2.0 * half[c]).abs() < 1e-12, "factor linearity {c}");
    }
}

#[test]
#[ignore = "awaiting deferred #54 real prefilter — placeholder ignores roughness entirely"]
fn ibl_compute_specular_varies_with_roughness() {
    // 真正的预滤波随粗糙度变化（粗糙度→mip 选择）；
    // 占位实现无论如何都返回常量，因此按设计目前会失败。
    let ibl = ImageBasedLighting::default();
    let smooth = ibl.compute_specular_ibl(DVec3::Y, 0.05);
    let rough = ibl.compute_specular_ibl(DVec3::Y, 0.95);
    assert_ne!(smooth, rough, "specular must depend on roughness");
}

#[test]
#[ignore = "awaiting deferred #54 real prefilter — placeholder ignores the reflection direction entirely"]
fn ibl_compute_specular_varies_with_reflection_direction() {
    // 不同的反射方向采样环境的不同部分。
    let ibl = ImageBasedLighting::default();
    let up = ibl.compute_specular_ibl(DVec3::Z, 0.5);
    let side = ibl.compute_specular_ibl(DVec3::X, 0.5);
    assert_ne!(up, side, "specular must depend on the reflection direction");
}

#[test]
fn ibl_compute_specular_zero_factor() {
    let mut ibl = ImageBasedLighting::default();
    ibl.set_factor(1.0, 0.0);
    let result = ibl.compute_specular_ibl(DVec3::Y, 0.5);
    assert_eq!(result, [0.0; 3]);
}

#[test]
fn ibl_sh_coefficient_count_is_9() {
    assert_eq!(SH_COEFFICIENT_COUNT, 9);
}

#[test]
fn ibl_default_sh_dc_term_positive() {
    let sh = default_spherical_harmonics();
    assert!(sh[0][0] > 0.0);
    assert!(sh[0][1] > 0.0);
    assert!(sh[0][2] > 0.0);
}

// ==================== CumulusCloud ====================

#[test]
fn cumulus_cloud_default_values() {
    let cloud = CumulusCloud::default();
    assert!(cloud.show);
    assert_eq!(cloud.position, DVec3::ZERO);
    assert_eq!(cloud.scale, [20.0, 12.0]);
    assert!((cloud.slice - (-1.0)).abs() < 1e-10);
    assert!((cloud.brightness - 1.0).abs() < 1e-10);
    assert_eq!(cloud.color, [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn cumulus_cloud_new_sets_scale_from_size() {
    let pos = DVec3::new(100.0, 200.0, 300.0);
    let size = DVec3::new(30.0, 20.0, 15.0);
    let cloud = CumulusCloud::new(pos, size);
    assert_eq!(cloud.position, pos);
    assert_eq!(cloud.maximum_size, size);
    assert_eq!(cloud.scale, [30.0, 20.0]);
}

#[test]
fn cumulus_cloud_effective_dimensions_no_slice() {
    let cloud = CumulusCloud::default();
    assert_eq!(cloud.effective_dimensions(), [20.0, 12.0]);
}

#[test]
fn cumulus_cloud_effective_dimensions_sliced() {
    let mut cloud = CumulusCloud::default();
    cloud.slice = 0.5;
    let dims = cloud.effective_dimensions();
    // factor = 1.0 - |0.5 - 0.5| * 0.5 = 1.0
    assert!((dims[0] - 20.0).abs() < 1e-10);

    cloud.slice = 0.0;
    let dims = cloud.effective_dimensions();
    // factor = 1.0 - |0.0 - 0.5| * 0.5 = 0.75
    assert!((dims[0] - 15.0).abs() < 1e-10);
}

#[test]
fn cumulus_cloud_slice_recommended() {
    let mut cloud = CumulusCloud::default();
    assert!(cloud.is_slice_recommended()); // -1.0

    cloud.slice = 0.5;
    assert!(cloud.is_slice_recommended());

    cloud.slice = 0.05;
    assert!(!cloud.is_slice_recommended());

    cloud.slice = 0.95;
    assert!(!cloud.is_slice_recommended());
}

// ==================== CloudCollection ====================

#[test]
fn cloud_collection_default_state() {
    let collection = CloudCollection::new();
    assert!(collection.show);
    assert!((collection.noise_detail - 16.0).abs() < 1e-10);
    assert_eq!(collection.noise_offset, DVec3::ZERO);
    assert!(collection.is_empty());
    assert!(collection.is_dirty());
}

#[test]
fn cloud_collection_add_and_index() {
    let mut collection = CloudCollection::new();
    let idx0 = collection.add(CumulusCloud::new(
        DVec3::new(1.0, 2.0, 3.0),
        DVec3::new(20.0, 12.0, 8.0),
    ));
    let idx1 = collection.add(CumulusCloud::new(
        DVec3::new(4.0, 5.0, 6.0),
        DVec3::new(15.0, 9.0, 9.0),
    ));

    assert_eq!(idx0, 0);
    assert_eq!(idx1, 1);
    assert_eq!(collection.len(), 2);
    assert_eq!(collection.get(0).unwrap().index(), 0);
    assert_eq!(collection.get(1).unwrap().index(), 1);
}

#[test]
fn cloud_collection_remove_reindexes() {
    let mut collection = CloudCollection::new();
    collection.add(CumulusCloud::default());
    collection.add(CumulusCloud::default());
    collection.add(CumulusCloud::default());

    let removed = collection.remove(0);
    assert!(removed.is_some());
    assert_eq!(collection.len(), 2);
    // 剩余的云被重新索引
    assert_eq!(collection.get(0).unwrap().index(), 0);
    assert_eq!(collection.get(1).unwrap().index(), 1);
}

#[test]
fn cloud_collection_remove_all() {
    let mut collection = CloudCollection::new();
    collection.add(CumulusCloud::default());
    collection.add(CumulusCloud::default());
    collection.remove_all();
    assert!(collection.is_empty());
}

#[test]
fn cloud_collection_visible_clouds() {
    let mut collection = CloudCollection::new();
    collection.add(CumulusCloud::default());
    let mut hidden = CumulusCloud::default();
    hidden.show = false;
    collection.add(hidden);
    collection.add(CumulusCloud::default());

    assert_eq!(collection.len(), 3);
    assert_eq!(collection.visible_clouds().count(), 2);
}

#[test]
fn cloud_collection_dirty_lifecycle() {
    let mut collection = CloudCollection::new();
    assert!(collection.is_dirty());

    collection.mark_clean();
    assert!(!collection.is_dirty());

    collection.add(CumulusCloud::default());
    assert!(collection.is_dirty());
}

#[test]
fn cloud_collection_bounding_sphere() {
    let mut collection = CloudCollection::new();
    assert!(collection.compute_bounding_sphere().is_none());

    collection.add(CumulusCloud::new(
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(10.0, 10.0, 10.0),
    ));
    collection.add(CumulusCloud::new(
        DVec3::new(100.0, 0.0, 0.0),
        DVec3::new(10.0, 10.0, 10.0),
    ));

    let (center, radius) = collection.compute_bounding_sphere().unwrap();
    assert!((center.x - 50.0).abs() < 1e-10);
    assert!(radius > 50.0);
}
