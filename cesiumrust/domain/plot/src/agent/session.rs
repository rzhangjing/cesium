//! Agent-facing **orchestration façade** (plan P3).
//!
//! [`PlotSession`] bundles the two pieces of server state an agent edits — the
//! [`Document`] and its [`HistoryStack`] — behind one in-process entry point. It
//! is the single surface every host (a Bevy bridge, a CLI, a future MCP shim)
//! shares: it owns nothing engine-specific, holds no renderer reference, and is
//! therefore fully unit-testable headless.
//!
//! The session is intentionally thin — it forwards to the [`action`](super::action)
//! write path, the [`query`](super::query) read path and the
//! [`schema`](super::schema) document I/O. Keeping it a thin aggregator (rather
//! than re-implementing logic) means the pure functions stay the single source of
//! truth and the invariants proven there hold here too.

use crate::agent::action::{apply_action, ActionError, AgentAction, Applied};
use crate::agent::query::{query, ElementSummary, QueryFilter};
use crate::agent::schema::{export_document_json, import_document_json};
use crate::io::PlotIoError;
use crate::model::ids::ElementId;
use crate::model::{Document, ViewContext};
use crate::ops::HistoryStack;

/// A plot document plus its undo / redo history, driven through a single
/// agent-facing API.
#[derive(Debug)]
pub struct PlotSession {
    /// The live scene document. Public so a host can read / borrow it directly
    /// (e.g. to render it); every *mutation* should still go through [`Self::apply`]
    /// so it stays undoable and validated.
    pub doc: Document,
    /// The undo / redo recorder kept in lock-step with `doc`.
    pub history: HistoryStack,
}

impl Default for PlotSession {
    /// An empty session with no layers (a `Create` must resolve a layer first).
    fn default() -> Self {
        Self {
            doc: Document::default(),
            history: HistoryStack::new(),
        }
    }
}

impl PlotSession {
    /// An empty session (no layers yet — add one or use [`Self::with_default_layer`]).
    pub fn new() -> Self {
        Self::default()
    }

    /// A session pre-seeded with a single default layer, ready to accept
    /// [`AgentAction::Create`] without an explicit target layer.
    pub fn with_default_layer() -> Self {
        Self {
            doc: Document::with_default_layer(),
            history: HistoryStack::new(),
        }
    }

    /// Validate, compile, apply and record one action as a single undo step.
    /// A rejected action short-circuits in validation and mutates nothing.
    pub fn apply(&mut self, action: AgentAction) -> Result<Applied, ActionError> {
        apply_action(&mut self.doc, &mut self.history, &action)
    }

    /// Apply actions in order, each its own undo step. Stops at the first error;
    /// already-applied steps stay committed (they are separate undo steps, so a
    /// host can roll them back one `undo` at a time).
    pub fn apply_all(&mut self, actions: Vec<AgentAction>) -> Result<Vec<Applied>, ActionError> {
        let mut applied = Vec::with_capacity(actions.len());
        for action in actions {
            applied.push(self.apply(action)?);
        }
        Ok(applied)
    }

    /// Undo the most recent step, returning the element ids it touched.
    pub fn undo(&mut self) -> Option<Vec<ElementId>> {
        self.history.undo(&mut self.doc)
    }

    /// Redo the most recently undone step, returning the touched ids.
    pub fn redo(&mut self) -> Option<Vec<ElementId>> {
        self.history.redo(&mut self.doc)
    }

    /// Whether an undo is available.
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// Whether a redo is available.
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// Number of recorded undo steps (for a "undo N" affordance).
    pub fn undo_len(&self) -> usize {
        self.history.undo_len()
    }

    /// Number of pending redo steps.
    pub fn redo_len(&self) -> usize {
        self.history.redo_len()
    }

    /// Replace the document from an [`Self::export_doc`] payload (or any GeoJSON
    /// `FeatureCollection`, imported best-effort). Clears history, since prior
    /// steps refer to the discarded document.
    pub fn import_doc(&mut self, text: &str) -> Result<(), PlotIoError> {
        self.doc = import_document_json(text)?;
        self.history.clear();
        Ok(())
    }

    /// Serialise the whole document (lossless GeoJSON).
    pub fn export_doc(&self) -> String {
        export_document_json(&self.doc)
    }

    /// Read-only situational query over the current document, evaluated with no
    /// view context: each summary's `visible` reflects only the manual flag and
    /// the layer switch. Use [`Self::query_with_view`] to additionally honour the
    /// scale band and per-mode show flags.
    pub fn query(&self, filter: &QueryFilter) -> Vec<ElementSummary> {
        self.query_with_view(filter, None)
    }

    /// Like [`Self::query`], but threads in an optional [`ViewContext`] so the
    /// computed `visible` flag also respects each element's scale band and
    /// per-mode (`show_in_flat` / `show_in_globe`) show flags — the same view
    /// state the renderer would apply. Pass `None` to skip that gating.
    pub fn query_with_view(
        &self,
        filter: &QueryFilter,
        view: Option<&ViewContext>,
    ) -> Vec<ElementSummary> {
        query(&self.doc, filter, view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::action::StylePatch;
    use crate::geo::GeoPoint;
    use crate::ops::DrawKind;
    use serde_json::{Map, Value};

    fn create(lon: f64, name: &str) -> AgentAction {
        AgentAction::Create {
            kind: DrawKind::Point,
            positions: vec![GeoPoint::surface(lon, 0.0)],
            name: Some(name.into()),
            style: None,
            attributes: None,
            layer: None,
        }
    }

    #[test]
    fn scripted_session_end_to_end() {
        let mut s = PlotSession::with_default_layer();

        // 1. Build three elements, remembering their ids.
        let a = s.apply(create(0.0, "alpha")).unwrap().new_id.unwrap();
        let b = s.apply(create(10.0, "bravo")).unwrap().new_id.unwrap();
        let c = s.apply(create(20.0, "charlie")).unwrap().new_id.unwrap();
        assert_eq!(s.doc.element_count(), 3);
        assert_eq!(s.history.undo_len(), 3);

        // 2. Move one.
        s.apply(AgentAction::Move {
            target: a,
            delta_lonlat: [5.0, 0.0],
        })
        .unwrap();
        match &s.doc.element(a).unwrap().geometry {
            crate::model::Geometry::Point(p) => assert_eq!((p.lon_deg, p.lat_deg), (5.0, 0.0)),
            other => panic!("{other:?}"),
        }

        // 3. Restyle another + set an attribute.
        s.apply(AgentAction::Style {
            target: b,
            patch: StylePatch {
                color: Some([1.0, 0.0, 0.0, 1.0]),
                ..Default::default()
            },
        })
        .unwrap();
        assert_eq!(s.doc.element(b).unwrap().style.color, [1.0, 0.0, 0.0, 1.0]);

        // 4. Query sees all three (name filter narrows it).
        let all = s.query(&QueryFilter::default());
        assert_eq!(all.len(), 3);
        let named = s.query(&QueryFilter {
            name_contains: Some("bravo".into()),
            ..Default::default()
        });
        assert_eq!(named.len(), 1);
        assert_eq!(named[0].id, b);

        // 5. Undo every step back to empty.
        assert!(s.can_undo());
        while s.can_undo() {
            s.undo();
        }
        assert_eq!(s.doc.element_count(), 0);
        // Redo walks them all back.
        while s.can_redo() {
            s.redo();
        }
        assert_eq!(s.doc.element_count(), 3);
        assert_eq!(s.doc.element(c).unwrap().name, "charlie");
    }

    #[test]
    fn apply_all_stops_at_first_error_and_keeps_prefix() {
        let mut s = PlotSession::with_default_layer();
        let err = s
            .apply_all(vec![
                create(0.0, "ok"),
                AgentAction::Delete {
                    target: ElementId(999),
                },
                create(1.0, "never"),
            ])
            .unwrap_err();
        assert_eq!(err, ActionError::UnknownTarget(ElementId(999)));
        // The first create committed; the failing delete and the trailing
        // create never ran.
        assert_eq!(s.doc.element_count(), 1);
        assert_eq!(s.history.undo_len(), 1);
    }

    #[test]
    fn export_import_roundtrips_the_session() {
        let mut s = PlotSession::with_default_layer();
        s.apply(create(0.0, "alpha")).unwrap();
        s.apply(create(5.0, "bravo")).unwrap();
        let text = s.export_doc();

        let mut s2 = PlotSession::new();
        s2.import_doc(&text).unwrap();
        assert_eq!(s2.doc.element_count(), 2);
        // Import clears history (steps referred to the discarded document).
        assert!(!s2.can_undo());
        // And the restored document equals the exported one.
        assert_eq!(s2.doc, s.doc);
    }

    #[test]
    fn new_session_has_no_layer_so_create_needs_one() {
        let mut s = PlotSession::new();
        let err = s.apply(create(0.0, "orphan")).unwrap_err();
        assert_eq!(err, ActionError::NoActiveLayer);
        // Supplying a layer explicitly is not possible without one existing, so
        // the with_default_layer constructor is the convenience path.
        assert_eq!(s.doc.element_count(), 0);
    }

    #[test]
    fn query_visible_defaults_true_without_view() {
        let mut s = PlotSession::with_default_layer();
        let id = s.apply(create(0.0, "v")).unwrap().new_id.unwrap();
        let rows = s.query(&QueryFilter::default());
        assert_eq!(rows.len(), 1);
        assert!(rows[0].visible, "manual + layer both default to visible");

        let mut merge = Map::new();
        merge.insert("kind".into(), Value::String("recon".into()));
        s.apply(AgentAction::SetAttributes {
            target: id,
            merge,
        })
        .unwrap();
        s.apply(AgentAction::SetVisible {
            target: id,
            visible: false,
        })
        .unwrap();
        assert!(!s.query(&QueryFilter::default())[0].visible);
    }
}
