//! PointCloud extended specs - get_world_position/get_color/get_normal/bounding_sphere/TimeDynamic
//! Ported from Scene/PointCloudSpec.js (A-class logic paths)

use cesium_tileset::point_cloud::{PointCloud, TimeDynamicPointCloud};
use glam::DVec3;

fn make_cloud(positions: Vec<[f32; 3]>) -> PointCloud {
    let len = positions.len() as u32;
    PointCloud {
        points_length: len,
        positions,
        colors: None,
        normals: None,
        batch_ids: None,
        rtc_center: None,
        constant_rgba: None,
        quantized_positions: None,
    }
}

// ─── get_world_position ─────────────────────────────────────────────────────

#[test]
fn world_position_basic() {
    let cloud = make_cloud(vec![[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]);
    let pos = cloud.get_world_position(0).unwrap();
    assert!((pos.x - 1.0).abs() < 1e-6);
    assert!((pos.y - 2.0).abs() < 1e-6);
    assert!((pos.z - 3.0).abs() < 1e-6);
}

#[test]
fn world_position_with_rtc_center() {
    let mut cloud = make_cloud(vec![[1.0, 2.0, 3.0]]);
    cloud.rtc_center = Some([100.0, 200.0, 300.0]);
    let pos = cloud.get_world_position(0).unwrap();
    assert!((pos.x - 101.0).abs() < 1e-6);
    assert!((pos.y - 202.0).abs() < 1e-6);
    assert!((pos.z - 303.0).abs() < 1e-6);
}

#[test]
fn world_position_out_of_bounds() {
    let cloud = make_cloud(vec![[1.0, 2.0, 3.0]]);
    assert!(cloud.get_world_position(5).is_none());
}

#[test]
fn world_position_second_point() {
    let cloud = make_cloud(vec![[0.0, 0.0, 0.0], [10.0, 20.0, 30.0]]);
    let pos = cloud.get_world_position(1).unwrap();
    assert!((pos.x - 10.0).abs() < 1e-6);
    assert!((pos.y - 20.0).abs() < 1e-6);
}

// ─── get_color ──────────────────────────────────────────────────────────────

#[test]
fn color_default_white() {
    let cloud = make_cloud(vec![[0.0, 0.0, 0.0]]);
    let c = cloud.get_color(0);
    assert_eq!(c, [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn color_per_point() {
    let mut cloud = make_cloud(vec![[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]]);
    cloud.colors = Some(vec![[1.0, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, 1.0]]);
    assert_eq!(cloud.get_color(0), [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(cloud.get_color(1), [0.0, 1.0, 0.0, 1.0]);
}

#[test]
fn color_constant_rgba() {
    let mut cloud = make_cloud(vec![[0.0, 0.0, 0.0]]);
    cloud.constant_rgba = Some([0.5, 0.5, 0.5, 0.8]);
    assert_eq!(cloud.get_color(0), [0.5, 0.5, 0.5, 0.8]);
}

#[test]
fn color_per_point_overrides_constant() {
    let mut cloud = make_cloud(vec![[0.0, 0.0, 0.0]]);
    cloud.colors = Some(vec![[1.0, 0.0, 0.0, 1.0]]);
    cloud.constant_rgba = Some([0.0, 0.0, 1.0, 1.0]);
    // Per-point takes priority
    assert_eq!(cloud.get_color(0), [1.0, 0.0, 0.0, 1.0]);
}

#[test]
fn color_out_of_range_falls_back_to_constant() {
    let mut cloud = make_cloud(vec![[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]]);
    cloud.colors = Some(vec![[1.0, 0.0, 0.0, 1.0]]); // only 1 color for 2 points
    cloud.constant_rgba = Some([0.0, 1.0, 0.0, 1.0]);
    // Index 1 is out of range for colors, falls back to constant
    assert_eq!(cloud.get_color(1), [0.0, 1.0, 0.0, 1.0]);
}

// ─── get_normal ─────────────────────────────────────────────────────────────

#[test]
fn normal_none_when_no_normals() {
    let cloud = make_cloud(vec![[0.0, 0.0, 0.0]]);
    assert!(cloud.get_normal(0).is_none());
}

#[test]
fn normal_returns_value() {
    let mut cloud = make_cloud(vec![[0.0, 0.0, 0.0]]);
    cloud.normals = Some(vec![[0.0, 0.0, 1.0]]);
    let n = cloud.get_normal(0).unwrap();
    assert_eq!(n, [0.0, 0.0, 1.0]);
}

#[test]
fn normal_out_of_range() {
    let mut cloud = make_cloud(vec![[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]]);
    cloud.normals = Some(vec![[0.0, 1.0, 0.0]]);
    assert!(cloud.get_normal(1).is_none());
}

// ─── compute_bounding_sphere ────────────────────────────────────────────────

#[test]
fn bounding_sphere_empty() {
    let cloud = make_cloud(vec![]);
    assert!(cloud.compute_bounding_sphere().is_none());
}

#[test]
fn bounding_sphere_single_point() {
    let cloud = make_cloud(vec![[5.0, 5.0, 5.0]]);
    let (center, radius) = cloud.compute_bounding_sphere().unwrap();
    assert!((center.x - 5.0).abs() < 1e-6);
    assert!((center.y - 5.0).abs() < 1e-6);
    assert!((center.z - 5.0).abs() < 1e-6);
    assert!(radius.abs() < 1e-6);
}

#[test]
fn bounding_sphere_two_points() {
    let cloud = make_cloud(vec![[0.0, 0.0, 0.0], [10.0, 0.0, 0.0]]);
    let (center, radius) = cloud.compute_bounding_sphere().unwrap();
    assert!((center.x - 5.0).abs() < 1e-6);
    assert!((radius - 5.0).abs() < 1e-6);
}

#[test]
fn bounding_sphere_with_rtc_center() {
    let mut cloud = make_cloud(vec![[0.0, 0.0, 0.0], [10.0, 0.0, 0.0]]);
    cloud.rtc_center = Some([100.0, 0.0, 0.0]);
    let (center, radius) = cloud.compute_bounding_sphere().unwrap();
    assert!((center.x - 105.0).abs() < 1e-6);
    assert!((radius - 5.0).abs() < 1e-6);
}

// ─── TimeDynamicPointCloud ──────────────────────────────────────────────────

#[test]
fn time_dynamic_not_dynamic_when_empty() {
    let td = TimeDynamicPointCloud::new(vec![], vec![]);
    assert!(!td.is_time_dynamic);
}

#[test]
fn time_dynamic_is_dynamic_with_timestamps() {
    let td = TimeDynamicPointCloud::new(
        vec![0.0, 1.0, 2.0],
        vec!["f0.pnts".into(), "f1.pnts".into(), "f2.pnts".into()],
    );
    assert!(td.is_time_dynamic);
}

#[test]
fn time_dynamic_frame_index_exact() {
    let td = TimeDynamicPointCloud::new(
        vec![0.0, 1.0, 2.0],
        vec!["a".into(), "b".into(), "c".into()],
    );
    assert_eq!(td.get_frame_index(0.0), Some(0));
    assert_eq!(td.get_frame_index(1.0), Some(1));
    assert_eq!(td.get_frame_index(2.0), Some(2));
}

#[test]
fn time_dynamic_frame_index_between() {
    let td = TimeDynamicPointCloud::new(
        vec![0.0, 1.0, 2.0],
        vec!["a".into(), "b".into(), "c".into()],
    );
    // Between 0 and 1, should return index 1 (first ts >= time)
    assert_eq!(td.get_frame_index(0.5), Some(1));
}

#[test]
fn time_dynamic_frame_index_after_all() {
    let td = TimeDynamicPointCloud::new(
        vec![0.0, 1.0, 2.0],
        vec!["a".into(), "b".into(), "c".into()],
    );
    // After all timestamps, returns last
    assert_eq!(td.get_frame_index(5.0), Some(2));
}

#[test]
fn time_dynamic_frame_index_empty() {
    let td = TimeDynamicPointCloud::new(vec![], vec![]);
    assert_eq!(td.get_frame_index(0.0), None);
}

#[test]
fn time_dynamic_get_uri() {
    let td = TimeDynamicPointCloud::new(
        vec![0.0, 1.0, 2.0],
        vec!["frame0.pnts".into(), "frame1.pnts".into(), "frame2.pnts".into()],
    );
    assert_eq!(td.get_uri(0.0), Some("frame0.pnts"));
    assert_eq!(td.get_uri(1.5), Some("frame2.pnts"));
    assert_eq!(td.get_uri(10.0), Some("frame2.pnts"));
}

#[test]
fn time_dynamic_interpolation_disabled() {
    let td = TimeDynamicPointCloud::new(
        vec![0.0, 1.0, 2.0],
        vec!["a".into(), "b".into(), "c".into()],
    );
    // interpolate defaults to false
    assert!(td.get_interpolation_factor(0.5).is_none());
}

#[test]
fn time_dynamic_interpolation_enabled() {
    let mut td = TimeDynamicPointCloud::new(
        vec![0.0, 1.0, 2.0],
        vec!["a".into(), "b".into(), "c".into()],
    );
    td.interpolate = true;
    let (i0, i1, factor) = td.get_interpolation_factor(0.5).unwrap();
    assert_eq!(i0, 0);
    assert_eq!(i1, 1);
    assert!((factor - 0.5).abs() < 1e-10);
}

#[test]
fn time_dynamic_interpolation_at_boundary() {
    let mut td = TimeDynamicPointCloud::new(
        vec![0.0, 1.0, 2.0],
        vec!["a".into(), "b".into(), "c".into()],
    );
    td.interpolate = true;
    // time=1.0 matches first interval [0,1] with factor=1.0
    let (i0, i1, factor) = td.get_interpolation_factor(1.0).unwrap();
    assert_eq!(i0, 0);
    assert_eq!(i1, 1);
    assert!((factor - 1.0).abs() < 1e-10);
}

#[test]
fn time_dynamic_interpolation_single_frame() {
    let mut td = TimeDynamicPointCloud::new(vec![0.0], vec!["a".into()]);
    td.interpolate = true;
    // Need at least 2 frames for interpolation
    assert!(td.get_interpolation_factor(0.0).is_none());
}
