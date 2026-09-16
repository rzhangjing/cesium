pub mod celestial_system;
pub mod sky_dome;
pub mod sky_system;

pub use celestial_system::celestial_system;
pub use celestial_system::LightingParams;
pub use sky_dome::{
    sky_dome_gate_enabled, SkyAtmosphereParams, SkyDome, SkyDomeMaterial,
    SKY_ATMOSPHERE_SHADER_HANDLE,
};
pub use sky_system::{sky_dome_setup, sky_system, SkyAtmosphere};

use bevy::prelude::*;

pub struct CesiumAtmospherePlugin;

impl Plugin for CesiumAtmospherePlugin {
    fn build(&self, app: &mut App) {
        // Headless-safe embedded WGSL registration. A bare `load_internal_asset!`
        // dereferences `Assets<Shader>` and panics under `MinimalPlugins` — the
        // hazard tracked in docs/deviations.md#dev-005 and closed by
        // `shader_registry` (M5-A). `sky_atmosphere.wgsl` sits next to this file
        // (not under `shaders/`) because it is a private implementation detail of
        // the atmosphere adapter, not a reusable material library.
        crate::shader_registry::try_load_internal_shader(
            app,
            SKY_ATMOSPHERE_SHADER_HANDLE,
            include_str!("sky_atmosphere.wgsl"),
            std::path::Path::new(file!())
                .parent()
                .unwrap()
                .join("sky_atmosphere.wgsl")
                .to_string_lossy(),
        );

        app.init_resource::<SkyAtmosphere>()
            .init_resource::<LightingParams>()
            // `sky_system` takes `ResMut<ClearColor>`, so the plugin must be able
            // to stand alone (same defect class as the terrain/imagery coupling
            // fixed in #47). `init_resource` is a no-op when the resource already
            // exists, and `ClearColor::default()` is `Color::BLACK` — exactly what
            // main.rs L469 inserts — so this cannot perturb the app's own value.
            .init_resource::<ClearColor>();

        // `MaterialPlugin::build` calls `init_asset::<M>()`, which dereferences
        // `AssetServer` — absent under `MinimalPlugins`. Guarded per
        // `shader_registry`'s contract; the systems below take
        // `Option<ResMut<Assets<SkyDomeMaterial>>>` and no-op when it is missing.
        if crate::shader_registry::asset_backend_available(app) {
            app.add_plugins(MaterialPlugin::<SkyDomeMaterial>::default());
        }

        // Seed the runtime dome flag from this adapter's own read of
        // `CESIUM_ENABLE_SKYDOME`. The application-layer
        // `feature_flags::skydome_enabled()` already decided whether this plugin
        // is registered at all (main.rs L513-516); re-reading it here keeps the
        // resource self-consistent when the plugin is mounted directly (tests,
        // third-party embedding) without the env var being set.
        let dome_enabled = sky_dome_gate_enabled();
        app.world_mut().resource_mut::<SkyAtmosphere>().dome = dome_enabled;

        // Chained so the sun direction published by `celestial_system` is picked
        // up by the dome material in the very same frame, and so `sky_system`
        // sees a spawned dome rather than racing it.
        app.add_systems(Update, (celestial_system, sky_dome_setup, sky_system).chain());
    }
}
