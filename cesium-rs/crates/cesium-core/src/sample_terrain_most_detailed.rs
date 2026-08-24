//! Ported from `packages/engine/Source/Core/sampleTerrainMostDetailed.js` (133 lines).
//!
//! ## Function-level alignment table
//!
//! | JS | Rust | Notes |
//! |---|---|---|
//! | `sampleTerrainMostDetailed` | [`sample_terrain_most_detailed`] | sync; JS is async (DEVIATION 1) |
//!
//! # DEVIATIONS
//! 1. The JS function is `async`; this Rust port is synchronous because
//!    `sample_terrain` is synchronous and the
//!    `terrainProvider.loadTileDataAvailability` async call is not yet
//!    ported (returns `None` / no-op).
//! 2. JS `availability` is a property on the terrain provider. The Rust
//!    port takes a `&TileAvailability` reference separately.
//! 3. The `maxLevel === 0` path calls `loadTileDataAvailability` (JS: may
//!    return a promise to load additional availability data). The Rust
//!    port skips this step (DEVIATION: no async IO).
//! 4. The recursive re-check after sampling (`changedPositions`) is
//!    preserved but will not find changes in the sync port (no new
//!    availability data was loaded).
//! 5. Same provider-type restriction as [`sample_terrain`] — see that
//!    module's DEVIATION 3.

use crate::cartesian2::Cartesian2;
use crate::cartographic::Cartographic;
use crate::ellipsoid_terrain_provider::EllipsoidTerrainProvider;
use crate::sample_terrain::sample_terrain;
use crate::tile_availability::TileAvailability;
use crate::tiling_scheme::TilingScheme;

/// Samples terrain at the most detailed available level for each position.
///
/// Mirrors `sampleTerrainMostDetailed`. The JS function is async; this
/// Rust port is synchronous (DEVIATION 1).
///
/// # Arguments
/// * `provider` — the terrain provider (currently [`EllipsoidTerrainProvider`])
/// * `availability` — tile availability information; required
/// * `positions` — the positions to update with terrain heights
/// * `reject_on_tile_fail` — if `true`, returns `Err(...)` on tile failure
pub fn sample_terrain_most_detailed(
    provider: &EllipsoidTerrainProvider,
    availability: &TileAvailability,
    positions: &mut [Cartographic],
    reject_on_tile_fail: bool,
) -> Result<(), String> {
    let tiling_scheme = provider.tiling_scheme();

    // Group positions by their maximum available level (mirrors JS `byLevel`).
    let mut by_level: Vec<Option<Vec<usize>>> = Vec::new();
    let mut max_levels: Vec<i32> = Vec::with_capacity(positions.len());

    for (i, position) in positions.iter().enumerate() {
        let max_level = availability.compute_maximum_level_at_position(position);
        max_levels.push(max_level);

        if max_level == 0 {
            // This is a special case where we have a parent terrain and we
            // are requesting heights from an area that isn't covered by the
            // top level terrain at all.
            // DEVIATION 3: the JS code calls loadTileDataAvailability here.
            let mut scratch = Cartesian2::default();
            tiling_scheme.position_to_tile_xy(position, 1, &mut scratch);
            // In JS: promise = terrainProvider.loadTileDataAvailability(...)
            // In Rust sync port: skip (no async IO available).
        }

        let level_idx = max_level.max(0) as usize;
        if level_idx >= by_level.len() {
            by_level.resize_with(level_idx + 1, || None);
        }
        by_level[level_idx]
            .get_or_insert_with(Vec::new)
            .push(i);
    }

    // Sample terrain at each level (mirrors JS `Promise.all(byLevel.map(...))`).
    for (level_idx, positions_at_level) in by_level.iter().enumerate() {
        if let Some(indices) = positions_at_level {
            // Gather positions for this level into a contiguous vec.
            let mut level_positions: Vec<Cartographic> =
                indices.iter().map(|&i| positions[i]).collect();

            sample_terrain(provider, level_idx as i32, &mut level_positions, reject_on_tile_fail)?;

            // Write back the updated heights.
            for (j, &i) in indices.iter().enumerate() {
                positions[i].height = level_positions[j].height;
            }
        }
    }

    // Check if any positions have a changed max level after sampling
    // (mirrors JS `changedPositions` re-check).
    let mut changed_positions_indices: Vec<usize> = Vec::new();
    for (i, position) in positions.iter().enumerate() {
        let new_max_level = availability.compute_maximum_level_at_position(position);
        if new_max_level != max_levels[i] {
            changed_positions_indices.push(i);
        }
    }

    if !changed_positions_indices.is_empty() {
        // Recursively sample changed positions at their new max levels.
        let mut changed: Vec<Cartographic> = changed_positions_indices
            .iter()
            .map(|&i| positions[i])
            .collect();

        sample_terrain_most_detailed(provider, availability, &mut changed, reject_on_tile_fail)?;

        // Write back updated heights.
        for (j, &i) in changed_positions_indices.iter().enumerate() {
            positions[i].height = changed[j].height;
        }
    }

    Ok(())
}

/// Variant that accepts an optional availability, mirroring the JS
/// `DeveloperError` when `availability` is `undefined`.
pub fn sample_terrain_most_detailed_checked(
    provider: &EllipsoidTerrainProvider,
    availability: Option<&TileAvailability>,
    positions: &mut [Cartographic],
    reject_on_tile_fail: bool,
) -> Result<(), String> {
    let availability = availability.ok_or_else(|| {
        "sampleTerrainMostDetailed requires a terrain provider that has tile availability."
            .to_string()
    })?;
    sample_terrain_most_detailed(provider, availability, positions, reject_on_tile_fail)
}
