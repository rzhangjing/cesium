//! Pure **screen-pixel** hit testing (plan §6 / §7).
//!
//! Every function here works on `[f64; 2]` screen coordinates (top-left origin,
//! y down — the same space [`bevy::camera::Camera::world_to_viewport`] returns),
//! so the whole hit / tolerance / inside logic is projection-agnostic and
//! unit-testable with no engine. The bridge projects a geometry's geographic
//! vertices through the active camera and then calls these.
//!
//! Each function returns the matched [`Part`] *and* the screen distance to it so
//! the caller can fold candidates through [`crate::model::pick::pick_best`] for
//! the §7 priority ranking.

use crate::model::pick::Part;

/// Default pointer tolerance in screen pixels for line / edge / vertex picks.
pub const DEFAULT_TOL_PX: f64 = 6.0;

/// Euclidean distance between two screen points.
#[inline]
pub fn distance_to_point(p: [f64; 2], a: [f64; 2]) -> f64 {
    let dx = p[0] - a[0];
    let dy = p[1] - a[1];
    (dx * dx + dy * dy).sqrt()
}

/// Shortest distance from `p` to the segment `a..b` (clamped, so it degrades to
/// a point distance at the endpoints).
pub fn distance_to_segment(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let abx = b[0] - a[0];
    let aby = b[1] - a[1];
    let apx = p[0] - a[0];
    let apy = p[1] - a[1];
    let len2 = abx * abx + aby * aby;
    let t = if len2 > 1e-12 {
        ((apx * abx + apy * aby) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let cx = a[0] + t * abx;
    let cy = a[1] + t * aby;
    distance_to_point(p, [cx, cy])
}

/// A point / icon marker: hit when the cursor is within `radius_px` of `center`.
/// Marks the whole marker as [`Part::Body`].
pub fn hit_point(cursor: [f64; 2], center: [f64; 2], radius_px: f64) -> Option<(Part, f64)> {
    let d = distance_to_point(cursor, center);
    if d <= radius_px.max(0.0) {
        Some((Part::Body, d))
    } else {
        None
    }
}

/// A polyline: vertices win over edges (so a corner grabs the drag handle in
/// edit modes), and among ties the nearest feature is returned. `pts` are the
/// projected screen vertices.
pub fn hit_polyline(cursor: [f64; 2], pts: &[[f64; 2]], tol_px: f64) -> Option<(Part, f64)> {
    if pts.is_empty() {
        return None;
    }
    // Nearest vertex.
    let mut best_v: Option<(usize, f64)> = None;
    for (i, p) in pts.iter().enumerate() {
        let d = distance_to_point(cursor, *p);
        if d <= tol_px && best_v.map(|(_, bd)| d < bd).unwrap_or(true) {
            best_v = Some((i, d));
        }
    }
    if let Some((i, d)) = best_v {
        return Some((Part::Vertex(i), d));
    }
    // Nearest segment.
    if pts.len() >= 2 {
        let mut best_e: Option<(usize, f64)> = None;
        for i in 0..pts.len() - 1 {
            let d = distance_to_segment(cursor, pts[i], pts[i + 1]);
            if d <= tol_px && best_e.map(|(_, bd)| d < bd).unwrap_or(true) {
                best_e = Some((i, d));
            }
        }
        if let Some((i, d)) = best_e {
            return Some((Part::Edge(i), d));
        }
    }
    None
}

/// Even-odd ray-cast inside test against one closed ring (last vertex implicitly
/// joins the first).
pub fn point_in_ring(p: [f64; 2], ring: &[[f64; 2]]) -> bool {
    if ring.len() < 3 {
        return false;
    }
    let mut inside = false;
    let n = ring.len();
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        let crosses = (a[1] > p[1]) != (b[1] > p[1]);
        if crosses {
            let x_int = a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]);
            if p[0] < x_int {
                inside = !inside;
            }
        }
    }
    inside
}

/// A simple polygon (outer ring + holes): boundary hits (vertices / edges of
/// either ring) resolve to [`Part::Edge`] with the nearest-feature index over the
/// concatenated `[outer, holes…]` vertex stream; otherwise an interior point
/// (inside the outer, outside every hole) resolves to [`Part::Body`].
pub fn hit_polygon(
    cursor: [f64; 2],
    outer: &[[f64; 2]],
    holes: &[[f64; 2]],
    tol_px: f64,
) -> Option<(Part, f64)> {
    hit_polygon_multi(cursor, outer, std::slice::from_ref(&holes.to_vec()), tol_px)
}

/// [`hit_polygon`] with an arbitrary number of interior hole rings. Edge indices
/// are numbered over the concatenated `[outer, holes…]` vertex stream.
pub fn hit_polygon_multi(
    cursor: [f64; 2],
    outer: &[[f64; 2]],
    holes: &[Vec<[f64; 2]>],
    tol_px: f64,
) -> Option<(Part, f64)> {
    // Boundary first: walk every ring's edges (with wrap-around closing).
    let mut best: Option<(usize, f64)> = None;
    let mut base = 0usize;
    for ring in std::iter::once(outer).chain(holes.iter().map(|h| h.as_slice())) {
        let n = ring.len();
        for i in 0..n {
            if n < 2 {
                break;
            }
            let a = ring[i];
            let b = ring[(i + 1) % n];
            let d = distance_to_segment(cursor, a, b);
            if d <= tol_px && best.map(|(_, bd)| d < bd).unwrap_or(true) {
                best = Some((base + i, d));
            }
        }
        base += n;
    }
    if let Some((i, d)) = best {
        return Some((Part::Edge(i), d));
    }
    // Interior (even-odd with holes XOR-ed out).
    if point_in_ring(cursor, outer) && holes.iter().all(|h| !point_in_ring(cursor, h)) {
        return Some((Part::Body, 0.0));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_distance_clamps_to_endpoints() {
        // Horizontal segment (0,0)-(10,0).
        assert!((distance_to_segment([5.0, 3.0], [0.0, 0.0], [10.0, 0.0]) - 3.0).abs() < 1e-9);
        // Beyond the left end → distance to the endpoint (−2, 3).
        let d = distance_to_segment([-2.0, 3.0], [0.0, 0.0], [10.0, 0.0]);
        assert!((d - (13.0f64).sqrt()).abs() < 1e-9, "{d}");
        // Degenerate zero-length segment behaves like a point distance.
        assert!((distance_to_segment([3.0, 4.0], [0.0, 0.0], [0.0, 0.0]) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn point_marker_within_radius_only() {
        assert!(hit_point([2.0, 0.0], [0.0, 0.0], 3.0).is_some());
        assert_eq!(hit_point([2.0, 0.0], [0.0, 0.0], 3.0).unwrap().0, Part::Body);
        assert!(hit_point([4.0, 0.0], [0.0, 0.0], 3.0).is_none());
    }

    #[test]
    fn polyline_prefers_nearest_vertex_then_edge() {
        let pts = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]];
        // Right on vertex 1 → Vertex(1).
        let (part, _) = hit_polyline([10.0, 0.5], &pts, 2.0).unwrap();
        assert_eq!(part, Part::Vertex(1));
        // Midway along edge 0, away from vertices → Edge(0).
        let (part, _) = hit_polyline([5.0, 0.5], &pts, 2.0).unwrap();
        assert_eq!(part, Part::Edge(0));
        // Far from everything → miss.
        assert!(hit_polyline([5.0, 5.0], &pts, 2.0).is_none());
    }

    #[test]
    fn ring_inside_even_odd() {
        let square = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        assert!(point_in_ring([5.0, 5.0], &square));
        assert!(!point_in_ring([15.0, 5.0], &square));
        assert!(!point_in_ring([5.0, 5.0], &[[0.0, 0.0], [1.0, 1.0]]), "open ring");
    }

    #[test]
    fn polygon_edge_before_body_and_hole_excludes() {
        let outer = [[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
        let hole = [[8.0, 8.0], [12.0, 8.0], [12.0, 12.0], [8.0, 12.0]];
        // On the outer boundary → an edge hit.
        let (part, _) = hit_polygon([0.5, 10.0], &outer, &hole, 2.0).unwrap();
        assert!(matches!(part, Part::Edge(_)), "boundary: {part:?}");
        // Deep inside, clear of the hole → Body.
        let (part, d) = hit_polygon([3.0, 3.0], &outer, &hole, 2.0).unwrap();
        assert_eq!(part, Part::Body);
        assert_eq!(d, 0.0);
        // Inside the hole → miss.
        assert!(hit_polygon([10.0, 10.0], &outer, &hole, 1.0).is_none());
        // Fully outside → miss.
        assert!(hit_polygon([30.0, 30.0], &outer, &hole, 2.0).is_none());
    }
}
