//! Ported from `packages/engine/Source/Scene/Multiple3DTileContent.js`.
//!
//! Manages multiple content payloads within a single 3D tile,
//! supporting the 3DTILES_multiple_contents extension.

/// Multiple 3D Tiles content in a single tile.
///
/// Manages the `3DTILES_multiple_contents` extension, allowing a single
/// tile to reference multiple content payloads (e.g. separate geometry
/// and metadata files).
/// Mirrors CesiumJS `Multiple3DTileContent` (~300 lines).
pub struct Multiple3DTileContent {
    /// The number of inner content entries.
    pub contents_length: u32,
    /// URLs of the inner content entries.
    pub content_urls: Vec<String>,
    /// Whether all inner contents are ready.
    pub ready: bool,
    /// The number of requests currently in flight.
    pub requests_in_flight: u32,
    /// The number of external tileset references.
    pub external_tileset_count: u32,
}

impl Multiple3DTileContent {
    /// Creates a new `Multiple3DTileContent`.
    pub fn new() -> Self {
        Self {
            contents_length: 0,
            content_urls: Vec::new(),
            ready: false,
            requests_in_flight: 0,
            external_tileset_count: 0,
        }
    }

    /// Returns whether all inner contents have been loaded.
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Returns the number of inner content entries.
    pub fn inner_length(&self) -> u32 {
        self.contents_length
    }

    /// Adds a content URL.
    pub fn add_content_url(&mut self, url: String) {
        self.content_urls.push(url);
        self.contents_length = self.content_urls.len() as u32;
    }
}

impl Default for Multiple3DTileContent {
    fn default() -> Self { Self::new() }
}
