use bevy::prelude::*;
use cesium_atmosphere::scattering::{
    AtmosphereParameters, compute_sky_color, compute_horizon_glow,
};
use glam::DVec3;

use crate::atmosphere::celestial_system::LightingParams;
use crate::atmosphere::sky_dome::{
    despawn_sky_dome, spawn_sky_dome, update_sun_direction, SkyAtmosphereParams, SkyDome,
    SkyDomeMaterial,
};
use crate::entity::time_system::AnimationClock;

#[derive(Resource, Debug, Clone)]
pub struct SkyAtmosphere {
    pub enabled: bool,
    /// When `true`, the sky is rendered by the GPU single-scattering dome
    /// (`sky_atmosphere.wgsl`) instead of the CPU `ClearColor` approximation.
    ///
    /// Seeded from `CESIUM_ENABLE_SKYDOME` by [`super::CesiumAtmospherePlugin`];
    /// defaults to `true` because that plugin is itself only registered when
    /// `feature_flags::skydome_enabled()` is true (main.rs L513-516), so the
    /// gate is already closed by the time this resource exists. Setting it to
    /// `false` at runtime falls back to the pre-M5-C `ClearColor` path and
    /// tears any live dome down — that is the escape hatch the unit tests use.
    pub dome: bool,
    pub atmosphere_params: AtmosphereParameters,
}

impl Default for SkyAtmosphere {
    fn default() -> Self {
        Self {
            enabled: true,
            dome: true,
            atmosphere_params: AtmosphereParameters::default(),
        }
    }
}

/// Idempotent sky-dome spawn / tear-down, driven by [`SkyAtmosphere::dome`].
///
/// Runs in `Update` (chained before [`sky_system`]) rather than `Startup` so
/// that toggling `dome` at runtime is honoured, and so that a headless
/// `MinimalPlugins` app — where `MaterialPlugin::<SkyDomeMaterial>` was skipped
/// by `shader_registry::asset_backend_available` and therefore
/// `Assets<SkyDomeMaterial>` does not exist — degrades to a no-op instead of
/// panicking on the missing resource. Both asset storages are taken as
/// `Option<ResMut<_>>` for exactly that reason.
pub fn sky_dome_setup(
    mut commands: Commands,
    sky: Res<SkyAtmosphere>,
    existing: Query<Entity, With<SkyDome>>,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<SkyDomeMaterial>>>,
) {
    if !sky.enabled || !sky.dome {
        // Gate OFF branch: the CPU ClearColor path below is the only sky
        // contributor, so no dome may remain alive to double-paint it.
        if !existing.is_empty() {
            despawn_sky_dome(&mut commands, existing.iter());
        }
        return;
    }
    if !existing.is_empty() {
        return;
    }
    let (Some(meshes), Some(materials)) = (meshes.as_deref_mut(), materials.as_deref_mut()) else {
        return;
    };
    spawn_sky_dome(
        &mut commands,
        meshes,
        materials,
        SkyAtmosphereParams::from_domain(&sky.atmosphere_params),
    );
}

pub fn sky_system(
    clock: Option<Res<AnimationClock>>,
    lighting: Res<LightingParams>,
    sky: Res<SkyAtmosphere>,
    mut clear_color: ResMut<ClearColor>,
    camera_query: Query<&Transform, With<Camera3d>>,
    dome_query: Query<&MeshMaterial3d<SkyDomeMaterial>, With<SkyDome>>,
    mut materials: Option<ResMut<Assets<SkyDomeMaterial>>>,
) {
    let clock = match clock {
        Some(c) => c,
        None => return,
    };
    if !sky.enabled {
        return;
    }

    let jd = clock.current_time();
    let julian_date = jd.total_days();

    let sun_dir = DVec3::new(
        lighting.sun_direction.x as f64,
        lighting.sun_direction.y as f64,
        lighting.sun_direction.z as f64,
    );

    let sun_elevation = sun_dir.z;

    let view_dir = if let Ok(cam_transform) = camera_query.get_single() {
        DVec3::new(
            cam_transform.forward().x as f64,
            cam_transform.forward().y as f64,
            cam_transform.forward().z as f64,
        )
    } else {
        sun_dir
    };

    // ── GPU single-scattering path (M5-C) ─────────────────────────────────
    // `sky_atmosphere.wgsl` owns the sky colour, so the CPU ClearColor below
    // must NOT also be written (that would double-paint the dome composite).
    // Only the sun-direction uniform is refreshed, and only when it actually
    // moved: under `FIXED_TIME` the clock is frozen, so this settles after the
    // first frame and the baseline capture stays bit-reproducible.
    if sky.dome {
        if let Ok(dome_material) = dome_query.get_single() {
            if let Some(materials) = materials.as_deref_mut() {
                update_sun_direction(materials, &dome_material.0, lighting.sun_direction);
            }
        }
        let _ = julian_date;
        return;
    }

    // ── gate OFF path: byte-for-byte the pre-M5-C CPU ClearColor sky ───────
    let sky_color = compute_sky_color(view_dir, sun_dir, 1000.0, &sky.atmosphere_params);
    let horizon_glow = compute_horizon_glow(sun_elevation);

    let r = (sky_color[0] as f32 * 0.3 + horizon_glow[0] as f32 * 0.3).clamp(0.0, 1.0);
    let g = (sky_color[1] as f32 * 0.3 + horizon_glow[1] as f32 * 0.3).clamp(0.0, 1.0);
    let b = (sky_color[2] as f32 * 0.3 + horizon_glow[2] as f32 * 0.3).clamp(0.0, 1.0);

    clear_color.0 = Color::srgb(r, g, b);

    let _ = julian_date;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sky_atmosphere_default() {
        let sky = SkyAtmosphere::default();
        assert!(sky.enabled);
    }

    #[test]
    fn test_sky_atmosphere_disabled() {
        let sky = SkyAtmosphere {
            enabled: false,
            ..Default::default()
        };
        assert!(!sky.enabled);
    }

    #[test]
    fn test_compute_sky_color_blue() {
        let params = AtmosphereParameters::default();
        let view = DVec3::new(0.0, 0.0, 1.0);
        let sun = DVec3::new(0.0, 1.0, 0.0);
        let color = compute_sky_color(view, sun, 0.0, &params);
        assert!(color.iter().any(|&c| c > 0.0), "Sky color should not be black");
    }

    #[test]
    fn test_horizon_glow_sunset() {
        let color = compute_horizon_glow(-0.1);
        assert!(color[0] > color[2], "Red should dominate at sunset");
    }

    #[test]
    fn test_horizon_glow_noon() {
        use std::f64::consts::PI;
        let color = compute_horizon_glow(PI / 2.0);
        assert!(color[2] > color[0], "Blue should dominate at noon");
    }
}
