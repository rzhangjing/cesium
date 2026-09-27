//! Coordinate snapping (plan §16 M9 "吸附").
//!
//! Snapping is a **pure**, view-independent fold over the document: given a raw
//! geographic point (the cursor resolved through `screen_to_geo`) it returns an
//! adjusted point that either lands on a regular lat/lon grid, latches onto a
//! nearby existing **vertex**, or projects onto a nearby **edge** (segment) of an
//! existing geometry. The bridge calls [`snap`] with a [`SnapConfig`] and applies
//! the result before the draft vertex is committed; the config defaults to
//! *disabled* so existing behaviour is unchanged until the user turns it on.
//!
//! Thresholds are geographic: vertex / edge distances are measured with
//! [`GeoPoint::surface_distance`] (metres), grid steps in degrees — enough for
//! authoring and fully deterministic for headless tests.

use crate::geo::GeoPoint;
use crate::model::document::Document;
use crate::model::geometry::{Geometry, PathSegment};
use crate::model::ids::ElementId;

/// Where a snapped coordinate came from (kept for preview / status display).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SnapResult {
    /// Nothing matched — the original point is returned unchanged.
    None(GeoPoint),
    /// Latched onto an existing vertex of another element.
    Vertex(GeoPoint),
    /// Projected onto an edge (segment) of another element.
    Edge(GeoPoint),
    /// Rounded onto the snapping grid.
    Grid(GeoPoint),
}

impl SnapResult {
    /// The (possibly adjusted) coordinate.
    pub fn point(&self) -> GeoPoint {
        match self {
            SnapResult::None(p)
            | SnapResult::Vertex(p)
            | SnapResult::Edge(p)
            | SnapResult::Grid(p) => *p,
        }
    }

    /// Whether any adjustment was applied.
    pub fn snapped(&self) -> bool {
        !matches!(self, SnapResult::None(_))
    }
}

/// Tunables for a snapping pass. The `Default` is **off** so wiring it into the
/// interaction FSM never changes existing draw behaviour until explicitly enabled.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapConfig {
    /// Master switch. When `false`, [`snap`] always returns [`SnapResult::None`].
    pub enabled: bool,
    /// Snap to existing vertices within this radius (metres). `0` disables.
    pub vertex_threshold_m: f64,
    /// Snap to existing edges within this radius (metres). `0` disables.
    pub edge_threshold_m: f64,
    /// Regular lat/lon grid step (degrees). `None` disables grid snapping.
    pub grid_step_deg: Option<f64>,
}

impl Default for SnapConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            vertex_threshold_m: 0.0,
            edge_threshold_m: 0.0,
            grid_step_deg: None,
        }
    }
}

/// Round a coordinate onto a regular `step_deg` lat/lon grid. A non-positive or
/// non-finite step is a no-op.
pub fn snap_to_grid(p: GeoPoint, step_deg: f64) -> GeoPoint {
    if !step_deg.is_finite() || step_deg <= 0.0 {
        return p;
    }
    let lon = (p.lon_deg / step_deg).round() * step_deg;
    let lat = (p.lat_deg / step_deg).round() * step_deg;
    GeoPoint::new(lon, lat, p.height_m)
}

/// The nearest existing vertex of any *other* element within `threshold_m`, if any.
/// `exclude` skips the element currently being edited (so a moving vertex does
/// not latch onto itself).
pub fn snap_to_vertex(
    doc: &Document,
    p: GeoPoint,
    threshold_m: f64,
    exclude: Option<ElementId>,
) -> Option<GeoPoint> {
    if threshold_m <= 0.0 {
        return None;
    }
    let mut best: Option<(f64, GeoPoint)> = None;
    for element in doc.elements() {
        if Some(element.id) == exclude {
            continue;
        }
        for v in element.geometry.vertices() {
            let d = p.surface_distance(v);
            if d <= threshold_m && best.map(|(bd, _)| d < bd).unwrap_or(true) {
                best = Some((d, v));
            }
        }
    }
    best.map(|(_, v)| v)
}

/// Every line segment of a geometry, as consecutive vertex pairs. Rings are
/// treated as closed (last → first). Point-like geometries contribute none.
fn segments(g: &Geometry) -> Vec<(GeoPoint, GeoPoint)> {
    fn chain(pts: &[GeoPoint], closed: bool, out: &mut Vec<(GeoPoint, GeoPoint)>) {
        for w in pts.windows(2) {
            out.push((w[0], w[1]));
        }
        if closed && pts.len() >= 3 {
            out.push((pts[pts.len() - 1], pts[0]));
        }
    }
    let mut out = Vec::new();
    match g {
        Geometry::Polyline(pl) => chain(&pl.positions, false, &mut out),
        Geometry::Polygon(pg) => {
            chain(&pg.outer, true, &mut out);
            for h in &pg.holes {
                chain(h, true, &mut out);
            }
        }
        Geometry::Rectangle(r) => {
            let corners = [
                GeoPoint::surface(r.west, r.south),
                GeoPoint::surface(r.east, r.south),
                GeoPoint::surface(r.east, r.north),
                GeoPoint::surface(r.west, r.north),
            ];
            chain(&corners, true, &mut out);
        }
        Geometry::Path(p) => {
            for s in &p.segments {
                match s {
                    PathSegment::Line(r) => chain(r, false, &mut out),
                    PathSegment::Arc(a) => {
                        out.push((a.start, a.center));
                        out.push((a.center, a.end));
                    }
                }
            }
        }
        Geometry::Composite(c) => {
            for part in &c.parts {
                out.extend(segments(part));
            }
        }
        _ => {}
    }
    out
}

/// The closest point on segment `a`–`b` to `p` (planar in lon/lat degrees, a fine
/// approximation at authoring scales).
fn closest_on_segment(a: GeoPoint, b: GeoPoint, p: GeoPoint) -> GeoPoint {
    let dx = b.lon_deg - a.lon_deg;
    let dy = b.lat_deg - a.lat_deg;
    let len2 = dx * dx + dy * dy;
    if len2 <= 1e-24 {
        return a;
    }
    let t = (((p.lon_deg - a.lon_deg) * dx + (p.lat_deg - a.lat_deg) * dy) / len2).clamp(0.0, 1.0);
    GeoPoint::surface(a.lon_deg + t * dx, a.lat_deg + t * dy)
}

/// The nearest point on any existing *edge* within `threshold_m`.
pub fn snap_to_edge(
    doc: &Document,
    p: GeoPoint,
    threshold_m: f64,
    exclude: Option<ElementId>,
) -> Option<GeoPoint> {
    if threshold_m <= 0.0 {
        return None;
    }
    let mut best: Option<(f64, GeoPoint)> = None;
    for element in doc.elements() {
        if Some(element.id) == exclude {
            continue;
        }
        for (a, b) in segments(&element.geometry) {
            let c = closest_on_segment(a, b, p);
            let d = p.surface_distance(c);
            if d <= threshold_m && best.map(|(bd, _)| d < bd).unwrap_or(true) {
                best = Some((d, c));
            }
        }
    }
    best.map(|(_, c)| c)
}

/// Snap a raw coordinate: **vertex → edge → grid** priority, each gate optional.
/// Disabled config (the default) returns the point untouched.
pub fn snap(
    doc: &Document,
    p: GeoPoint,
    cfg: &SnapConfig,
    exclude: Option<ElementId>,
) -> SnapResult {
    if !cfg.enabled {
        return SnapResult::None(p);
    }
    if let Some(v) = snap_to_vertex(doc, p, cfg.vertex_threshold_m, exclude) {
        return SnapResult::Vertex(v);
    }
    if let Some(e) = snap_to_edge(doc, p, cfg.edge_threshold_m, exclude) {
        return SnapResult::Edge(e);
    }
    if let Some(step) = cfg.grid_step_deg {
        let g = snap_to_grid(p, step);
        if g != p {
            return SnapResult::Grid(g);
        }
    }
    SnapResult::None(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::geometry::{Polygon, Polyline, Rectangle};
    use crate::model::Document;

    fn p(lon: f64, lat: f64) -> GeoPoint {
        GeoPoint::surface(lon, lat)
    }

    fn doc() -> Document {
        let mut d = Document::default();
        let l = d.new_layer("L");
        // A polyline with an explicit vertex at (1.0, 1.0).
        let line = d.make_element(
            "line",
            Geometry::Polyline(Polyline {
                positions: vec![p(0.0, 0.0), p(1.0, 1.0), p(2.0, 0.0)],
            }),
        );
        d.add_element_to_layer(l, line);
        // A rectangle far away (its corners are also vertices).
        let rect = d.make_element(
            "rect",
            Geometry::Rectangle(Rectangle {
                west: 10.0,
                south: 10.0,
                east: 11.0,
                north: 11.0,
            }),
        );
        d.add_element_to_layer(l, rect);
        d
    }

    #[test]
    fn disabled_by_default_is_a_noop() {
        let d = doc();
        let raw = p(1.0001, 1.0001);
        let r = snap(&d, raw, &SnapConfig::default(), None);
        assert_eq!(r, SnapResult::None(raw));
        assert!(!r.snapped());
    }

    #[test]
    fn vertex_snap_latches_within_threshold() {
        let d = doc();
        let cfg = SnapConfig {
            enabled: true,
            vertex_threshold_m: 20_000.0, // ~0.18°
            ..Default::default()
        };
        let near = p(1.0005, 1.0005); // just off the (1,1) vertex
        match snap(&d, near, &cfg, None) {
            SnapResult::Vertex(v) => {
                assert!((v.lon_deg - 1.0).abs() < 1e-9 && (v.lat_deg - 1.0).abs() < 1e-9);
            }
            other => panic!("expected vertex snap, got {other:?}"),
        }
    }

    #[test]
    fn vertex_snap_respects_exclude() {
        let d = doc();
        // find the polyline id by name
        let line = d
            .elements()
            .find(|e| e.name == "line")
            .unwrap()
            .id;
        let cfg = SnapConfig {
            enabled: true,
            vertex_threshold_m: 20_000.0,
            edge_threshold_m: 20_000.0,
            grid_step_deg: None,
        };
        // Excluding every element → nothing to snap to (falls through to None).
        let near = p(1.0005, 1.0005);
        let r = snap(&d, near, &cfg, Some(line));
        // The rectangle is far, so edge/vertex from the line is suppressed too.
        assert!(matches!(r, SnapResult::None(_)), "exclude should drop the line, got {r:?}");
    }

    #[test]
    fn edge_snap_projects_onto_segment() {
        let d = doc();
        let cfg = SnapConfig {
            enabled: true,
            // vertex radius tight so a mid-edge point prefers the edge,
            edge_threshold_m: 15_000.0,
            ..Default::default()
        };
        // Point near the middle of the (0,0)-(1,1) segment but off it, and far
        // from any vertex.
        let off = p(0.5, 0.45);
        match snap(&d, off, &cfg, None) {
            SnapResult::Edge(c) => {
                // Projection onto the diagonal keeps lon ≈ lat.
                assert!((c.lon_deg - c.lat_deg).abs() < 1e-6, "{c:?}");
                assert!(c.lon_deg > 0.3 && c.lon_deg < 0.7, "{c:?}");
            }
            other => panic!("expected edge snap, got {other:?}"),
        }
    }

    #[test]
    fn grid_snap_rounds_to_step() {
        assert_eq!(snap_to_grid(p(1.03, 2.98), 0.5), p(1.0, 3.0));
        assert_eq!(snap_to_grid(p(-1.24, 45.6), 1.0), p(-1.0, 46.0));
        // Non-positive step is a no-op.
        assert_eq!(snap_to_grid(p(1.03, 2.98), 0.0), p(1.03, 2.98));
    }

    #[test]
    fn grid_used_when_nothing_near() {
        let d = doc();
        let cfg = SnapConfig {
            enabled: true,
            vertex_threshold_m: 100.0,
            edge_threshold_m: 100.0,
            grid_step_deg: Some(0.5),
        };
        // Far from geometry, off-grid.
        let far = p(50.13, -30.44);
        match snap(&d, far, &cfg, None) {
            SnapResult::Grid(g) => assert_eq!(g, p(50.0, -30.5)),
            other => panic!("expected grid snap, got {other:?}"),
        }
    }

    #[test]
    fn closed_polygon_rings_yield_closing_segment() {
        let mut d = Document::default();
        let l = d.new_layer("L");
        let poly = d.make_element(
            "tri",
            Geometry::Polygon(Polygon {
                outer: vec![p(0.0, 0.0), p(2.0, 0.0), p(1.0, 2.0)],
                holes: vec![],
            }),
        );
        let poly_id = poly.id;
        d.add_element_to_layer(l, poly);
        // A triangle: 2 consecutive + 1 closing = 3 edges (rings are closed).
        let segs = segments(&d.element(poly_id).unwrap().geometry);
        assert_eq!(segs.len(), 3);
    }
}
