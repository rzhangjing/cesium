//! Tests for `cesium_core::S2Cell`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::s2_cell::S2Cell;

fn magnitude(v: &Cartesian3) -> f64 {
    (v.x * v.x + v.y * v.y + v.z * v.z).sqrt()
}

// In this implementation, token "1" → cell_id=1 → level 30 (leaf).
// For higher-level cells, we test via is_valid_id / get_level directly.

#[test]
fn from_token_creates_cell() {
    let cell = S2Cell::from_token("1");
    assert_eq!(cell.level(), 30);
}

#[test]
fn is_valid_id_accepts_known_good() {
    // cell_id = 1 is a valid level-30 cell
    assert!(S2Cell::is_valid_id(1));
    // cell_id = 0 is invalid
    assert!(!S2Cell::is_valid_id(0));
}

#[test]
fn get_level_from_cell_id() {
    // cell_id = 1 → lsb at bit 0 → level = 30 - 0 = 30
    assert_eq!(S2Cell::get_level(1), 30);
    // cell_id = 4 → lsb at bit 2 → level = 30 - 1 = 29
    assert_eq!(S2Cell::get_level(4), 29);
}

#[test]
fn get_parent_of_leaf() {
    let cell = S2Cell::from_token("1"); // level 30
    let parent = cell.get_parent();
    assert_eq!(parent.level(), 29);
}

#[test]
fn get_parent_at_level() {
    let cell = S2Cell::from_token("1"); // level 30
    let ancestor = cell.get_parent_at_level(25);
    assert_eq!(ancestor.level(), 25);
}

#[test]
fn get_center_returns_nonzero_cartesian() {
    let cell = S2Cell::from_token("1");
    let center = cell.get_center(None);
    assert!(magnitude(&center) > 0.0);
}

#[test]
fn get_vertex_returns_four_vertices() {
    let cell = S2Cell::from_token("1");
    for i in 0..4 {
        let v = cell.get_vertex(i, None);
        assert!(magnitude(&v) > 0.0);
    }
}

#[test]
fn is_valid_token_rejects_empty() {
    assert!(!S2Cell::is_valid_token(""));
}

#[test]
fn is_valid_token_accepts_hex() {
    assert!(S2Cell::is_valid_token("1"));
    assert!(S2Cell::is_valid_token("abc"));
}
