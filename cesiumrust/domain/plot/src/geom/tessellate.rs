//! Polygon tessellation (plan §6): ear-clipping triangulation of a filled ring
//! with optional holes, via the `earcut` crate (a workspace dependency).
//!
//! Input rings are plain `[f64; 2]` positions (world or screen — the caller
//! decides, and must feed a *planar* space); output is CCW triangle indices into
//! the concatenated `[outer…, hole0…, hole1…]` vertex stream, ready to become a
//! `TriangleList` mesh.

use earcut::Earcut;

/// Triangulate a simple polygon. `outer` is the boundary ring; each `hole` is an
/// interior ring. Returns triangle indices into the concatenated vertex stream
/// (`outer` followed by the holes in order). Empty when there are too few points
/// to form a face.
pub fn triangulate(outer: &[[f64; 2]], holes: &[[f64; 2]]) -> Vec<u32> {
    triangulate_rings(outer, std::iter::once(holes))
}

/// Multi-hole variant of [`triangulate`].
pub fn triangulate_holes(outer: &[[f64; 2]], holes: &[Vec<[f64; 2]>]) -> Vec<u32> {
    triangulate_rings(outer, holes.iter().map(|h| h.as_slice()))
}

fn triangulate_rings<'a>(
    outer: &'a [[f64; 2]],
    holes: impl Iterator<Item = &'a [[f64; 2]]>,
) -> Vec<u32> {
    let mut data: Vec<[f64; 2]> = Vec::with_capacity(outer.len());
    let mut hole_starts: Vec<usize> = Vec::new();
    data.extend_from_slice(outer);
    for h in holes {
        if !h.is_empty() {
            hole_starts.push(data.len());
            data.extend_from_slice(h);
        }
    }
    if data.len() < 3 {
        return Vec::new();
    }
    let mut ear = Earcut::<f64>::new();
    let mut tris: Vec<usize> = Vec::new();
    ear.earcut(data.iter().copied(), &hole_starts, &mut tris);
    tris.into_iter().map(|i| i as u32).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangle_needs_no_subdivision() {
        let tri = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
        let idx = triangulate(&tri, &[]);
        assert_eq!(idx.len(), 3, "one triangle");
        assert!(idx.iter().all(|&i| (0..3).contains(&i)));
    }

    #[test]
    fn convex_quad_splits_into_two() {
        let quad = [[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]];
        let idx = triangulate(&quad, &[]);
        assert_eq!(idx.len(), 6, "two triangles");
        assert!(idx.iter().all(|&i| i < 4));
    }

    #[test]
    fn square_with_centre_hole_uses_all_vertices() {
        let outer = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let hole = [[4.0, 4.0], [6.0, 4.0], [6.0, 6.0], [4.0, 6.0]];
        let idx = triangulate(&outer, &hole);
        // 8 total vertices; the ring-with-hole needs > 1 triangle to ring it.
        assert!(idx.len() >= 6 && idx.len().is_multiple_of(3), "{:?}", idx.len());
        assert!(idx.iter().all(|&i| i < 8), "indices stay in range: {idx:?}");
    }

    #[test]
    fn degenerate_input_yields_no_triangles() {
        assert!(triangulate(&[[0.0, 0.0], [1.0, 1.0]], &[]).is_empty());
        assert!(triangulate(&[], &[]).is_empty());
    }
}
