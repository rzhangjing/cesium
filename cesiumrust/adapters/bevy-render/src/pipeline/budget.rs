//! Per-loader mesh/texture upload budget weights (M1.4).
//!
//! Sourced from the core `BudgetPolicy` (`cesium_pipeline::DefaultBudget`), whose
//! `MAX_MESH_UPLOADS_PER_FRAME = 12` is the golden-path per-frame GPU upload cap
//! (`dynamic_globe.rs:53`). M1.4 splits that magnitude across the three
//! mesh/texture-producing P0 loaders by relative cost, so a gate-ON frame never
//! uploads more than its weight — bounding frame-thread GPU work exactly like the
//! golden path does.
//!
//! | Loader  | Weight | Rationale                                             |
//! |---------|--------|-------------------------------------------------------|
//! | terrain | 6      | quantized-mesh + skirts (heaviest per-tile mesh)       |
//! | tileset | 4      | b3dm/glTF meshes                                       |
//! | imagery | 12     | texture uploads (keeps the full mesh-budget magnitude) |
//!
//! The tileset *json* loader produces no mesh, so it carries no budget.
//!
//! # Gate-OFF neutrality
//! The legacy (gate OFF) path passes [`UNBOUNDED`] so every resolved tile is
//! processed in-frame — byte-identical to the pre-migration drain, which had no
//! budget. The weights only take effect on the gate-ON pipeline path.

use cesium_pipeline::DefaultBudget;
use cesium_ports_driven::BudgetPolicy;

/// Sentinel budget for the legacy (gate OFF) path: process all resolved tiles.
pub const UNBOUNDED: usize = usize::MAX;

/// Terrain mesh-upload weight (`dynamic_globe` semantics: heavy quantized-mesh).
pub const TERRAIN_MESH_WEIGHT: usize = 6;
/// Tileset content mesh-upload weight (b3dm/glTF).
pub const TILESET_MESH_WEIGHT: usize = 4;
/// Imagery texture-upload weight, aligned to `MAX_MESH_UPLOADS_PER_FRAME`.
pub const IMAGERY_TEXTURE_WEIGHT: usize = 12;

/// Terrain per-frame mesh budget, capped by the core `BudgetPolicy` so a future
/// lowering of `MAX_MESH_UPLOADS_PER_FRAME` automatically tightens this weight.
pub fn terrain_mesh_budget() -> usize {
    TERRAIN_MESH_WEIGHT.min(DefaultBudget.max_mesh_uploads_per_frame())
}

/// Tileset per-frame mesh budget, capped by the core `BudgetPolicy`.
pub fn tileset_mesh_budget() -> usize {
    TILESET_MESH_WEIGHT.min(DefaultBudget.max_mesh_uploads_per_frame())
}

/// Imagery per-frame texture budget, capped by the core `BudgetPolicy`.
pub fn imagery_texture_budget() -> usize {
    IMAGERY_TEXTURE_WEIGHT.min(DefaultBudget.max_mesh_uploads_per_frame())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_match_spec() {
        assert_eq!(TERRAIN_MESH_WEIGHT, 6);
        assert_eq!(TILESET_MESH_WEIGHT, 4);
        assert_eq!(IMAGERY_TEXTURE_WEIGHT, 12);
    }

    #[test]
    fn budgets_bounded_by_core_policy() {
        // Every weight must be clamped to the golden-path per-frame upload cap.
        let cap = DefaultBudget.max_mesh_uploads_per_frame();
        assert_eq!(cap, 12);
        assert_eq!(terrain_mesh_budget(), 6);
        assert_eq!(tileset_mesh_budget(), 4);
        assert_eq!(imagery_texture_budget(), 12);
        assert!(terrain_mesh_budget() <= cap);
        assert!(tileset_mesh_budget() <= cap);
        assert!(imagery_texture_budget() <= cap);
    }

    #[test]
    fn unbounded_is_max_for_legacy_path() {
        // Gate OFF must drain everything in-frame (no budget), preserving the
        // pre-migration behaviour exactly.
        assert_eq!(UNBOUNDED, usize::MAX);
        assert!(UNBOUNDED > imagery_texture_budget());
    }
}
