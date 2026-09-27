//! The pure multi-dimensional visibility evaluator (plan §10).
//!
//! [`eval_visibility`] is a total function of `(Document, ViewContext, Filters)`
//! returning which elements are **visible** and, a subset, which are **pickable**
//! (visible *and* selectable under their layer / group locks). Every one of the
//! ten dimensions is an independent gate folded with AND; the time dimension is
//! a reserved no-op in M1 (returns pass) pending M9's clock wiring.
//!
//! Keeping this a free function — not a system, not resource-driven — is what
//! lets the whole显隐 rule table be unit-tested without an engine (plan §15).

use std::collections::BTreeSet;

use serde_json::Value;

use crate::model::document::Document;
use crate::model::element::Element;
use crate::model::filters::Filters;
use crate::model::group::Group;
use crate::model::ids::ElementId;
use crate::model::layer::Layer;
use crate::model::view::{ViewContext, ViewMode};

/// Result of a visibility pass.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VisibilityResult {
    /// Elements to draw, in no particular order (the bridge re-sorts).
    pub visible: BTreeSet<ElementId>,
    /// Visible **and** pickable — the candidate set for hit-testing (plan §7).
    pub pickable: BTreeSet<ElementId>,
}

/// Evaluate every element against the ten visibility dimensions.
pub fn eval_visibility(doc: &Document, view: &ViewContext, filters: &Filters) -> VisibilityResult {
    let mut out = VisibilityResult::default();

    // §10.1 master switch: nothing is visible (nor pickable) when off.
    if !filters.overlay_enabled {
        return out;
    }

    for element in doc.elements() {
        let Some((layer_id, group_chain)) = doc.element_context(element.id) else {
            continue; // orphan node in a corrupt tree — skip defensively
        };
        let Some(layer) = doc.layer(layer_id) else {
            continue;
        };

        if !is_visible(doc, element, layer, &group_chain, view, filters) {
            continue;
        }
        out.visible.insert(element.id);

        if is_pickable(element, layer, &group_chain, doc, filters) {
            out.pickable.insert(element.id);
        }
    }
    out
}

/// The AND of every *visibility* dimension (§10.2–§10.10, master switch already
/// checked by the caller).
fn is_visible(
    doc: &Document,
    element: &Element,
    layer: &Layer,
    group_chain: &[crate::model::ids::GroupId],
    view: &ViewContext,
    filters: &Filters,
) -> bool {
    // §10.2 layer on/off.
    if !layer.visible {
        return false;
    }
    // §10.3 every group up the chain visible.
    if !group_chain.iter().all(|g| {
        doc.group(*g).map(|grp| grp.visible).unwrap_or(false)
    }) {
        return false;
    }
    // §10.4 element's own manual toggle.
    if !element.flags.visible_manual {
        return false;
    }
    // §10.5 type dimension.
    if let Some(allowed) = &filters.enabled_types {
        if !allowed.contains(&element.geometry.kind()) {
            return false;
        }
    }
    // §10.6 view-mode dimension.
    let style = &element.style;
    match view.mode {
        ViewMode::Globe if !style.show_in_globe => return false,
        ViewMode::Flat if !style.show_in_flat => return false,
        _ => {}
    }
    // §10.7 scale band.
    if !element
        .scale_visibility
        .allows(view.pixels_per_world, view.meters_per_pixel)
    {
        return false;
    }
    // §10.8 selection focus.
    if filters.only_selected && !filters.selected.contains(&element.id) {
        return false;
    }
    // §10.9 attribute dimension.
    if !attributes_pass(element, &filters.attribute_predicates) {
        return false;
    }
    // §10.10 time window — reserved; always passes in M1.
    true
}

/// Pickability (plan §7/§9): element `selectable`, its layer `selectable`, and
/// no *locked* group in the chain (a locked group makes members individually
/// non-grabbable). The selection-focus filter also narrows the pickable set.
fn is_pickable(
    element: &Element,
    layer: &Layer,
    group_chain: &[crate::model::ids::GroupId],
    doc: &Document,
    filters: &Filters,
) -> bool {
    if !element.flags.selectable || !layer.selectable {
        return false;
    }
    let any_locked = group_chain.iter().any(|g| {
        doc.group(*g)
            .map(|grp: &Group| grp.locked)
            .unwrap_or(false)
    });
    if any_locked {
        return false;
    }
    // When focusing on the selection, only selected ids are pickable too.
    if filters.only_selected && !filters.selected.contains(&element.id) {
        return false;
    }
    true
}

fn attributes_pass(element: &Element, predicates: &[(String, Value)]) -> bool {
    predicates.iter().all(|(k, v)| match element.attributes.get(k) {
        Some(actual) => actual == v,
        None => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::GeoPoint;
    use crate::model::geometry::{Geometry, Polyline};
    use crate::model::ids::LayerId;
    use crate::model::Document;
    use std::collections::HashSet;

    fn doc_with_layers() -> (Document, LayerId, LayerId) {
        let mut doc = Document::default();
        let a = doc.new_layer("A");
        let b = doc.new_layer("B");
        (doc, a, b)
    }

    fn add_point(doc: &mut Document, layer: LayerId, lon: f64, lat: f64) -> ElementId {
        let ne = doc.make_element("p", Geometry::Point(GeoPoint::surface(lon, lat)));
        let id = ne.id;
        doc.add_element_to_layer(layer, ne);
        id
    }

    fn view(mode: ViewMode, px: f64, mpp: f64) -> ViewContext {
        ViewContext {
            mode,
            pixels_per_world: px,
            meters_per_pixel: mpp,
            ..Default::default()
        }
    }

    #[test]
    fn master_switch_gates_everything() {
        let (mut doc, a, _b) = doc_with_layers();
        let e = add_point(&mut doc, a, 0.0, 0.0);
        let off = Filters::default(); // overlay_enabled false
        let r = eval_visibility(&doc, &view(ViewMode::Globe, 0.0, 0.0), &off);
        assert!(r.visible.is_empty() && r.pickable.is_empty());

        let on = Filters::enabled();
        let r = eval_visibility(&doc, &view(ViewMode::Globe, 0.0, 0.0), &on);
        assert!(r.visible.contains(&e));
    }

    #[test]
    fn layer_visibility_hides_members() {
        let (mut doc, a, b) = doc_with_layers();
        let ea = add_point(&mut doc, a, 0.0, 0.0);
        let eb = add_point(&mut doc, b, 1.0, 1.0);
        doc.layer_mut(a).unwrap().visible = false;
        let r = eval_visibility(&doc, &view(ViewMode::Globe, 0.0, 0.0), &Filters::enabled());
        assert!(!r.visible.contains(&ea), "hidden layer hides its element");
        assert!(r.visible.contains(&eb));
    }

    #[test]
    fn manual_flag_and_group_chain_inherit() {
        let (mut doc, a, _b) = doc_with_layers();
        let g = doc.new_group_in_layer(a, "g");
        let ne = doc.make_element("e", Geometry::Point(GeoPoint::surface(0.0, 0.0)));
        let e = ne.id;
        doc.add_element_to_group(g, ne);
        let f = Filters::enabled();
        let v = view(ViewMode::Globe, 0.0, 0.0);

        assert!(eval_visibility(&doc, &v, &f).visible.contains(&e));
        // Hide the group → element disappears.
        doc.group_mut(g).unwrap().visible = false;
        assert!(!eval_visibility(&doc, &v, &f).visible.contains(&e));
        doc.group_mut(g).unwrap().visible = true;
        // Element's own manual off.
        doc.element_mut(e).unwrap().flags.visible_manual = false;
        assert!(!eval_visibility(&doc, &v, &f).visible.contains(&e));
    }

    #[test]
    fn type_dimension_filters_kinds() {
        let (mut doc, a, _b) = doc_with_layers();
        let pt = add_point(&mut doc, a, 0.0, 0.0);
        let line = {
            let ne = doc.make_element(
                "line",
                Geometry::Polyline(Polyline {
                    positions: vec![GeoPoint::surface(0.0, 0.0), GeoPoint::surface(1.0, 1.0)],
                }),
            );
            let id = ne.id;
            doc.add_element_to_layer(a, ne);
            id
        };
        let mut only_points = HashSet::new();
        only_points.insert(crate::model::geometry::GeometryKind::Point);
        let f = Filters {
            enabled_types: Some(only_points),
            ..Filters::enabled()
        };
        let r = eval_visibility(&doc, &view(ViewMode::Globe, 0.0, 0.0), &f);
        assert!(r.visible.contains(&pt));
        assert!(!r.visible.contains(&line));
    }

    #[test]
    fn view_mode_dimension() {
        let (mut doc, a, _b) = doc_with_layers();
        let e = add_point(&mut doc, a, 0.0, 0.0);
        doc.element_mut(e).unwrap().style.show_in_globe = false; // flat-only
        // Globe view hides it, flat view shows it.
        let rg = eval_visibility(&doc, &view(ViewMode::Globe, 0.0, 0.0), &Filters::enabled());
        assert!(!rg.visible.contains(&e));
        let rf = eval_visibility(&doc, &view(ViewMode::Flat, 0.0, 0.0), &Filters::enabled());
        assert!(rf.visible.contains(&e));
    }

    #[test]
    fn scale_band_dimension() {
        let (mut doc, a, _b) = doc_with_layers();
        let e = add_point(&mut doc, a, 0.0, 0.0);
        doc.element_mut(e).unwrap().scale_visibility.min_pixels_per_world = Some(100.0);
        let f = Filters::enabled();
        // zoomed out (px=50) → hidden; zoomed in (px=200) → shown.
        assert!(!eval_visibility(&doc, &view(ViewMode::Flat, 50.0, 0.0), &f).visible.contains(&e));
        assert!(eval_visibility(&doc, &view(ViewMode::Flat, 200.0, 0.0), &f).visible.contains(&e));
    }

    #[test]
    fn selection_focus_dimension_narrows_both_sets() {
        let (mut doc, a, _b) = doc_with_layers();
        let e1 = add_point(&mut doc, a, 0.0, 0.0);
        let e2 = add_point(&mut doc, a, 1.0, 1.0);
        let f = Filters {
            only_selected: true,
            selected: [e1].into_iter().collect(),
            ..Filters::enabled()
        };
        let r = eval_visibility(&doc, &view(ViewMode::Globe, 0.0, 0.0), &f);
        assert_eq!(r.visible, [e1].into_iter().collect::<BTreeSet<_>>());
        assert!(r.pickable.contains(&e1));
        assert!(!r.pickable.contains(&e2));
    }

    #[test]
    fn attribute_dimension_and_semantics() {
        let (mut doc, a, _b) = doc_with_layers();
        let friend = add_point(&mut doc, a, 0.0, 0.0);
        doc.element_mut(friend)
            .unwrap()
            .attributes
            .insert("side".into(), Value::String("friend".into()));
        let hostile = add_point(&mut doc, a, 1.0, 1.0);
        doc.element_mut(hostile)
            .unwrap()
            .attributes
            .insert("side".into(), Value::String("hostile".into()));

        let f = Filters {
            attribute_predicates: vec![("side".into(), Value::String("friend".into()))],
            ..Filters::enabled()
        };
        let r = eval_visibility(&doc, &view(ViewMode::Globe, 0.0, 0.0), &f);
        assert!(r.visible.contains(&friend));
        assert!(!r.visible.contains(&hostile));
    }

    #[test]
    fn locked_group_and_unselectable_layer_block_pickable_only() {
        let (mut doc, a, _b) = doc_with_layers();
        let g = doc.new_group_in_layer(a, "g");
        let ne = doc.make_element("e", Geometry::Point(GeoPoint::surface(0.0, 0.0)));
        let e = ne.id;
        doc.add_element_to_group(g, ne);
        doc.group_mut(g).unwrap().locked = true;
        let r = eval_visibility(&doc, &view(ViewMode::Globe, 0.0, 0.0), &Filters::enabled());
        assert!(r.visible.contains(&e), "locked but still visible");
        assert!(!r.pickable.contains(&e), "locked group is not pickable");
    }

    #[test]
    fn unselectable_layer_blocks_pickable() {
        let (mut doc, a, _b) = doc_with_layers();
        let e = add_point(&mut doc, a, 0.0, 0.0);
        doc.layer_mut(a).unwrap().selectable = false;
        let r = eval_visibility(&doc, &view(ViewMode::Globe, 0.0, 0.0), &Filters::enabled());
        assert!(r.visible.contains(&e));
        assert!(!r.pickable.contains(&e));
    }
}
