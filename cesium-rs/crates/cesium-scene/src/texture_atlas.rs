//! Ported from `packages/engine/Source/Renderer/TextureAtlas.js`.
//!
//! A texture atlas that packs multiple images into a single texture.
//!
//! M3/S3 materialization: the CPU half of the CesiumJS `TextureAtlas`
//! (image packing + texture-coordinate rectangles) is ported one-to-one;
//! the GPU texture is (re)built lazily on the first `texture()` call after
//! any `add_image`, mirroring the JS `_destroyTexture` + rebuild-on-dirty
//! behavior.

use std::collections::HashMap;
use std::sync::Arc;

use cesium_renderer::context::Context;
use cesium_renderer::texture::{Texture, TextureOptions, TextureSource};

/// Texture coordinates of one packed image, in `[0, 1]` atlas space
/// (mirrors the JS `TextureCoordinateRectangle`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureCoordinateRectangle {
    pub x_min: f64,
    pub y_min: f64,
    pub x_max: f64,
    pub y_max: f64,
}

/// One image packed into the atlas.
struct PackedImage {
    /// The image identifier (mirrors the JS `imageId`).
    id: String,
    /// Pixel offset of the image inside the atlas.
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    /// RGBA8 pixel data, row-major.
    rgba: Vec<u8>,
}

/// A texture atlas that packs multiple images into a single texture.
pub struct TextureAtlas {
    /// The packed images, in insertion order.
    images: Vec<PackedImage>,
    /// imageId → index into `images` (mirrors the JS `_imagesWithIds`).
    image_index: HashMap<String, usize>,
    /// The current atlas dimensions (power-of-two, grown on demand).
    width: u32,
    height: u32,
    /// Row-pack cursor (mirrors the JS border-padded row layout).
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
    /// Whether the GPU texture is stale.
    dirty: bool,
    /// The GPU texture (lazily created/rebuilt).
    texture: Option<Arc<Texture>>,
}

/// Padding between packed images (mirrors the JS one-pixel border that
/// keeps linear sampling from bleeding across images).
const BORDER_IN_PIXELS: u32 = 1;

impl TextureAtlas {
    /// Creates an empty atlas.
    pub fn new() -> Self {
        Self {
            images: Vec::new(),
            image_index: HashMap::new(),
            width: 1,
            height: 1,
            cursor_x: BORDER_IN_PIXELS,
            cursor_y: BORDER_IN_PIXELS,
            row_height: 0,
            dirty: true,
            texture: None,
        }
    }

    /// Packs an image and returns its texture-coordinate rectangle.
    ///
    /// Mirrors CesiumJS `TextureAtlas#addImage`: re-adding the same `id`
    /// returns the existing rectangle; otherwise the image is row-packed
    /// (growing the atlas to the next power of two when it overflows).
    pub fn add_image(&mut self, id: &str, width: u32, height: u32, rgba: Vec<u8>) -> TextureCoordinateRectangle {
        if let Some(index) = self.image_index.get(id) {
            return self.rectangle_for(*index);
        }

        let image_width = width + 2 * BORDER_IN_PIXELS;
        let image_height = height + 2 * BORDER_IN_PIXELS;

        // Wrap to the next row when the image does not fit on the
        // current one (mirrors the JS row packing).
        if self.cursor_x + image_width > self.width && self.cursor_x > BORDER_IN_PIXELS {
            self.cursor_x = BORDER_IN_PIXELS;
            self.cursor_y += self.row_height;
            self.row_height = 0;
        }

        // Grow the atlas (power-of-two dimensions) until the image fits.
        while self.width < self.cursor_x + image_width {
            self.width = (self.width * 2).next_power_of_two();
        }
        while self.height < self.cursor_y + image_height {
            self.height = (self.height * 2).next_power_of_two();
        }

        let x = self.cursor_x;
        let y = self.cursor_y;
        self.cursor_x += image_width;
        self.row_height = self.row_height.max(image_height);

        let index = self.images.len();
        self.image_index.insert(id.to_string(), index);
        self.images.push(PackedImage {
            id: id.to_string(),
            x,
            y,
            width,
            height,
            rgba,
        });
        self.dirty = true;
        self.rectangle_for(index)
    }

    /// Returns the texture-coordinate rectangle of the image at `index`.
    fn rectangle_for(&self, index: usize) -> TextureCoordinateRectangle {
        let image = &self.images[index];
        let width = self.width as f64;
        let height = self.height as f64;
        TextureCoordinateRectangle {
            x_min: image.x as f64 / width,
            y_min: image.y as f64 / height,
            x_max: (image.x + image.width) as f64 / width,
            y_max: (image.y + image.height) as f64 / height,
        }
    }

    /// Returns the rectangle of a previously added image, if present
    /// (mirrors the JS lookup used by `BillboardCollection`).
    pub fn rectangle_of(&self, id: &str) -> Option<TextureCoordinateRectangle> {
        self.image_index.get(id).map(|index| self.rectangle_for(*index))
    }

    /// Returns the number of packed images (mirrors the JS
    /// `numberOfImages` property).
    pub fn number_of_images(&self) -> usize {
        self.images.len()
    }

    /// Returns the atlas width/height in pixels.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Returns the GPU texture, (re)building it when dirty.
    ///
    /// Mirrors the JS lazy texture creation inside `addImage`/`update`;
    /// the wgpu port needs the context, hence the explicit parameter.
    pub fn texture(&mut self, context: &Context) -> Option<Arc<Texture>> {
        if !self.dirty {
            return self.texture.clone();
        }
        if self.images.is_empty() {
            return None;
        }

        let mut pixels = vec![0u8; (self.width * self.height * 4) as usize];
        for image in &self.images {
            let expected = (image.width * image.height * 4) as usize;
            if image.rgba.len() != expected {
                log::warn!("texture atlas image {} has {} bytes, expected {}", image.id, image.rgba.len(), expected);
            }
            for row in 0..image.height {
                let src_start = (row * image.width * 4) as usize;
                let src_end = (src_start + (image.width * 4) as usize).min(image.rgba.len());
                if src_start >= image.rgba.len() {
                    break;
                }
                let dst_start = (((image.y + row) * self.width + image.x) * 4) as usize;
                pixels[dst_start..dst_start + (src_end - src_start)]
                    .copy_from_slice(&image.rgba[src_start..src_end]);
            }
        }

        let texture = context.create_texture(TextureOptions {
            source: Some(TextureSource {
                width: self.width,
                height: self.height,
                array_buffer_view: pixels.clone(),
            }),
            ..Default::default()
        });
        texture.upload_source(context.queue(), &TextureSource {
            width: self.width,
            height: self.height,
            array_buffer_view: pixels,
        });

        self.texture = Some(Arc::new(texture));
        self.dirty = false;
        self.texture.clone()
    }

    /// Destroys the GPU texture (mirrors the JS `destroy`).
    pub fn destroy(&mut self) {
        self.texture = None;
        self.images.clear();
        self.image_index.clear();
        self.dirty = true;
    }
}

impl Default for TextureAtlas {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mirrors TextureAtlasSpec: "adds a single image".
    #[test]
    fn adds_a_single_image() {
        let mut atlas = TextureAtlas::new();
        let rect = atlas.add_image("a", 2, 2, vec![255u8; 16]);
        assert_eq!(atlas.number_of_images(), 1);
        // The image sits at the 1px border; atlas grew to a power of two.
        assert!(rect.x_min >= 1.0 / atlas.dimensions().0 as f64);
        assert!(rect.y_min >= 1.0 / atlas.dimensions().1 as f64);
        assert!(rect.x_max > rect.x_min);
        assert!(rect.y_max > rect.y_min);
    }

    /// Mirrors TextureAtlasSpec: "returns the same rectangle for a
    /// duplicate image id".
    #[test]
    fn duplicate_id_returns_existing_rectangle() {
        let mut atlas = TextureAtlas::new();
        let first = atlas.add_image("a", 2, 2, vec![255u8; 16]);
        let second = atlas.add_image("a", 2, 2, vec![0u8; 16]);
        assert_eq!(first, second);
        assert_eq!(atlas.number_of_images(), 1);
    }

    /// Mirrors TextureAtlasSpec: "packs multiple images without overlap".
    /// Note: like the JS atlas, growing the texture invalidates previously
    /// returned rectangles, so the first rectangle is re-fetched after the
    /// second add.
    #[test]
    fn packs_multiple_images_row_major() {
        let mut atlas = TextureAtlas::new();
        atlas.add_image("a", 4, 4, vec![255u8; 64]);
        atlas.add_image("b", 4, 4, vec![128u8; 64]);
        assert_eq!(atlas.number_of_images(), 2);
        let r0 = atlas.rectangle_of("a").unwrap();
        let r1 = atlas.rectangle_of("b").unwrap();
        // Non-overlapping packing (rows or columns).
        assert!(r1.x_min >= r0.x_max || r1.y_min >= r0.y_max);
        assert_eq!(atlas.rectangle_of("missing"), None);
    }

    /// Mirrors TextureAtlasSpec growth cases: oversized images grow the
    /// atlas to a power-of-two square that contains them.
    #[test]
    fn grows_to_fit_oversized_image() {
        let mut atlas = TextureAtlas::new();
        atlas.add_image("big", 30, 50, vec![1u8; 30 * 50 * 4]);
        let (width, height) = atlas.dimensions();
        assert!(width.is_power_of_two());
        assert!(height.is_power_of_two());
        assert!(width >= 32);
        assert!(height >= 64);
    }
}
