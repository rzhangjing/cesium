//! GeoJSON encode / decode for the scene document (plan §12, M8).
//!
//! Two guarantees, deliberately layered:
//!  * **Interoperability** — every feature carries a standard GeoJSON geometry
//!    (`Point` / `LineString` / `Polygon`) so other tools read the file; free
//!    business attributes are spread into `properties`.
//!  * **Losslessness** — a top-level `x-plot` extension member embeds the whole
//!    [`Document`] (layers, groups, ids, styles, flags, scale / time windows).
//!    Our reader prefers that payload, so `to_geojson` → `from_geojson` restores
//!    the document *exactly* (verified by a round-trip test). A file with no
//!    `x-plot` (foreign GeoJSON) still imports best-effort: a default layer plus
//!    one element per feature.
//!
//! Geometry kinds with no GeoJSON equivalent (circle / ellipse / arc / path /
//! composite) still export a representative `Point` for interop; their exact
//! parameters survive in the `x-plot` document.

use serde_json::{json, Map, Value};

use crate::geo::GeoPoint;
use crate::model::geometry::{Geometry, Polygon, Polyline};
use crate::model::Document;

/// IO failure surface.
#[derive(Debug, thiserror::Error)]
pub enum PlotIoError {
    /// Malformed or unreadable JSON.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// Input was neither an `x-plot` document nor a GeoJSON `FeatureCollection`.
    #[error("expected a GeoJSON FeatureCollection")]
    NotFeatureCollection,
}

/// Serialise the whole document as a GeoJSON `FeatureCollection` (pretty).
pub fn to_geojson(doc: &Document) -> Result<String, PlotIoError> {
    let mut features: Vec<Value> = Vec::new();
    for id in doc.flatten_draw_order() {
        let Some(el) = doc.element(id) else {
            continue;
        };
        // `properties` = business attributes (spread) + `name` + `x-plot`.
        let mut props = match serde_json::to_value(&el.attributes)? {
            Value::Object(m) => m,
            _ => Map::new(),
        };
        props.insert("name".into(), json!(el.name));
        props.insert("x-plot".into(), serde_json::to_value(el)?);
        features.push(json!({
            "type": "Feature",
            "properties": props,
            "geometry": geometry_to_gj(&el.geometry),
        }));
    }

    let mut fc = Map::new();
    fc.insert("type".into(), json!("FeatureCollection"));
    fc.insert("features".into(), Value::Array(features));
    fc.insert(
        "x-plot".into(),
        json!({ "version": 1, "document": serde_json::to_value(doc)? }),
    );
    Ok(serde_json::to_string_pretty(&Value::Object(fc))?)
}

/// Parse a GeoJSON string back into a [`Document`]. Prefers the lossless
/// `x-plot.document` payload; otherwise imports foreign GeoJSON best-effort
/// into a single fresh layer.
pub fn from_geojson(s: &str) -> Result<Document, PlotIoError> {
    let v: Value = serde_json::from_str(s)?;

    // Lossless path: our own export embeds the full document.
    if let Some(dv) = v.get("x-plot").and_then(|x| x.get("document")) {
        let mut doc: Document = serde_json::from_value(dv.clone())?;
        doc.rebuild_parents();
        return Ok(doc);
    }

    // Foreign GeoJSON: one default active layer, one element per feature.
    let feats = v
        .get("features")
        .and_then(|f| f.as_array())
        .ok_or(PlotIoError::NotFeatureCollection)?;
    let mut doc = Document::default();
    let layer = doc.new_layer("导入");
    doc.focus_layer(layer);
    for (i, f) in feats.iter().enumerate() {
        let Some(gj) = f.get("geometry") else {
            continue;
        };
        let Some(geo) = gj_to_geometry(gj) else {
            continue;
        };
        let props = f.get("properties").and_then(|p| p.as_object());
        let name = props
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("要素 {}", i + 1));
        let mut ne = doc.make_element(name, geo);
        if let Some(p) = props {
            for (k, val) in p {
                if k != "name" {
                    ne.element.attributes.insert(k.clone(), val.clone());
                }
            }
        }
        doc.add_element_to_layer(layer, ne);
    }
    Ok(doc)
}

// ── geometry ⇄ GeoJSON ──────────────────────────────────────────────────────

/// A position `[lon, lat, height]`.
fn coord(p: GeoPoint) -> Value {
    json!([p.lon_deg, p.lat_deg, p.height_m])
}

/// A linear-ring coordinate list, closing the ring (GeoJSON wants first == last;
/// the model stores it open).
fn closed_ring(ring: &[GeoPoint]) -> Vec<Value> {
    let mut v: Vec<Value> = ring.iter().map(|p| coord(*p)).collect();
    if let Some(first) = ring.first() {
        v.push(coord(*first));
    }
    v
}

/// Best-effort standard GeoJSON geometry for interop. Kinds without an exact
/// equivalent fall back to a representative anchor `Point` (or `null`).
fn geometry_to_gj(g: &Geometry) -> Option<Value> {
    match g {
        Geometry::Point(p) => Some(json!({"type": "Point", "coordinates": coord(*p)})),
        Geometry::Icon(i) => Some(json!({"type": "Point", "coordinates": coord(i.at)})),
        Geometry::Label(l) => Some(json!({"type": "Point", "coordinates": coord(l.at)})),
        Geometry::Polyline(pl) => Some(json!({
            "type": "LineString",
            "coordinates": pl.positions.iter().map(|p| coord(*p)).collect::<Vec<_>>(),
        })),
        Geometry::Rectangle(r) => {
            let ring = [
                GeoPoint::surface(r.west, r.south),
                GeoPoint::surface(r.east, r.south),
                GeoPoint::surface(r.east, r.north),
                GeoPoint::surface(r.west, r.north),
            ];
            Some(json!({"type": "Polygon", "coordinates": [closed_ring(&ring)]}))
        }
        Geometry::Polygon(pg) => {
            let mut rings = vec![closed_ring(&pg.outer)];
            for h in &pg.holes {
                rings.push(closed_ring(h));
            }
            Some(json!({"type": "Polygon", "coordinates": rings}))
        }
        // No standard form: representative point for interop (exact params live
        // in the x-plot document).
        other => other
            .anchor()
            .map(|a| json!({"type": "Point", "coordinates": coord(a)})),
    }
}

/// A single GeoJSON position `[lon, lat, h]`.
fn position(v: &Value) -> Option<GeoPoint> {
    let p = v.as_array()?;
    if p.len() < 2 {
        return None;
    }
    let lon = p[0].as_f64()?;
    let lat = p[1].as_f64()?;
    let h = p.get(2).and_then(|x| x.as_f64()).unwrap_or(0.0);
    Some(GeoPoint::new(lon, lat, h))
}

/// Parse a flat coordinate array (`[lon, lat, h]`…) into model points, dropping a
/// trailing duplicated closing vertex when present.
fn positions(v: &Value) -> Option<Vec<GeoPoint>> {
    let arr = v.as_array()?;
    let mut out: Vec<GeoPoint> = Vec::with_capacity(arr.len());
    for c in arr {
        out.push(position(c)?);
    }
    // Strip the closing duplicate (first == last) GeoJSON mandates but we omit.
    if out.len() >= 2 {
        let (a, b) = (out.first().unwrap().lon_deg, out.last().unwrap().lon_deg);
        let (al, bl) = (out.first().unwrap().lat_deg, out.last().unwrap().lat_deg);
        if (a - b).abs() < 1e-12 && (al - bl).abs() < 1e-12 {
            out.pop();
        }
    }
    Some(out)
}

/// Foreign GeoJSON geometry → model (best-effort; unsupported kinds → `None`).
fn gj_to_geometry(v: &Value) -> Option<Geometry> {
    let ty = v.get("type")?.as_str()?;
    match ty {
        "Point" => Some(Geometry::Point(position(v.get("coordinates")?)?)),
        "MultiPoint" | "LineString" => {
            let pts = positions(v.get("coordinates")?)?;
            if pts.len() == 1 {
                Some(Geometry::Point(pts[0]))
            } else if pts.len() >= 2 {
                Some(Geometry::Polyline(Polyline { positions: pts }))
            } else {
                None
            }
        }
        "Polygon" => {
            let rings = v.get("coordinates")?.as_array()?;
            let outer = positions(rings.first()?)?;
            if outer.len() < 3 {
                return None;
            }
            let mut holes = Vec::new();
            for r in rings.iter().skip(1) {
                if let Some(h) = positions(r) {
                    if h.len() >= 3 {
                        holes.push(h);
                    }
                }
            }
            Some(Geometry::Polygon(Polygon { outer, holes }))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::geometry::{Circle, LabelAnchor, LabelGeometry, Rectangle};
    use crate::model::style::Style;
    use serde_json::Value;

    fn p(lon: f64, lat: f64) -> GeoPoint {
        GeoPoint::surface(lon, lat)
    }

    /// A document exercising layers, a group, several geometry kinds, styles,
    /// attributes and layer flags / order.
    fn rich_doc() -> Document {
        let mut doc = Document::default();
        let back = doc.new_layer("底图");
        let front = doc.new_layer("标绘");
        doc.layer_mut(back).unwrap().order = 0;
        doc.layer_mut(back).unwrap().visible = false;
        doc.layer_mut(front).unwrap().order = 10;
        doc.layer_mut(front).unwrap().opacity = 0.5;
        doc.focus_layer(front);

        // Plain point with a colour + attribute.
        let mut pt = doc.make_element("观察点", Geometry::Point(p(116.4, 39.9)));
        pt.element.style = Style::default().with_color([1.0, 0.0, 0.0, 1.0]);
        pt.element
            .attributes
            .insert("side".into(), Value::String("friend".into()));
        doc.add_element_to_layer(front, pt);

        // A group in the back layer holding a labelled element + a rectangle.
        let g = doc.new_group_in_layer(back, "编队");
        let lbl = doc.make_element(
            "标签",
            Geometry::Label(LabelGeometry {
                at: p(10.0, 20.0),
                text: "前沿".into(),
                anchor: LabelAnchor::Bottom,
                offset_px: [0.0, 4.0],
            }),
        );
        doc.add_element_to_group(g, lbl);
        let rect = doc.make_element(
            "区",
            Geometry::Rectangle(Rectangle {
                west: 0.0,
                south: 0.0,
                east: 1.0,
                north: 2.0,
            }),
        );
        doc.add_element_to_group(g, rect);

        // A polyline and a polygon-with-hole and a circle in the front layer.
        let line = doc.make_element(
            "路线",
            Geometry::Polyline(Polyline {
                positions: vec![p(0.0, 0.0), p(1.0, 1.0), p(2.0, 0.0)],
            }),
        );
        doc.add_element_to_layer(front, line);
        let poly = doc.make_element(
            "防区",
            Geometry::Polygon(Polygon {
                outer: vec![p(0.0, 0.0), p(4.0, 0.0), p(4.0, 4.0), p(0.0, 4.0)],
                holes: vec![vec![p(1.0, 1.0), p(2.0, 1.0), p(2.0, 2.0), p(1.0, 2.0)]],
            }),
        );
        doc.add_element_to_layer(front, poly);
        let circle = doc.make_element(
            "射程",
            Geometry::Circle(Circle {
                center: p(5.0, 5.0),
                radius_m: 12_345.0,
            }),
        );
        let cid = circle.id;
        doc.add_element_to_layer(front, circle);
        doc.element_mut(cid).unwrap().flags.visible_manual = false;

        doc
    }

    #[test]
    fn lossless_roundtrip_restores_document_exactly() {
        let doc = rich_doc();
        let text = to_geojson(&doc).unwrap();
        let back = from_geojson(&text).unwrap();
        // Full structural equality (layers, groups, ids, styles, flags, order).
        assert_eq!(back, doc);
        assert_eq!(back.layers().len(), 2);
        assert_eq!(back.element_count(), 6);
        assert_eq!(back.active_layer(), doc.active_layer());
    }

    #[test]
    fn exports_standard_geometries_for_interop() {
        let mut doc = Document::with_default_layer();
        let l = doc.active_layer().unwrap();
        let a = doc.make_element("pt", Geometry::Point(p(1.5, 2.5)));
        let b = doc.make_element(
            "ln",
            Geometry::Polyline(Polyline {
                positions: vec![p(0.0, 0.0), p(1.0, 1.0)],
            }),
        );
        let c = doc.make_element(
            "pg",
            Geometry::Polygon(Polygon {
                outer: vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0)],
                holes: vec![],
            }),
        );
        doc.add_element_to_layer(l, a);
        doc.add_element_to_layer(l, b);
        doc.add_element_to_layer(l, c);
        let text = to_geojson(&doc).unwrap();
        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["type"], "FeatureCollection");
        let feats = v["features"].as_array().unwrap();
        assert_eq!(feats.len(), 3);
        assert_eq!(feats[0]["geometry"]["type"], "Point");
        assert_eq!(feats[0]["geometry"]["coordinates"], json!([1.5, 2.5, 0.0]));
        assert_eq!(feats[1]["geometry"]["type"], "LineString");
        assert_eq!(feats[1]["geometry"]["coordinates"].as_array().unwrap().len(), 2);
        // Polygon ring is closed: first coord == last coord (3 stored → 4 emitted).
        let ring = feats[2]["geometry"]["coordinates"][0].as_array().unwrap();
        assert_eq!(ring.len(), 4);
        assert_eq!(ring[0], ring[3]);
        assert_eq!(feats[0]["properties"]["name"], "pt");
    }

    #[test]
    fn foreign_geojson_imports_into_active_layer() {
        let text = r#"{
            "type": "FeatureCollection",
            "features": [
                { "type": "Feature", "properties": { "name": "起点", "side": "hostile" },
                  "geometry": { "type": "Point", "coordinates": [3.0, 4.0] } },
                { "type": "Feature", "properties": { "name": "边界" },
                  "geometry": { "type": "Polygon",
                    "coordinates": [ [[0,0],[2,0],[2,2],[0,2],[0,0]] ] } },
                { "type": "Feature", "properties": {},
                  "geometry": { "type": "MultiPolygon",
                    "coordinates": [ [[[0,0],[1,0],[1,1],[0,0]]] ] } }
            ]
        }"#;
        let doc = from_geojson(text).unwrap();
        // The unsupported MultiPolygon feature is skipped → 2 elements.
        assert_eq!(doc.element_count(), 2);
        let layer = doc.active_layer().unwrap();
        assert_eq!(doc.layer(layer).unwrap().name, "导入");
        let ids: Vec<_> = doc.element_ids().collect();
        let start = doc.element(ids[0]).unwrap();
        assert_eq!(start.name, "起点");
        assert_eq!(start.geometry, Geometry::Point(p(3.0, 4.0)));
        assert_eq!(
            start.attributes.get("side"),
            Some(&Value::String("hostile".into()))
        );
        let border = doc.element(ids[1]).unwrap();
        assert!(matches!(border.geometry, Geometry::Polygon(_)));
        // The imported polygon ring is de-closed (4 unique vertices, not 5).
        match &border.geometry {
            Geometry::Polygon(pg) => assert_eq!(pg.outer.len(), 4),
            _ => unreachable!(),
        }
    }

    #[test]
    fn rejects_non_featurecollection() {
        assert!(matches!(
            from_geojson(r#"{"type": "Feature"}"#),
            Err(PlotIoError::NotFeatureCollection)
        ));
        assert!(matches!(
            from_geojson("not json"),
            Err(PlotIoError::Json(_))
        ));
    }
}
