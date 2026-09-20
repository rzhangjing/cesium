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

        // `MaterialPlugin::build` needs *two* asset-backend resources, and
        // guarding only the first is what turns an asset-enabled-but-render-less
        // app into a panic:
        //   1. `init_asset::<M>()` dereferences `AssetServer` — absent under
        //      `MinimalPlugins` (`shader_registry::asset_backend_available`); and
        //   2. it internally adds `PrepassPipelinePlugin<M>`, whose `build` calls
        //      `load_internal_asset!` and so dereferences `Assets<Shader>`
        //      unconditionally (bevy_pbr-0.15.3 `prepass/mod.rs` L70). That
        //      storage is inserted by the *render* stack, not by `AssetPlugin`, so
        //      `AssetPlugin` alone is not enough — exactly the resource
        //      `shader_registry::shader_assets_available` exists to guard on.
        // On the normal GPU path both hold (`DefaultPlugins` runs first), so the
        // material registers as before. The systems below take
        // `Option<ResMut<Assets<SkyDomeMaterial>>>` and no-op when either is not.
        let asset_backend = crate::shader_registry::asset_backend_available(app);
        let shader_assets = crate::shader_registry::shader_assets_available(app);
        if asset_backend && shader_assets {
            app.add_plugins(MaterialPlugin::<SkyDomeMaterial>::default());
        } else {
            // The guard above is correct — without it `MaterialPlugin::build`
            // panics inside `PrepassPipelinePlugin`'s unconditional
            // `load_internal_asset!` (DEV-021) — but *skipping silently* is a dead
            // end for anyone who mounts this plugin before `DefaultPlugins` (specs
            // integration tests, third-party embedding): the dome simply never
            // appears and nothing says why. The alternative to this log is a
            // crash, not a working sky, so say which of the two storages is
            // missing instead of leaving it to guesswork.
            warn!(
                "[M5-C] SkyDomeMaterial 未注册：缺 AssetServer 或 Assets<Shader>（需 DefaultPlugins 先行）；sky dome 不可见 \
                 (asset_backend_available={asset_backend}, shader_assets_available={shader_assets})",
            );
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
