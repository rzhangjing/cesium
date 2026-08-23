//! Tests for `cesium_core::PolygonHierarchy`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::polygon_hierarchy::PolygonHierarchy;

#[test]
fn new_creates_empty_hierarchy() {
    let h = PolygonHierarchy::new(Vec::new(), Vec::new());
    assert!(h.positions.is_empty());
    assert!(h.holes.is_empty());
}

#[test]
fn new_with_positions() {
    let positions = vec![
        Cartesian3::new(0.0, 0.0, 0.0),
        Cartesian3::new(1.0, 0.0, 0.0),
        Cartesian3::new(0.5, 1.0, 0.0),
    ];
    let h = PolygonHierarchy::new(positions.clone(), Vec::new());
    assert_eq!(h.positions.len(), 3);
}

#[test]
fn new_with_holes() {
    let outer = vec![
        Cartesian3::new(0.0, 0.0, 0.0),
        Cartesian3::new(10.0, 0.0, 0.0),
        Cartesian3::new(5.0, 10.0, 0.0),
    ];
    let hole_positions = vec![
        Cartesian3::new(3.0, 2.0, 0.0),
        Cartesian3::new(7.0, 2.0, 0.0),
        Cartesian3::new(5.0, 6.0, 0.0),
    ];
    let hole = PolygonHierarchy::new(
        vec![
            Cartesian3::new(3.0, 2.0, 0.0),
            Cartesian3::new(7.0, 2.0, 0.0),
            Cartesian3::new(5.0, 6.0, 0.0),
        ],
        Vec::new(),
    );
    let h = PolygonHierarchy::new(outer, vec![hole]);
    assert_eq!(h.holes.len(), 1);
    assert_eq!(h.holes[0].positions.len(), 3);
}
