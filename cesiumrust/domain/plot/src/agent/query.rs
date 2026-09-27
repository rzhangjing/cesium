//! Agent-facing **read-only situational query** (plan P2).
//!
//! Where [`action`](super::action) is the write path, this is the "understand the
//! picture" half an agent needs: a pure [`query`] that filters the document by
//! bounding box / attributes / geometry kind / layer / name and returns
//! serialisable [`ElementSummary`] rows, plus [`measure_length`] /
//! [`measure_area`] read-outs. No mutation, no engine types — deterministically
//! unit-testable headless.
//!
//! The `bbox` filter tests against each element's *conservative* geographic
//! bounds (see `Geometry::bounds`), which already cover great-circle bulge and
//! parameterised-figure extent, so a broad-phase overlap can never drop an
//! element that actually intersects the window.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::geo::GeoBounds;
use crate::model::geometry::GeometryKind;
use crate::model::ids::{ElementId, LayerId};
use crate::model::{Document, ViewContext, ViewMode};
use crate::ops::measure::{measure_area_m2, measure_length_m};

/// A conjunctive filter: an element matches only when it satisfies *every*
/// present criterion. An all-`None` / empty filter returns every element.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct QueryFilter {
    /// Keep elements whose bounds overlap this window (inclusive).
    pub bbox: Option<GeoBounds>,
    /// Keep elements whose attributes contain all of these exact key/values.
    pub attributes: Vec<(String, Value)>,
    /// Keep only this coarse geometry class.
    pub kind: Option<GeometryKind>,
    /// Keep only elements belonging to this layer.
    pub layer: Option<LayerId>,
    /// Keep elements whose name contains this substring (case-sensitive).
    pub name_contains: Option<String>,
    /// Restrict to pickable / selectable elements.
    pub selectable_only: bool,
}

/// A serialisable summary row for a matched element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementSummary {
    pub id: ElementId,
    pub name: String,
    pub kind: GeometryKind,
    /// The layer the element resolves to (its ancestor chain's layer root).
    pub layer: Option<LayerId>,
    pub bounds: GeoBounds,
    pub attributes: Map<String, Value>,
    /// Effective visibility: the manual flag AND-ed with the layer switch and,
    /// when a `view` was supplied, the scale band and per-mode show flags.
    pub visible: bool,
}

/// Broad-phase overlap of two axis-aligned lat/lon boxes (inclusive). An empty
/// box overlaps nothing.
fn overlaps(a: &GeoBounds, b: &GeoBounds) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    a.west_deg <= b.east_deg
        && a.east_deg >= b.west_deg
        && a.south_deg <= b.north_deg
        && a.north_deg >= b.south_deg
}

/// Every requested `(key, value)` pair is present in `attrs` with an equal value.
fn attr_matches(attrs: &Map<String, Value>, want: &[(String, Value)]) -> bool {
    want.iter().all(|(k, v)| attrs.get(k) == Some(v))
}

/// Return a summary for every element matching `filter`, in stable draw order.
/// `view` is optional: when given, the computed `visible` flag additionally
/// honours the element's scale band and its per-mode show flags.
pub fn query(
    doc: &Document,
    filter: &QueryFilter,
    view: Option<&ViewContext>,
) -> Vec<ElementSummary> {
    let mut out = Vec::new();
    for id in doc.flatten_draw_order() {
        let Some(el) = doc.element(id) else {
            continue;
        };
        if filter.selectable_only && !el.flags.selectable {
            continue;
        }
        if let Some(kind) = filter.kind {
            if el.geometry.kind() != kind {
                continue;
            }
        }
        if let Some(needle) = &filter.name_contains {
            if !el.name.contains(needle.as_str()) {
                continue;
            }
        }
        let layer = doc.element_context(id).map(|(l, _)| l);
        if let Some(ly) = filter.layer {
            if layer != Some(ly) {
                continue;
            }
        }
        if let Some(window) = &filter.bbox {
            if !overlaps(window, &el.bounds) {
                continue;
            }
        }
        if !attr_matches(&el.attributes, &filter.attributes) {
            continue;
        }
        let mut visible = el.flags.visible_manual
            && layer
                .and_then(|l| doc.layer(l))
                .map(|l| l.visible)
                .unwrap_or(false);
        if let Some(v) = view {
            visible &= el
                .scale_visibility
                .allows(v.pixels_per_world, v.meters_per_pixel);
            visible &= match v.mode {
                ViewMode::Flat => el.style.show_in_flat,
                ViewMode::Globe => el.style.show_in_globe,
            };
        }
        out.push(ElementSummary {
            id,
            name: el.name.clone(),
            kind: el.geometry.kind(),
            layer,
            bounds: el.bounds,
            attributes: el.attributes.clone(),
            visible,
        });
    }
    out
}

/// Great-circle / perimeter length (metres) of an element, or `None` for an
/// unknown id. Point-like kinds measure `0.0`.
pub fn measure_length(doc: &Document, id: ElementId) -> Option<f64> {
    doc.element(id).map(|e| measure_length_m(&e.geometry))
}

/// Enclosed area (m²) of an element, or `None` for an unknown id. Open /
/// point-like kinds measure `0.0`.
pub fn measure_area(doc: &Document, id: ElementId) -> Option<f64> {
    doc.element(id).map(|e| measure_area_m2(&e.geometry))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::GeoPoint;
    use crate::model::geometry::{Geometry, Polyline};
    use crate::model::ids::ElementId;

    fn pt(lon: f64, lat: f64) -> GeoPoint {
        GeoPoint::surface(lon, lat)
    }

    fn line(doc: &mut Document, name: &str, a: GeoPoint, b: GeoPoint) -> ElementId {
        let layer = doc.active_layer().unwrap();
        let ne = doc.make_element(
            name,
            Geometry::Polyline(Polyline {
                positions: vec![a, b],
            }),
        );
        let id = ne.id;
        doc.add_element_to_layer(layer, ne);
        id
    }

    fn box_filter(w: f64, s: f64, e: f64, n: f64) -> QueryFilter {
        QueryFilter {
            bbox: Some(GeoBounds {
                west_deg: w,
                south_deg: s,
                east_deg: e,
                north_deg: n,
            }),
            ..Default::default()
        }
    }

    #[test]
    fn empty_filter_returns_all_in_order() {
        let mut doc = Document::with_default_layer();
        line(&mut doc, "a", pt(0.0, 0.0), pt(1.0, 1.0));
        line(&mut doc, "b", pt(5.0, 5.0), pt(6.0, 6.0));
        let got = query(&doc, &QueryFilter::default(), None);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].name, "a");
        assert_eq!(got[1].name, "b");
    }

    #[test]
    fn kind_and_name_and_layer_filters() {
        let mut doc = Document::with_default_layer();
        let id = line(&mut doc, "front-line", pt(0.0, 0.0), pt(1.0, 0.0));
        let pid = {
            let layer = doc.active_layer().unwrap();
            let ne = doc.make_element("depot", Geometry::Point(pt(2.0, 2.0)));
            let id = ne.id;
            doc.add_element_to_layer(layer, ne);
            id
        };
        let only_lines = query(
            &doc,
            &QueryFilter {
                kind: Some(GeometryKind::Line),
                ..Default::default()
            },
            None,
        );
        assert_eq!(only_lines.len(), 1);
        assert_eq!(only_lines[0].id, id);

        let by_name = query(
            &doc,
            &QueryFilter {
                name_contains: Some("depot".into()),
                ..Default::default()
            },
            None,
        );
        assert_eq!(by_name.len(), 1);
        assert_eq!(by_name[0].id, pid);

        let layer = doc.active_layer().unwrap();
        let by_layer = query(
            &doc,
            &QueryFilter {
                layer: Some(layer),
                ..Default::default()
            },
            None,
        );
        assert_eq!(by_layer.len(), 2);
        // A layer nothing belongs to → empty.
        let none = query(
            &doc,
            &QueryFilter {
                layer: Some(LayerId(999)),
                ..Default::default()
            },
            None,
        );
        assert!(none.is_empty());
    }

    #[test]
    fn attribute_filter_is_exact_subset() {
        let mut doc = Document::with_default_layer();
        let layer = doc.active_layer().unwrap();
        let mut ne = doc.make_element("hostile", Geometry::Point(pt(0.0, 0.0)));
        ne.element
            .attributes
            .insert("side".into(), Value::String("hostile".into()));
        let hid = ne.id;
        doc.add_element_to_layer(layer, ne);
        let _friend = {
            let mut ne2 = doc.make_element("friend", Geometry::Point(pt(1.0, 1.0)));
            ne2.element
                .attributes
                .insert("side".into(), Value::String("friend".into()));
            let id = ne2.id;
            doc.add_element_to_layer(layer, ne2);
            id
        };
        let got = query(
            &doc,
            &QueryFilter {
                attributes: vec![("side".into(), Value::String("hostile".into()))],
                ..Default::default()
            },
            None,
        );
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id, hid);
    }

    #[test]
    fn bbox_overlap_hits_and_misses() {
        let mut doc = Document::with_default_layer();
        let id = line(&mut doc, "square-edge", pt(0.0, 0.0), pt(2.0, 2.0));
        let hit = query(&doc, &box_filter(1.0, 1.0, 3.0, 3.0), None);
        assert_eq!(hit.len(), 1);
        assert_eq!(hit[0].id, id);
        let miss = query(&doc, &box_filter(40.0, 40.0, 50.0, 50.0), None);
        assert!(miss.is_empty());
    }

    #[test]
    fn bbox_does_not_miss_great_circle_bulge() {
        // A 60°N parallel chord follows a great circle that bulges poleward, so
        // its true extent reaches above the 60.0° of its raw control points. A
        // naive control-point box would top out at exactly 60.0 and wrongly
        // reject a window sitting just north of it (the classic silent
        // broad-phase miss); the element's *conservative* bounds cover the bulge.
        let mut doc = Document::with_default_layer();
        let id = line(&mut doc, "arctic", pt(0.0, 60.0), pt(10.0, 60.0));
        let bounds = doc.element(id).unwrap().bounds;
        assert!(
            bounds.north_deg > 60.05,
            "bounds must cover the bulge, got {bounds:?}"
        );
        // A window strictly above the raw 60.0 latitude, but inside the bulge,
        // still matches — a control-point-only box would miss it.
        let got = query(&doc, &box_filter(4.0, 60.02, 6.0, 60.04), None);
        assert_eq!(got.len(), 1, "bulge window must not miss the line");
        assert_eq!(got[0].id, id);
        // A window above the real bounds top is correctly rejected (not a lie).
        let miss = query(&doc, &box_filter(4.0, 60.5, 6.0, 60.9), None);
        assert!(miss.is_empty(), "window above the true bulge must miss");
    }

    #[test]
    fn visibility_reflects_manual_flag_and_view_scale() {
        let mut doc = Document::with_default_layer();
        let layer = doc.active_layer().unwrap();
        let mut ne = doc.make_element("band", Geometry::Point(pt(0.0, 0.0)));
        ne.element.scale_visibility.min_pixels_per_world = Some(100.0);
        ne.element.scale_visibility.max_pixels_per_world = Some(1000.0);
        let id = ne.id;
        doc.add_element_to_layer(layer, ne);

        // No view → only manual + layer govern visibility (band ignored).
        let plain = query(&doc, &QueryFilter::default(), None).remove(0);
        assert!(plain.id == id && plain.visible);

        // A view outside the band hides it; a view inside keeps it visible.
        let out_of_band = ViewContext {
            pixels_per_world: 5000.0,
            ..Default::default()
        };
        assert!(!query(&doc, &QueryFilter::default(), Some(&out_of_band))[0].visible);
        let in_band = ViewContext {
            pixels_per_world: 500.0,
            ..Default::default()
        };
        assert!(query(&doc, &QueryFilter::default(), Some(&in_band))[0].visible);
    }

    #[test]
    fn measure_wrappers_match_geometry() {
        let mut doc = Document::with_default_layer();
        // One degree of longitude at the equator ≈ 111.32 km.
        let id = line(&mut doc, "deg", pt(0.0, 0.0), pt(1.0, 0.0));
        let len = measure_length(&doc, id).unwrap();
        assert!((len - 111_319.0).abs() < 200.0, "got {len}");
        assert_eq!(measure_area(&doc, id).unwrap(), 0.0, "open line has no area");
        assert!(measure_length(&doc, ElementId(9999)).is_none());
    }
}
