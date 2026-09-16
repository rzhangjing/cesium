//! Offline disk-backed tile fetcher with STRICT_OFFLINE semantics.
//!
//! Implements the [`TileFetcher`] port by reading tile bytes from a local
//! directory. Two on-disk layouts are supported (selected by
//! [`FileTileScheme`]):
//!
//! * [`FileTileScheme::Xyz`] — the canonical `{root}/{level}/{x}/{y}.{ext}`
//!   pyramid used by the viewer-demo offline imagery fixture and by the
//!   CesiumJS `NaturalEarthII` asset (blueprint:
//!   `cesium-rs/examples/viewer-demo/src/main.rs` L601-623).
//! * [`FileTileScheme::Quadkey`] — Bing-style single-segment quadkeys stored
//!   as `{root}/{quadkey}.{ext}` (quadkey digits `0`-`3`, level = digit
//!   count).
//!
//! # STRICT_OFFLINE contract
//!
//! The offline viewer-demo path never falls back to HTTP. When
//! [`FileTileFetcher::with_strict_offline`] is enabled and a caller passes an
//! `http://` or `https://` URL to [`TileFetcher::fetch`], the fetcher panics
//! *synchronously* — before the returned future is even built — so the
//! violation is loud and cannot be silently swallowed by an async runtime.
//!
//! # Async shape
//!
//! [`TileFetcher::fetch`] returns a boxed future for port compatibility, but
//! the disk read itself is plain [`std::fs::read`] inside an `async move`
//! block. No tokio runtime, no `spawn_blocking`: the future is `Send` and
//! resolves on the first poll, which matches the "offline = synchronous
//! local IO" contract.

use cesium_ports_driven::{PortError, PortResult, TileFetcher};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

/// Tile addressing scheme interpreted from the URL passed to
/// [`TileFetcher::fetch`].
///
/// The scheme determines how the URL tail maps to a disk path under
/// [`FileTileFetcher::root`]:
///
/// | Scheme   | URL tail            | Disk path                       |
/// |----------|---------------------|---------------------------------|
/// | `Xyz`    | `0/0/0[.ext]`       | `{root}/0/0/0.{ext}`            |
/// | `Quadkey`| `120[.ext]`         | `{root}/120.{ext}`              |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTileScheme {
    /// `{level}/{x}/{y}` XYZ pyramid (row 0 at the north pole).
    Xyz,
    /// Bing-style quadkey string (digits `0`-`3`, level = digit count).
    Quadkey,
}

/// Offline tile fetcher reading bytes from a local directory.
///
/// Clonable, `Send + Sync`, and free of any HTTP dependency — safe to share
/// across threads without a runtime.
#[derive(Debug, Clone)]
pub struct FileTileFetcher {
    root: PathBuf,
    scheme: FileTileScheme,
    strict_offline: bool,
    extension: String,
}

impl FileTileFetcher {
    /// Creates a fetcher rooted at `root` using the given URL scheme.
    ///
    /// The default extension is `png` (the viewer-demo offline imagery
    /// fixture writes `{level}/{x}/{y}.png`). Override with
    /// [`Self::with_extension`] when the tileset uses `jpg`/`jpeg`.
    pub fn new(root: impl AsRef<Path>, scheme: FileTileScheme) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            scheme,
            strict_offline: false,
            extension: "png".to_string(),
        }
    }

    /// Enables STRICT_OFFLINE semantics: any `http://` or `https://` URL
    /// passed to [`TileFetcher::fetch`] triggers an immediate panic.
    ///
    /// This is the offline viewer-demo contract — there is no HTTP fallback
    /// path, so a network URL reaching this fetcher is a wiring bug and
    /// must fail loudly.
    pub fn with_strict_offline(mut self, strict: bool) -> Self {
        self.strict_offline = strict;
        self
    }

    /// Overrides the default tile file extension (default: `png`).
    pub fn with_extension(mut self, ext: impl Into<String>) -> Self {
        self.extension = ext.into();
        self
    }

    /// Root directory of the tile pyramid.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Active URL scheme.
    pub fn scheme(&self) -> FileTileScheme {
        self.scheme
    }

    /// Whether STRICT_OFFLINE semantics are active.
    pub fn strict_offline(&self) -> bool {
        self.strict_offline
    }

    /// Resolves a URL to a disk path under [`Self::root`].
    ///
    /// # Panics
    ///
    /// Panics if [`Self::strict_offline`] is `true` and `url` starts with
    /// `http://` or `https://` — the STRICT_OFFLINE contract forbids any
    /// network fallback.
    fn resolve(&self, url: &str) -> PortResult<PathBuf> {
        if self.strict_offline && is_http_url(url) {
            panic!(
                "STRICT_OFFLINE violation: FileTileFetcher received network URL '{}' \
                 (offline mode forbids HTTP fallback; wire the offline root instead)",
                url
            );
        }
        let tail = strip_scheme_and_host(url);
        if tail.is_empty() {
            return Err(PortError::NotFound(format!(
                "FileTileFetcher: empty tile path after stripping scheme from '{}'",
                url
            )));
        }
        // Absolute paths (POSIX `/...` or Windows `C:\...`) bypass the root
        // join so `file:///abs/path.png` URLs resolve correctly.
        let candidate = Path::new(tail);
        let path = if candidate.is_absolute() || has_windows_drive(tail) {
            candidate.to_path_buf()
        } else {
            self.root.join(tail)
        };
        // Append the configured extension when the caller omitted it, so
        // `fetch("0/0/0")` and `fetch("0/0/0.png")` both resolve.
        if path.extension().is_none() {
            return Ok(path.with_extension(&self.extension));
        }
        Ok(path)
    }
}

impl TileFetcher for FileTileFetcher {
    fn fetch<'a>(
        &'a self,
        url: &'a str,
        _priority: f64,
    ) -> Pin<Box<dyn Future<Output = PortResult<Vec<u8>>> + Send + 'a>> {
        // Resolve synchronously so a STRICT_OFFLINE violation panics before
        // the future is even built (loud failure, no async swallowing).
        let path = match self.resolve(url) {
            Ok(p) => p,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        Box::pin(async move {
            std::fs::read(&path).map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    PortError::NotFound(format!(
                        "FileTileFetcher: tile not found at '{}'",
                        path.display()
                    ))
                } else {
                    PortError::Network(format!(
                        "FileTileFetcher: IO error reading '{}': {}",
                        path.display(),
                        e
                    ))
                }
            })
        })
    }

    fn cancel(&self, _url: &str) {
        // Disk reads are synchronous and non-cancellable; the future
        // resolves on first poll, so there is nothing to cancel.
    }
}

/// Returns `true` when `url` starts with `http://` or `https://`
/// (case-insensitive).
fn is_http_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// Strips a `scheme://host` prefix, returning the path tail.
///
/// * `file:///abs/path.png` → `/abs/path.png` (leading slash preserved so
///   the path stays absolute).
/// * `https://host/a/b.png` → `a/b.png` (host stripped).
/// * `a/b.png` (no scheme) → `a/b.png` (returned unchanged).
fn strip_scheme_and_host(url: &str) -> &str {
    let Some(idx) = url.find("://") else {
        return url;
    };
    let scheme = &url[..idx];
    let after = &url[idx + 3..];
    // `file://` URLs keep the leading slash to remain absolute on POSIX;
    // on Windows, `file:///C:/path` yields `/C:/path` whose leading slash
    // must be dropped so the drive letter becomes the path prefix.
    if scheme.eq_ignore_ascii_case("file") {
        if let Some(slash) = after.find('/') {
            return strip_windows_drive_slash(&after[slash..]);
        }
        return after;
    }
    // http(s)://host/path → drop host, keep path after the first '/'.
    match after.find('/') {
        Some(slash) => &after[slash + 1..],
        None => "",
    }
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

/// Detects a Windows drive-letter prefix like `C:` at the start of `s`.
fn has_windows_drive(s: &str) -> bool {
    let mut chars = s.chars();
    match (chars.next(), chars.next()) {
        (Some(c), Some(':')) => c.is_ascii_alphabetic(),
        _ => false,
    }
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
            "cesium-file-tile-fetcher-{}-{}-{}",
            tag,
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    // --- URL helpers ---------------------------------------------------------

    #[test]
    fn test_is_http_url() {
        assert!(is_http_url("http://example.com/a.png"));
        assert!(is_http_url("HTTPS://example.com/a.png"));
        assert!(!is_http_url("file:///abs/a.png"));
        assert!(!is_http_url("0/0/0.png"));
        assert!(!is_http_url("/abs/a.png"));
    }

    #[test]
    fn test_strip_scheme_and_host_file() {
        assert_eq!(strip_scheme_and_host("file:///abs/a.png"), "/abs/a.png");
        assert_eq!(strip_scheme_and_host("file://host/a.png"), "/a.png");
    }

    #[test]
    fn test_strip_scheme_and_host_https() {
        assert_eq!(
            strip_scheme_and_host("https://example.com/tiles/0/0/0.png"),
            "tiles/0/0/0.png"
        );
        assert_eq!(strip_scheme_and_host("https://example.com"), "");
    }

    #[test]
    fn test_strip_scheme_and_host_plain() {
        assert_eq!(strip_scheme_and_host("0/0/0.png"), "0/0/0.png");
    }

    #[test]
    fn test_has_windows_drive() {
        assert!(has_windows_drive("C:/tiles"));
        assert!(has_windows_drive("d:\\tiles"));
        assert!(!has_windows_drive("/tiles"));
        assert!(!has_windows_drive("0/0/0"));
    }

    // --- resolve -------------------------------------------------------------

    #[test]
    fn test_resolve_xyz_relative_appends_extension() {
        let dir = unique_temp_dir("resolve-xyz");
        let fetcher = FileTileFetcher::new(&dir, FileTileScheme::Xyz);
        let path = fetcher.resolve("0/0/0").unwrap();
        assert_eq!(path, dir.join("0/0/0.png"));
    }

    #[test]
    fn test_resolve_xyz_relative_keeps_extension() {
        let dir = unique_temp_dir("resolve-xyz-ext");
        let fetcher = FileTileFetcher::new(&dir, FileTileScheme::Xyz);
        let path = fetcher.resolve("1/2/3.jpg").unwrap();
        assert_eq!(path, dir.join("1/2/3.jpg"));
    }

    #[test]
    fn test_resolve_quadkey_relative() {
        let dir = unique_temp_dir("resolve-qk");
        let fetcher = FileTileFetcher::new(&dir, FileTileScheme::Quadkey);
        let path = fetcher.resolve("120").unwrap();
        assert_eq!(path, dir.join("120.png"));
    }

    #[test]
    fn test_resolve_custom_extension() {
        let dir = unique_temp_dir("resolve-ext");
        let fetcher =
            FileTileFetcher::new(&dir, FileTileScheme::Xyz).with_extension("jpeg");
        let path = fetcher.resolve("0/0/0").unwrap();
        assert_eq!(path, dir.join("0/0/0.jpeg"));
    }

    #[test]
    fn test_resolve_file_url_absolute() {
        let dir = unique_temp_dir("resolve-file-url");
        let fetcher = FileTileFetcher::new(&dir, FileTileScheme::Xyz);
        // A file:// URL pointing outside the root must stay absolute.
        let abs = dir.join("abs.png");
        let url = format!("file:///{}", abs.display().to_string().replace('\\', "/"));
        let path = fetcher.resolve(&url).unwrap();
        assert!(path.is_absolute());
        assert_eq!(path.file_name().unwrap(), "abs.png");
    }

    #[test]
    fn test_resolve_https_tail_under_root_when_not_strict() {
        let dir = unique_temp_dir("resolve-https-tail");
        let fetcher = FileTileFetcher::new(&dir, FileTileScheme::Xyz);
        // Not strict: the https tail is rebased under the offline root.
        let path = fetcher
            .resolve("https://example.com/tiles/0/0/0.png")
            .unwrap();
        assert_eq!(path, dir.join("tiles/0/0/0.png"));
    }

    #[test]
    fn test_resolve_empty_tail_errors() {
        let dir = unique_temp_dir("resolve-empty");
        let fetcher = FileTileFetcher::new(&dir, FileTileScheme::Xyz);
        let err = fetcher.resolve("https://example.com").unwrap_err();
        assert!(matches!(err, PortError::NotFound(_)));
    }

    // --- STRICT_OFFLINE panic ------------------------------------------------

    #[test]
    #[should_panic(expected = "STRICT_OFFLINE violation")]
    fn test_strict_offline_panics_on_https() {
        let dir = unique_temp_dir("strict-https");
        let fetcher =
            FileTileFetcher::new(&dir, FileTileScheme::Xyz).with_strict_offline(true);
        // Panics synchronously inside `resolve`, before the future is built.
        let _ = fetcher.resolve("https://example.com/0/0/0.png");
    }

    #[test]
    #[should_panic(expected = "STRICT_OFFLINE violation")]
    fn test_strict_offline_panics_on_http() {
        let dir = unique_temp_dir("strict-http");
        let fetcher =
            FileTileFetcher::new(&dir, FileTileScheme::Xyz).with_strict_offline(true);
        let _ = fetcher.resolve("http://example.com/0/0/0.png");
    }

    #[test]
    fn test_strict_offline_allows_file_and_relative() {
        let dir = unique_temp_dir("strict-ok");
        let fetcher =
            FileTileFetcher::new(&dir, FileTileScheme::Xyz).with_strict_offline(true);
        // Relative and file:// URLs are fine under STRICT_OFFLINE.
        assert!(fetcher.resolve("0/0/0").is_ok());
        assert!(fetcher
            .resolve(&format!(
                "file:///{}",
                dir.join("a.png").display().to_string().replace('\\', "/")
            ))
            .is_ok());
    }

    #[tokio::test]
    #[should_panic(expected = "STRICT_OFFLINE violation")]
    async fn test_strict_offline_panics_via_fetch() {
        let dir = unique_temp_dir("strict-fetch");
        let fetcher =
            FileTileFetcher::new(&dir, FileTileScheme::Xyz).with_strict_offline(true);
        // The panic fires synchronously inside `fetch`, so awaiting is moot.
        let _ = fetcher.fetch("https://example.com/0/0/0.png", 1.0).await;
    }

    // --- end-to-end fetch ----------------------------------------------------

    #[tokio::test]
    async fn test_fetch_xyz_returns_bytes() {
        let dir = unique_temp_dir("fetch-xyz");
        std::fs::create_dir_all(dir.join("0/0")).unwrap();
        std::fs::write(dir.join("0/0/0.png"), b"tile-bytes").unwrap();

        let fetcher = FileTileFetcher::new(&dir, FileTileScheme::Xyz);
        let data = fetcher.fetch("0/0/0", 1.0).await.unwrap();
        assert_eq!(data, b"tile-bytes");
    }

    #[tokio::test]
    async fn test_fetch_xyz_with_extension_in_url() {
        let dir = unique_temp_dir("fetch-xyz-ext");
        std::fs::create_dir_all(dir.join("2/3")).unwrap();
        std::fs::write(dir.join("2/3/1.jpg"), b"jpeg-bytes").unwrap();

        let fetcher = FileTileFetcher::new(&dir, FileTileScheme::Xyz);
        let data = fetcher.fetch("2/3/1.jpg", 1.0).await.unwrap();
        assert_eq!(data, b"jpeg-bytes");
    }

    #[tokio::test]
    async fn test_fetch_quadkey_returns_bytes() {
        let dir = unique_temp_dir("fetch-qk");
        std::fs::write(dir.join("120.png"), b"qk-bytes").unwrap();

        let fetcher = FileTileFetcher::new(&dir, FileTileScheme::Quadkey);
        let data = fetcher.fetch("120", 1.0).await.unwrap();
        assert_eq!(data, b"qk-bytes");
    }

    #[tokio::test]
    async fn test_fetch_missing_tile_is_not_found() {
        let dir = unique_temp_dir("fetch-missing");
        let fetcher = FileTileFetcher::new(&dir, FileTileScheme::Xyz);
        let err = fetcher.fetch("9/9/9", 1.0).await.unwrap_err();
        assert!(matches!(err, PortError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_fetch_cancel_is_noop() {
        let dir = unique_temp_dir("fetch-cancel");
        std::fs::create_dir_all(dir.join("0/0")).unwrap();
        std::fs::write(dir.join("0/0/0.png"), b"x").unwrap();

        let fetcher = FileTileFetcher::new(&dir, FileTileScheme::Xyz);
        fetcher.cancel("0/0/0");
        // cancel is a no-op; the read still succeeds.
        let data = fetcher.fetch("0/0/0", 1.0).await.unwrap();
        assert_eq!(data, b"x");
    }

    #[test]
    fn test_accessors() {
        let dir = unique_temp_dir("accessors");
        let fetcher = FileTileFetcher::new(&dir, FileTileScheme::Quadkey)
            .with_strict_offline(true)
            .with_extension("jpg");
        assert_eq!(fetcher.root(), dir.as_path());
        assert_eq!(fetcher.scheme(), FileTileScheme::Quadkey);
        assert!(fetcher.strict_offline());
    }
}
