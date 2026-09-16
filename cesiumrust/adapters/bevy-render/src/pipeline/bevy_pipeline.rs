//! `CesiumPipelinePlugin` — opt-in assembly of the M1 pipeline binding layer.

use bevy::prelude::*;

use super::bindings::PipelineEvictionStats;
use super::gpu_handle::BevyGpuHandleCache;
use super::system_wiring::gpu_cache_eviction_system;

/// Opt-in plugin that wires the `cesium-pipeline` core into a Bevy app.
///
/// Registers:
/// - [`BevyGpuHandleCache`] resource (golden-path defaults: capacity 3000,
///   base-layer zoom 3),
/// - [`PipelineEvictionStats`] observation resource,
/// - the [`gpu_cache_eviction_system`] in `Update`.
///
/// # Rollout guard
/// This plugin is **not** added to the default cesium-app runtime in M1.3;
/// `CESIUM_ENABLE_PIPELINE` stays OFF. Adding it is a no-op on the golden path
/// because nothing inserts into the cache until the M1.4/M1.5 loader migration
/// feeds it — the eviction system simply finds an empty cache each frame.
#[derive(Default, Debug, Clone, Copy)]
pub struct CesiumPipelinePlugin;

impl Plugin for CesiumPipelinePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BevyGpuHandleCache>()
            .init_resource::<PipelineEvictionStats>()
            .add_systems(Update, gpu_cache_eviction_system);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The plugin must install its resources + system and run headless without
    /// panicking (no render/asset plugins required).
    #[test]
    fn plugin_installs_resources_and_runs() {
        let mut app = App::new();
        app.add_plugins(CesiumPipelinePlugin);
        app.update();

        assert!(app.world().contains_resource::<BevyGpuHandleCache>());
        assert!(app.world().contains_resource::<PipelineEvictionStats>());
        // Empty cache → eviction is a no-op.
        assert!(app.world().resource::<BevyGpuHandleCache>().is_empty());
        let stats = app.world().resource::<PipelineEvictionStats>();
        assert_eq!(stats.evicted, 0);
        assert_eq!(stats.deferred, 0);
    }
}
