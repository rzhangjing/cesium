use bevy::prelude::*;
use cesium_atmosphere::scattering::{
    AtmosphereParameters, compute_sky_color, compute_horizon_glow,
};
use glam::DVec3;

use crate::atmosphere::celestial_system::LightingParams;
use crate::entity::time_system::AnimationClock;

#[derive(Resource, Debug, Clone)]
pub struct SkyAtmosphere {
    pub enabled: bool,
    pub atmosphere_params: AtmosphereParameters,
}

impl Default for SkyAtmosphere {
    fn default() -> Self {
        Self {
            enabled: true,
            atmosphere_params: AtmosphereParameters::default(),
        }
    }
}

pub fn sky_system(
    clock: Option<Res<AnimationClock>>,
    lighting: Res<LightingParams>,
    sky: Res<SkyAtmosphere>,
    mut clear_color: ResMut<ClearColor>,
    camera_query: Query<&Transform, With<Camera3d>>,
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
