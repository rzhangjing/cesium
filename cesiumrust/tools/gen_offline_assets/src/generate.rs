//! Deterministic offline asset generation (imagery pyramid + heightmap-1.0 terrain).
//!
//! Every routine here is a pure function of the tile coordinates `(level, x, y)`
//! — there is **no RNG, no wall-clock, and no network**. Two runs produce
//! byte-identical output, so the generator is trivially reproducible.
//!
//! The on-disk layouts mirror the viewer-demo blueprint
//! (`cesium-rs/examples/viewer-demo/src/main.rs`) and are the exact inputs the
//! M3.1 offline fetchers (`FileTileFetcher` / `FileTerrainFetcher`) read back:
//!
//! * **Imagery** — XYZ pyramid `{root}/{level}/{x}/{y}.png`, geographic y
//!   order (row 0 at the north pole), 256×256 RGBA procedural tiles.
//! * **Terrain** — heightmap-1.0 tileset: a `layer.json` descriptor plus
//!   `{root}/{level}/{x}/{disk_y}.terrain` tiles with **TMS** y order on disk
//!   (`disk_y = (1 << level) - 1 - y_geo`). Each tile is a `65×65` grid of
//!   `u16`-LE encoded heights + 1-byte childTileMask + 1-byte waterMask
//!   (`65*65*2 + 2 = 8452` bytes).

use std::f64::consts::{FRAC_PI_2, PI};
use std::fs;
use std::io;
use std::path::Path;

/// Default highest level for the offline imagery pyramid (blueprint
/// `OFFLINE_IMAGERY_MAXIMUM_LEVEL`). Levels `0..=3` form a geographic pyramid
/// of `2 + 8 + 32 + 128 = 170` tiles.
pub const IMAGERY_DEFAULT_MAX_LEVEL: u32 = 3;
/// Default highest level for the offline terrain tileset (blueprint
/// `OFFLINE_TERRAIN_MAXIMUM_LEVEL`). Levels `0..=4` form `682` tiles.
pub const TERRAIN_DEFAULT_MAX_LEVEL: u32 = 4;
/// Heightmap grid width (heightmap-1.0 default; blueprint `TERRAIN_GRID_SIZE`).
pub const TERRAIN_GRID_SIZE: usize = 65;
/// Byte size of one heightmap-1.0 tile (`65*65` u16 + childTileMask + waterMask).
pub const HEIGHTMAP_TILE_BYTES: usize = TERRAIN_GRID_SIZE * TERRAIN_GRID_SIZE * 2 + 2;
/// Edge length of a generated imagery tile in pixels.
const IMAGERY_TILE_SIZE: u32 = 256;

/// Aggregated outcome of one generation pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenStats {
    /// Number of tiles written (or already present when skipped).
    pub tiles: usize,
    /// Total payload bytes across all tiles (excludes directory overhead).
    pub bytes: u64,
    /// `true` when generation was skipped because the output already existed.
    pub skipped: bool,
}

/// Returns the number of tiles in a geographic pyramid over `0..=max_level`
/// where each level has `(2 << level)` columns and `(1 << level)` rows.
fn pyramid_tile_count(max_level: u32) -> usize {
    let mut total = 0usize;
    for level in 0..=max_level {
        let columns = 2usize << level;
        let rows = 1usize << level;
        total += columns * rows;
    }
    total
}

/// Generates the offline imagery XYZ pyramid under `root` when missing.
///
/// Idempotent: returns early (with `skipped = true`) when `root/0` already
/// exists, unless `force` is set. Tiles are 256×256 RGBA PNGs laid out as
/// `{root}/{level}/{x}/{y}.png` with geographic y order — exactly the paths
/// `FileTileFetcher::new(root, FileTileScheme::Xyz)` resolves.
pub fn ensure_imagery(root: &Path, max_level: u32, force: bool) -> io::Result<GenStats> {
    let expected = pyramid_tile_count(max_level);
    if !force && root.join("0").is_dir() {
        return Ok(GenStats {
            tiles: expected,
            bytes: 0,
            skipped: true,
        });
    }

    let mut bytes: u64 = 0;
    for level in 0..=max_level {
        let columns = 2u32 << level;
        let rows = 1u32 << level;
        for x in 0..columns {
            for y in 0..rows {
                let image = generate_tile(x, y, columns, rows);
                let path = root.join(format!("{level}/{x}/{y}.png"));
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                image.save(&path).map_err(|e| {
                    io::Error::other(format!("PNG encode {level}/{x}/{y}: {e}"))
                })?;
                bytes += fs::metadata(&path)?.len();
            }
        }
    }
    Ok(GenStats {
        tiles: expected,
        bytes,
        skipped: false,
    })
}

/// Generates the offline heightmap-1.0 terrain tileset under `root` when missing.
///
/// Idempotent: returns early (with `skipped = true`) when `root/layer.json`
/// already exists, unless `force` is set. Writes a `layer.json` descriptor plus
/// `{root}/{level}/{x}/{disk_y}.terrain` tiles with TMS y order on disk — the
/// exact layout `FileTerrainFetcher::new(root, TerrainScheme::Tms, max_level)`
/// (and `from_layer_url`) decode.
pub fn ensure_terrain(root: &Path, max_level: u32, force: bool) -> io::Result<GenStats> {
    let expected = pyramid_tile_count(max_level);
    if !force && root.join("layer.json").is_file() {
        return Ok(GenStats {
            tiles: expected,
            bytes: 0,
            skipped: true,
        });
    }

    fs::create_dir_all(root)?;
    fs::write(root.join("layer.json"), layer_json(max_level))?;

    let mut bytes: u64 = 0;
    for level in 0..=max_level {
        let columns = 2u32 << level;
        let rows = 1u32 << level;
        for x in 0..columns {
            for y_geo in 0..rows {
                let payload = terrain_tile_payload(level, max_level);
                // TMS y order on disk: geographic row 0 (north) is stored last.
                let disk_y = rows - y_geo - 1;
                let path = root.join(format!("{level}/{x}/{disk_y}.terrain"));
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&path, &payload)?;
                bytes += payload.len() as u64;
            }
        }
    }
    Ok(GenStats {
        tiles: expected,
        bytes,
        skipped: false,
    })
}

/// Builds one heightmap-1.0 tile payload: `65×65` u16-LE heights (west→east
/// ramp so the decoded mesh is visibly non-flat), then a childTileMask byte and
/// a water-mask byte.
fn terrain_tile_payload(level: u32, max_level: u32) -> Vec<u8> {
    let mut buffer: Vec<u8> = Vec::with_capacity(HEIGHTMAP_TILE_BYTES);
    for _row in 0..TERRAIN_GRID_SIZE {
        for col in 0..TERRAIN_GRID_SIZE {
            let u = col as f64 / (TERRAIN_GRID_SIZE - 1) as f64;
            let height = 100.0 * f64::from(level) + 300.0 * u;
            buffer.extend_from_slice(&encode_terrain_height(height).to_le_bytes());
        }
    }
    // childTileMask: all four children exist below the leaf level, none at it.
    let child_mask: u8 = if level < max_level { 0x0F } else { 0x00 };
    buffer.push(child_mask);
    // One-byte water mask (all land).
    buffer.push(0);
    buffer
}

/// Encodes a metric height into the heightmap-1.0 u16 domain. Inverse of the
/// fetcher's decode (`height_m = encoded / 5 - 1000`), matching the blueprint's
/// `encode_terrain_height`.
fn encode_terrain_height(height_meters: f64) -> u16 {
    ((height_meters + 1000.0) * 5.0).round() as u16
}

/// Renders the `layer.json` descriptor consumed by `FileTerrainFetcher`
/// (`read_maxzoom` scans the `"maxzoom"` integer; `"scheme": "tms"` documents
/// the on-disk y order).
fn layer_json(maxzoom: u32) -> String {
    format!(
        "{{\n  \"tilejson\": \"2.1.0\",\n  \"format\": \"heightmap-1.0\",\n  \
         \"version\": \"1.0.0\",\n  \"scheme\": \"tms\",\n  \
         \"projection\": \"EPSG:4326\",\n  \"maxzoom\": {maxzoom},\n  \
         \"tiles\": [\"{{z}}/{{x}}/{{y}}.terrain\"]\n}}\n",
        maxzoom = maxzoom
    )
}

/// Renders one 256×256 imagery tile for the geographic scheme (y = 0 at the
/// north pole), a pure function of `(x, y, columns, rows)`.
///
/// The pattern is deliberately asymmetric — a red north-polar cap, a blue
/// south-polar cap, and a green/white checker with a longitude gradient in the
/// mid latitudes — so UV flips, seams, and stretching are obvious when the tile
/// is sampled back through the imagery pipeline.
fn generate_tile(x: u32, y: u32, columns: u32, rows: u32) -> image::RgbaImage {
    let size = IMAGERY_TILE_SIZE;
    let west = -PI + f64::from(x) * (2.0 * PI) / f64::from(columns);
    let north = FRAC_PI_2 - f64::from(y) * PI / f64::from(rows);
    let lon_step = (2.0 * PI) / f64::from(columns) / f64::from(size);
    let lat_step = -PI / f64::from(rows) / f64::from(size);

    let mut image = image::RgbaImage::new(size, size);
    for py in 0..size {
        let latitude = north + (f64::from(py) + 0.5) * lat_step;
        for px in 0..size {
            let longitude = west + (f64::from(px) + 0.5) * lon_step;
            let color = if latitude > PI / 4.0 {
                // North polar cap: red (UV-flip marker — must appear on top).
                [255, 24, 24, 255]
            } else if latitude < -PI / 4.0 {
                // South polar cap: blue (must appear at the bottom).
                [24, 64, 255, 255]
            } else {
                // Mid latitudes: green/white checker + longitude gradient.
                let checker = ((px / 32) + (py / 32)) % 2 == 0;
                let gradient = ((longitude + PI) / (2.0 * PI) * 128.0) as u8;
                if checker {
                    [32 + gradient / 4, 200, 64, 255]
                } else {
                    [232, 232, 224, 255]
                }
            };
            image.put_pixel(px, py, image::Rgba(color));
        }
    }
    image
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_terrain_height_matches_fetcher_decode() {
        // encode then decode with the fetcher's inverse formula.
        for h in [0.0, 100.0, 300.0, 700.0, -1000.0] {
            let encoded = encode_terrain_height(h);
            let decoded = f64::from(encoded) / 5.0 - 1000.0;
            assert!((decoded - h).abs() < 0.21, "{h} -> {decoded}");
        }
    }

    #[test]
    fn terrain_tile_payload_has_exact_byte_size() {
        let payload = terrain_tile_payload(2, 4);
        assert_eq!(payload.len(), HEIGHTMAP_TILE_BYTES);
        // childTileMask is present below the leaf level.
        assert_eq!(payload[HEIGHTMAP_TILE_BYTES - 2], 0x0F);
        assert_eq!(payload[HEIGHTMAP_TILE_BYTES - 1], 0);
        let leaf = terrain_tile_payload(4, 4);
        assert_eq!(leaf[HEIGHTMAP_TILE_BYTES - 2], 0x00);
    }

    #[test]
    fn pyramid_tile_count_matches_blueprint() {
        // imagery levels 0..=3 = 2 + 8 + 32 + 128 = 170.
        assert_eq!(pyramid_tile_count(3), 170);
        // terrain levels 0..=4 = 170 + 512 = 682.
        assert_eq!(pyramid_tile_count(4), 682);
    }

    #[test]
    fn layer_json_contains_maxzoom() {
        let json = layer_json(4);
        assert!(json.contains("\"maxzoom\": 4"));
        assert!(json.contains("\"scheme\": \"tms\""));
        assert!(json.contains("\"format\": \"heightmap-1.0\""));
    }

    #[test]
    fn generate_tile_is_deterministic() {
        let a = generate_tile(1, 0, 2, 1);
        let b = generate_tile(1, 0, 2, 1);
        assert_eq!(a.as_raw(), b.as_raw());
    }
}
