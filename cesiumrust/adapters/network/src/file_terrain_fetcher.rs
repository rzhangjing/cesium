//! Offline disk-backed terrain fetcher (heightmap-1.0 format).
//!
//! Implements the [`TerrainProvider`] port by reading `.terrain` tiles from a
//! local directory laid out as `{root}/{level}/{x}/{y}.terrain`, mirroring the
//! viewer-demo offline heightmap fixture (blueprint:
//! `cesium-rs/examples/viewer-demo/src/main.rs` L638-686, `ensure_offline_terrain`).
//!
//! # On-disk format: heightmap-1.0
//!
//! Each tile is a `65×65` grid of `u16`-LE encoded heights followed by one
//! childTileMask byte and one water-mask byte (`65*65*2 + 2 = 8452` bytes
//! total). The metric height is recovered as
//!
//! ```text
//! height_m = encoded / 5 - 1000
//! ```
//!
//! which is the inverse of the blueprint's `encode_terrain_height`
//! (`encoded = (height_m + 1000) * 5`).
//!
//! # Y-order convention
//!
//! [`TerrainScheme::Tms`] stores row `y = 0` at the **south** pole (the
//! heightmap-1.0 / `layer.json` `"scheme": "tms"` default), so the disk row is
//! `(1 << level) - 1 - y_geo`. [`TerrainScheme::Geographic`] stores row 0 at
//! the north (no flip). The viewer-demo fixture writes TMS on disk.
//!
//! # Geometry contract (IO layer, f64 preserved)
//!
//! The decoded [`GeometryData`] places vertices on a unit tile frame:
//! `u ∈ [-1, 1]` west→east, `v ∈ [-1, 1]` north→south (row 0 = north), and
//! `z = height_m`. Indices form a `64×64` triangle grid. All values stay
//! `f64` — the IO layer never downcasts to `f32` (that happens only at the
//! GPU boundary).
//!
//! # STRICT_OFFLINE contract
//!
//! [`FileTerrainFetcher::from_layer_url`] panics when STRICT_OFFLINE is
//! enabled and the `layer.json` URL is an `http://`/`https://` URL — the
//! offline path has no network fallback.

use cesium_geospatial::geometry::PrimitiveType;
use cesium_geospatial::{BoundingSphere, GeometryData, Rectangle};
use cesium_ports_driven::{PortError, PortResult, TerrainProvider};
use glam::DVec3;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

/// Heightmap grid width (heightmap-1.0 default; blueprint `TERRAIN_GRID_SIZE`).
const HEIGHTMAP_GRID_SIZE: usize = 65;
/// Total byte size of one heightmap-1.0 tile (`65*65` u16 + childTileMask + waterMask).
const HEIGHTMAP_TILE_BYTES: usize = HEIGHTMAP_GRID_SIZE * HEIGHTMAP_GRID_SIZE * 2 + 2;
/// heightmap-1.0 decode scale: `height_m = encoded / 5 - 1000`.
const HEIGHTMAP_SCALE: f64 = 1.0 / 5.0;
/// heightmap-1.0 decode offset (meters).
const HEIGHTMAP_OFFSET: f64 = -1000.0;

/// On-disk y-order convention for terrain tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainScheme {
    /// TMS: row `y = 0` stored at the **south** pole; the disk row is
    /// `(1 << level) - 1 - y_geo`. This is the heightmap-1.0 / `layer.json`
    /// `"scheme": "tms"` default used by the viewer-demo fixture.
    Tms,
    /// Geographic: row `y = 0` stored at the **north** pole (no flip).
    Geographic,
}

/// Offline terrain fetcher reading heightmap-1.0 tiles from a local directory.
#[derive(Debug, Clone)]
pub struct FileTerrainFetcher {
    root: PathBuf,
    scheme: TerrainScheme,
    strict_offline: bool,
    maximum_level: u32,
    rectangle: Rectangle,
}

impl FileTerrainFetcher {
    /// Creates a fetcher rooted at `root`.
    ///
    /// `maximum_level` is the deepest level served (the viewer-demo fixture
    /// uses `4`). The rectangle defaults to the full globe
    /// ([`Rectangle::MAX_VALUE`]).
    pub fn new(root: impl AsRef<Path>, scheme: TerrainScheme, maximum_level: u32) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            scheme,
            strict_offline: false,
            maximum_level,
            rectangle: Rectangle::MAX_VALUE,
        }
    }

    /// Enables STRICT_OFFLINE semantics: a network `layer.json` URL passed to
    /// [`Self::from_layer_url`] triggers an immediate panic.
    pub fn with_strict_offline(mut self, strict: bool) -> Self {
        self.strict_offline = strict;
        self
    }

    /// Overrides the covered rectangle (default: full globe).
    pub fn with_rectangle(mut self, rectangle: Rectangle) -> Self {
        self.rectangle = rectangle;
        self
    }

    /// Loads a fetcher from a `layer.json` URL.
    ///
    /// Only `file://` URLs are supported; the root directory is the parent of
    /// `layer.json`. The `maximum_level` is read from the JSON's `maxzoom`
    /// field when present, defaulting to `0`.
    ///
    /// # Panics
    ///
    /// Panics if `strict_offline` is `true` and `layer_json_url` is an
    /// `http://`/`https://` URL — the STRICT_OFFLINE contract forbids network
    /// fallback.
    pub fn from_layer_url(
        layer_json_url: &str,
        scheme: TerrainScheme,
        strict_offline: bool,
    ) -> PortResult<Self> {
        if strict_offline && is_http_url(layer_json_url) {
            panic!(
                "STRICT_OFFLINE violation: FileTerrainFetcher received network layer URL '{}' \
                 (offline mode forbids HTTP fallback; wire the offline root instead)",
                layer_json_url
            );
        }
        let path = layer_url_to_path(layer_json_url)?;
        let root = path
            .parent()
            .ok_or_else(|| {
                PortError::NotFound(format!(
                    "FileTerrainFetcher: layer.json '{}' has no parent directory",
                    path.display()
                ))
            })?
            .to_path_buf();
        let maximum_level = read_maxzoom(&path).unwrap_or(0);
        Ok(Self {
            root,
            scheme,
            strict_offline,
            maximum_level,
            rectangle: Rectangle::MAX_VALUE,
        })
    }

    /// Root directory of the terrain tileset.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Active y-order scheme.
    pub fn scheme(&self) -> TerrainScheme {
        self.scheme
    }

    /// Whether STRICT_OFFLINE semantics are active.
    pub fn strict_offline(&self) -> bool {
        self.strict_offline
    }

    /// Resolves the disk path of tile `(x, y, level)`, applying the TMS y-flip
    /// when [`TerrainScheme::Tms`] is active.
    fn tile_path(&self, x: u32, y: u32, level: u32) -> PathBuf {
        let disk_y = match self.scheme {
            TerrainScheme::Tms => (1u32 << level).saturating_sub(1).saturating_sub(y),
            TerrainScheme::Geographic => y,
        };
        self.root
            .join(level.to_string())
            .join(x.to_string())
            .join(format!("{}.terrain", disk_y))
    }

    /// Reads and decodes one heightmap-1.0 tile into a [`GeometryData`].
    fn read_tile(&self, x: u32, y: u32, level: u32) -> PortResult<GeometryData> {
        let path = self.tile_path(x, y, level);
        let bytes = std::fs::read(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                PortError::NotFound(format!(
                    "FileTerrainFetcher: terrain tile not found at '{}'",
                    path.display()
                ))
            } else {
                PortError::Network(format!(
                    "FileTerrainFetcher: IO error reading '{}': {}",
                    path.display(),
                    e
                ))
            }
        })?;
        decode_heightmap(&bytes)
    }
}

impl TerrainProvider for FileTerrainFetcher {
    fn rectangle(&self) -> Rectangle {
        self.rectangle
    }

    fn maximum_level(&self) -> u32 {
        self.maximum_level
    }

    fn request_tile_geometry<'a>(
        &'a self,
        x: u32,
        y: u32,
        level: u32,
    ) -> Pin<Box<dyn Future<Output = PortResult<GeometryData>> + Send + 'a>> {
        if level > self.maximum_level {
            return Box::pin(async move {
                Err(PortError::NotFound(format!(
                    "FileTerrainFetcher: level {} exceeds maximum_level {}",
                    level, self.maximum_level
                )))
            });
        }
        // Read synchronously; the future resolves on first poll (offline IO).
        let result = self.read_tile(x, y, level);
        Box::pin(async move { result })
    }

    fn get_availability(&self, x: u32, y: u32, level: u32) -> bool {
        if level > self.maximum_level {
            return false;
        }
        self.tile_path(x, y, level).is_file()
    }
}

/// Decodes a heightmap-1.0 payload into a [`GeometryData`] grid.
///
/// The payload must be at least [`HEIGHTMAP_TILE_BYTES`] bytes: a `65×65`
/// grid of `u16`-LE encoded heights, then one childTileMask byte and one
/// water-mask byte (the two mask bytes are ignored for geometry).
fn decode_heightmap(bytes: &[u8]) -> PortResult<GeometryData> {
    if bytes.len() < HEIGHTMAP_TILE_BYTES {
        return Err(PortError::Decode(format!(
            "heightmap-1.0 tile too small: {} bytes (expected >= {})",
            bytes.len(),
            HEIGHTMAP_TILE_BYTES
        )));
    }
    let grid = HEIGHTMAP_GRID_SIZE;
    let last = (grid - 1) as f64;
    let mut positions: Vec<[f64; 3]> = Vec::with_capacity(grid * grid);
    for row in 0..grid {
        for col in 0..grid {
            let i = (row * grid + col) * 2;
            let encoded = u16::from_le_bytes([bytes[i], bytes[i + 1]]);
            let height = encoded as f64 * HEIGHTMAP_SCALE + HEIGHTMAP_OFFSET;
            // Unit tile frame: u ∈ [-1, 1] west→east, v ∈ [-1, 1] north→south
            // (row 0 = north), z = height in meters. f64 throughout.
            let u = (col as f64 / last) * 2.0 - 1.0;
            let v = 1.0 - (row as f64 / last) * 2.0;
            positions.push([u, v, height]);
        }
    }
    // 64×64 triangle grid (two triangles per cell).
    let mut indices: Vec<u32> = Vec::with_capacity((grid - 1) * (grid - 1) * 6);
    for row in 0..(grid - 1) {
        for col in 0..(grid - 1) {
            let a = (row * grid + col) as u32;
            let b = a + 1;
            let c = a + grid as u32;
            let d = c + 1;
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    let bounding_sphere = sphere_from_positions(&positions);
    Ok(GeometryData {
        positions,
        normals: None,
        tex_coords: None,
        tangents: None,
        bitangents: None,
        indices,
        bounding_sphere,
        primitive_type: PrimitiveType::Triangles,
    })
}

/// Computes a bounding sphere enclosing all positions (AABB center + max
/// distance). Avoids a glam dependency in the hot path by working on
/// `[f64; 3]` directly.
fn sphere_from_positions(positions: &[[f64; 3]]) -> BoundingSphere {
    if positions.is_empty() {
        return BoundingSphere {
            center: DVec3::ZERO,
            radius: 0.0,
        };
    }
    let mut min = [f64::MAX; 3];
    let mut max = [f64::MIN; 3];
    for p in positions {
        for axis in 0..3 {
            if p[axis] < min[axis] {
                min[axis] = p[axis];
            }
            if p[axis] > max[axis] {
                max[axis] = p[axis];
            }
        }
    }
    let center = [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ];
    let mut radius_sq = 0.0f64;
    for p in positions {
        let dx = p[0] - center[0];
        let dy = p[1] - center[1];
        let dz = p[2] - center[2];
        let d = dx * dx + dy * dy + dz * dz;
        if d > radius_sq {
            radius_sq = d;
        }
    }
    BoundingSphere {
        center: DVec3::from_array(center),
        radius: radius_sq.sqrt(),
    }
}

/// Returns `true` when `url` starts with `http://` or `https://`
/// (case-insensitive).
fn is_http_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// Converts a `layer.json` URL to a filesystem path.
///
/// Accepts `file:///abs/path/layer.json`, bare absolute paths, and relative
/// paths. Network URLs are rejected (the offline fetcher has no HTTP backend).
fn layer_url_to_path(url: &str) -> PortResult<PathBuf> {
    let tail = if let Some(idx) = url.find("://") {
        let scheme = &url[..idx];
        if !scheme.eq_ignore_ascii_case("file") {
            return Err(PortError::NotFound(format!(
                "FileTerrainFetcher: unsupported layer URL scheme '{}' (only file:// is allowed offline)",
                scheme
            )));
        }
        let after = &url[idx + 3..];
        // file:///abs/path → /abs/path (keep leading slash); on Windows
        // file:///C:/path → C:/path (drop the leading slash so the drive
        // letter becomes the path prefix).
        match after.find('/') {
            Some(slash) => strip_windows_drive_slash(&after[slash..]),
            None => after,
        }
    } else {
        url
    };
    if tail.is_empty() {
        return Err(PortError::NotFound(
            "FileTerrainFetcher: empty layer.json path".to_string(),
        ));
    }
    Ok(PathBuf::from(tail))
}

/// On Windows, a `file:///C:/path` URL yields a tail of `/C:/path`; the
/// leading slash must be dropped so the drive letter becomes the path
/// prefix. POSIX tails (`/abs/path`) are returned unchanged.
fn strip_windows_drive_slash(tail: &str) -> &str {
    let b = tail.as_bytes();
    if b.len() >= 3 && b[0] == b'/' && b[2] == b':' && b[1].is_ascii_alphabetic() {
        &tail[1..]
    } else {
        tail
    }
}

/// Reads the `maxzoom` field from a `layer.json` file. Returns `None` when
/// the file is absent or the field is missing/unparseable.
///
/// A minimal scan (no JSON dependency): finds `"maxzoom"` and parses the
/// following integer. Sufficient for the deterministic offline fixture.
fn read_maxzoom(layer_json: &Path) -> Option<u32> {
    let text = std::fs::read_to_string(layer_json).ok()?;
    let key = "\"maxzoom\"";
    let start = text.find(key)? + key.len();
    let rest = &text[start..];
    // Skip whitespace and the ':' separator, then collect digits.
    let digits: String = rest
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse::<u32>().ok()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn unique_temp_dir(tag: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "cesium-file-terrain-fetcher-{}-{}-{}",
            tag,
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    /// Encodes a metric height into the heightmap-1.0 u16 domain
    /// (inverse of `decode_heightmap`: `encoded = (height_m + 1000) * 5`).
    fn encode_height(height_m: f64) -> u16 {
        ((height_m + 1000.0) * 5.0).round() as u16
    }

    /// Builds a heightmap-1.0 tile payload from a `65×65` grid of metric
    /// heights (row-major, row 0 = north), plus the two trailing mask bytes.
    fn build_heightmap_payload(heights: &[f64; HEIGHTMAP_GRID_SIZE * HEIGHTMAP_GRID_SIZE]) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::with_capacity(HEIGHTMAP_TILE_BYTES);
        for h in heights {
            buffer.extend_from_slice(&encode_height(*h).to_le_bytes());
        }
        buffer.push(0x0F); // childTileMask: all four children
        buffer.push(0); // water mask: all land
        buffer
    }

    /// A flat tile at `height_m` everywhere.
    fn flat_heights(height_m: f64) -> [f64; HEIGHTMAP_GRID_SIZE * HEIGHTMAP_GRID_SIZE] {
        [height_m; HEIGHTMAP_GRID_SIZE * HEIGHTMAP_GRID_SIZE]
    }

    // --- helpers -------------------------------------------------------------

    #[test]
    fn test_is_http_url() {
        assert!(is_http_url("http://example.com/layer.json"));
        assert!(is_http_url("HTTPS://example.com/layer.json"));
        assert!(!is_http_url("file:///abs/layer.json"));
        assert!(!is_http_url("/abs/layer.json"));
    }

    #[test]
    fn test_layer_url_to_path_file() {
        let p = layer_url_to_path("file:///tiles/layer.json").unwrap();
        assert_eq!(p, PathBuf::from("/tiles/layer.json"));
    }

    #[test]
    fn test_layer_url_to_path_bare() {
        let p = layer_url_to_path("/tiles/layer.json").unwrap();
        assert_eq!(p, PathBuf::from("/tiles/layer.json"));
    }

    #[test]
    fn test_layer_url_to_path_rejects_https() {
        let err = layer_url_to_path("https://example.com/layer.json").unwrap_err();
        assert!(matches!(err, PortError::NotFound(_)));
    }

    #[test]
    fn test_read_maxzoom() {
        let dir = unique_temp_dir("maxzoom");
        let path = dir.join("layer.json");
        std::fs::write(
            &path,
            "{\n  \"format\": \"heightmap-1.0\",\n  \"maxzoom\": 4,\n  \"scheme\": \"tms\"\n}\n",
        )
        .unwrap();
        assert_eq!(read_maxzoom(&path), Some(4));
    }

    #[test]
    fn test_read_maxzoom_missing_field() {
        let dir = unique_temp_dir("maxzoom-none");
        let path = dir.join("layer.json");
        std::fs::write(&path, "{\"format\": \"heightmap-1.0\"}").unwrap();
        assert_eq!(read_maxzoom(&path), None);
    }

    // --- heightmap decode ----------------------------------------------------

    #[test]
    fn test_decode_heightmap_flat() {
        let payload = build_heightmap_payload(&flat_heights(100.0));
        let geom = decode_heightmap(&payload).unwrap();
        assert_eq!(geom.positions.len(), HEIGHTMAP_GRID_SIZE * HEIGHTMAP_GRID_SIZE);
        assert_eq!(
            geom.indices.len(),
            (HEIGHTMAP_GRID_SIZE - 1) * (HEIGHTMAP_GRID_SIZE - 1) * 6
        );
        assert_eq!(geom.primitive_type, PrimitiveType::Triangles);
        // Every vertex sits at height 100 m (± u16 rounding).
        for p in &geom.positions {
            assert!((p[2] - 100.0).abs() < 0.21, "height {} off", p[2]);
        }
        // Corner UVs span the unit tile frame.
        let first = geom.positions[0];
        assert!((first[0] - (-1.0)).abs() < 1e-9);
        assert!((first[1] - 1.0).abs() < 1e-9);
        let last = geom.positions[HEIGHTMAP_GRID_SIZE * HEIGHTMAP_GRID_SIZE - 1];
        assert!((last[0] - 1.0).abs() < 1e-9);
        assert!((last[1] - (-1.0)).abs() < 1e-9);
    }

    #[test]
    fn test_decode_heightmap_ramp() {
        // West→east ramp matching the blueprint fixture (height = 300 * u).
        let mut heights = [0.0f64; HEIGHTMAP_GRID_SIZE * HEIGHTMAP_GRID_SIZE];
        for row in 0..HEIGHTMAP_GRID_SIZE {
            for col in 0..HEIGHTMAP_GRID_SIZE {
                let u = col as f64 / (HEIGHTMAP_GRID_SIZE - 1) as f64;
                heights[row * HEIGHTMAP_GRID_SIZE + col] = 300.0 * u;
            }
        }
        let payload = build_heightmap_payload(&heights);
        let geom = decode_heightmap(&payload).unwrap();
        // West edge (col 0) ≈ 0 m, east edge (col 64) ≈ 300 m.
        let west = geom.positions[0][2];
        let east = geom.positions[HEIGHTMAP_GRID_SIZE - 1][2];
        assert!((west - 0.0).abs() < 0.21, "west {}", west);
        assert!((east - 300.0).abs() < 0.21, "east {}", east);
    }

    #[test]
    fn test_decode_heightmap_too_small() {
        let err = decode_heightmap(&[0u8; 10]).unwrap_err();
        assert!(matches!(err, PortError::Decode(_)));
    }

    #[test]
    fn test_sphere_from_positions_empty() {
        let s = sphere_from_positions(&[]);
        assert_eq!(s.center, DVec3::ZERO);
        assert_eq!(s.radius, 0.0);
    }

    // --- tile_path / TMS flip ------------------------------------------------

    #[test]
    fn test_tile_path_tms_flips_y() {
        let dir = unique_temp_dir("tile-path-tms");
        let fetcher = FileTerrainFetcher::new(&dir, TerrainScheme::Tms, 4);
        // level 2 → 4 rows; geographic y=0 (north) → disk y=3.
        let path = fetcher.tile_path(1, 0, 2);
        assert_eq!(path, dir.join("2").join("1").join("3.terrain"));
        // geographic y=3 (south) → disk y=0.
        let path = fetcher.tile_path(1, 3, 2);
        assert_eq!(path, dir.join("2").join("1").join("0.terrain"));
    }

    #[test]
    fn test_tile_path_geographic_no_flip() {
        let dir = unique_temp_dir("tile-path-geo");
        let fetcher = FileTerrainFetcher::new(&dir, TerrainScheme::Geographic, 4);
        let path = fetcher.tile_path(1, 0, 2);
        assert_eq!(path, dir.join("2").join("1").join("0.terrain"));
        let path = fetcher.tile_path(1, 3, 2);
        assert_eq!(path, dir.join("2").join("1").join("3.terrain"));
    }

    // --- STRICT_OFFLINE panic ------------------------------------------------

    #[test]
    #[should_panic(expected = "STRICT_OFFLINE violation")]
    fn test_from_layer_url_strict_panics_on_https() {
        let _ = FileTerrainFetcher::from_layer_url(
            "https://example.com/terrain/layer.json",
            TerrainScheme::Tms,
            true,
        );
    }

    #[test]
    #[should_panic(expected = "STRICT_OFFLINE violation")]
    fn test_from_layer_url_strict_panics_on_http() {
        let _ = FileTerrainFetcher::from_layer_url(
            "http://example.com/terrain/layer.json",
            TerrainScheme::Tms,
            true,
        );
    }

    #[test]
    fn test_from_layer_url_strict_allows_file() {
        let dir = unique_temp_dir("from-url-ok");
        std::fs::write(
            dir.join("layer.json"),
            "{\"format\":\"heightmap-1.0\",\"maxzoom\":3,\"scheme\":\"tms\"}",
        )
        .unwrap();
        let url = format!(
            "file:///{}",
            dir.join("layer.json").display().to_string().replace('\\', "/")
        );
        let fetcher =
            FileTerrainFetcher::from_layer_url(&url, TerrainScheme::Tms, true).unwrap();
        assert_eq!(fetcher.maximum_level(), 3);
        assert!(fetcher.strict_offline());
        // The resolved root must point back at the same directory on disk
        // (separator-agnostic: works on Windows and POSIX).
        assert!(fetcher.root().join("layer.json").is_file());
    }

    #[test]
    fn test_from_layer_url_not_strict_rejects_https_with_error() {
        // Without STRICT_OFFLINE, an https URL is still rejected (no HTTP
        // backend) but as a PortError rather than a panic.
        let err = FileTerrainFetcher::from_layer_url(
            "https://example.com/terrain/layer.json",
            TerrainScheme::Tms,
            false,
        )
        .unwrap_err();
        assert!(matches!(err, PortError::NotFound(_)));
    }

    // --- end-to-end request_tile_geometry ------------------------------------

    #[tokio::test]
    async fn test_request_tile_geometry_returns_grid() {
        let dir = unique_temp_dir("request-geom");
        // Write a level-0 TMS tile: geographic (0,0) → disk y = 0.
        std::fs::create_dir_all(dir.join("0").join("0")).unwrap();
        let payload = build_heightmap_payload(&flat_heights(50.0));
        std::fs::write(dir.join("0").join("0").join("0.terrain"), &payload).unwrap();

        let fetcher = FileTerrainFetcher::new(&dir, TerrainScheme::Tms, 4);
        let geom = fetcher.request_tile_geometry(0, 0, 0).await.unwrap();
        assert_eq!(geom.positions.len(), HEIGHTMAP_GRID_SIZE * HEIGHTMAP_GRID_SIZE);
        assert!((geom.positions[0][2] - 50.0).abs() < 0.21);
    }

    #[tokio::test]
    async fn test_request_tile_geometry_missing_is_not_found() {
        let dir = unique_temp_dir("request-missing");
        let fetcher = FileTerrainFetcher::new(&dir, TerrainScheme::Tms, 4);
        let err = fetcher.request_tile_geometry(0, 0, 0).await.unwrap_err();
        assert!(matches!(err, PortError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_request_tile_geometry_level_exceeds_max() {
        let dir = unique_temp_dir("request-exceed");
        let fetcher = FileTerrainFetcher::new(&dir, TerrainScheme::Tms, 2);
        let err = fetcher.request_tile_geometry(0, 0, 5).await.unwrap_err();
        assert!(matches!(err, PortError::NotFound(_)));
    }

    #[test]
    fn test_get_availability() {
        let dir = unique_temp_dir("availability");
        std::fs::create_dir_all(dir.join("1").join("0")).unwrap();
        let payload = build_heightmap_payload(&flat_heights(0.0));
        // level 1 TMS: geographic (0,0) → disk y = 1.
        std::fs::write(dir.join("1").join("0").join("1.terrain"), &payload).unwrap();

        let fetcher = FileTerrainFetcher::new(&dir, TerrainScheme::Tms, 4);
        assert!(fetcher.get_availability(0, 0, 1));
        assert!(!fetcher.get_availability(1, 0, 1)); // disk y=0 absent
        assert!(!fetcher.get_availability(0, 0, 9)); // level > max
    }

    #[test]
    fn test_rectangle_and_maximum_level() {
        let dir = unique_temp_dir("accessors");
        let fetcher = FileTerrainFetcher::new(&dir, TerrainScheme::Geographic, 7);
        assert_eq!(fetcher.rectangle(), Rectangle::MAX_VALUE);
        assert_eq!(fetcher.maximum_level(), 7);
        assert_eq!(fetcher.scheme(), TerrainScheme::Geographic);
    }

    #[test]
    fn test_with_rectangle_override() {
        let dir = unique_temp_dir("rect-override");
        let rect = Rectangle::from_degrees(-10.0, -20.0, 30.0, 40.0);
        let fetcher =
            FileTerrainFetcher::new(&dir, TerrainScheme::Tms, 4).with_rectangle(rect);
        assert_eq!(fetcher.rectangle(), rect);
    }
}
