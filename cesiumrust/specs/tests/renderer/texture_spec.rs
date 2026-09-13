//! Renderer/TextureSpec.js, TextureAtlasSpec.js, FramebufferSpec.js
//! → Rust integration tests

use cesium_scene::{
    Texture, PixelFormat, PixelDatatype, TextureFilter, TextureWrap,
    Framebuffer, TextureAtlas,
};

// === Texture ===

#[test]
fn test_texture_new() {
    let tex = Texture::new(0, 256, 256);
    assert_eq!(tex.id, 0);
    assert_eq!(tex.width, 256);
    assert_eq!(tex.height, 256);
    assert_eq!(tex.format, PixelFormat::Rgba);
    assert_eq!(tex.datatype, PixelDatatype::UnsignedByte);
    assert!(!tex.generate_mipmaps);
}

#[test]
fn test_texture_with_mipmaps() {
    let tex = Texture::new(1, 512, 512).with_mipmaps();
    assert!(tex.generate_mipmaps);
    assert_eq!(tex.min_filter, TextureFilter::LinearMipmapLinear);
}

#[test]
fn test_texture_default_filters() {
    let tex = Texture::new(2, 64, 64);
    assert_eq!(tex.min_filter, TextureFilter::Linear);
    assert_eq!(tex.mag_filter, TextureFilter::Linear);
    assert_eq!(tex.wrap_s, TextureWrap::ClampToEdge);
    assert_eq!(tex.wrap_t, TextureWrap::ClampToEdge);
}

// === PixelFormat ===

#[test]
fn test_pixel_format_default() {
    assert_eq!(PixelFormat::default(), PixelFormat::Rgba);
}

#[test]
fn test_pixel_format_variants() {
    assert_ne!(PixelFormat::Rgba, PixelFormat::Rgb);
    assert_ne!(PixelFormat::Depth, PixelFormat::DepthStencil);
}

// === PixelDatatype ===

#[test]
fn test_pixel_datatype_default() {
    assert_eq!(PixelDatatype::default(), PixelDatatype::UnsignedByte);
}

// === TextureFilter ===

#[test]
fn test_texture_filter_default() {
    assert_eq!(TextureFilter::default(), TextureFilter::Linear);
}

// === TextureWrap ===

#[test]
fn test_texture_wrap_default() {
    assert_eq!(TextureWrap::default(), TextureWrap::ClampToEdge);
}

// === Framebuffer ===

#[test]
fn test_framebuffer_new() {
    let fb = Framebuffer::new(0, 1024, 768);
    assert_eq!(fb.width, 1024);
    assert_eq!(fb.height, 768);
    assert!(fb.color_textures.is_empty());
    assert!(fb.depth_texture.is_none());
    assert!(fb.stencil_texture.is_none());
}

#[test]
fn test_framebuffer_attach() {
    let mut fb = Framebuffer::new(0, 512, 512);
    fb.attach_color(10);
    fb.attach_color(11);
    fb.attach_depth(20);
    assert_eq!(fb.color_textures.len(), 2);
    assert_eq!(fb.depth_texture, Some(20));
}

// === TextureAtlas ===

#[test]
fn test_texture_atlas_new() {
    let atlas = TextureAtlas::new(0, 512, 512, 2);
    assert_eq!( atlas.texture.width, 512);
    assert_eq!(atlas.padding, 2);
    assert_eq!(atlas.entry_count(), 0);
}

#[test]
fn test_texture_atlas_add_entry() {
    let mut atlas = TextureAtlas::new(0, 256, 256, 1);
    let e1 = atlas.add_entry(64, 64);
    assert!(e1.is_some());
    let e1 = e1.unwrap();
    assert_eq!(e1.width, 64);
    assert_eq!(e1.height, 64);
    assert_eq!(atlas.entry_count(), 1);
}

#[test]
fn test_texture_atlas_multiple_entries() {
    let mut atlas = TextureAtlas::new(0, 256, 256, 1);
    let e1 = atlas.add_entry(32, 32).unwrap();
    let e2 = atlas.add_entry(32, 32).unwrap();
    // Entries should not overlap
    assert!(e1.x + e1.width + 1 <= e2.x || e2.x + e2.width + 1 <= e1.x || e1.y != e2.y);
    assert_eq!(atlas.entry_count(), 2);
}
