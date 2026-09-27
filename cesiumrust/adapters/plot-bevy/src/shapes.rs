//! Geometry → renderable vertex rings, shared by the view sync and the picker so
//! both see exactly the same tessellated silhouette (plan §6 / §7).
//!
//! The parametric primitives (rectangle / circle / ellipse / arc / path) are
//! sampled through the pure [`cesium_plot::geom::sample`]; in the globe view the
//! rings are additionally densified along great circles so a curved boundary
//! hugs the sphere instead of cutting a chord.

use cesium_plot::geo::GeoPoint;
use cesium_plot::geom::sample::{
    arc_ring, circle_ring, ellipse_ring, rectangle_ring, subdivide_great_circle,
    DEFAULT_SEGMENTS, GREAT_CIRCLE_STEP_RAD,
};
use cesium_plot::model::geometry::{Geometry, PathSegment};
use cesium_plot::model::ViewMode;

/// Closed boundary rings (outer + holes) for a *filled face* kind, or `None` for
/// a non-face geometry. Globe faces are great-circle densified.
pub fn face_rings(geo: &Geometry, mode: ViewMode) -> Option<(Vec<GeoPoint>, Vec<Vec<GeoPoint>>)> {
    let (outer, holes) = match geo {
        Geometry::Polygon(p) => (p.outer.clone(), p.holes.clone()),
        Geometry::Rectangle(r) => (rectangle_ring(r), Vec::new()),
        Geometry::Circle(c) => (circle_ring(c, DEFAULT_SEGMENTS), Vec::new()),
        Geometry::Ellipse(e) => (ellipse_ring(e, DEFAULT_SEGMENTS), Vec::new()),
        _ => return None,
    };
    if outer.len() < 3 {
        return None;
    }
    if matches!(mode, ViewMode::Globe) {
        let outer = subdivide_great_circle(&outer, GREAT_CIRCLE_STEP_RAD);
        let holes = holes
            .iter()
            .map(|h| subdivide_great_circle(h, GREAT_CIRCLE_STEP_RAD))
            .collect();
        Some((outer, holes))
    } else {
        Some((outer, holes))
    }
}

/// A single open stroke (vertex chain) for a *line-ish* geometry — polyline, arc
/// or path — or `None` for a face / point / icon / label. Globe strokes are
/// great-circle densified so a long line follows the sphere (plan §6).
pub fn stroke_positions(geo: &Geometry, mode: ViewMode) -> Option<Vec<GeoPoint>> {
    let pts = match geo {
        Geometry::Polyline(pl) => pl.positions.clone(),
        Geometry::Arc(a) => arc_ring(a, DEFAULT_SEGMENTS),
        Geometry::Path(p) => {
            let mut v: Vec<GeoPoint> = Vec::new();
            for seg in &p.segments {
                let ring = match seg {
                    PathSegment::Line(r) => r.clone(),
                    PathSegment::Arc(a) => arc_ring(a, DEFAULT_SEGMENTS),
                };
                for (i, g) in ring.iter().enumerate() {
                    // Drop a shared join vertex so consecutive segments do not
                    // duplicate the touching point.
                    if i == 0 && v.last().copied() == Some(*g) {
                        continue;
                    }
                    v.push(*g);
                }
            }
            v
        }
        _ => return None,
    };
    if pts.len() < 2 {
        return None;
    }
    if matches!(mode, ViewMode::Globe) {
        Some(subdivide_great_circle(&pts, GREAT_CIRCLE_STEP_RAD))
    } else {
        Some(pts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_plot::model::geometry::{Circle, Path, Polyline, Rectangle};

    #[test]
    fn circle_becomes_a_face_ring() {
        let geo = Geometry::Circle(Circle {
            center: GeoPoint::surface(0.0, 0.0),
            radius_m: 50_000.0,
        });
        let (outer, holes) = face_rings(&geo, ViewMode::Flat).unwrap();
        assert_eq!(outer.len(), DEFAULT_SEGMENTS);
        assert!(holes.is_empty());
    }

    #[test]
    fn rectangle_is_a_four_corner_face() {
        let geo = Geometry::Rectangle(Rectangle {
            west: 0.0,
            south: 0.0,
            east: 1.0,
            north: 1.0,
        });
        let (outer, _) = face_rings(&geo, ViewMode::Flat).unwrap();
        assert_eq!(outer.len(), 4);
    }

    #[test]
    fn polyline_is_a_stroke_not_a_face() {
        let geo = Geometry::Polyline(Polyline {
            positions: vec![GeoPoint::surface(0.0, 0.0), GeoPoint::surface(1.0, 1.0)],
        });
        assert!(face_rings(&geo, ViewMode::Flat).is_none());
        assert_eq!(stroke_positions(&geo, ViewMode::Flat).unwrap().len(), 2);
    }

    #[test]
    fn path_dedups_shared_joins() {
        let geo = Geometry::Path(Path {
            segments: vec![
                PathSegment::Line(vec![
                    GeoPoint::surface(0.0, 0.0),
                    GeoPoint::surface(1.0, 0.0),
                ]),
                PathSegment::Line(vec![
                    GeoPoint::surface(1.0, 0.0),
                    GeoPoint::surface(2.0, 0.0),
                ]),
            ],
        });
        let stroke = stroke_positions(&geo, ViewMode::Flat).unwrap();
        // The touching vertex (1,0) appears once → 3 total, not 4.
        assert_eq!(stroke.len(), 3, "{stroke:?}");
    }

    #[test]
    fn globe_strokes_are_densified() {
        let geo = Geometry::Polyline(Polyline {
            positions: vec![GeoPoint::surface(0.0, 0.0), GeoPoint::surface(30.0, 0.0)],
        });
        let flat = stroke_positions(&geo, ViewMode::Flat).unwrap();
        let globe = stroke_positions(&geo, ViewMode::Globe).unwrap();
        assert_eq!(flat.len(), 2);
        assert!(globe.len() > 2, "great-circle adds intermediate vertices");
    }
}
