//! Default budget policy — verbatim constants from `dynamic_globe.rs` (L48-73).
//!
//! These values are the **golden-path reference** and must not drift.
//! Any future tuning should be done via a custom `BudgetPolicy` impl,
//! not by modifying these defaults.

use cesium_ports_driven::BudgetPolicy;

/// Default budget matching `dynamic_globe.rs` constants exactly.
///
/// | Constant | Value | Source line |
/// |----------|-------|-------------|
/// | `DOWNLOAD_THREADS` | 16 | L48 |
/// | `MAX_MESH_UPLOADS_PER_FRAME` | 12 | L53 |
/// | `MAX_SPAWNS_PER_FRAME` | 16 | L54 |
/// | `MAX_TEXTURE_UPLOADS_PER_FRAME` | 16 | L55 |
/// | `MAX_DESPAWNS_PER_FRAME` | 24 | L58 |
/// | `MAX_TILE_ENTITIES` | 1800 | L65 |
/// | `BASE_LAYER_ZOOM` | 3 | L70 |
/// | `MAX_GPU_CACHE_ENTRIES` | 3000 | L73 |
#[derive(Debug, Clone, Copy)]
pub struct DefaultBudget;

impl DefaultBudget {
    /// `dynamic_globe.rs:48` — parallel download/mesh-build worker threads.
    pub const DOWNLOAD_THREADS: usize = 16;
    /// `dynamic_globe.rs:53` — mesh GPU uploads per frame.
    pub const MAX_MESH_UPLOADS_PER_FRAME: usize = 12;
    /// `dynamic_globe.rs:54` — entity spawns per frame.
    pub const MAX_SPAWNS_PER_FRAME: usize = 16;
    /// `dynamic_globe.rs:55` — texture GPU uploads per frame.
    pub const MAX_TEXTURE_UPLOADS_PER_FRAME: usize = 16;
    /// `dynamic_globe.rs:58` — entity despawns per frame.
    pub const MAX_DESPAWNS_PER_FRAME: usize = 24;
    /// `dynamic_globe.rs:65` — hard cap on live tile entities.
    pub const MAX_TILE_ENTITIES: usize = 1800;
    /// `dynamic_globe.rs:70` — coarsest permanently-resident zoom level.
    pub const BASE_LAYER_ZOOM: u32 = 3;
    /// `dynamic_globe.rs:73` — GPU handle cache upper bound.
    pub const MAX_GPU_CACHE_ENTRIES: usize = 3000;
}

impl Default for DefaultBudget {
    fn default() -> Self {
        Self
    }
}

impl BudgetPolicy for DefaultBudget {
    #[inline]
    fn download_threads(&self) -> usize {
        Self::DOWNLOAD_THREADS
    }

    #[inline]
    fn max_mesh_uploads_per_frame(&self) -> usize {
        Self::MAX_MESH_UPLOADS_PER_FRAME
    }

    #[inline]
    fn max_spawns_per_frame(&self) -> usize {
        Self::MAX_SPAWNS_PER_FRAME
    }

    #[inline]
    fn max_texture_uploads_per_frame(&self) -> usize {
        Self::MAX_TEXTURE_UPLOADS_PER_FRAME
    }

    #[inline]
    fn max_despawns_per_frame(&self) -> usize {
        Self::MAX_DESPAWNS_PER_FRAME
    }

    #[inline]
    fn max_tile_entities(&self) -> usize {
        Self::MAX_TILE_ENTITIES
    }

    #[inline]
    fn base_layer_zoom(&self) -> u32 {
        Self::BASE_LAYER_ZOOM
    }

    #[inline]
    fn max_gpu_cache_entries(&self) -> usize {
        Self::MAX_GPU_CACHE_ENTRIES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_budget_matches_dynamic_globe_constants() {
        let b = DefaultBudget;
        assert_eq!(b.download_threads(), 16);
        assert_eq!(b.max_mesh_uploads_per_frame(), 12);
        assert_eq!(b.max_spawns_per_frame(), 16);
        assert_eq!(b.max_texture_uploads_per_frame(), 16);
        assert_eq!(b.max_despawns_per_frame(), 24);
        assert_eq!(b.max_tile_entities(), 1800);
        assert_eq!(b.base_layer_zoom(), 3);
        assert_eq!(b.max_gpu_cache_entries(), 3000);
    }

    #[test]
    fn termination_invariant_holds() {
        // dynamic_globe.rs L1471-1472: MAX_TILE_ENTITIES << MAX_GPU_CACHE_ENTRIES
        // guarantees evictable (dead) entries always exist.
        let b = DefaultBudget;
        assert!(
            b.max_tile_entities() < b.max_gpu_cache_entries(),
            "Termination invariant violated: MAX_TILE_ENTITIES({}) must be < MAX_GPU_CACHE_ENTRIES({})",
            b.max_tile_entities(),
            b.max_gpu_cache_entries()
        );
    }
}
