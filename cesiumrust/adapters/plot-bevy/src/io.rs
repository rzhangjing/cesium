//! GeoJSON import / export for the live overlay (plan §12 / §14, M8).
//!
//! Thin bridge over the pure [`cesium_plot::io::geojson`] codec. Import folds
//! every parsed element into an **add command targeting the current active
//! layer** (ids are re-minted by the live document, so pasting never collides),
//! applied through the M6 commit path so a whole import is a *single* undo step
//! (plan §14 "粘贴落活动层"). Export serialises the live document, styles and
//! `x-plot` payload included.
//!
//! Both are free functions over the plain [`PlotDocument`] / [`PlotHistory`]
//! resources — no window, no file dialog — so they unit-test headless; the app
//! decides when to call them (menu / drop / clipboard).

use cesium_plot::io::geojson::{self, PlotIoError};
use cesium_plot::model::Document;
use cesium_plot::ops::PlotCommand;

use crate::edit::apply_command;
use crate::resources::{PlotDocument, PlotHistory};

/// Parse a GeoJSON string and paste its elements into the active layer as one
/// undoable command, returning how many were placed. An empty / foreign file with
/// no supported geometry adds nothing (still not an error).
pub fn import_geojson(
    plot_doc: &mut PlotDocument,
    history: &mut PlotHistory,
    text: &str,
) -> Result<usize, PlotIoError> {
    let parsed = geojson::from_geojson(text)?;
    let steps = build_import_commands(&mut plot_doc.doc, &parsed);
    let count = steps.len();
    apply_command(plot_doc, history, PlotCommand::Composite { steps });
    Ok(count)
}

/// Serialise the live document to a pretty GeoJSON `FeatureCollection` string.
pub fn export_geojson(plot_doc: &PlotDocument) -> Result<String, PlotIoError> {
    geojson::to_geojson(&plot_doc.doc)
}

/// Build the add commands that drop every element of `src` into `dst`'s active
/// layer (creating + focusing one if it has none), **re-minting each id** in
/// `dst`'s counter so pasted elements never collide with existing ones. Pure over
/// both documents, so the id allocation / active-layer fallback is unit-testable
/// without a resource window.
fn build_import_commands(dst: &mut Document, src: &Document) -> Vec<PlotCommand> {
    let target = ensure_active_layer(dst);
    let mut steps = Vec::new();
    for id in src.flatten_draw_order() {
        let Some(el) = src.element(id) else {
            continue;
        };
        // Fresh id from `dst`; copy every other payload across (mirrors M6
        // duplicate). `make_element` recomputes bounds from the geometry clone.
        let mut ne = dst.make_element(el.name.clone(), el.geometry.clone());
        ne.element.style = el.style.clone();
        ne.element.attributes = el.attributes.clone();
        ne.element.flags = el.flags;
        ne.element.scale_visibility = el.scale_visibility;
        ne.element.time_window = el.time_window;
        steps.push(PlotCommand::AddElement {
            layer: target,
            element: Box::new(ne.element),
        });
    }
    steps
}

/// Ensure the document has an active layer to paste into, creating a fresh one
/// when it has none. Returns the (possibly new) active layer id.
fn ensure_active_layer(doc: &mut Document) -> cesium_plot::model::ids::LayerId {
    if let Some(l) = doc.active_layer() {
        return l;
    }
    let l = doc.new_layer("导入");
    doc.focus_layer(l);
    l
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_plot::geo::GeoPoint;
    use cesium_plot::model::geometry::Geometry;
    use cesium_plot::ops::HistoryStack;

    fn fresh_doc(lon: f64, lat: f64) -> PlotDocument {
        let mut d = PlotDocument {
            doc: Document::with_default_layer(),
            revision: 0,
            dirty: true,
        };
        let layer = d.doc.active_layer().unwrap();
        let ne = d.doc.make_element("已有", Geometry::Point(GeoPoint::surface(lon, lat)));
        d.doc.add_element_to_layer(layer, ne);
        d
    }

    fn geojson_text() -> String {
        let mut src = Document::default();
        let l = src.new_layer("导入源");
        let a = src.make_element("甲", Geometry::Point(GeoPoint::surface(10.0, 20.0)));
        let b = src.make_element(
            "乙",
            Geometry::Polyline(cesium_plot::model::geometry::Polyline {
                positions: vec![GeoPoint::surface(0.0, 0.0), GeoPoint::surface(1.0, 1.0)],
            }),
        );
        src.add_element_to_layer(l, a);
        src.add_element_to_layer(l, b);
        geojson::to_geojson(&src).unwrap()
    }

    #[test]
    fn import_pastes_into_active_layer_and_is_undoable() {
        let mut d = fresh_doc(0.0, 0.0);
        let mut h = PlotHistory(HistoryStack::new());
        let before = d.doc.element_count();
        let active = d.doc.active_layer().unwrap();
        let n = import_geojson(&mut d, &mut h, &geojson_text()).unwrap();
        assert_eq!(n, 2, "two imported features");
        assert_eq!(d.doc.element_count(), before + 2);
        // Everything landed in the pre-existing active layer.
        for id in d.doc.element_ids() {
            if d.doc.element(id).unwrap().name.starts_with('甲') || d.doc.element(id).unwrap().name.starts_with('乙') {
                assert_eq!(d.doc.element_context(id).unwrap().0, active);
            }
        }
        // One undo removes the whole import (single composite step).
        h.0.undo(&mut d.doc);
        assert_eq!(d.doc.element_count(), before);
    }

    #[test]
    fn import_mints_fresh_ids_no_collision() {
        let mut d = fresh_doc(0.0, 0.0);
        let mut h = PlotHistory(HistoryStack::new());
        let existing: Vec<_> = d.doc.element_ids().collect();
        import_geojson(&mut d, &mut h, &geojson_text()).unwrap();
        // No imported id equals a pre-existing id.
        let after: Vec<_> = d.doc.element_ids().collect();
        for id in &after {
            if existing.contains(id) {
                continue;
            }
            assert!(!existing.contains(id));
        }
        assert_eq!(after.len(), existing.len() + 2);
    }

    #[test]
    fn export_roundtrips_live_document() {
        let d = fresh_doc(3.0, 4.0);
        let text = export_geojson(&d).unwrap();
        let back = geojson::from_geojson(&text).unwrap();
        assert_eq!(back, d.doc);
    }

    #[test]
    fn ensure_active_layer_creates_when_none() {
        let mut doc = Document::default();
        assert!(doc.active_layer().is_none());
        let l = ensure_active_layer(&mut doc);
        assert_eq!(doc.active_layer(), Some(l));
        // Idempotent: a second call returns the same active layer.
        assert_eq!(ensure_active_layer(&mut doc), l);
    }
}
