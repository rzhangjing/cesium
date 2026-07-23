//! cesium-atmosphere: Atmospheric and celestial domain models
//!
//! Maps to CesiumJS:
//! - `Scene/SkyAtmosphere.js`
//! - `Scene/SkyBox.js`
//! - `Scene/Sun.js`
//! - `Scene/Moon.js`
//! - `Core/Simon1994PlanetaryPositions.js`
//!
//! # Features
//! - Sun/Moon position computation (simplified VSOP87/lunar theory)
//! - ECI ↔ ECEF coordinate transformation
//! - Rayleigh/Mie atmospheric scattering model
//! - Sky color computation
//! - Lighting configuration

pub mod celestial;
pub mod scattering;

pub use celestial::{
    compute_sun_position_eci, compute_sun_position_ecef,
    compute_sun_direction_eci,
    compute_moon_position_eci, compute_moon_position_ecef,
    compute_moon_direction_eci,
    compute_gmst, eci_to_ecef,
    AU_IN_METERS, J2000_EPOCH,
};
pub use scattering::{
    AtmosphereParameters, SkyBoxConfig, LightingConfig,
    rayleigh_phase, mie_phase, atmospheric_density,
    compute_sky_color, compute_horizon_glow,
};
