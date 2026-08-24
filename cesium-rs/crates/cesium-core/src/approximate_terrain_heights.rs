//! Ported from `packages/engine/Source/Core/ApproximateTerrainHeights.js` (256 lines).
//!
//! A collection of functions for approximating terrain height.
//!
//! ## Method-level alignment table
//!
//! | JS | Rust | Notes |
//! |---|---|---|
//! | `initialize` | [`initialize`] | DEVIATION: reads `approximateTerrainHeights.json` from the CesiumJS source tree instead of `Resource.fetchJson(buildModuleUrl(...))` |
//! | `getMinimumMaximumHeights` | [`get_minimum_maximum_heights`] | |
//! | `getBoundingSphere` | [`get_bounding_sphere`] | |
//! | `getTileXYLevel` (private) | [`get_tile_xy_level`] | |
//! | `initialized` | [`initialized`] | |
//! | `_terrainHeightsMaxLevel` / `_defaultMaxTerrainHeight` / `_defaultMinTerrainHeight` | [`TERRAIN_HEIGHTS_MAX_LEVEL`] / [`DEFAULT_MAX_TERRAIN_HEIGHT`] / [`DEFAULT_MIN_TERRAIN_HEIGHT`] | |
//!
//! DEVIATION: the JS `_terrainHeights` / `_initPromise` module globals are
//! mirrored with a `Mutex<Option<HashMap>>` slot; `initialize` reads the
//! JSON asset directly from disk (no DOM / `CESIUM_BASE_URL` available).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::check;
use crate::developer_error::throw_developer_error;
use crate::ellipsoid::Ellipsoid;
use crate::geographic_tiling_scheme::GeographicTilingScheme;
use crate::rectangle::Rectangle;
use crate::tiling_scheme::TilingScheme;

/// Mirrors `ApproximateTerrainHeights._terrainHeightsMaxLevel`.
pub const TERRAIN_HEIGHTS_MAX_LEVEL: i32 = 6;
/// Mirrors `ApproximateTerrainHeights._defaultMaxTerrainHeight`.
pub const DEFAULT_MAX_TERRAIN_HEIGHT: f64 = 9000.0;
/// Mirrors `ApproximateTerrainHeights._defaultMinTerrainHeight`.
pub const DEFAULT_MIN_TERRAIN_HEIGHT: f64 = -100000.0;

/// Minimum and maximum terrain heights for a rectangle.
///
/// Mirrors the `{ minimumTerrainHeight, maximumTerrainHeight }` result of
/// `getMinimumMaximumHeights`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinimumMaximumTerrainHeights {
    pub minimum_terrain_height: f64,
    pub maximum_terrain_height: f64,
}

fn terrain_heights_slot() -> &'static Mutex<Option<HashMap<String, [f64; 2]>>> {
    static SLOT: Mutex<Option<HashMap<String, [f64; 2]>>> = Mutex::new(None);
    &SLOT
}

fn lock_slot() -> std::sync::MutexGuard<'static, Option<HashMap<String, [f64; 2]>>> {
    terrain_heights_slot()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Locates the `approximateTerrainHeights.json` asset inside the CesiumJS
/// source tree next to this workspace.
fn asset_path() -> Option<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    // crates/cesium-core -> crates -> cesium-rs workspace root -> cesium repo root
    let candidate = manifest_dir
        .join("..")
        .join("..")
        .join("..")
        .join("packages")
        .join("engine")
        .join("Source")
        .join("Assets")
        .join("approximateTerrainHeights.json");
    let normalized = dunce_canonicalize(&candidate);
    if normalized.is_file() {
        return Some(normalized);
    }
    None
}

fn dunce_canonicalize(path: &Path) -> PathBuf {
    // Avoid a dependency on `dunce`; canonicalize is only used for existence
    // checks, so fall back to the raw path on failure.
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Initializes the minimum and maximum terrain heights.
///
/// Mirrors `ApproximateTerrainHeights.initialize`. Subsequent calls while
/// initialized are no-ops (JS returns the cached `_initPromise`).
///
/// DEVIATION: loads the JSON asset from disk instead of
/// `Resource.fetchJson(buildModuleUrl("Assets/approximateTerrainHeights.json"))`.
pub fn initialize() -> Result<(), String> {
    if initialized() {
        return Ok(());
    }

    let path = asset_path()
        .ok_or_else(|| "approximateTerrainHeights.json asset not found".to_string())?;
    let contents = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let json: HashMap<String, [f64; 2]> =
        serde_json::from_str(&contents).map_err(|e| e.to_string())?;

    *lock_slot() = Some(json);
    Ok(())
}

/// Installs a terrain-heights table directly (test hook mirroring direct
/// `_terrainHeights` assignment in the JS spec's `afterAll`).
pub fn set_terrain_heights(heights: Option<HashMap<String, [f64; 2]>>) {
    *lock_slot() = heights;
}

/// Exposes the `_terrainHeights` slot guard for spec mirrors that need to
/// save/clear/restore the global state (mirrors direct `_terrainHeights`
/// access in `ApproximateTerrainHeightsSpec.js`).
pub fn terrain_heights_slot_for_test() -> std::sync::MutexGuard<'static, Option<HashMap<String, [f64; 2]>>> {
    lock_slot()
}

/// Determines if the terrain heights are initialized and ready to use.
///
/// Mirrors `ApproximateTerrainHeights.initialized`.
pub fn initialized() -> bool {
    lock_slot().is_some()
}

/// Computes the minimum and maximum terrain heights for a given rectangle.
///
/// Mirrors `ApproximateTerrainHeights.getMinimumMaximumHeights`.
pub fn get_minimum_maximum_heights(
    rectangle: Option<&Rectangle>,
    ellipsoid: Option<&Ellipsoid>,
) -> MinimumMaximumTerrainHeights {
    //>>includeStart('debug', pragmas.debug);
    if cfg!(debug_assertions) {
        check::defined("rectangle", rectangle);
        if !initialized() {
            throw_developer_error(
                "You must call ApproximateTerrainHeights.initialize and wait for the promise to resolve before using this function",
            );
        }
    }
    //>>includeEnd('debug');
    let rectangle = rectangle.unwrap();
    let ellipsoid_owned;
    let ellipsoid = match ellipsoid {
        Some(e) => e,
        None => {
            ellipsoid_owned = Ellipsoid::WGS84;
            &ellipsoid_owned
        }
    };

    let xy_level = get_tile_xy_level(rectangle);

    // Get the terrain min/max for that tile
    let mut min_terrain_height = DEFAULT_MIN_TERRAIN_HEIGHT;
    let mut max_terrain_height = DEFAULT_MAX_TERRAIN_HEIGHT;
    if let Some(xy_level) = xy_level {
        let key = format!("{}-{}-{}", xy_level.level, xy_level.x, xy_level.y);
        let heights = lock_slot().as_ref().and_then(|map| map.get(&key).copied());
        if let Some(heights) = heights {
            min_terrain_height = heights[0];
            max_terrain_height = heights[1];
        }

        // Compute min by taking the center of the NE->SW diagonal and finding
        // distance to the surface.
        let northeast = Rectangle::northeast(rectangle);
        let southwest = Rectangle::southwest(rectangle);
        let mut diagonal_ne = Cartesian3::default();
        let mut diagonal_sw = Cartesian3::default();
        ellipsoid.cartographic_to_cartesian(&northeast, &mut diagonal_ne);
        ellipsoid.cartographic_to_cartesian(&southwest, &mut diagonal_sw);

        let mut center_cartesian = Cartesian3::default();
        Cartesian3::midpoint(&diagonal_sw, &diagonal_ne, &mut center_cartesian);

        let mut surface_cartesian = Cartesian3::default();
        let surface_position = ellipsoid.scale_to_geodetic_surface(
            &center_cartesian,
            &mut surface_cartesian,
        );
        if surface_position {
            let distance = Cartesian3::distance(&center_cartesian, &surface_cartesian);
            min_terrain_height = min_terrain_height.min(-distance);
        } else {
            min_terrain_height = DEFAULT_MIN_TERRAIN_HEIGHT;
        }
    }

    min_terrain_height = DEFAULT_MIN_TERRAIN_HEIGHT.max(min_terrain_height);

    MinimumMaximumTerrainHeights {
        minimum_terrain_height: min_terrain_height,
        maximum_terrain_height: max_terrain_height,
    }
}

/// Computes the bounding sphere based on the tile heights in the rectangle.
///
/// Mirrors `ApproximateTerrainHeights.getBoundingSphere`.
pub fn get_bounding_sphere(
    rectangle: Option<&Rectangle>,
    ellipsoid: Option<&Ellipsoid>,
) -> BoundingSphere {
    //>>includeStart('debug', pragmas.debug);
    if cfg!(debug_assertions) {
        check::defined("rectangle", rectangle);
        if !initialized() {
            throw_developer_error(
                "You must call ApproximateTerrainHeights.initialize and wait for the promise to resolve before using this function",
            );
        }
    }
    //>>includeEnd('debug');
    let rectangle = rectangle.unwrap();
    let ellipsoid_owned;
    let ellipsoid = match ellipsoid {
        Some(e) => e,
        None => {
            ellipsoid_owned = Ellipsoid::WGS84;
            &ellipsoid_owned
        }
    };

    let xy_level = get_tile_xy_level(rectangle);

    // Get the terrain max for that tile
    let mut max_terrain_height = DEFAULT_MAX_TERRAIN_HEIGHT;
    if let Some(xy_level) = xy_level {
        let key = format!("{}-{}-{}", xy_level.level, xy_level.x, xy_level.y);
        let heights = lock_slot().as_ref().and_then(|map| map.get(&key).copied());
        if let Some(heights) = heights {
            max_terrain_height = heights[1];
        }
    }

    let result = BoundingSphere::from_rectangle_3d(Some(rectangle), Some(ellipsoid), 0.0, None);
    let upper = BoundingSphere::from_rectangle_3d(
        Some(rectangle),
        Some(ellipsoid),
        max_terrain_height,
        None,
    );

    BoundingSphere::union(&result, &upper, None)
}

/// Tile x/y/level of the deepest tile (up to [`TERRAIN_HEIGHTS_MAX_LEVEL`])
/// that fully contains the rectangle.
///
/// Mirrors the private `getTileXYLevel` function.
struct TileXYLevel {
    x: i32,
    y: i32,
    level: i32,
}

fn get_tile_xy_level(rectangle: &Rectangle) -> Option<TileXYLevel> {
    let corners = [
        Cartographic::from_radians_new(rectangle.east, rectangle.north, Some(0.0)),
        Cartographic::from_radians_new(rectangle.west, rectangle.north, Some(0.0)),
        Cartographic::from_radians_new(rectangle.east, rectangle.south, Some(0.0)),
        Cartographic::from_radians_new(rectangle.west, rectangle.south, Some(0.0)),
    ];

    // Determine which tile the bounding rectangle is in
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let mut last_level_x = 0;
    let mut last_level_y = 0;
    let mut current_x = 0;
    let mut current_y = 0;
    let max_level = TERRAIN_HEIGHTS_MAX_LEVEL;
    let mut tile_xy = Cartesian2::default();

    let mut i = 0;
    while i <= max_level {
        let mut failed = false;
        for (j, corner) in corners.iter().enumerate() {
            if tiling_scheme.position_to_tile_xy(corner, i, &mut tile_xy).is_none() {
                failed = true;
                break;
            }
            if j == 0 {
                current_x = tile_xy.x as i32;
                current_y = tile_xy.y as i32;
            } else if current_x != tile_xy.x as i32 || current_y != tile_xy.y as i32 {
                failed = true;
                break;
            }
        }

        if failed {
            break;
        }

        last_level_x = current_x;
        last_level_y = current_y;
        i += 1;
    }

    if i == 0 {
        return None;
    }

    Some(TileXYLevel {
        x: last_level_x,
        y: last_level_y,
        level: if i > max_level { max_level } else { i - 1 },
    })
}
