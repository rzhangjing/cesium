//! Shadow map extended specs - tests for ShadowMap, ShadowBias, and cascade computation
//!
//! Covers: bias construction, shadow map construction, fade factor, cascade splits

use cesium_shadow::{ShadowBias, ShadowMap, ShadowMapConfig};
use glam::DVec3;

const EPSILON3: f64 = 1e-3;
const EPSILON6: f64 = 1e-6;

// ─── ShadowBias construction ─────────────────────────────────────────────────

#[test]
fn shadow_bias_terrain_default() {
    let bias = ShadowBias::terrain(false);
    assert!(bias.polygon_offset);
    assert!(bias.polygon_offset_factor > 0.0);
    assert!(bias.polygon_offset_units > 0.0);
    assert!(!bias.normal_offset);
    assert!(bias.depth_bias > 0.0);
}

#[test]
fn shadow_bias_terrain_with_normal_offset() {
    let bias = ShadowBias::terrain(true);
    assert!(bias.normal_offset);
    assert!(bias.normal_offset_scale > 0.0);
}

#[test]
fn shadow_bias_primitive_default() {
    let bias = ShadowBias::primitive(false);
    assert!(bias.polygon_offset);
    assert!(!bias.normal_offset);
    assert!(bias.depth_bias > 0.0);
}

#[test]
fn shadow_bias_primitive_with_normal_offset() {
    let bias = ShadowBias::primitive(true);
    assert!(bias.normal_offset);
}

#[test]
fn shadow_bias_point_default() {
    let bias = ShadowBias::point(false);
    assert!(bias.depth_bias > 0.0);
}

#[test]
fn shadow_bias_terrain_vs_primitive() {
    let terrain = ShadowBias::terrain(false);
    let primitive = ShadowBias::primitive(false);
    // Terrain should have larger depth bias than primitive
    assert!(
        terrain.depth_bias > primitive.depth_bias,
        "terrain depth_bias {} should be > primitive {}",
        terrain.depth_bias,
        primitive.depth_bias
    );
}

// ─── ShadowMap construction ──────────────────────────────────────────────────

#[test]
fn shadow_map_for_sun() {
    let sun_dir = DVec3::new(0.0, 0.0, -1.0);
    let shadow_map = ShadowMap::for_sun(sun_dir);
    assert!(shadow_map.pass_count() >= 1);
}

#[test]
fn shadow_map_for_point_light() {
    let position = DVec3::new(10.0, 10.0, 10.0);
    let radius = 100.0;
    let shadow_map = ShadowMap::for_point_light(position, radius);
    assert!(shadow_map.pass_count() >= 1);
}

#[test]
fn shadow_map_for_spot_light() {
    let position = DVec3::new(0.0, 10.0, 0.0);
    let direction = DVec3::new(0.0, -1.0, 0.0);
    let shadow_map = ShadowMap::for_spot_light(position, direction);
    assert!(shadow_map.pass_count() >= 1);
}

#[test]
fn shadow_map_new_with_config() {
    let config = ShadowMapConfig::default();
    let light_dir = DVec3::new(0.0, 0.0, -1.0);
    let shadow_map = ShadowMap::new(config, light_dir);
    assert!(shadow_map.pass_count() >= 1);
}

// ─── ShadowMap fade ──────────────────────────────────────────────────────────

#[test]
fn shadow_map_fade_factor_noon() {
    let config = ShadowMapConfig::default();
    let shadow_map = ShadowMap::new(config, DVec3::new(0.0, 0.0, -1.0));
    let fade = shadow_map.compute_fade_factor(std::f64::consts::FRAC_PI_2);
    // Noon (high elevation) should have full shadow
    assert!(
        (fade - 1.0).abs() < EPSILON3,
        "noon fade should be ~1.0, got {}",
        fade
    );
}

#[test]
fn shadow_map_fade_factor_sunset() {
    let config = ShadowMapConfig::default();
    let shadow_map = ShadowMap::new(config, DVec3::new(1.0, 0.0, 0.0));
    let fade = shadow_map.compute_fade_factor(0.0);
    // Sunset (low elevation) should fade shadows
    assert!(
        fade < 1.0,
        "sunset fade should be < 1.0, got {}",
        fade
    );
}

#[test]
fn shadow_map_fade_factor_night() {
    let config = ShadowMapConfig::default();
    let shadow_map = ShadowMap::new(config, DVec3::new(0.0, 0.0, 1.0));
    let fade = shadow_map.compute_fade_factor(-std::f64::consts::FRAC_PI_4);
    // Night (negative elevation) should have no shadows
    assert!(
        fade.abs() < EPSILON3,
        "night fade should be ~0.0, got {}",
        fade
    );
}

// ─── ShadowMap cascade splits ────────────────────────────────────────────────

#[test]
fn shadow_map_cascade_splits_linear() {
    let config = ShadowMapConfig::default();
    let shadow_map = ShadowMap::new(config, DVec3::new(0.0, 0.0, -1.0));
    let near = 1.0;
    let far = 100.0;
    let lambda = 0.0; // Pure linear
    let splits = shadow_map.compute_cascade_splits(near, far, lambda);
    // Should produce at least 1 split
    assert!(!splits.is_empty(), "should produce at least 1 split");
    // Splits should be monotonically increasing
    for i in 1..splits.len() {
        assert!(
            splits[i] > splits[i - 1],
            "splits should be increasing: {} <= {}",
            splits[i],
            splits[i - 1]
        );
    }
}

#[test]
fn shadow_map_cascade_splits_logarithmic() {
    let config = ShadowMapConfig::default();
    let shadow_map = ShadowMap::new(config, DVec3::new(0.0, 0.0, -1.0));
    let near = 1.0;
    let far = 100.0;
    let lambda = 1.0; // Pure logarithmic
    let splits = shadow_map.compute_cascade_splits(near, far, lambda);
    // Should produce at least 1 split
    assert!(!splits.is_empty(), "should produce at least 1 split");
    // Logarithmic splits should be closer together near the camera
    if splits.len() >= 3 {
        let gap1 = splits[1] - splits[0];
        let gap2 = splits[2] - splits[1];
        assert!(
            gap2 > gap1,
            "log splits should have larger gaps further away: gap1={}, gap2={}",
            gap1,
            gap2
        );
    }
}

#[test]
fn shadow_map_cascade_splits_practical() {
    let config = ShadowMapConfig::default();
    let shadow_map = ShadowMap::new(config, DVec3::new(0.0, 0.0, -1.0));
    let near = 0.1;
    let far = 1000.0;
    let lambda = 0.5; // Practical blend
    let splits = shadow_map.compute_cascade_splits(near, far, lambda);
    // All splits should be within [near, far]
    for &split in &splits {
        assert!(
            split >= near && split <= far,
            "split {} out of range [{}, {}]",
            split,
            near,
            far
        );
    }
}

// ─── ShadowMap bias types ────────────────────────────────────────────────────

#[test]
fn shadow_map_bias_for_type_terrain() {
    let config = ShadowMapConfig::default();
    let shadow_map = ShadowMap::new(config, DVec3::new(0.0, 0.0, -1.0));
    let bias = shadow_map.bias_for_type(cesium_shadow::ShadowBiasType::Terrain);
    assert!(bias.depth_bias > 0.0);
}

#[test]
fn shadow_map_bias_for_type_primitive() {
    let config = ShadowMapConfig::default();
    let shadow_map = ShadowMap::new(config, DVec3::new(0.0, 0.0, -1.0));
    let bias = shadow_map.bias_for_type(cesium_shadow::ShadowBiasType::Primitive);
    assert!(bias.depth_bias > 0.0);
}

// ─── ShadowMap update ────────────────────────────────────────────────────────

#[test]
fn shadow_map_update_fade() {
    let config = ShadowMapConfig::default();
    let mut shadow_map = ShadowMap::new(config, DVec3::new(0.0, 0.0, -1.0));
    shadow_map.update_fade(std::f64::consts::FRAC_PI_4);
    // Should update internal fade state
    let fade = shadow_map.compute_fade_factor(std::f64::consts::FRAC_PI_4);
    assert!(fade >= 0.0 && fade <= 1.0);
}

#[test]
fn shadow_map_pass_count_cascades() {
    let mut config = ShadowMapConfig::default();
    config.cascade_count = 4;
    let shadow_map = ShadowMap::new(config, DVec3::new(0.0, 0.0, -1.0));
    assert_eq!(shadow_map.pass_count(), 4);
}
