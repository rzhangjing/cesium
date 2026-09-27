//! The geometric variants a plot element can take.
//!
//! Every variant stores only [`GeoPoint`]s / metres — the geographic source of
//! truth (plan §3/§5). Rasterising to vertices and projecting into world space
//! happens later (`geom` sampling + the render bridge), never here.
//! [`Geometry::Composite`] is the reserved extension slot for combined /
//! military-symbol geometries: M1 only guarantees the basic types are modelled,
//! tree/IO/visibility round-trip through Composite but nothing samples it yet.

use serde::{Deserialize, Serialize};

use crate::geo::{GeoBounds, GeoPoint};

/// A closed ring: at least 3 points, first == last implied (not stored twice).
pub type Ring = Vec<GeoPoint>;

/// Where a label sits relative to its anchor coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LabelAnchor {
    #[default]
    Center,
    Left,
    Right,
    Top,
    Bottom,
}

/// An icon reference: which symbol + the anchor it is placed at.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IconGeometry {
    /// Geographic anchor.
    pub at: GeoPoint,
    /// Registry key selecting the icon image (resolved by the bridge).
    pub key: String,
}

/// A text label anchored at a coordinate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LabelGeometry {
    /// Geographic anchor.
    pub at: GeoPoint,
    /// Text to render (ASCII under the current bundled font).
    pub text: String,
    /// Anchor alignment.
    pub anchor: LabelAnchor,
    /// Pixel offset from the projected anchor (x right, y up).
    pub offset_px: [f32; 2],
}

/// A connected open line of >= 2 vertices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Polyline {
    pub positions: Vec<GeoPoint>,
}

/// A simple polygon: one outer ring + optional holes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Polygon {
    pub outer: Ring,
    pub holes: Vec<Ring>,
}

/// A lat/lon axis-aligned rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rectangle {
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
}

/// A ground circle: centre + radius in metres (sampled later).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Circle {
    pub center: GeoPoint,
    pub radius_m: f64,
}

/// A ground ellipse: centre, semi-axes (metres) and bearing (degrees, clockwise
/// from north). Sampled later.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Ellipse {
    pub center: GeoPoint,
    pub semi_major_m: f64,
    pub semi_minor_m: f64,
    pub rotation_deg: f64,
}

/// A circular arc through three points: start → (via) center → end.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Arc3 {
    pub start: GeoPoint,
    pub center: GeoPoint,
    pub end: GeoPoint,
}

/// One segment of a [`Path`] (mixed primitive pieces stitched together).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PathSegment {
    Line(Ring),
    Arc(Arc3),
}

/// A compound path of mixed straight / arc segments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Path {
    pub segments: Vec<PathSegment>,
}

/// Reserved classifier for combined / military symbol kinds (M9+).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SymbolKind {
    /// Untyped composite — just draws its parts.
    #[default]
    Generic,
}

/// A geometry built from other geometries (extension slot for symbol library).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Composite {
    pub kind: SymbolKind,
    pub parts: Vec<Geometry>,
}

impl Composite {
    /// Assemble a composite symbol from a classifier and its constituent
    /// geometries. This is the reserved M9+ military-symbol integration point:
    /// a symbol library resolves a `SymbolKind` to a `Vec<Geometry>` and folds
    /// them here; everything downstream (tree / IO / visibility / sampling) then
    /// treats the symbol as a single geometry whose [`vertices`](Geometry::vertices)
    /// and bounds are the union of its parts.
    pub fn new(kind: SymbolKind, parts: Vec<Geometry>) -> Self {
        Self { kind, parts }
    }

    /// Number of constituent parts.
    pub fn len(&self) -> usize {
        self.parts.len()
    }

    /// Whether the composite holds no parts.
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }
}

/// The full geometry union.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Geometry {
    Point(GeoPoint),
    Icon(IconGeometry),
    Label(LabelGeometry),
    Polyline(Polyline),
    Polygon(Polygon),
    Rectangle(Rectangle),
    Circle(Circle),
    Ellipse(Ellipse),
    Arc(Arc3),
    Path(Path),
    Composite(Composite),
}

/// Coarse class used by the type-dimension of the visibility filters (§10.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GeometryKind {
    Point,
    Icon,
    Label,
    Line,
    Polygon,
    Rectangle,
    Circle,
    Ellipse,
    Arc,
    Path,
    Composite,
}

impl GeometryKind {
    /// Every kind, in a stable display order — the type-dimension filter panel
    /// (plan §10.5 / §9) offers one toggle per entry.
    pub const ALL: [GeometryKind; 11] = [
        GeometryKind::Point,
        GeometryKind::Icon,
        GeometryKind::Label,
        GeometryKind::Line,
        GeometryKind::Polygon,
        GeometryKind::Rectangle,
        GeometryKind::Circle,
        GeometryKind::Ellipse,
        GeometryKind::Arc,
        GeometryKind::Path,
        GeometryKind::Composite,
    ];

    /// The full list (see [`GeometryKind::ALL`]).
    pub fn all() -> &'static [GeometryKind] {
        &Self::ALL
    }
}

impl Geometry {
    /// The visibility-filter class of this geometry.
    pub fn kind(&self) -> GeometryKind {
        match self {
            Geometry::Point(_) => GeometryKind::Point,
            Geometry::Icon(_) => GeometryKind::Icon,
            Geometry::Label(_) => GeometryKind::Label,
            Geometry::Polyline(_) => GeometryKind::Line,
            Geometry::Polygon(_) => GeometryKind::Polygon,
            Geometry::Rectangle(_) => GeometryKind::Rectangle,
            Geometry::Circle(_) => GeometryKind::Circle,
            Geometry::Ellipse(_) => GeometryKind::Ellipse,
            Geometry::Arc(_) => GeometryKind::Arc,
            Geometry::Path(_) => GeometryKind::Path,
            Geometry::Composite(_) => GeometryKind::Composite,
        }
    }

    /// Every stored vertex, used for a conservative bounds. Circles / ellipses
    /// only contribute their centre here; the sampling pass (`geom`, M4) is
    /// responsible for the true extent, so the cached element bounds for those
    /// are a centre proxy until then.
    pub fn vertices(&self) -> Vec<GeoPoint> {
        match self {
            Geometry::Point(p) => vec![*p],
            Geometry::Icon(i) => vec![i.at],
            Geometry::Label(l) => vec![l.at],
            Geometry::Polyline(pl) => pl.positions.clone(),
            Geometry::Polygon(pg) => {
                let mut v = pg.outer.clone();
                for h in &pg.holes {
                    v.extend_from_slice(h);
                }
                v
            }
            Geometry::Rectangle(r) => vec![
                GeoPoint::surface(r.west, r.south),
                GeoPoint::surface(r.east, r.north),
            ],
            Geometry::Circle(c) => vec![c.center],
            Geometry::Ellipse(e) => vec![e.center],
            Geometry::Arc(a) => vec![a.start, a.center, a.end],
            Geometry::Path(p) => p
                .segments
                .iter()
                .flat_map(|s| match s {
                    PathSegment::Line(r) => r.clone(),
                    PathSegment::Arc(a) => vec![a.start, a.center, a.end],
                })
                .collect(),
            Geometry::Composite(c) => c.parts.iter().flat_map(|g| g.vertices()).collect(),
        }
    }

    /// Conservative geographic bounds that provably contain every vertex the
    /// sampler ([`crate::geom::sample`]) — and therefore the renderer and the
    /// picker via `shapes::{stroke_positions, face_rings}` — can produce for this
    /// geometry. It is deliberately the *globe* (great-circle densified) extent,
    /// which is a superset of the flat extent, so the same box is a safe
    /// broad-phase reject in both view modes:
    ///  * polylines / polygon + hole edges / rectangle edges / path lines are
    ///    subdivided along their great circles, capturing the poleward bulge a
    ///    sparse control-point box would miss;
    ///  * circles / ellipses / arcs use their sampled ring, which already sits at
    ///    the true outer extent (further subdivision only pulls inward);
    ///  * point-like kinds reduce to their anchor (the picker adds pixel slack).
    ///
    /// Computed once per edit ([`Element::set_geometry`](crate::model::element::Element::set_geometry)),
    /// never on the render / pick hot path.
    pub fn bounds(&self) -> GeoBounds {
        use crate::geom::sample::{
            arc_ring, circle_ring, ellipse_ring, rectangle_ring, subdivide_great_circle,
            DEFAULT_SEGMENTS, GREAT_CIRCLE_STEP_RAD,
        };
        match self {
            Geometry::Point(p) => GeoBounds::from_point(*p),
            Geometry::Icon(i) => GeoBounds::from_point(i.at),
            Geometry::Label(l) => GeoBounds::from_point(l.at),
            Geometry::Polyline(pl) => {
                let sub = subdivide_great_circle(&pl.positions, GREAT_CIRCLE_STEP_RAD);
                GeoBounds::from_points(&sub)
            }
            Geometry::Polygon(pg) => {
                let mut acc = GeoBounds::from_points(&subdivide_closed(&pg.outer, GREAT_CIRCLE_STEP_RAD));
                for h in &pg.holes {
                    acc = acc.union(GeoBounds::from_points(&subdivide_closed(h, GREAT_CIRCLE_STEP_RAD)));
                }
                acc
            }
            Geometry::Rectangle(r) => {
                let ring = rectangle_ring(r);
                GeoBounds::from_points(&subdivide_closed(&ring, GREAT_CIRCLE_STEP_RAD))
            }
            Geometry::Circle(c) => GeoBounds::from_points(&circle_ring(c, DEFAULT_SEGMENTS)),
            Geometry::Ellipse(e) => GeoBounds::from_points(&ellipse_ring(e, DEFAULT_SEGMENTS)),
            Geometry::Arc(a) => GeoBounds::from_points(&arc_ring(a, DEFAULT_SEGMENTS)),
            Geometry::Path(p) => {
                let mut acc = GeoBounds::empty();
                for seg in &p.segments {
                    let pts = match seg {
                        PathSegment::Line(r) => subdivide_great_circle(r, GREAT_CIRCLE_STEP_RAD),
                        PathSegment::Arc(a) => arc_ring(a, DEFAULT_SEGMENTS),
                    };
                    acc = acc.union(GeoBounds::from_points(&pts));
                }
                acc
            }
            Geometry::Composite(c) => {
                let mut acc = GeoBounds::empty();
                for part in &c.parts {
                    acc = acc.union(part.bounds());
                }
                acc
            }
        }
    }

    /// A single representative anchor coordinate for point-like geometries
    /// (used by label / icon placement and single-vertex moves).
    pub fn anchor(&self) -> Option<GeoPoint> {
        match self {
            Geometry::Point(p) => Some(*p),
            Geometry::Icon(i) => Some(i.at),
            Geometry::Label(l) => Some(l.at),
            Geometry::Circle(c) => Some(c.center),
            Geometry::Ellipse(e) => Some(e.center),
            _ => None,
        }
    }
}

/// Close a ring (repeat the first vertex at the end) then densify it along great
/// circles, so the closing edge's poleward bulge is included too. Falls back to
/// the raw slice for a ring too short to have an edge.
fn subdivide_closed(ring: &[GeoPoint], step_rad: f64) -> Vec<GeoPoint> {
    use crate::geom::sample::subdivide_great_circle;
    if ring.len() < 2 {
        return ring.to_vec();
    }
    let mut closed = ring.to_vec();
    closed.push(ring[0]);
    subdivide_great_circle(&closed, step_rad)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_matches_variant() {
        assert_eq!(Geometry::Point(GeoPoint::surface(0.0, 0.0)).kind(), GeometryKind::Point);
        assert_eq!(
            Geometry::Polyline(Polyline { positions: vec![GeoPoint::surface(1.0, 1.0), GeoPoint::surface(2.0, 2.0)] }).kind(),
            GeometryKind::Line
        );
        let comp = Geometry::Composite(Composite {
            kind: SymbolKind::Generic,
            parts: vec![Geometry::Point(GeoPoint::surface(0.0, 0.0))],
        });
        assert_eq!(comp.kind(), GeometryKind::Composite);
    }

    #[test]
    fn vertices_and_bounds() {
        let rect = Geometry::Rectangle(Rectangle { west: -10.0, south: -20.0, east: 30.0, north: 40.0 });
        let b = rect.bounds();
        // The E/W sides are meridians (longitude preserved exactly); the N/S
        // sides are constant-latitude chords whose great circle bulges poleward,
        // so the conservative box contains the corners and grows north of 40°.
        assert!(
            b.west_deg <= -10.0 && b.east_deg >= 30.0 && b.south_deg <= -20.0 && b.north_deg >= 40.0,
            "corners must be contained: {b:?}",
        );
        assert!(b.north_deg > 40.0 && b.north_deg < 50.0, "bulge must be captured, not absurd: {b:?}");

        let poly = Geometry::Polygon(Polygon {
            outer: vec![GeoPoint::surface(0.0, 0.0), GeoPoint::surface(1.0, 0.0), GeoPoint::surface(0.0, 1.0)],
            holes: vec![vec![GeoPoint::surface(5.0, 5.0)]],
        });
        // Holes widen the conservative bounds too.
        assert_eq!(poly.vertices().len(), 4);
        assert_eq!(poly.bounds().east_deg, 5.0);
    }

    #[test]
    fn bounds_covers_every_sampled_vertex() {
        use crate::geom::sample::{
            circle_ring, ellipse_ring, subdivide_great_circle, DEFAULT_SEGMENTS,
            GREAT_CIRCLE_STEP_RAD,
        };
        // A long high-latitude east-west line: its densified great circle bulges
        // poleward past the 60°N control points, and bounds must cover every
        // vertex the renderer / picker will actually test.
        let line = Geometry::Polyline(Polyline {
            positions: vec![GeoPoint::surface(0.0, 60.0), GeoPoint::surface(60.0, 60.0)],
        });
        let b = line.bounds();
        let dens = subdivide_great_circle(&line.vertices(), GREAT_CIRCLE_STEP_RAD);
        assert!(dens.iter().all(|p| b.contains(*p)), "bulge vertex outside bounds");
        assert!(b.north_deg > 61.0, "must capture the poleward bulge, got {b:?}");

        // A circle: bounds must span the whole sampled disc, not collapse to the
        // centre the way the old control-vertex box did.
        let c = Circle { center: GeoPoint::surface(10.0, 50.0), radius_m: 500_000.0 };
        let cb = Geometry::Circle(c).bounds();
        assert!(cb.width_deg() > 1.0 && cb.height_deg() > 1.0, "circle bounds degenerate: {cb:?}");
        assert!(circle_ring(&c, DEFAULT_SEGMENTS).iter().all(|p| cb.contains(*p)));

        // Likewise an oriented ellipse.
        let e = Ellipse {
            center: GeoPoint::surface(0.0, 0.0),
            semi_major_m: 800_000.0,
            semi_minor_m: 300_000.0,
            rotation_deg: 30.0,
        };
        let eb = Geometry::Ellipse(e).bounds();
        assert!(ellipse_ring(&e, DEFAULT_SEGMENTS).iter().all(|p| eb.contains(*p)), "ellipse ring outside bounds");
    }

    #[test]
    fn serde_roundtrip_preserves_variant() {
        let g = Geometry::Circle(Circle { center: GeoPoint::new(10.0, 20.0, 300.0), radius_m: 1234.5 });
        let s = serde_json::to_string(&g).unwrap();
        let back: Geometry = serde_json::from_str(&s).unwrap();
        assert_eq!(g, back);
    }
}
