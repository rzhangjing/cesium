//! Ported from `packages/engine/Source/Renderer/TextureAtlas.js`.
//!
//! An atlas of small images packed into a single texture. Used for
//! efficiently rendering many small images (icons, glyphs, etc.) with
//! a single draw call.

use cesium_core::bounding_rectangle::BoundingRectangle;
use crate::texture::Texture;

/// An entry in the texture atlas representing a sub-region.
#[derive(Debug, Clone)]
pub struct TextureAtlasEntry {
    /// The index of this entry.
    pub index: usize,
    /// The bounding rectangle of this entry within the atlas texture.
    pub coords: BoundingRectangle,
}

/// An atlas of small images packed into a single texture.
///
/// Mirrors the CesiumJS `TextureAtlas` which packs multiple small images
/// into a single GPU texture for efficient rendering.
pub struct TextureAtlas {
    /// The packed texture.
    texture: Option<Texture>,
    /// The entries (sub-regions) in the atlas.
    entries: Vec<TextureAtlasEntry>,
    /// The border width in pixels (for preventing bleeding).
    border_width: u32,
    /// Whether the atlas is dirty and needs repacking.
    dirty: bool,
    is_destroyed: bool,
}

impl TextureAtlas {
    /// Creates a new texture atlas.
    pub fn new() -> Self {
        Self {
            texture: None,
            entries: Vec::new(),
            border_width: 1,
            dirty: true,
            is_destroyed: false,
        }
    }

    /// Creates a texture atlas with the given border width.
    pub fn with_border_width(border_width: u32) -> Self {
        Self {
            texture: None,
            entries: Vec::new(),
            border_width,
            dirty: true,
            is_destroyed: false,
        }
    }

    /// Returns the packed texture.
    pub fn texture(&self) -> Option<&Texture> {
        self.texture.as_ref()
    }

    /// Sets the packed texture.
    pub fn set_texture(&mut self, texture: Texture) {
        self.texture = Some(texture);
    }

    /// Returns the entries in the atlas.
    pub fn entries(&self) -> &[TextureAtlasEntry] {
        &self.entries
    }

    /// Returns the number of entries in the atlas.
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Adds an entry to the atlas.
    pub fn add_entry(&mut self, coords: BoundingRectangle) -> usize {
        let index = self.entries.len();
        self.entries.push(TextureAtlasEntry { index, coords });
        self.dirty = true;
        index
    }

    /// Returns the coordinates for the entry at the given index.
    pub fn get_coords(&self, index: usize) -> Option<&BoundingRectangle> {
        self.entries.get(index).map(|e| &e.coords)
    }

    /// Returns the border width.
    pub fn border_width(&self) -> u32 {
        self.border_width
    }

    /// Returns whether the atlas is dirty and needs repacking.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Marks the atlas as clean (after repacking).
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Returns whether this atlas has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys the atlas and its texture.
    pub fn destroy(&mut self) {
        if let Some(ref mut tex) = self.texture {
            tex.destroy();
        }
        self.entries.clear();
        self.is_destroyed = true;
    }
}

impl Default for TextureAtlas {
    fn default() -> Self { Self::new() }
}
