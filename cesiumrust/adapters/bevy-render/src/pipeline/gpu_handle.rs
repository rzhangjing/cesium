//! Bevy `Handle`-typed GPU cache — a thin wrapper over the core
//! `cesium_pipeline::GpuCache`.
//!
//! All eviction logic (the three invariants of `dynamic_globe.rs::evict_gpu_cache`,
//! L1476-1502) lives in the core crate. This wrapper only:
//! 1. Fixes the value type to Bevy asset `Handle`s.
//! 2. Releases GPU assets on eviction — dropping a Bevy `Handle` decrements the
//!    asset's strong-ref count, which is exactly the "remove handles" step at
//!    L1493-1498 (`gpu_textures.remove(&old)` etc.).
//! 3. Bridges the live-entity predicate to a Bevy `Query` (see `system_wiring`).
//!
//! The three invariants (delegated, not reimplemented):
//! - **BASE_LAYER exemption** (L1483): `z <= BASE_LAYER_ZOOM` never evicted.
//! - **Live-entity deferral** (L1487-1491): live tiles pushed back (花屏防护核心).
//! - **Termination** (L1471-1472): `MAX_TILE_ENTITIES(1800) << MAX_GPU_CACHE_ENTRIES(3000)`.

use bevy::prelude::*;
use cesium_pipeline::base_layer::BaseLayerGuard;
use cesium_pipeline::gpu_cache::{EvictionResult, GpuCache};
use cesium_pipeline::DefaultBudget;
use cesium_ports_driven::EvictionPolicy;

use super::TileKey;

/// GPU asset handles for a single tile (Bevy strong references).
///
/// Mirrors the per-tile handle maps in `dynamic_globe.rs` (`gpu_textures`,
/// `gpu_meshes`, `gpu_materials`, L1493-1495). Dropping these releases the
/// underlying GPU assets.
pub struct GpuTileHandles {
    /// Imagery/albedo texture handle (`gpu_textures`, L1493).
    pub texture: Handle<Image>,
    /// Terrain/tile mesh handle (`gpu_meshes`, L1495).
    pub mesh: Option<Handle<Mesh>>,
    /// PBR material handle (`gpu_materials`, L1494).
    pub material: Option<Handle<StandardMaterial>>,
}

/// Bevy `Resource` wrapping the core FIFO GPU handle cache.
///
/// The heavy lifting (FIFO order, base-layer lock, live deferral, termination)
/// is performed by `cesium_pipeline::GpuCache`, which also implements the M1.1
/// `EvictionPolicy<TileKey>` contract. See [`BevyGpuHandleCache::as_eviction_policy`].
#[derive(Resource)]
pub struct BevyGpuHandleCache {
    inner: GpuCache<TileKey, GpuTileHandles>,
}

impl BevyGpuHandleCache {
    /// Create a cache with an explicit capacity and base-layer zoom.
    ///
    /// `zoom_of` for `TileKey = (x, y, zoom)` is `|k| k.2`, matching the
    /// `old.2 <= BASE_LAYER_ZOOM` check at `dynamic_globe.rs:1483`.
    pub fn new(max_entries: usize, base_layer_zoom: u32) -> Self {
        Self {
            inner: GpuCache::new(
                max_entries,
                BaseLayerGuard::with_zoom(base_layer_zoom),
                |k: &TileKey| k.2,
            ),
        }
    }

    /// Create a cache using the golden-path defaults
    /// (`MAX_GPU_CACHE_ENTRIES = 3000`, `BASE_LAYER_ZOOM = 3`).
    pub fn with_defaults() -> Self {
        Self::new(
            DefaultBudget::MAX_GPU_CACHE_ENTRIES,
            DefaultBudget::BASE_LAYER_ZOOM,
        )
    }

    /// Insert (or update) the GPU handles for a tile. Re-inserting an existing
    /// key does not change its FIFO position (matches dynamic_globe: re-uploads
    /// do not reset eviction priority).
    pub fn insert(&mut self, key: TileKey, handles: GpuTileHandles) {
        self.inner.insert(key, handles);
    }

    /// Borrow the cached handles for a tile.
    pub fn get(&self, key: &TileKey) -> Option<&GpuTileHandles> {
        self.inner.get(key)
    }

    /// Whether a tile currently has cached GPU handles.
    pub fn contains_key(&self, key: &TileKey) -> bool {
        self.inner.contains_key(key)
    }

    /// Explicitly remove a tile's handles (returns them so the caller may
    /// keep or drop them). Rare vs FIFO eviction.
    pub fn remove(&mut self, key: &TileKey) -> Option<GpuTileHandles> {
        self.inner.remove(key)
    }

    /// Number of cached tiles.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Mark a tile as backed (or not) by a live entity, feeding invariant 2.
    /// Delegates to the core cache's live-set.
    pub fn set_live(&mut self, key: TileKey, live: bool) {
        self.inner.set_live(key, live);
    }

    /// Run one FIFO eviction pass, releasing GPU handles for evicted tiles.
    ///
    /// `is_live` corresponds to `dynamic_globe.rs:1487`
    /// (`mgr.tile_entities.contains_key(&old)`). Evicted handles are dropped
    /// inside the core cache, releasing their Bevy asset strong-refs — this is
    /// the binding-layer equivalent of L1493-1498 (`gpu_*.remove(&old)`).
    ///
    /// Returns `(evicted, deferred)` counts for observation, exactly like the
    /// M0.4-instrumented `evict_gpu_cache` return value.
    pub fn evict<F>(&mut self, is_live: F) -> EvictionResult
    where
        F: Fn(&TileKey) -> bool,
    {
        self.inner.evict(is_live)
    }

    /// View this cache as the core M1.1 `EvictionPolicy<TileKey>` trait object.
    ///
    /// Demonstrates that the binding layer does not reimplement eviction: the
    /// three invariants are queried straight from the core contract.
    pub fn as_eviction_policy(&self) -> &dyn EvictionPolicy<TileKey> {
        &self.inner
    }
}

impl Default for BevyGpuHandleCache {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_ports_driven::BudgetPolicy;

    /// Build a placeholder handle set (payload identity is irrelevant — eviction
    /// is driven purely by the tile key).
    fn handles(id: u128) -> GpuTileHandles {
        GpuTileHandles {
            texture: Handle::weak_from_u128(id),
            mesh: None,
            material: None,
        }
    }

    /// Invariant 3 (FIFO order) + eviction: the oldest *dead* entry goes first.
    /// Mirrors `dynamic_globe.rs:1479-1500`.
    #[test]
    fn fifo_eviction_drops_oldest_dead() {
        let mut cache = BevyGpuHandleCache::new(3, 3);
        cache.insert((0, 0, 5), handles(1));
        cache.insert((1, 0, 5), handles(2));
        cache.insert((2, 0, 5), handles(3));
        cache.insert((3, 0, 5), handles(4)); // exceeds cap → evict 1

        let r = cache.evict(|_| false);
        assert_eq!(r.evicted, 1);
        assert_eq!(r.deferred, 0);
        assert!(!cache.contains_key(&(0, 0, 5)), "oldest must be evicted");
        assert!(cache.contains_key(&(3, 0, 5)), "newest must be kept");
    }

    /// Invariant 1 (BASE_LAYER permanent exemption): `z <= 3` never evicted.
    /// Mirrors `dynamic_globe.rs:1483` (`if old.2 <= BASE_LAYER_ZOOM { continue }`).
    #[test]
    fn base_layer_permanently_exempt() {
        let mut cache = BevyGpuHandleCache::new(2, 3);
        cache.insert((0, 0, 3), handles(1)); // base layer (z=3)
        cache.insert((1, 1, 5), handles(2)); // normal
        cache.insert((2, 2, 5), handles(3)); // normal
        cache.insert((3, 3, 5), handles(4)); // normal

        let r = cache.evict(|_| false);
        assert_eq!(r.evicted, 1);
        assert!(cache.contains_key(&(0, 0, 3)), "base layer must survive");
        assert!(!cache.contains_key(&(1, 1, 5)), "oldest normal evicted");
    }

    /// Invariant 2 (live-entity push-back deferral — 花屏防护核心).
    /// Mirrors `dynamic_globe.rs:1487-1491`.
    #[test]
    fn live_entity_deferred_and_pushed_back() {
        let mut cache = BevyGpuHandleCache::new(2, 3);
        cache.insert((1, 1, 5), handles(1)); // oldest, but LIVE
        cache.insert((2, 2, 5), handles(2));
        cache.insert((3, 3, 5), handles(3)); // exceeds cap

        // Mark the oldest as live: it must be deferred, and the next dead entry
        // evicted instead.
        let r = cache.evict(|k| *k == (1, 1, 5));
        assert_eq!(r.deferred, 1);
        assert!(cache.contains_key(&(1, 1, 5)), "live tile must be deferred");
        assert!(!cache.contains_key(&(2, 2, 5)), "dead tile evicted instead");
    }

    /// Invariant 3 (termination): because live entities are capped far below
    /// the cache capacity, eviction always terminates and frees dead entries.
    /// Mirrors `dynamic_globe.rs:1471-1472`.
    #[test]
    fn termination_guaranteed_live_below_capacity() {
        // Global invariant: MAX_TILE_ENTITIES(1800) << MAX_GPU_CACHE_ENTRIES(3000).
        // Read via the BudgetPolicy contract (runtime values, not consts).
        let budget = DefaultBudget;
        assert!(budget.max_tile_entities() < budget.max_gpu_cache_entries());

        let mut cache = BevyGpuHandleCache::new(5, 3);
        let live_a = (6, 0, 5);
        let live_b = (7, 0, 5);
        for i in 0..6u32 {
            cache.insert((i, 0, 5), handles(u128::from(i))); // 6 dead at front
        }
        cache.insert(live_a, handles(100)); // 2 live at back
        cache.insert(live_b, handles(101));

        // 8 entries, cap 5 → evict exactly the 3 oldest dead; the 2 live tiles
        // sit at the tail and are never reached. Terminates deterministically.
        let r = cache.evict(|k| *k == live_a || *k == live_b);
        assert_eq!(r.evicted, 3);
        assert_eq!(r.deferred, 0);
        assert_eq!(cache.len(), 5);
        assert!(cache.contains_key(&live_a) && cache.contains_key(&live_b));
    }

    /// The wrapper delegates the invariants to the core `EvictionPolicy`
    /// contract rather than reimplementing them.
    #[test]
    fn delegates_to_core_eviction_policy() {
        let mut cache = BevyGpuHandleCache::with_defaults();
        cache.insert((0, 0, 3), handles(1)); // base layer
        cache.set_live((0, 0, 3), true);

        let policy = cache.as_eviction_policy();
        // Invariant: FIFO order exposed from core.
        assert_eq!(policy.evict_order().len(), 1);
        // Invariant 1: base layer (z<=3) never evicted; z>3 evictable.
        assert!(policy.never_evict(&(0, 0, 3)));
        assert!(!policy.never_evict(&(0, 0, 5)));
        // Invariant 2: live-entity deferral flag sourced from core live-set.
        assert!(policy.defer_if_live(&(0, 0, 3)));
    }

    /// `with_defaults` must use the golden-path capacity + base zoom.
    #[test]
    fn defaults_match_golden_path_constants() {
        let cache = BevyGpuHandleCache::with_defaults();
        let policy = cache.as_eviction_policy();
        // BASE_LAYER_ZOOM = 3 → z=3 exempt, z=4 not.
        assert!(policy.never_evict(&(0, 0, 3)));
        assert!(!policy.never_evict(&(0, 0, 4)));
        assert!(cache.is_empty());
    }
}
