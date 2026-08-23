//! Tests for `cesium_core::TexturePacker`.

use cesium_core::texture_packer::TexturePacker;

#[test]
fn new_creates_packer() {
    let packer = TexturePacker::new(256, 256, 1);
    assert!(packer.root().index.is_none());
}

#[test]
fn pack_first_item_succeeds() {
    let mut packer = TexturePacker::new(256, 256, 0);
    let result = packer.pack(0, 64, 64);
    assert!(result.is_some());
}

#[test]
fn pack_multiple_items() {
    let mut packer = TexturePacker::new(256, 256, 0);
    assert!(packer.pack(0, 64, 64).is_some());
    assert!(packer.pack(1, 64, 64).is_some());
    assert!(packer.pack(2, 64, 64).is_some());
}

#[test]
fn pack_item_too_large_returns_none() {
    let mut packer = TexturePacker::new(64, 64, 0);
    let result = packer.pack(0, 128, 128);
    assert!(result.is_none());
}

#[test]
fn pack_with_border_padding() {
    let mut packer = TexturePacker::new(256, 256, 4);
    assert!(packer.pack(0, 32, 32).is_some());
    assert!(packer.pack(1, 32, 32).is_some());
}
