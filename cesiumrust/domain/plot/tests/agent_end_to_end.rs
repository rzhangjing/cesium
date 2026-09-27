//! Hermetic, gate-running demonstration of the agent empowerment façade
//! (plan `Agent 赋能态势标绘`).
//!
//! Unlike the in-crate unit tests, this is an **integration test**: it consumes
//! `cesium-plot` exactly as an external host would — via the public re-exports
//! under `cesium_plot::agent` and **raw JSON payloads** (never constructing Rust
//! intent types by hand). That makes it a two-for-one check:
//!
//!  * it proves the whole façade is actually reachable / ergonomic from outside
//!    the crate (a missing `pub use` fails to compile here, not in production);
//!  * it locks the real "agent → server → agent" loop end to end: pull the
//!    action schema, send a `Create`, read the minted id back, feed that id into
//!    a follow-up `Move`, query / measure the result, undo it, and round-trip the
//!    document — all with a rejected action verified to mutate nothing.

use cesium_plot::agent::{
    action_json_schema, measure_area, measure_length, ActionError, AgentAction, ElementSummary,
    PlotSession, QueryFilter, EXAMPLE_ACTION,
};
use cesium_plot::model::{ElementId, GeometryKind, ViewContext};
use serde_json::{json, Value};

/// The summary for `id` in `rows`, panicking otherwise.
fn only(rows: &[ElementSummary], id: ElementId) -> &ElementSummary {
    rows.iter()
        .find(|e| e.id == id)
        .unwrap_or_else(|| panic!("no summary for {id:?} among {:?}", ids(rows)))
}

fn ids(rows: &[ElementSummary]) -> Vec<ElementId> {
    rows.iter().map(|e| e.id).collect()
}

#[test]
fn agent_drives_a_plot_session_over_json() {
    let mut session = PlotSession::with_default_layer();

    // ── 0. Contract discovery ───────────────────────────────────────────────
    // An agent can self-describe before sending anything: pull the JSON Schema
    // for the action vocabulary and a canonical example payload.
    let schema: Value = action_json_schema();
    assert_eq!(schema["title"], json!("AgentAction"));
    assert!(schema["$defs"]["AgentAction"]["oneOf"].is_array());
    // The example is a real, applyable intent (deserialised like any other input).
    let example: AgentAction = serde_json::from_str(EXAMPLE_ACTION).unwrap();
    let friendly_id = session.apply(example).unwrap().new_id.expect("example creates");

    // ── 1. Write: send a raw-JSON Create (a hostile deny-zone rectangle) ─────
    let create_json = r#"{
        "Create": {
            "kind": "Rectangle",
            "positions": [
                { "lon_deg": 10.0, "lat_deg": 10.0, "height_m": 0.0 },
                { "lon_deg": 20.0, "lat_deg": 20.0, "height_m": 0.0 }
            ],
            "name": "deny-zone",
            "attributes": { "side": "hostile", "priority": 2 }
        }
    }"#;
    let create: AgentAction = serde_json::from_str(create_json).unwrap();
    let zone = session.apply(create).unwrap().new_id.expect("zone created");

    assert_eq!(session.doc.element_count(), 2);
    assert_eq!(session.undo_len(), 2);

    // ── 2. Read: query the picture back (attribute + kind filters) ───────────
    let hostiles = session.query(&QueryFilter {
        attributes: vec![("side".into(), json!("hostile"))],
        ..Default::default()
    });
    assert_eq!(ids(&hostiles), vec![zone], "only the zone is hostile");
    assert_eq!(only(&hostiles, zone).name, "deny-zone");

    let rects = session.query(&QueryFilter {
        kind: Some(GeometryKind::Rectangle),
        ..Default::default()
    });
    assert_eq!(ids(&rects), vec![zone]);

    // Geometry quantities an agent can "understand" the scene by.
    let perimeter = measure_length(&session.doc, zone).unwrap();
    let area = measure_area(&session.doc, zone).unwrap();
    assert!(perimeter > 1_000_000.0, "a 10°×10° zone perimeter, got {perimeter}");
    assert!(area > 1_000_000.0, "a 10°×10° zone area, got {area}");
    // An open polyline measures no enclosed area.
    assert_eq!(measure_area(&session.doc, friendly_id).unwrap(), 0.0);

    // View-aware query: with the default Globe view both elements stay visible,
    // exercising the scale-band / per-mode gating branch of the read path.
    let with_view =
        session.query_with_view(&QueryFilter::default(), Some(&ViewContext::default()));
    assert_eq!(with_view.len(), 2);
    assert!(with_view.iter().all(|e| e.visible));

    // ── 3. Thread the server-minted id back into a follow-up Move ────────────
    let bounds_before = only(&session.query(&QueryFilter::default()), zone).bounds;
    let move_json = format!(
        r#"{{"Move": {{ "target": {}, "delta_lonlat": [5.0, 0.0] }} }}"#,
        zone.raw()
    );
    let mv: AgentAction = serde_json::from_str(&move_json).unwrap();
    let touched = session.apply(mv).unwrap().touched_ids;
    assert_eq!(touched, vec![zone]);

    let bounds_after = only(&session.query(&QueryFilter::default()), zone).bounds;
    assert!(
        (bounds_after.west_deg - bounds_before.west_deg - 5.0).abs() < 1e-9,
        "west should shift +5°: {bounds_before:?} → {bounds_after:?}"
    );

    // ── 4. Undo the move; the zone snaps back exactly ────────────────────────
    assert!(session.can_undo());
    let undone = session.undo().expect("undo reports touched ids");
    assert_eq!(undone, vec![zone]);
    let bounds_restored = only(&session.query(&QueryFilter::default()), zone).bounds;
    assert_eq!(bounds_restored, bounds_before);

    // ── 5. A rejected action is validated first and mutates nothing ──────────
    let bad: AgentAction = serde_json::from_str(r#"{ "Delete": { "target": 424242 } }"#).unwrap();
    let err = session.apply(bad).unwrap_err();
    assert_eq!(err, ActionError::UnknownTarget(ElementId::new(424242)));
    assert_eq!(session.doc.element_count(), 2, "failed action left no trace");

    // ── 6. Export / re-import the whole document through the public façade ───
    let json_text = session.export_doc();
    let mut restored = PlotSession::new();
    restored.import_doc(&json_text).unwrap();
    assert_eq!(restored.doc, session.doc, "lossless document round-trip");
    // Import clears history (prior steps belonged to the discarded document).
    assert!(!restored.can_undo());
}

#[test]
fn batch_and_sequential_creation_differ_in_undo_granularity() {
    // A Batch collapses to ONE undo step; sequential applies are one step each.
    let mut session = PlotSession::with_default_layer();
    let mk = |lon: f64| {
        json!({ "Create": {
            "kind": "Point",
            "positions": [{ "lon_deg": lon, "lat_deg": 0.0, "height_m": 0.0 }]
        }})
    };
    let batch_json = json!({ "Batch": { "actions": [ mk(1.0), mk(2.0), mk(3.0) ] } });
    let batch: AgentAction = serde_json::from_str(&batch_json.to_string()).unwrap();
    session.apply(batch).unwrap();
    assert_eq!(session.doc.element_count(), 3);
    assert_eq!(session.undo_len(), 1, "batch is a single undo step");

    assert!(session.undo().is_some(), "one undo clears the whole batch");
    assert_eq!(session.doc.element_count(), 0);
}
