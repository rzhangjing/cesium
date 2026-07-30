//! Globe atmosphere extended specs - GroundAtmosphere/GlobeLighting/SkyAtmosphere/SkyBox
//! Ported from Scene/SkyAtmosphereSpec.js + Scene/GlobeSpec.js (A-class scattering)

use cesium_globe::atmosphere::{
    GroundAtmosphere, GlobeLighting, SkyAtmosphereConfig, SkyBoxConfig,
};
use glam::DVec3;

// ─── GroundAtmosphere ───────────────────────────────────────────────────────

#[test]
fn ground_atmosphere_defaults() {
    let atm = GroundAtmosphere::default();
    assert!(atm.rayleigh_coefficients[0] > 0.0);
    assert!(atm.rayleigh_coefficients[1] > 0.0);
    assert!(atm.rayleigh_coefficients[2] > 0.0);
    // Blue channel should be strongest (blue sky)
    assert!(atm.rayleigh_coefficients[2] > atm.rayleigh_coefficients[0]);
    assert!(atm.mie_coefficient > 0.0);
    assert!(atm.mie_g > 0.0 && atm.mie_g < 1.0);
    assert!(atm.scale_height > 0.0);
    assert!(atm.sun_intensity > 0.0);
}

#[test]
fn sky_color_looking_at_sun() {
    let atm = GroundAtmosphere::default();
    let view_dir = DVec3::X; // Looking towards sun
    let sun_dir = DVec3::X;
    let color = atm.compute_sky_color(view_dir, sun_dir, 0.0);

    // Should be bright (looking at sun)
    assert!(color[0] > 0.0);
    assert!(color[1] > 0.0);
    assert!(color[2] > 0.0);
}

#[test]
fn sky_color_looking_away_from_sun() {
    let atm = GroundAtmosphere::default();
    let view_dir = -DVec3::X; // Looking away from sun
    let sun_dir = DVec3::X;
    let color = atm.compute_sky_color(view_dir, sun_dir, 0.0);

    // Should still have some color (scattered light)
    assert!(color[0] >= 0.0);
    assert!(color[1] >= 0.0);
    assert!(color[2] >= 0.0);
}

#[test]
fn sky_color_blue_channel_dominant() {
    let atm = GroundAtmosphere::default();
    let view_dir = DVec3::new(0.0, 1.0, 0.0); // Perpendicular to sun
    let sun_dir = DVec3::X;
    let color = atm.compute_sky_color(view_dir, sun_dir, 0.0);

    // Blue should be dominant due to Rayleigh scattering
    // (unless the color is all zero which shouldn't happen)
    if color[0] + color[1] + color[2] > 0.001 {
        assert!(color[2] >= color[0], "blue >= red for Rayleigh scattering");
    }
}

#[test]
fn sky_color_higher_altitude_dimmer() {
    let atm = GroundAtmosphere::default();
    let view_dir = DVec3::X;
    let sun_dir = DVec3::X;

    let color_low = atm.compute_sky_color(view_dir, sun_dir, 0.0);
    let color_high = atm.compute_sky_color(view_dir, sun_dir, 50000.0);

    // At higher altitude, less atmosphere → dimmer
    let brightness_low: f64 = color_low.iter().sum();
    let brightness_high: f64 = color_high.iter().sum();
    assert!(brightness_high < brightness_low, "higher altitude should be dimmer");
}

#[test]
fn sky_color_all_channels_clamped() {
    let atm = GroundAtmosphere::default();
    let view_dir = DVec3::X;
    let sun_dir = DVec3::X;
    let color = atm.compute_sky_color(view_dir, sun_dir, 0.0);

    for &c in &color {
        assert!(c >= 0.0 && c <= 1.0, "color channel must be in [0,1]");
    }
}

// ─── Horizon glow ───────────────────────────────────────────────────────────

#[test]
fn horizon_glow_at_sunset() {
    let atm = GroundAtmosphere::default();
    // Sun at horizon (elevation = 0)
    let glow = atm.compute_horizon_glow(0.0);

    // Should be strong orange/red glow
    assert!(glow[0] > 0.5, "red channel strong at sunset");
    assert!(glow[0] > glow[1], "red > green for sunset glow");
    assert!(glow[1] > glow[2], "green > blue for sunset glow");
}

#[test]
fn horizon_glow_high_sun() {
    let atm = GroundAtmosphere::default();
    // Sun high above horizon
    let glow = atm.compute_horizon_glow(1.0); // ~57 degrees

    // Should be very dim (exponential decay)
    assert!(glow[0] < 0.01, "glow should be dim when sun is high");
}

#[test]
fn horizon_glow_below_horizon() {
    let atm = GroundAtmosphere::default();
    // Sun below horizon
    let glow = atm.compute_horizon_glow(-0.5);

    // Should still have some glow (twilight)
    assert!(glow[0] > 0.0);
}

// ─── Zenith color ───────────────────────────────────────────────────────────

#[test]
fn zenith_color_daytime() {
    let atm = GroundAtmosphere::default();
    let color = atm.compute_zenith_color(1.0); // Sun high

    // Blue sky during day
    assert!(color[2] > color[0], "blue > red during day");
    assert!(color[2] > color[1], "blue > green during day");
}

#[test]
fn zenith_color_night() {
    let atm = GroundAtmosphere::default();
    let color = atm.compute_zenith_color(-1.0); // Sun below horizon

    // Dark at night
    assert!(color[0] < 0.01);
    assert!(color[1] < 0.01);
    assert!(color[2] < 0.01);
}

#[test]
fn zenith_color_sunset_transition() {
    let atm = GroundAtmosphere::default();
    let color_low = atm.compute_zenith_color(0.1);
    let color_high = atm.compute_zenith_color(0.5);

    // Higher sun → brighter zenith
    let b_low: f64 = color_low.iter().sum();
    let b_high: f64 = color_high.iter().sum();
    assert!(b_high > b_low);
}

// ─── SkyAtmosphereConfig ────────────────────────────────────────────────────

#[test]
fn sky_atmosphere_config_defaults() {
    let config = SkyAtmosphereConfig::default();
    assert!(config.show);
    assert!((config.hue_shift).abs() < 1e-10);
    assert!((config.saturation_shift).abs() < 1e-10);
    assert!((config.brightness_shift).abs() < 1e-10);
    // Atmosphere radius should be > Earth radius
    assert!(config.atmosphere_radius > 6378137.0);
}

// ─── SkyBoxConfig ───────────────────────────────────────────────────────────

#[test]
fn sky_box_config_defaults() {
    let config = SkyBoxConfig::default();
    assert!(config.show);
    assert!(config.sources.is_none());
    assert!(config.radius > 1e10, "star radius should be very large");
}

// ─── GlobeLighting ──────────────────────────────────────────────────────────

#[test]
fn globe_lighting_defaults() {
    let lighting = GlobeLighting::default();
    assert!(!lighting.enabled);
    assert!((lighting.sun_direction - DVec3::X).length() < 1e-10);
    assert!(lighting.sun_color[0] > 0.9);
    assert!(lighting.ambient_color[0] < 0.2);
    assert!(lighting.specular_intensity > 0.0);
}
