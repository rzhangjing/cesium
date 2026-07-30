//! Traversal extended specs - TilePriority/MemoryAdjustedSse/can_traverse
//! Ported from Scene/Cesium3DTilesetTraversalSpec.js (A-class priority/memory paths)

use cesium_tileset::traversal::{can_traverse, MemoryAdjustedSse, TilePriority};
use cesium_tileset::tile::{Tile, TileContent, TileRefine};
use cesium_tileset::bounding_volume::BoundingVolume;
use glam::DVec3;

fn make_tile(geometric_error: f64, children: Vec<Tile>) -> Tile {
    Tile {
        bounding_volume: BoundingVolume::from_sphere(DVec3::ZERO, 100.0),
        geometric_error,
        refine: Some(TileRefine::Replace),
        transform: None,
        content: Some(TileContent {
            uri: "tile.b3dm".to_string(),
            bounding_volume: None,
            group: None,
        }),
        contents: None,
        children,
        viewer_request_volume: None,
        extras: None,
    }
}

fn make_leaf(geometric_error: f64) -> Tile {
    make_tile(geometric_error, vec![])
}

// ─── TilePriority ───────────────────────────────────────────────────────────

#[test]
fn priority_value_basic() {
    let p = TilePriority {
        distance: 100.0,
        depth: 2,
        is_ancestor: false,
    };
    // value = distance + depth * 0.01 = 100.0 + 0.02 = 100.02
    assert!((p.value() - 100.02).abs() < 1e-10);
}

#[test]
fn priority_value_ancestor_bonus() {
    let p = TilePriority {
        distance: 100.0,
        depth: 2,
        is_ancestor: true,
    };
    // value = -1000 + 100.0 + 0.02 = -899.98
    assert!((p.value() - (-899.98)).abs() < 1e-10);
}

#[test]
fn priority_ancestor_always_before_non_ancestor() {
    let ancestor = TilePriority {
        distance: 1000.0,
        depth: 10,
        is_ancestor: true,
    };
    let non_ancestor = TilePriority {
        distance: 1.0,
        depth: 0,
        is_ancestor: false,
    };
    assert!(ancestor < non_ancestor, "ancestor should have higher priority (lower value)");
}

#[test]
fn priority_closer_distance_higher_priority() {
    let close = TilePriority {
        distance: 10.0,
        depth: 1,
        is_ancestor: false,
    };
    let far = TilePriority {
        distance: 1000.0,
        depth: 1,
        is_ancestor: false,
    };
    assert!(close < far);
}

#[test]
fn priority_same_distance_lower_depth_first() {
    let shallow = TilePriority {
        distance: 100.0,
        depth: 1,
        is_ancestor: false,
    };
    let deep = TilePriority {
        distance: 100.0,
        depth: 5,
        is_ancestor: false,
    };
    assert!(shallow < deep);
}

#[test]
fn priority_ordering_eq() {
    let a = TilePriority { distance: 50.0, depth: 3, is_ancestor: false };
    let b = TilePriority { distance: 50.0, depth: 3, is_ancestor: false };
    assert_eq!(a, b);
}

// ─── MemoryAdjustedSse ──────────────────────────────────────────────────────

#[test]
fn memory_sse_under_50_percent() {
    let mut mas = MemoryAdjustedSse::new(16.0, 1_000_000);
    mas.current_memory_bytes = 400_000; // 40% usage
    assert!((mas.adjusted_sse() - 16.0).abs() < 1e-10, "under 50% should use base SSE");
}

#[test]
fn memory_sse_at_50_percent() {
    let mut mas = MemoryAdjustedSse::new(16.0, 1_000_000);
    mas.current_memory_bytes = 500_000; // exactly 50%
    assert!((mas.adjusted_sse() - 16.0).abs() < 1e-10, "at 50% should use base SSE");
}

#[test]
fn memory_sse_at_75_percent() {
    let mut mas = MemoryAdjustedSse::new(16.0, 1_000_000);
    mas.current_memory_bytes = 750_000; // 75% usage
    // t = (0.75 - 0.5) / 0.5 = 0.5
    // adjusted = 16 * (1 + 0.5) = 24
    assert!((mas.adjusted_sse() - 24.0).abs() < 1e-10);
}

#[test]
fn memory_sse_at_100_percent() {
    let mut mas = MemoryAdjustedSse::new(16.0, 1_000_000);
    mas.current_memory_bytes = 1_000_000; // 100% usage
    // t = (1.0 - 0.5) / 0.5 = 1.0
    // adjusted = 16 * (1 + 1) = 32
    assert!((mas.adjusted_sse() - 32.0).abs() < 1e-10);
}

#[test]
fn memory_sse_over_100_percent() {
    let mut mas = MemoryAdjustedSse::new(16.0, 1_000_000);
    mas.current_memory_bytes = 1_500_000; // 150% usage
    // overage = 1.5 - 1.0 = 0.5
    // adjusted = 16 * (2 + 0.5 * 4) = 16 * 4 = 64
    assert!((mas.adjusted_sse() - 64.0).abs() < 1e-10);
}

#[test]
fn memory_sse_zero_max() {
    let mas = MemoryAdjustedSse::new(16.0, 0);
    assert!((mas.adjusted_sse() - 16.0).abs() < 1e-10, "zero max should return base");
}

#[test]
fn memory_sse_is_over_limit_true() {
    let mut mas = MemoryAdjustedSse::new(16.0, 1_000_000);
    mas.current_memory_bytes = 1_000_001;
    assert!(mas.is_over_limit());
}

#[test]
fn memory_sse_is_over_limit_false() {
    let mut mas = MemoryAdjustedSse::new(16.0, 1_000_000);
    mas.current_memory_bytes = 999_999;
    assert!(!mas.is_over_limit());
}

#[test]
fn memory_sse_is_over_limit_exact() {
    let mut mas = MemoryAdjustedSse::new(16.0, 1_000_000);
    mas.current_memory_bytes = 1_000_000;
    assert!(!mas.is_over_limit(), "exactly at limit is not over");
}

// ─── can_traverse ───────────────────────────────────────────────────────────

#[test]
fn can_traverse_no_children_no_implicit() {
    let tile = make_leaf(100.0);
    assert!(!can_traverse(&tile, 200.0, 16.0, false), "no children + no implicit = cannot traverse");
}

#[test]
fn can_traverse_no_children_with_implicit() {
    let tile = make_leaf(100.0);
    assert!(can_traverse(&tile, 200.0, 16.0, true), "implicit content allows traversal");
}

#[test]
fn can_traverse_sse_exceeds_max() {
    let child = make_leaf(10.0);
    let tile = make_tile(100.0, vec![child]);
    assert!(can_traverse(&tile, 200.0, 16.0, false), "SSE > max should allow traversal");
}

#[test]
fn can_traverse_sse_below_max() {
    let child = make_leaf(10.0);
    let tile = make_tile(100.0, vec![child]);
    assert!(!can_traverse(&tile, 10.0, 16.0, false), "SSE <= max should not traverse");
}

#[test]
fn can_traverse_sse_equals_max() {
    let child = make_leaf(10.0);
    let tile = make_tile(100.0, vec![child]);
    assert!(!can_traverse(&tile, 16.0, 16.0, false), "SSE == max should not traverse");
}

#[test]
fn can_traverse_multiple_children() {
    let children = vec![make_leaf(10.0), make_leaf(5.0)];
    let tile = make_tile(100.0, children);
    assert!(can_traverse(&tile, 50.0, 16.0, false));
}
