//! Bevy system assembly for the pipeline binding layer.
//!
//! Systems here are **frame-thread safe**: they perform only bounded, pure-CPU
//! ECS work (build a live-key set, delegate one eviction pass to the core).
//! No network, no blocking, no tokio — downloads stay on the core's ureq worker
//! pool, off the frame thread.

use std::collections::HashSet;

use bevy::prelude::*;

use super::bindings::{PipelineEvictionStats, PipelineTile};
use super::gpu_handle::BevyGpuHandleCache;
use super::TileKey;

/// Per-frame GPU handle cache eviction.
///
/// Builds the live-key set from `PipelineTile` entities — the direct analogue of
/// `dynamic_globe.rs:1487` (`mgr.tile_entities.contains_key(&old)`) — then
/// delegates the entire eviction pass to the core cache. Evicted Bevy `Handle`s
/// are dropped inside the core (releasing GPU asset strong-refs, L1493-1498).
///
/// The three invariants (base-layer exemption, live-entity deferral, FIFO
/// termination) are enforced by the core `EvictionPolicy`, not here.
pub fn gpu_cache_eviction_system(
    mut cache: ResMut<BevyGpuHandleCache>,
    tiles: Query<&PipelineTile>,
    mut stats: ResMut<PipelineEvictionStats>,
) {
    let live: HashSet<TileKey> = tiles.iter().map(|t| t.key).collect();
    let result = cache.evict(|key| live.contains(key));
    stats.evicted = result.evicted;
    stats.deferred = result.deferred;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::gpu_handle::GpuTileHandles;

    fn handles(id: u128) -> GpuTileHandles {
        GpuTileHandles {
            texture: Handle::weak_from_u128(id),
            mesh: None,
            material: None,
        }
    }

    /// End-to-end: the system defers a live tile, evicts a dead one, and records
    /// the counts into `PipelineEvictionStats`.
    #[test]
    fn eviction_system_defers_live_and_evicts_dead() {
        let mut app = App::new();
        app.init_resource::<PipelineEvictionStats>();

        let mut cache = BevyGpuHandleCache::new(2, 3);
        cache.insert((1, 1, 5), handles(1)); // oldest — will be LIVE
        cache.insert((2, 2, 5), handles(2)); // dead
        cache.insert((3, 3, 5), handles(3)); // dead, newest
        app.insert_resource(cache);

        // One live entity covering the oldest key.
        app.world_mut().spawn(PipelineTile::new((1, 1, 5)));
        app.add_systems(Update, gpu_cache_eviction_system);

        app.update();

        let stats = *app.world().resource::<PipelineEvictionStats>();
        assert_eq!(stats.deferred, 1, "live tile must be deferred");
        assert_eq!(stats.evicted, 1, "one dead tile must be evicted");

        let cache = app.world().resource::<BevyGpuHandleCache>();
        assert!(cache.contains_key(&(1, 1, 5)), "live survives");
        assert!(!cache.contains_key(&(2, 2, 5)), "oldest dead evicted");
        assert!(cache.contains_key(&(3, 3, 5)), "newest survives");
    }

    /// With no live entities and a cache under capacity, the system is a no-op.
    #[test]
    fn eviction_system_noop_under_capacity() {
        let mut app = App::new();
        app.init_resource::<PipelineEvictionStats>();

        let mut cache = BevyGpuHandleCache::new(10, 3);
        cache.insert((1, 1, 5), handles(1));
        cache.insert((2, 2, 5), handles(2));
        app.insert_resource(cache);
        app.add_systems(Update, gpu_cache_eviction_system);

        app.update();

        let stats = *app.world().resource::<PipelineEvictionStats>();
        assert_eq!(stats.evicted, 0);
        assert_eq!(stats.deferred, 0);
        assert_eq!(app.world().resource::<BevyGpuHandleCache>().len(), 2);
    }
}
