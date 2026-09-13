//! PostProcess + Geocoder + Panorama specs
//! Ported from CesiumJS Scene/PostProcessStageLibrarySpec.js + Core/GeocoderServiceSpec.js + Scene/PanoramaSpec.js

use cesium_effects::{
    AmbientOcclusionConfig, BloomConfig, ColorCorrectionConfig, CubeMapPanorama, FogConfig,
    GeocodeType, GeocoderDestination, GeocoderResult, GeocoderService, MockGeocoderService,
    PostProcessPipeline, PostProcessStageType, ToneMappingConfig, ToneMappingOperator,
    EquirectangularPanorama, DEFAULT_PANORAMA_RADIUS,
};
use glam::{DMat4, DVec3};

// ==================== Bloom ====================

#[test]
fn bloom_below_threshold_zero() {
    let bloom = BloomConfig {
        enabled: true,
        threshold: 0.8,
        intensity: 1.0,
        ..Default::default()
    };
    assert!((bloom.compute_bloom(0.5)).abs() < 1e-10);
    assert!((bloom.compute_bloom(0.8)).abs() < 1e-10);
}

#[test]
fn bloom_above_threshold() {
    let bloom = BloomConfig {
        enabled: true,
        threshold: 0.8,
        intensity: 2.0,
        ..Default::default()
    };
    let result = bloom.compute_bloom(1.0);
    assert!((result - 0.4).abs() < 1e-10); // (1.0 - 0.8) * 2.0
}

#[test]
fn bloom_disabled_returns_zero() {
    let bloom = BloomConfig::default(); // disabled
    assert!((bloom.compute_bloom(10.0)).abs() < 1e-10);
}

// ==================== Ambient Occlusion ====================

#[test]
fn ao_no_occlusion_returns_one() {
    let ao = AmbientOcclusionConfig {
        enabled: true,
        intensity: 3.0,
        ..Default::default()
    };
    assert!((ao.compute_ao(0.0) - 1.0).abs() < 1e-10);
}

#[test]
fn ao_full_occlusion_clamped() {
    let ao = AmbientOcclusionConfig {
        enabled: true,
        intensity: 3.0,
        ..Default::default()
    };
    assert!((ao.compute_ao(1.0)).abs() < 1e-10); // clamped to 0
}

#[test]
fn ao_disabled_returns_one() {
    let ao = AmbientOcclusionConfig::default(); // disabled
    assert!((ao.compute_ao(1.0) - 1.0).abs() < 1e-10);
}

// ==================== Fog ====================

#[test]
fn fog_below_minimum_distance_zero() {
    let fog = FogConfig::default();
    assert!((fog.compute_fog_factor(50.0)).abs() < 1e-10);
}

#[test]
fn fog_exponential_at_distance() {
    let fog = FogConfig {
        enabled: true,
        density: 1.0e-3,
        ..Default::default()
    };
    let factor = fog.compute_fog_factor(10000.0);
    assert!(factor > 0.99);
}

#[test]
fn fog_disabled_returns_zero() {
    let fog = FogConfig {
        enabled: false,
        ..Default::default()
    };
    assert!((fog.compute_fog_factor(100000.0)).abs() < 1e-10);
}

#[test]
fn fog_apply_blends_color() {
    let fog = FogConfig {
        enabled: true,
        density: 1.0e-3,
        color: DVec3::new(1.0, 1.0, 1.0),
        ..Default::default()
    };
    let pixel = DVec3::new(0.0, 0.0, 0.0);
    let result = fog.apply_fog(pixel, 10000.0);
    assert!(result.x > 0.9);
    assert!(result.y > 0.9);
}

// ==================== Tone Mapping ====================

#[test]
fn tone_mapping_none_passthrough() {
    let config = ToneMappingConfig {
        operator: ToneMappingOperator::None,
        exposure: 1.0,
        white_point: 1.0,
    };
    let hdr = DVec3::new(0.5, 0.7, 0.9);
    let ldr = config.apply(hdr);
    assert!((ldr - hdr).length() < 1e-10);
}

#[test]
fn tone_mapping_reinhard_compresses() {
    let config = ToneMappingConfig {
        operator: ToneMappingOperator::Reinhard,
        exposure: 1.0,
        white_point: 100.0,
    };
    let hdr = DVec3::new(2.0, 2.0, 2.0);
    let ldr = config.apply(hdr);
    // Simple Reinhard: x/(1+x) ≈ 2/3
    assert!((ldr.x - 2.0 / 3.0).abs() < 0.01);
}

#[test]
fn tone_mapping_aces_maps_below_one() {
    let config = ToneMappingConfig {
        operator: ToneMappingOperator::AcesFilmic,
        exposure: 1.0,
        white_point: 1.0,
    };
    let hdr = DVec3::new(1.0, 1.0, 1.0);
    let ldr = config.apply(hdr);
    assert!(ldr.x < 1.0);
    assert!(ldr.x > 0.0);
}

#[test]
fn tone_mapping_exposure_scales() {
    let config = ToneMappingConfig {
        operator: ToneMappingOperator::None,
        exposure: 2.0,
        white_point: 1.0,
    };
    let hdr = DVec3::new(0.3, 0.3, 0.3);
    let ldr = config.apply(hdr);
    assert!((ldr.x - 0.6).abs() < 1e-10);
}

// ==================== Color Correction ====================

#[test]
fn color_correction_disabled_passthrough() {
    let cc = ColorCorrectionConfig::default(); // disabled
    let color = DVec3::new(0.3, 0.5, 0.7);
    let result = cc.apply(color);
    assert!((result - color).length() < 1e-10);
}

#[test]
fn color_correction_brightness() {
    let cc = ColorCorrectionConfig {
        enabled: true,
        brightness: 0.1,
        contrast: 1.0,
        saturation: 1.0,
        hue: 0.0,
    };
    let color = DVec3::new(0.5, 0.5, 0.5);
    let result = cc.apply(color);
    assert!((result.x - 0.6).abs() < 1e-10);
}

#[test]
fn color_correction_saturation_zero_grayscale() {
    let cc = ColorCorrectionConfig {
        enabled: true,
        brightness: 0.0,
        contrast: 1.0,
        saturation: 0.0,
        hue: 0.0,
    };
    let color = DVec3::new(1.0, 0.0, 0.0);
    let result = cc.apply(color);
    assert!((result.x - result.y).abs() < 1e-10);
    assert!((result.y - result.z).abs() < 1e-10);
}

// ==================== Pipeline ====================

#[test]
fn pipeline_default_enabled_stages() {
    let pipeline = PostProcessPipeline::new();
    let stages = pipeline.enabled_stages();
    // Default: fog enabled + tone mapping ACES
    assert_eq!(stages.len(), 2);
    assert!(stages.contains(&PostProcessStageType::Fog));
    assert!(stages.contains(&PostProcessStageType::ToneMapping));
}

#[test]
fn pipeline_all_stages_enabled() {
    let mut pipeline = PostProcessPipeline::new();
    pipeline.bloom.enabled = true;
    pipeline.ambient_occlusion.enabled = true;
    pipeline.color_correction.enabled = true;

    let stages = pipeline.enabled_stages();
    assert_eq!(stages.len(), 5);
}

// ==================== Geocoder ====================

#[test]
fn geocode_type_default_is_search() {
    assert_eq!(GeocodeType::default(), GeocodeType::Search);
}

#[test]
fn geocoder_result_rectangle_destination() {
    let result = GeocoderResult {
        display_name: "New York".to_string(),
        destination: GeocoderDestination::Rectangle([-1.2985, 0.7086, -1.2968, 0.7098]),
        attributions: vec![],
    };
    if let GeocoderDestination::Rectangle(r) = result.destination {
        assert!(r[0] < r[2]); // west < east
        assert!(r[1] < r[3]); // south < north
    } else {
        panic!("Expected Rectangle");
    }
}

#[test]
fn geocoder_result_point_destination() {
    let result = GeocoderResult {
        display_name: "Eiffel Tower".to_string(),
        destination: GeocoderDestination::Point {
            longitude: 0.0407,
            latitude: 0.8517,
            height: Some(330.0),
        },
        attributions: vec![],
    };
    if let GeocoderDestination::Point { height, .. } = result.destination {
        assert_eq!(height, Some(330.0));
    } else {
        panic!("Expected Point");
    }
}

#[test]
fn mock_geocoder_returns_results() {
    let results = vec![GeocoderResult {
        display_name: "Test".to_string(),
        destination: GeocoderDestination::Point {
            longitude: 0.0,
            latitude: 0.0,
            height: None,
        },
        attributions: vec![],
    }];
    let service = MockGeocoderService::with_results(results);
    let geocode_results = service.geocode("test", GeocodeType::Search);
    assert_eq!(geocode_results.len(), 1);
    assert_eq!(geocode_results[0].display_name, "Test");
}

#[test]
fn mock_geocoder_credit() {
    let mut service = MockGeocoderService::new();
    assert!(service.credit().is_none());
    service.credit = Some("Credit".to_string());
    assert_eq!(service.credit(), Some("Credit"));
}

// ==================== EquirectangularPanorama ====================

#[test]
fn equirectangular_panorama_defaults() {
    let pano = EquirectangularPanorama::default();
    assert_eq!(pano.transform, DMat4::IDENTITY);
    assert!((pano.radius - DEFAULT_PANORAMA_RADIUS).abs() < 1e-10);
    assert!((pano.repeat_horizontal - 1.0).abs() < 1e-10);
    assert!(pano.show);
}

#[test]
fn equirectangular_uv_forward_direction() {
    let pano = EquirectangularPanorama::new("test.jpg");
    let dir = DVec3::new(1.0, 0.0, 0.0);
    let uv = pano.direction_to_uv(dir);
    assert!((uv[0] - 0.5).abs() < 1e-10);
    assert!((uv[1] - 0.5).abs() < 1e-10);
}

#[test]
fn equirectangular_uv_roundtrip() {
    let pano = EquirectangularPanorama::new("test.jpg");
    let dir = DVec3::new(0.5, 0.5, 0.707).normalize();
    let uv = pano.direction_to_uv(dir);
    let dir_back = pano.uv_to_direction(uv[0], uv[1]);
    assert!((dir_back - dir).length() < 1e-10);
}

#[test]
fn equirectangular_uv_poles() {
    let pano = EquirectangularPanorama::new("test.jpg");
    let north = DVec3::new(0.0, 0.0, 1.0);
    let uv_n = pano.direction_to_uv(north);
    assert!((uv_n[1] - 1.0).abs() < 1e-10);

    let south = DVec3::new(0.0, 0.0, -1.0);
    let uv_s = pano.direction_to_uv(south);
    assert!(uv_s[1].abs() < 1e-10);
}

// ==================== CubeMapPanorama ====================

#[test]
fn cubemap_panorama_incomplete_by_default() {
    let pano = CubeMapPanorama::default();
    assert!(!pano.is_complete());
    assert!((pano.radius - DEFAULT_PANORAMA_RADIUS).abs() < 1e-10);
}

#[test]
fn cubemap_panorama_complete_with_faces() {
    let faces = [
        "px.jpg".to_string(),
        "nx.jpg".to_string(),
        "py.jpg".to_string(),
        "ny.jpg".to_string(),
        "pz.jpg".to_string(),
        "nz.jpg".to_string(),
    ];
    let pano = CubeMapPanorama::new(faces);
    assert!(pano.is_complete());
}

#[test]
fn cubemap_direction_to_face() {
    let pano = CubeMapPanorama::default();

    let (face, _) = pano.direction_to_face_uv(DVec3::new(1.0, 0.0, 0.0));
    assert_eq!(face, 0); // +X
    let (face, _) = pano.direction_to_face_uv(DVec3::new(-1.0, 0.0, 0.0));
    assert_eq!(face, 1); // -X
    let (face, _) = pano.direction_to_face_uv(DVec3::new(0.0, 1.0, 0.0));
    assert_eq!(face, 2); // +Y
    let (face, _) = pano.direction_to_face_uv(DVec3::new(0.0, -1.0, 0.0));
    assert_eq!(face, 3); // -Y
    let (face, _) = pano.direction_to_face_uv(DVec3::new(0.0, 0.0, 1.0));
    assert_eq!(face, 4); // +Z
    let (face, _) = pano.direction_to_face_uv(DVec3::new(0.0, 0.0, -1.0));
    assert_eq!(face, 5); // -Z
}
