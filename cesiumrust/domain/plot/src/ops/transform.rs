//! Pure geometry transforms (plan §8): the vertex-level edits an element can
//! undergo — translate, rotate about a pivot, scale about a pivot, and replace a
//! single vertex. All operate in lon/lat space so they are view-mode independent
//! (rotate / scale are the 2D planar approximation the plan allows for element
//! transforms; the true extent of circles / ellipses is rescaled analytically).

use crate::geo::GeoPoint;
use crate::model::geometry::{Geometry, PathSegment};

/// Shift every stored vertex of `geo` by `(dlon, dlat)` degrees.
pub fn translate(geo: &Geometry, dlon: f64, dlat: f64) -> Geometry {
    let mut out = geo.clone();
    map_vertices_in_place(&mut out, &mut |p| {
        p.lon_deg += dlon;
        p.lat_deg += dlat;
    });
    out
}

/// Rotate every vertex about `pivot` (lon/lat) by `deg` degrees (CCW in the
/// planar lon/lat frame). Parametric faces keep their size, only re-anchoring
/// their defining centre.
pub fn rotate(geo: &Geometry, pivot: (f64, f64), deg: f64) -> Geometry {
    let rad = deg.to_radians();
    let (cos, sin) = (rad.cos(), rad.sin());
    let mut out = geo.clone();
    map_vertices_in_place(&mut out, &mut |p| {
        let dx = p.lon_deg - pivot.0;
        let dy = p.lat_deg - pivot.1;
        p.lon_deg = pivot.0 + dx * cos - dy * sin;
        p.lat_deg = pivot.1 + dx * sin + dy * cos;
    });
    out
}

/// Scale every vertex's distance from `pivot` by `factor`. Radii / semi-axes of
/// parametric faces scale by the same factor.
pub fn scale(geo: &Geometry, pivot: (f64, f64), factor: f64) -> Geometry {
    let mut out = geo.clone();
    map_vertices_in_place(&mut out, &mut |p| {
        p.lon_deg = pivot.0 + (p.lon_deg - pivot.0) * factor;
        p.lat_deg = pivot.1 + (p.lat_deg - pivot.1) * factor;
    });
    match &mut out {
        Geometry::Circle(c) => c.radius_m *= factor,
        Geometry::Ellipse(e) => {
            e.semi_major_m *= factor;
            e.semi_minor_m *= factor;
        }
        _ => {}
    }
    out
}

/// Replace the `index`-th stored vertex (ordering follows [`Geometry::vertices`])
/// with `at`. Returns `None` when the index is out of range or the geometry has
/// no addressable vertices.
pub fn set_vertex(geo: &Geometry, index: usize, at: GeoPoint) -> Option<Geometry> {
    let mut out = geo.clone();
    let mut next = 0usize;
    let mut try_take = move || {
        let i = next;
        next += 1;
        i
    };
    match &mut out {
        Geometry::Point(p) => {
            if try_take() == index {
                *p = at;
                Some(out)
            } else {
                None
            }
        }
        Geometry::Icon(ic) => {
            if try_take() == index {
                ic.at = at;
                Some(out)
            } else {
                None
            }
        }
        Geometry::Label(l) => {
            if try_take() == index {
                l.at = at;
                Some(out)
            } else {
                None
            }
        }
        Geometry::Polyline(pl) => {
            if index < pl.positions.len() {
                pl.positions[index] = at;
                Some(out)
            } else {
                None
            }
        }
        Geometry::Polygon(pg) => set_in_rings(&mut pg.outer, &mut pg.holes, index, at).then_some(out),
        Geometry::Rectangle(_) | Geometry::Composite(_) => None,
        Geometry::Circle(c) => {
            if index == 0 {
                c.center = at;
                Some(out)
            } else {
                None
            }
        }
        Geometry::Ellipse(e) => {
            if index == 0 {
                e.center = at;
                Some(out)
            } else {
                None
            }
        }
        Geometry::Arc(a) => match index {
            0 => {
                a.start = at;
                Some(out)
            }
            1 => {
                a.center = at;
                Some(out)
            }
            2 => {
                a.end = at;
                Some(out)
            }
            _ => None,
        },
        Geometry::Path(p) => {
            let mut cur = 0usize;
            for seg in &mut p.segments {
                let ring_len = match seg {
                    PathSegment::Line(r) => r.len(),
                    PathSegment::Arc(_) => 3,
                };
                if index < cur + ring_len {
                    let local = index - cur;
                    match seg {
                        PathSegment::Line(r) => r[local] = at,
                        PathSegment::Arc(a) => match local {
                            0 => a.start = at,
                            1 => a.center = at,
                            _ => a.end = at,
                        },
                    }
                    return Some(out);
                }
                cur += ring_len;
            }
            None
        }
    }
}

/// Apply `f` to every stored vertex, in the same order [`Geometry::vertices`]
/// yields them. Circles / ellipses expose only their centre.
fn map_vertices_in_place(geo: &mut Geometry, f: &mut dyn FnMut(&mut GeoPoint)) {
    match geo {
        Geometry::Point(p) => f(p),
        Geometry::Icon(i) => f(&mut i.at),
        Geometry::Label(l) => f(&mut l.at),
        Geometry::Polyline(pl) => pl.positions.iter_mut().for_each(&mut *f),
        Geometry::Polygon(pg) => {
            pg.outer.iter_mut().for_each(&mut *f);
            for h in &mut pg.holes {
                h.iter_mut().for_each(&mut *f);
            }
        }
        Geometry::Rectangle(r) => {
            let mut sw = GeoPoint::surface(r.west, r.south);
            let mut en = GeoPoint::surface(r.east, r.north);
            f(&mut sw);
            f(&mut en);
            r.west = sw.lon_deg;
            r.south = sw.lat_deg;
            r.east = en.lon_deg;
            r.north = en.lat_deg;
        }
        Geometry::Circle(c) => f(&mut c.center),
        Geometry::Ellipse(e) => f(&mut e.center),
        Geometry::Arc(a) => {
            f(&mut a.start);
            f(&mut a.center);
            f(&mut a.end);
        }
        Geometry::Path(p) => {
            for seg in &mut p.segments {
                match seg {
                    PathSegment::Line(r) => r.iter_mut().for_each(&mut *f),
                    PathSegment::Arc(a) => {
                        f(&mut a.start);
                        f(&mut a.center);
                        f(&mut a.end);
                    }
                }
            }
        }
        Geometry::Composite(c) => {
            for part in &mut c.parts {
                map_vertices_in_place(part, &mut *f);
            }
        }
    }
}

/// Set the flattened-index vertex across an outer ring + holes.
fn set_in_rings(outer: &mut [GeoPoint], holes: &mut [Vec<GeoPoint>], index: usize, at: GeoPoint) -> bool {
    if index < outer.len() {
        outer[index] = at;
        return true;
    }
    let mut cur = outer.len();
    for h in holes {
        if index < cur + h.len() {
            h[index - cur] = at;
            return true;
        }
        cur += h.len();
    }
    false
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::geometry::{Arc3, Circle, Polygon, Polyline, Rectangle};

    fn p(lon: f64, lat: f64) -> GeoPoint {
        GeoPoint::surface(lon, lat)
    }

    #[test]
    fn translate_shifts_every_vertex() {
        let g = Geometry::Polyline(Polyline {
            positions: vec![p(0.0, 0.0), p(1.0, 2.0)],
        });
        let t = translate(&g, 10.0, -5.0);
        let vs = t.vertices();
        assert_eq!((vs[0].lon_deg, vs[0].lat_deg), (10.0, -5.0));
        assert_eq!((vs[1].lon_deg, vs[1].lat_deg), (11.0, -3.0));
    }

    #[test]
    fn rectangle_translation_moves_corners() {
        let g = Geometry::Rectangle(Rectangle {
            west: 0.0,
            south: 0.0,
            east: 2.0,
            north: 3.0,
        });
        match translate(&g, 1.0, 1.0) {
            Geometry::Rectangle(r) => {
                assert_eq!((r.west, r.south, r.east, r.north), (1.0, 1.0, 3.0, 4.0))
            }
            _ => panic!("stays a rectangle"),
        }
    }

    #[test]
    fn rotate_90_about_origin() {
        let g = Geometry::Point(p(1.0, 0.0));
        match rotate(&g, (0.0, 0.0), 90.0) {
            Geometry::Point(pt) => {
                assert!(pt.lon_deg.abs() < 1e-9, "{}", pt.lon_deg);
                assert!((pt.lat_deg - 1.0).abs() < 1e-9, "{}", pt.lat_deg);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn scale_moves_radius_and_vertices() {
        let g = Geometry::Circle(Circle {
            center: p(0.0, 0.0),
            radius_m: 100.0,
        });
        match scale(&g, (0.0, 0.0), 3.0) {
            Geometry::Circle(c) => assert!((c.radius_m - 300.0).abs() < 1e-9),
            _ => unreachable!(),
        }
        let line = Geometry::Polyline(Polyline {
            positions: vec![p(0.0, 0.0), p(2.0, 0.0)],
        });
        let s = scale(&line, (0.0, 0.0), 2.0);
        assert_eq!(s.vertices()[1].lon_deg, 4.0);
    }

    #[test]
    fn set_vertex_hits_polygon_holes_by_flat_index() {
        let g = Geometry::Polygon(Polygon {
            outer: vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0)],
            holes: vec![vec![p(5.0, 5.0)]],
        });
        // Index 3 is the single hole vertex.
        let edited = set_vertex(&g, 3, p(9.0, 9.0)).unwrap();
        match edited {
            Geometry::Polygon(pg) => assert_eq!(pg.holes[0][0], p(9.0, 9.0)),
            _ => unreachable!(),
        }
        // Out of range → None.
        assert!(set_vertex(&g, 4, p(0.0, 0.0)).is_none());
    }

    #[test]
    fn set_vertex_arc_by_index() {
        let g = Geometry::Arc(Arc3 {
            start: p(0.0, 0.0),
            center: p(1.0, 1.0),
            end: p(2.0, 0.0),
        });
        match set_vertex(&g, 1, p(5.0, 5.0)) {
            Some(Geometry::Arc(a)) => assert_eq!(a.center, p(5.0, 5.0)),
            other => panic!("{other:?}"),
        }
    }
}
