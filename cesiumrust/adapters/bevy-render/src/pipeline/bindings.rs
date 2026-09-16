//! Bevy Component/Resource bridges for the pipeline binding layer.
//!
//! These are the minimal ECS glue types that let the core pipeline cache
//! interoperate with Bevy entities. They carry no logic.

use bevy::prelude::*;

use super::TileKey;

/// Marker component tagging an entity as a pipeline-managed tile.
///
/// Carries the tile key so systems can map entity ↔ GPU-cache entry and build
/// the live-entity set used by the eviction deferral invariant
/// (`dynamic_globe.rs:1487`, `mgr.tile_entities.contains_key(&old)`).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PipelineTile {
    /// The tile key `(x, y, zoom)` this entity renders.
    pub key: TileKey,
}

impl PipelineTile {
    /// Create a marker for the given tile key.
    pub fn new(key: TileKey) -> Self {
        Self { key }
    }
}

/// Per-frame eviction counters, written by the eviction system.
///
/// Pure observation (M0.4-style): mirrors the `(evicted, deferred)` return of
/// the instrumented `evict_gpu_cache` (`dynamic_globe.rs:1476`). Feeds the
/// `evict_n` / `evict_deferred` columns of the M1.1 `PipelineStats` CSV mapping.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipelineEvictionStats {
    /// GPU handle entries evicted this frame.
    pub evicted: u32,
    /// Entries deferred this frame (live-entity push-back, 花屏防护).
    pub deferred: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_tile_carries_key() {
        let t = PipelineTile::new((4, 5, 6));
        assert_eq!(t.key, (4, 5, 6));
        assert_eq!(t.key.2, 6); // zoom component drives base-layer check
    }

    #[test]
    fn eviction_stats_default_zeroed() {
        let s = PipelineEvictionStats::default();
        assert_eq!(s.evicted, 0);
        assert_eq!(s.deferred, 0);
    }
}
