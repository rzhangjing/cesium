//! Scene/PointCloud + PointCloudShading → Rust 集成测试。
//!
//! 参考 CesiumJS：
//! - Scene/PointCloud
//! - Scene/PointCloudShading
//! - Scene/PointCloudEyeDomeLighting
//!
//! A 类测试：PointCloudShading 默认值/衰减/EDL、
//! QuantizedPositions 反量化、PointCloud from_feature_table。
//! C 类省略：WebGL 着色器渲染、framebuffer 操作。

use cesium_tileset::point_cloud::{PointCloud, PointCloudShading, QuantizedPositions};
use cesium_tileset::batch_table::FeatureTable;
use serde_json::json;

// === PointCloudShading ===

#[test]
fn shading_defaults() {
    let shading = PointCloudShading::default();
    assert!(!shading.attenuation);
    assert!((shading.base_resolution - 0.0).abs() < 1e-10);
    assert!(shading.eye_dome_lighting);
    assert!((shading.eye_dome_lighting_strength - 1.0).abs() < 1e-10);
    assert!((shading.eye_dome_lighting_radius - 1.0).abs() < 1e-10);
    assert!(!shading.back_face_culling);
    assert!(shading.normal_shading);
}

#[test]
fn shading_attenuation_disabled() {
    let shading = PointCloudShading::default(); // attenuation = false
    let size = shading.compute_attenuated_size(5.0, 100.0, 1080.0);
    // 禁用时，返回 base_size
    assert!((size - 5.0).abs() < 1e-10);
}

#[test]
fn shading_attenuation_enabled() {
    let mut shading = PointCloudShading::default();
    shading.attenuation = true;
    let size = shading.compute_attenuated_size(5.0, 540.0, 1080.0);
    // attenuation_factor = 1080 * 0.5 = 540
    // attenuated = 5.0 * (540 / 540) = 5.0
    assert!((size - 5.0).abs() < 1e-10);
}

#[test]
fn shading_attenuation_close_larger() {
    let mut shading = PointCloudShading::default();
    shading.attenuation = true;
    let size_close = shading.compute_attenuated_size(5.0, 100.0, 1080.0);
    let size_far = shading.compute_attenuated_size(5.0, 1000.0, 1080.0);
    // 更近的点应显得更大
    assert!(size_close > size_far);
}

#[test]
fn shading_attenuation_clamped() {
    let mut shading = PointCloudShading::default();
    shading.attenuation = true;
    // 非常近 → 应被钳制到最大 64
    let size = shading.compute_attenuated_size(10.0, 1.0, 1080.0);
    assert!(size <= 64.0);
    // 非常远 → 应被钳制到最小 1
    let size_far = shading.compute_attenuated_size(10.0, 1000000.0, 1080.0);
    assert!(size_far >= 1.0);
}

#[test]
fn shading_attenuation_zero_distance() {
    let mut shading = PointCloudShading::default();
    shading.attenuation = true;
    let size = shading.compute_attenuated_size(5.0, 0.0, 1080.0);
    // 距离为零时返回 base_size
    assert!((size - 5.0).abs() < 1e-10);
}

// === 眼罩光照 ===

#[test]
fn edl_disabled_returns_one() {
    let mut shading = PointCloudShading::default();
    shading.eye_dome_lighting = false;
    let response = shading.compute_edl_response(100.0, &[200.0, 300.0]);
    assert!((response - 1.0).abs() < 1e-10);
}

#[test]
fn edl_no_neighbors_returns_one() {
    let shading = PointCloudShading::default();
    let response = shading.compute_edl_response(100.0, &[]);
    assert!((response - 1.0).abs() < 1e-10);
}

#[test]
fn edl_same_depth_returns_one() {
    let shading = PointCloudShading::default();
    let response = shading.compute_edl_response(100.0, &[100.0, 100.0, 100.0]);
    // 相同深度 → 无差异 → response = 1.0
    assert!((response - 1.0).abs() < 1e-10);
}

#[test]
fn edl_occluding_edge_darker() {
    let shading = PointCloudShading::default();
    // 点比相邻点近得多（遮挡边缘）
    let response = shading.compute_edl_response(10.0, &[1000.0, 1000.0, 1000.0, 1000.0]);
    // 应更暗（小于 1.0）
    assert!(response < 1.0);
    assert!(response >= 0.0);
}

#[test]
fn edl_behind_neighbors_no_darkening() {
    let shading = PointCloudShading::default();
    // 点比相邻点远（非遮挡）
    let response = shading.compute_edl_response(1000.0, &[10.0, 10.0]);
    // diff = max(log2(10) - log2(1000), 0) = 0 → 无变暗
    assert!((response - 1.0).abs() < 1e-10);
}

// === QuantizedPositions ===

#[test]
fn quantized_dequantize_basic() {
    let qp = QuantizedPositions {
        values: vec![0, 32768, 65535, 65535, 0, 32768],
        volume_offset: [0.0, 0.0, 0.0],
        volume_scale: [10.0, 20.0, 30.0],
    };

    // 点 0：(0/65535, 32768/65535, 65535/65535) * scale + offset
    let p0 = qp.dequantize(0);
    assert!((p0[0] - 0.0).abs() < 0.01); // 0/65535 * 10
    assert!((p0[1] - 10.0).abs() < 0.01); // ~0.5 * 20
    assert!((p0[2] - 30.0).abs() < 0.01); // 1.0 * 30

    // 点 1：(65535/65535, 0/65535, 32768/65535) * scale
    let p1 = qp.dequantize(1);
    assert!((p1[0] - 10.0).abs() < 0.01); // 1.0 * 10
    assert!((p1[1] - 0.0).abs() < 0.01); // 0.0 * 20
    assert!((p1[2] - 15.0).abs() < 0.01); // ~0.5 * 30
}

#[test]
fn quantized_dequantize_with_offset() {
    let qp = QuantizedPositions {
        values: vec![0, 0, 0],
        volume_offset: [100.0, 200.0, 300.0],
        volume_scale: [10.0, 10.0, 10.0],
    };
    let p = qp.dequantize(0);
    assert!((p[0] - 100.0).abs() < 0.01);
    assert!((p[1] - 200.0).abs() < 0.01);
    assert!((p[2] - 300.0).abs() < 0.01);
}

#[test]
fn quantized_dequantize_out_of_range() {
    let qp = QuantizedPositions {
        values: vec![0, 0, 0],
        volume_offset: [0.0, 0.0, 0.0],
        volume_scale: [1.0, 1.0, 1.0],
    };
    // 索引 1 越界（只有 1 个点 = 3 个值）
    let p = qp.dequantize(5);
    assert_eq!(p, [0.0, 0.0, 0.0]);
}

// === 从 FeatureTable 构造 PointCloud ===

#[test]
fn point_cloud_from_feature_table() {
    // 创建含 2 个点的要素表
    let mut binary = Vec::new();
    binary.extend_from_slice(&1.0f32.to_le_bytes());
    binary.extend_from_slice(&2.0f32.to_le_bytes());
    binary.extend_from_slice(&3.0f32.to_le_bytes());
    binary.extend_from_slice(&4.0f32.to_le_bytes());
    binary.extend_from_slice(&5.0f32.to_le_bytes());
    binary.extend_from_slice(&6.0f32.to_le_bytes());

    let json = json!({
        "POINTS_LENGTH": 2,
        "POSITION": { "byteOffset": 0 },
        "RTC_CENTER": [100.0, 200.0, 300.0]
    });

    let ft = FeatureTable::new(Some(json), binary);
    let pc = PointCloud::from_feature_table(&ft).unwrap();

    assert_eq!(pc.points_length, 2);
    assert_eq!(pc.positions.len(), 2);
    assert!((pc.positions[0][0] - 1.0).abs() < 1e-6);
    assert!((pc.positions[1][2] - 6.0).abs() < 1e-6);
    assert!(pc.rtc_center.is_some());
    let rtc = pc.rtc_center.unwrap();
    assert!((rtc[0] - 100.0).abs() < 1e-10);
}

#[test]
fn point_cloud_no_positions_returns_none() {
    let json = json!({ "POINTS_LENGTH": 5 });
    let ft = FeatureTable::new(Some(json), vec![]);
    let pc = PointCloud::from_feature_table(&ft);
    assert!(pc.is_none());
}
