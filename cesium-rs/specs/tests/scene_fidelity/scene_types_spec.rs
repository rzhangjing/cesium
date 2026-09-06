//! Specs for Scene types substantiated from stubs:
//! - `Light` trait, `DirectionalLight`, `SunLight`
//! - `TileDiscardPolicy` trait, `NeverTileDiscardPolicy`,
//!   `DiscardEmptyTileImagePolicy`, `DiscardMissingTileImagePolicy`
//! - `EllipsoidSurfaceAppearance`

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;

use cesium_scene::directional_light::{DirectionalLight, DirectionalLightOptions};
use cesium_scene::discard_empty_tile_image_policy::DiscardEmptyTileImagePolicy;
use cesium_scene::discard_missing_tile_image_policy::{
    DiscardMissingTileImagePolicy, DiscardMissingTileImagePolicyOptions,
};
use cesium_scene::ellipsoid_surface_appearance::{
    EllipsoidSurfaceAppearance, EllipsoidSurfaceAppearanceOptions,
};
use cesium_scene::light::Light;
use cesium_scene::never_tile_discard_policy::NeverTileDiscardPolicy;
use cesium_scene::sun_light::{SunLight, SunLightOptions};
use cesium_scene::tile_discard_policy::TileDiscardPolicy;

// ─── DirectionalLight ──────────────────────────────────────────────

#[test]
fn directional_light_stores_direction_color_and_intensity() {
    let light = DirectionalLight::new(DirectionalLightOptions {
        direction: Cartesian3::new(0.0, 1.0, 0.0),
        color: Some(Color { red: 1.0, green: 0.0, blue: 0.0, alpha: 1.0 }),
        intensity: Some(3.0),
    });
    assert_eq!(light.direction, Cartesian3::new(0.0, 1.0, 0.0));
    assert_eq!(light.color.red, 1.0);
    assert_eq!(light.color.green, 0.0);
    assert_eq!(light.intensity, 3.0);
}

#[test]
fn directional_light_defaults_color_to_white_and_intensity_to_one() {
    let light = DirectionalLight::new(DirectionalLightOptions {
        direction: Cartesian3::new(1.0, 0.0, 0.0),
        color: None,
        intensity: None,
    });
    assert_eq!(light.color, Color::WHITE);
    assert_eq!(light.intensity, 1.0);
}

#[test]
fn directional_light_implements_light_trait() {
    let light = DirectionalLight::new(DirectionalLightOptions {
        direction: Cartesian3::new(0.0, 0.0, -1.0),
        color: None,
        intensity: Some(0.5),
    });
    let light_ref: &dyn Light = &light;
    assert_eq!(light_ref.intensity(), 0.5);
    assert_eq!(light_ref.color(), &Color::WHITE);
}

#[test]
#[should_panic(expected = "options.direction cannot be zero-length")]
fn directional_light_panics_on_zero_direction() {
    DirectionalLight::new(DirectionalLightOptions {
        direction: Cartesian3::ZERO,
        color: None,
        intensity: None,
    });
}

// ─── SunLight ──────────────────────────────────────────────────────

#[test]
fn sun_light_defaults() {
    let light = SunLight::new(None);
    assert_eq!(light.color, Color::WHITE);
    assert_eq!(light.intensity, 2.0);
}

#[test]
fn sun_light_custom_options() {
    let light = SunLight::new(Some(SunLightOptions {
        color: Some(Color { red: 0.5, green: 0.5, blue: 0.5, alpha: 1.0 }),
        intensity: Some(4.0),
    }));
    assert_eq!(light.color.red, 0.5);
    assert_eq!(light.intensity, 4.0);
}

#[test]
fn sun_light_implements_light_trait() {
    let light = SunLight::new(None);
    let light_ref: &dyn Light = &light;
    assert_eq!(light_ref.intensity(), 2.0);
}

// ─── NeverTileDiscardPolicy ────────────────────────────────────────

#[test]
fn never_policy_is_always_ready_and_never_discards() {
    let policy = NeverTileDiscardPolicy::new();
    assert!(policy.is_ready());
    assert!(!policy.should_discard_image(&[255; 16], 2));
    assert!(!policy.should_discard_image(&[0; 16], 2));
}

// ─── DiscardEmptyTileImagePolicy ───────────────────────────────────

#[test]
fn empty_policy_is_ready_and_discards_all_zero_pixels() {
    let policy = DiscardEmptyTileImagePolicy::new();
    assert!(policy.is_ready());
    // All zeros → empty → discard
    assert!(policy.should_discard_image(&[0; 64], 4));
}

#[test]
fn empty_policy_does_not_discard_non_zero_pixels() {
    let policy = DiscardEmptyTileImagePolicy::new();
    // Has at least one non-zero byte → not empty → keep
    let mut pixels = [0u8; 64];
    pixels[0] = 255;
    assert!(!policy.should_discard_image(&pixels, 4));
}

// ─── DiscardMissingTileImagePolicy ─────────────────────────────────

#[test]
fn missing_policy_starts_not_ready() {
    let policy = DiscardMissingTileImagePolicy::new(
        DiscardMissingTileImagePolicyOptions {
            pixels_to_check: vec![Cartesian2 { x: 0.0, y: 0.0 }],
            disable_check_if_all_pixels_are_transparent: None,
        },
    );
    assert!(!policy.is_ready());
}

#[test]
fn missing_policy_becomes_ready_after_setting_reference_pixels() {
    let mut policy = DiscardMissingTileImagePolicy::new(
        DiscardMissingTileImagePolicyOptions {
            pixels_to_check: vec![Cartesian2 { x: 0.0, y: 0.0 }],
            disable_check_if_all_pixels_are_transparent: None,
        },
    );
    // 2×2 RGBA image
    let reference = vec![
        255, 0, 0, 255,   0, 255, 0, 255,
        0, 0, 255, 255,   255, 255, 0, 255,
    ];
    policy.set_missing_image_pixels(reference, None);
    assert!(policy.is_ready());
}

#[test]
fn missing_policy_discards_matching_image() {
    let mut policy = DiscardMissingTileImagePolicy::new(
        DiscardMissingTileImagePolicyOptions {
            pixels_to_check: vec![Cartesian2 { x: 0.0, y: 0.0 }],
            disable_check_if_all_pixels_are_transparent: None,
        },
    );
    // Reference: pixel (0,0) = [255, 0, 0, 255]
    let reference = vec![
        255, 0, 0, 255,   0, 255, 0, 255,
        0, 0, 255, 255,   255, 255, 0, 255,
    ];
    policy.set_missing_image_pixels(reference, None);

    // Test image with same pixel (0,0) → should discard
    let test_image = vec![
        255, 0, 0, 255,   128, 128, 128, 255,
        64, 64, 64, 255,  32, 32, 32, 255,
    ];
    assert!(policy.should_discard_image(&test_image, 2));
}

#[test]
fn missing_policy_keeps_non_matching_image() {
    let mut policy = DiscardMissingTileImagePolicy::new(
        DiscardMissingTileImagePolicyOptions {
            pixels_to_check: vec![Cartesian2 { x: 0.0, y: 0.0 }],
            disable_check_if_all_pixels_are_transparent: None,
        },
    );
    // Reference: pixel (0,0) = [255, 0, 0, 255]
    let reference = vec![
        255, 0, 0, 255,   0, 255, 0, 255,
        0, 0, 255, 255,   255, 255, 0, 255,
    ];
    policy.set_missing_image_pixels(reference, None);

    // Test image with different pixel (0,0) → should NOT discard
    let test_image = vec![
        0, 0, 255, 255,   128, 128, 128, 255,
        64, 64, 64, 255,  32, 32, 32, 255,
    ];
    assert!(!policy.should_discard_image(&test_image, 2));
}

#[test]
fn missing_policy_disable_skips_check() {
    let mut policy = DiscardMissingTileImagePolicy::new(
        DiscardMissingTileImagePolicyOptions {
            pixels_to_check: vec![Cartesian2 { x: 0.0, y: 0.0 }],
            disable_check_if_all_pixels_are_transparent: None,
        },
    );
    policy.disable();
    assert!(policy.is_ready());
    // After disable, should never discard
    assert!(!policy.should_discard_image(&[255; 16], 2));
}

// ─── EllipsoidSurfaceAppearance ────────────────────────────────────

#[test]
fn ellipsoid_surface_appearance_defaults() {
    let appearance = EllipsoidSurfaceAppearance::new(None);
    assert!(appearance.translucent);
    assert!(!appearance.flat);
    assert!(!appearance.face_forward);
    assert!(!appearance.above_ground);
    assert!(!appearance.closed);
    assert_eq!(appearance.material.type_name, "Color");
}

#[test]
fn ellipsoid_surface_appearance_vertex_format() {
    let appearance = EllipsoidSurfaceAppearance::new(None);
    let vf = appearance.vertex_format();
    assert!(vf.position);
    assert!(vf.st);
    assert!(!vf.normal);
    assert!(!vf.tangent);
    assert!(!vf.bitangent);
    assert!(!vf.color);
}

#[test]
fn ellipsoid_surface_appearance_above_ground_sets_face_forward() {
    let appearance = EllipsoidSurfaceAppearance::new(Some(
        EllipsoidSurfaceAppearanceOptions {
            above_ground: Some(true),
            ..Default::default()
        },
    ));
    assert!(appearance.above_ground);
    // face_forward defaults to above_ground
    assert!(appearance.face_forward);
    // translucent default is true, so blending should be on
    assert!(appearance.render_state.blending);
}

#[test]
fn ellipsoid_surface_appearance_custom_options() {
    let appearance = EllipsoidSurfaceAppearance::new(Some(
        EllipsoidSurfaceAppearanceOptions {
            flat: Some(true),
            translucent: Some(false),
            face_forward: Some(true),
            ..Default::default()
        },
    ));
    assert!(appearance.flat);
    assert!(!appearance.translucent);
    assert!(appearance.face_forward);
    assert!(!appearance.render_state.blending);
    assert!(appearance.is_translucent() == false);
}
