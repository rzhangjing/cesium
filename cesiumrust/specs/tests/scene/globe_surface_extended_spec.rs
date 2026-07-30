//! GlobeSurface extended specs - ported from GlobeSpec.js
//!
//! Tests NearFarScalar interpolation, GlobeSurface ray picking,
//! horizon distance/dip angle, visible hemisphere, tile SSE computation,
//! GlobeTranslucency, ShadowMode, GlobeConfig defaults.

use cesium_globe::{GlobeConfig, GlobeSurface, GlobeTranslucency, NearFarScalar, ShadowMode};
use cesium_geospatial::Ellipsoid;
use glam::DVec3;

// ─── NearFarScalar ─────────────────────────────────────────────────────────

#[test]
fn near_far_scalar_at_near() {
    let nfs = NearFarScalar::new(100.0, 10.0, 1000.0, 1.0);
    assert!((nfs.interpolate(100.0) - 10.0).abs() < 1e-10);
}

#[test]
fn near_far_scalar_at_far() {
    let nfs = NearFarScalar::new(100.0, 10.0, 1000.0, 1.0);
    assert!((nfs.interpolate(1000.0) - 1.0).abs() < 1e-10);
}

#[test]
fn near_far_scalar_before_near_clamps() {
    let nfs = NearFarScalar::new(100.0, 10.0, 1000.0, 1.0);
    assert!((nfs.interpolate(50.0) - 10.0).abs() < 1e-10);
}

#[test]
fn near_far_scalar_beyond_far_clamps() {
    let nfs = NearFarScalar::new(100.0, 10.0, 1000.0, 1.0);
    assert!((nfs.interpolate(2000.0) - 1.0).abs() < 1e-10);
}

#[test]
fn near_far_scalar_midpoint() {
    let nfs = NearFarScalar::new(0.0, 0.0, 100.0, 100.0);
    assert!((nfs.interpolate(50.0) - 50.0).abs() < 1e-10);
}

#[test]
fn near_far_scalar_quarter() {
    let nfs = NearFarScalar::new(0.0, 0.0, 100.0, 200.0);
    assert!((nfs.interpolate(25.0) - 50.0).abs() < 1e-10);
}

// ─── GlobeSurface Pick ─────────────────────────────────────────────────────

#[test]
fn globe_pick_from_above_hits() {
    let globe = GlobeSurface::new();
    let origin = DVec3::new(0.0, 0.0, Ellipsoid::WGS84.maximum_radius() + 1000000.0);
    let direction = DVec3::new(0.0, 0.0, -1.0);

    let hit = globe.pick(origin, direction);
    assert!(hit.is_some());
    let point = hit.unwrap();
    // Should hit near the north pole (z ≈ polar radius)
    assert!(point.z > 6300000.0);
}

#[test]
fn globe_pick_misses() {
    let globe = GlobeSurface::new();
    // Ray pointing away from globe
    let origin = DVec3::new(0.0, 0.0, Ellipsoid::WGS84.maximum_radius() + 1000000.0);
    let direction = DVec3::new(0.0, 0.0, 1.0); // pointing away

    let hit = globe.pick(origin, direction);
    assert!(hit.is_none());
}

#[test]
fn globe_pick_tangent_misses() {
    let globe = GlobeSurface::new();
    // Ray parallel to surface, far from globe
    let r = Ellipsoid::WGS84.maximum_radius();
    let origin = DVec3::new(0.0, r + 100000.0, 0.0);
    let direction = DVec3::new(1.0, 0.0, 0.0); // parallel

    let hit = globe.pick(origin, direction);
    assert!(hit.is_none());
}

#[test]
fn globe_pick_from_inside() {
    let globe = GlobeSurface::new();
    // Origin inside the ellipsoid
    let origin = DVec3::new(0.0, 0.0, 0.0);
    let direction = DVec3::new(0.0, 0.0, 1.0);

    let hit = globe.pick(origin, direction);
    assert!(hit.is_some());
    let point = hit.unwrap();
    // Should hit the surface in +z direction
    assert!(point.z > 6300000.0);
}

// ─── Horizon Distance / Dip Angle ─────────────────────────────────────────

#[test]
fn horizon_distance_zero_height() {
    let globe = GlobeSurface::new();
    let d = globe.horizon_distance(0.0);
    assert!(d.abs() < 1e-6);
}

#[test]
fn horizon_distance_positive_height() {
    let globe = GlobeSurface::new();
    let r = Ellipsoid::WGS84.maximum_radius();
    let h = 10000.0; // 10 km
    let expected = (2.0 * r * h + h * h).sqrt();
    let d = globe.horizon_distance(h);
    assert!((d - expected).abs() < 1.0);
    // Should be roughly 357 km for 10km height on Earth
    assert!(d > 300000.0 && d < 400000.0);
}

#[test]
fn horizon_dip_angle_zero_height() {
    let globe = GlobeSurface::new();
    let dip = globe.horizon_dip_angle(0.0);
    assert!(dip.abs() < 1e-10);
}

#[test]
fn horizon_dip_angle_positive() {
    let globe = GlobeSurface::new();
    let dip = globe.horizon_dip_angle(10000.0);
    // Dip angle should be small positive (a few degrees)
    assert!(dip > 0.0);
    assert!(dip < 0.1); // less than ~5.7 degrees
}

// ─── Visible Hemisphere ────────────────────────────────────────────────────

#[test]
fn visible_hemisphere_facing_camera() {
    let globe = GlobeSurface::new();
    let r = Ellipsoid::WGS84.maximum_radius();
    // Point on +Z surface, camera further along +Z
    let position = DVec3::new(0.0, 0.0, r * 0.99);
    let camera = DVec3::new(0.0, 0.0, r * 2.0);
    assert!(globe.is_on_visible_hemisphere(position, camera));
}

#[test]
fn visible_hemisphere_facing_away() {
    let globe = GlobeSurface::new();
    let r = Ellipsoid::WGS84.maximum_radius();
    // Point on -Z surface, camera on +Z side
    let position = DVec3::new(0.0, 0.0, -r * 0.99);
    let camera = DVec3::new(0.0, 0.0, r * 2.0);
    assert!(!globe.is_on_visible_hemisphere(position, camera));
}

// ─── Tile SSE ──────────────────────────────────────────────────────────────

#[test]
fn compute_tile_sse_basic() {
    let globe = GlobeSurface::new();
    let sse = globe.compute_tile_sse(100.0, 10000.0, 1080.0, 1.0);
    // SSE = (100 * 1080) / (10000 * 1.0) = 10.8
    assert!((sse - 10.8).abs() < 1e-6);
}

#[test]
fn compute_tile_sse_zero_distance() {
    let globe = GlobeSurface::new();
    let sse = globe.compute_tile_sse(100.0, 0.0, 1080.0, 1.0);
    assert_eq!(sse, f64::MAX);
}

#[test]
fn should_refine_tile_above_threshold() {
    let globe = GlobeSurface::new();
    // Default maximum_screen_space_error = 2.0
    assert!(globe.should_refine_tile(5.0));
    assert!(!globe.should_refine_tile(1.0));
    assert!(!globe.should_refine_tile(2.0)); // not strictly greater
}

// ─── GlobeTranslucency ─────────────────────────────────────────────────────

#[test]
fn translucency_disabled_returns_one() {
    let t = GlobeTranslucency::new(false);
    assert!((t.front_alpha() - 1.0).abs() < 1e-10);
    assert!((t.back_alpha() - 1.0).abs() < 1e-10);
}

#[test]
fn translucency_enabled_uses_configured_alpha() {
    let mut t = GlobeTranslucency::new(true);
    t.front_face_alpha = 0.7;
    t.back_face_alpha = 0.3;
    assert!((t.front_alpha() - 0.7).abs() < 1e-10);
    assert!((t.back_alpha() - 0.3).abs() < 1e-10);
}

#[test]
fn translucency_default() {
    let t = GlobeTranslucency::default();
    assert!(!t.enabled);
    assert!((t.front_face_alpha - 1.0).abs() < 1e-10);
    assert!((t.back_face_alpha - 1.0).abs() < 1e-10);
}

// ─── ShadowMode / GlobeConfig ──────────────────────────────────────────────

#[test]
fn shadow_mode_default() {
    assert_eq!(ShadowMode::default(), ShadowMode::ReceiveOnly);
}

#[test]
fn globe_config_defaults() {
    let config = GlobeConfig::default();
    assert!(config.show);
    assert!(!config.depth_test_against_terrain);
    assert!(!config.translucency_enabled);
    assert!(config.maximum_screen_space_error > 0.0);
    assert!(config.tile_cache_size > 0);
}

// ─── GlobeSurface Normal ───────────────────────────────────────────────────

#[test]
fn surface_normal_at_equator() {
    let globe = GlobeSurface::new();
    let r = Ellipsoid::WGS84.maximum_radius();
    let position = DVec3::new(r, 0.0, 0.0);
    let normal = globe.get_surface_normal(position);
    // At equator on X axis, normal should point in +X
    assert!((normal.x - 1.0).abs() < 0.01);
    assert!(normal.y.abs() < 0.01);
    assert!(normal.z.abs() < 0.01);
}

#[test]
fn surface_normal_at_pole() {
    let globe = GlobeSurface::new();
    let r = Ellipsoid::WGS84.minimum_radius();
    let position = DVec3::new(0.0, 0.0, r);
    let normal = globe.get_surface_normal(position);
    // At north pole, normal should point in +Z
    assert!(normal.x.abs() < 0.01);
    assert!(normal.y.abs() < 0.01);
    assert!((normal.z - 1.0).abs() < 0.01);
}
