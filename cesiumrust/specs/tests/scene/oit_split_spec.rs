//! OIT + SplitDirection specs
//! Ported from CesiumJS Scene/OITSpec.js + Scene/SplitDirectionSpec.js

use cesium_effects::{
    BlendEquation, BlendFunction, OitCapabilities, OitConfig, OitMode, SplitDirection,
    SplitterConfig,
};
use glam::DVec4;

// ==================== OIT: Capabilities ====================

#[test]
fn oit_capabilities_mrt_mode() {
    let caps = OitCapabilities {
        mrt_supported: true,
        float_blend_supported: true,
        depth_texture_supported: true,
        color_buffer_float: true,
    };
    assert!(caps.translucent_mrt_supported());
    assert!(!caps.translucent_multipass_supported());
    assert!(caps.is_supported());
}

#[test]
fn oit_capabilities_multipass_fallback() {
    let caps = OitCapabilities {
        mrt_supported: false,
        float_blend_supported: true,
        depth_texture_supported: true,
        color_buffer_float: true,
    };
    assert!(!caps.translucent_mrt_supported());
    assert!(caps.translucent_multipass_supported());
    assert!(caps.is_supported());
}

#[test]
fn oit_capabilities_unsupported() {
    let caps = OitCapabilities::default();
    assert!(!caps.is_supported());
    assert!(!caps.translucent_mrt_supported());
    assert!(!caps.translucent_multipass_supported());
}

// ==================== OIT: Config ====================

#[test]
fn oit_config_from_mrt_capabilities() {
    let caps = OitCapabilities {
        mrt_supported: true,
        float_blend_supported: true,
        depth_texture_supported: true,
        color_buffer_float: true,
    };
    let config = OitConfig::from_capabilities(&caps);
    assert_eq!(config.mode, OitMode::WeightedBlendedMrt);
    assert!(config.is_active());
}

#[test]
fn oit_config_from_multipass_capabilities() {
    let caps = OitCapabilities {
        mrt_supported: false,
        float_blend_supported: true,
        depth_texture_supported: true,
        color_buffer_float: true,
    };
    let config = OitConfig::from_capabilities(&caps);
    assert_eq!(config.mode, OitMode::WeightedBlendedMultipass);
    assert!(config.is_active());
}

#[test]
fn oit_config_unsupported_is_none() {
    let caps = OitCapabilities::default();
    let config = OitConfig::from_capabilities(&caps);
    assert_eq!(config.mode, OitMode::None);
    assert!(!config.is_active());
}

#[test]
fn oit_config_defaults() {
    let config = OitConfig::default();
    assert_eq!(config.mode, OitMode::None);
    assert_eq!(config.num_samples, 1);
    assert!(!config.use_hdr);
    assert_eq!(config.blend_equation, BlendEquation::Add);
    assert_eq!(config.source_blend, BlendFunction::One);
    assert_eq!(config.destination_blend, BlendFunction::One);
}

// ==================== OIT: Weight function ====================

#[test]
fn oit_weight_near_greater_than_far() {
    let config = OitConfig::default();
    let near = config.compute_weight(1.0, 1.0);
    let far = config.compute_weight(1.0, 100.0);
    assert!(near > far);
}

#[test]
fn oit_weight_zero_alpha_is_zero() {
    let config = OitConfig::default();
    let w = config.compute_weight(0.0, 10.0);
    assert!((w).abs() < 1e-10);
}

#[test]
fn oit_weight_proportional_to_alpha() {
    let config = OitConfig::default();
    let w1 = config.compute_weight(0.5, 10.0);
    let w2 = config.compute_weight(1.0, 10.0);
    assert!((w2 / w1 - 2.0).abs() < 1e-10);
}

// ==================== OIT: Accumulate + Composite ====================

#[test]
fn oit_accumulate_revealage() {
    let config = OitConfig::default();
    let color = DVec4::new(1.0, 0.0, 0.0, 0.3);
    let (_, revealage) = config.accumulate_fragment(color, 10.0);
    assert!((revealage - 0.7).abs() < 1e-10); // 1 - alpha
}

#[test]
fn oit_composite_fully_opaque_returns_opaque() {
    let config = OitConfig::default();
    let opaque = DVec4::new(0.2, 0.4, 0.6, 1.0);
    let result = config.composite(opaque, DVec4::ZERO, 1.0);
    assert!((result - opaque).length() < 1e-10);
}

#[test]
fn oit_composite_blends_translucent() {
    let config = OitConfig::default();
    let opaque = DVec4::new(0.0, 0.0, 1.0, 1.0);
    let accumulation = DVec4::new(0.5, 0.0, 0.0, 0.5);
    let revealage = 0.5;
    let result = config.composite(opaque, accumulation, revealage);
    // Should have both red and blue
    assert!(result.x > 0.0);
    assert!(result.z > 0.0);
}

// ==================== SplitDirection ====================

#[test]
fn split_direction_shader_values() {
    assert!((SplitDirection::Left.to_shader_value() - (-1.0)).abs() < 1e-10);
    assert!((SplitDirection::None.to_shader_value()).abs() < 1e-10);
    assert!((SplitDirection::Right.to_shader_value() - 1.0).abs() < 1e-10);
}

#[test]
fn split_direction_from_shader_value() {
    assert_eq!(SplitDirection::from_shader_value(-1.0), SplitDirection::Left);
    assert_eq!(SplitDirection::from_shader_value(0.0), SplitDirection::None);
    assert_eq!(SplitDirection::from_shader_value(1.0), SplitDirection::Right);
    assert_eq!(SplitDirection::from_shader_value(0.3), SplitDirection::None);
}

#[test]
fn split_direction_is_split() {
    assert!(SplitDirection::Left.is_split());
    assert!(!SplitDirection::None.is_split());
    assert!(SplitDirection::Right.is_split());
}

#[test]
fn split_direction_should_show_at() {
    let split_pos = 0.5;
    // None always shows
    assert!(SplitDirection::None.should_show_at(0.0, split_pos));
    assert!(SplitDirection::None.should_show_at(1.0, split_pos));
    // Left shows at/before split
    assert!(SplitDirection::Left.should_show_at(0.3, split_pos));
    assert!(SplitDirection::Left.should_show_at(0.5, split_pos));
    assert!(!SplitDirection::Left.should_show_at(0.7, split_pos));
    // Right shows after split
    assert!(!SplitDirection::Right.should_show_at(0.3, split_pos));
    assert!(SplitDirection::Right.should_show_at(0.7, split_pos));
}

// ==================== SplitterConfig ====================

#[test]
fn splitter_config_default() {
    let config = SplitterConfig::default();
    assert!(!config.enabled);
    assert!((config.split_position - 0.5).abs() < 1e-10);
}

#[test]
fn splitter_config_clamps_position() {
    let config = SplitterConfig::new(true, 1.5);
    assert!((config.split_position - 1.0).abs() < 1e-10);
    let config2 = SplitterConfig::new(true, -0.5);
    assert!((config2.split_position).abs() < 1e-10);
}

#[test]
fn splitter_config_set_position_clamps() {
    let mut config = SplitterConfig::default();
    config.set_split_position(2.0);
    assert!((config.split_position - 1.0).abs() < 1e-10);
    config.set_split_position(-1.0);
    assert!((config.split_position).abs() < 1e-10);
}

#[test]
fn splitter_config_position_pixels() {
    let config = SplitterConfig::new(true, 0.25);
    assert!((config.split_position_pixels(1920.0) - 480.0).abs() < 1e-10);
}
