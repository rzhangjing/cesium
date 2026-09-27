//! The undo / redo history stack (plan §8 / §14): a pure, view-independent
//! recorder of [`PlotCommand`]s the bridge drives from its edit gestures and
//! keyboard shortcuts.
//!
//! The stack is deliberately dumb so it is trivially testable: it stores the
//! forward commands, applies their inverses on `undo`, and never inspects the
//! [`Document`] itself. `record` clears the redo branch (a fresh edit after an
//! undo invalidates the redo tail), matching every desktop editor's contract.

use crate::model::document::Document;

use super::command::PlotCommand;

/// An unbounded (bounded by an optional cap) undo / redo stack.
#[derive(Debug, Clone, Default)]
pub struct HistoryStack {
    undo: Vec<PlotCommand>,
    redo: Vec<PlotCommand>,
    /// `Some(n)` trims the undo branch to the newest `n` entries; `None` = unlim
    /// ited (the memory footprint of one plot session is small enough to keep).
    cap: Option<usize>,
}

impl HistoryStack {
    /// An unlimited stack.
    pub fn new() -> Self {
        Self::default()
    }

    /// A stack that keeps at most `cap` undo steps.
    pub fn with_cap(cap: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            cap: Some(cap),
        }
    }

    /// Record a just-applied command, dropping the redo branch.
    pub fn record(&mut self, command: PlotCommand) {
        self.undo.push(command);
        self.redo.clear();
        if let Some(cap) = self.cap {
            while self.undo.len() > cap {
                self.undo.remove(0);
            }
        }
    }

    /// True when there is at least one command to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// True when there is at least one command to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Number of recorded undo steps (for a "undo N" label / tests).
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    /// Number of pending redo steps.
    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    /// Undo the most recent command: apply its inverse, move it to the redo
    /// branch. Returns the ids the command touched so the bridge can reconcile
    /// those visuals; `None` when the stack is empty.
    pub fn undo(&mut self, doc: &mut Document) -> Option<Vec<crate::model::ids::ElementId>> {
        let command = self.undo.pop()?;
        let inverse = command.inverse();
        inverse.apply(doc);
        let targets = command.targets();
        self.redo.push(command);
        Some(targets)
    }

    /// Redo the most recently undone command: re-apply it, move it back to the
    /// undo branch. Returns the touched ids; `None` when nothing is redoable.
    pub fn redo(&mut self, doc: &mut Document) -> Option<Vec<crate::model::ids::ElementId>> {
        let command = self.redo.pop()?;
        command.apply(doc);
        let targets = command.targets();
        self.undo.push(command);
        Some(targets)
    }

    /// Forget all history (e.g. after loading a fresh document).
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::GeoPoint;
    use crate::model::geometry::Geometry;
    use crate::model::ids::{ElementId, LayerId};
    use crate::model::{Document, Element};

    fn p(lon: f64, lat: f64) -> GeoPoint {
        GeoPoint::surface(lon, lat)
    }

    fn add(id: u64, layer: LayerId) -> PlotCommand {
        PlotCommand::AddElement {
            layer,
            element: Box::new(Element::new(ElementId(id), "pt", Geometry::Point(p(0.0, 0.0)))),
        }
    }

    /// Mirror the bridge contract: apply a command to the document, then record
    /// it so it becomes undoable.
    fn commit(h: &mut HistoryStack, doc: &mut Document, command: PlotCommand) {
        command.apply(doc);
        h.record(command);
    }

    #[test]
    fn starts_empty() {
        let h = HistoryStack::new();
        assert!(!h.can_undo());
        assert!(!h.can_redo());
    }

    #[test]
    fn undo_redo_walks_the_stack() {
        let mut doc = Document::with_default_layer();
        let layer = doc.active_layer().unwrap();
        let mut h = HistoryStack::new();

        commit(&mut h, &mut doc, add(1, layer));
        commit(&mut h, &mut doc, add(2, layer));
        assert_eq!(doc.element_count(), 2);
        assert!(h.can_undo() && !h.can_redo());

        // Undo the second add → element 2 gone, it becomes redoable.
        let touched = h.undo(&mut doc).unwrap();
        assert_eq!(touched, vec![ElementId(2)]);
        assert!(doc.element(ElementId(2)).is_none());
        assert!(h.can_undo() && h.can_redo());

        // Undo the first → doc empty.
        h.undo(&mut doc);
        assert!(doc.element(ElementId(1)).is_none());
        assert!(!h.can_undo() && h.can_redo());

        // Redo both back in original order.
        h.redo(&mut doc);
        h.redo(&mut doc);
        assert!(doc.element(ElementId(1)).is_some());
        assert!(doc.element(ElementId(2)).is_some());
        assert!(h.can_undo() && !h.can_redo());
    }

    #[test]
    fn recording_after_undo_drops_the_redo_branch() {
        let mut doc = Document::with_default_layer();
        let layer = doc.active_layer().unwrap();
        let mut h = HistoryStack::new();
        commit(&mut h, &mut doc, add(1, layer));
        h.undo(&mut doc);
        assert!(h.can_redo());
        // A new edit clears the redo tail.
        commit(&mut h, &mut doc, add(3, layer));
        assert!(!h.can_redo());
        // The undone add1 left the undo branch empty; add3 is now the sole step.
        assert_eq!(h.undo_len(), 1);
    }

    #[test]
    fn cap_trims_the_oldest_undos() {
        let doc = Document::with_default_layer();
        let layer = doc.active_layer().unwrap();
        let mut h = HistoryStack::with_cap(2);
        h.record(add(1, layer));
        h.record(add(2, layer));
        h.record(add(3, layer));
        assert_eq!(h.undo_len(), 2);
    }

    #[test]
    fn undo_on_empty_stack_is_a_noop() {
        let mut doc = Document::with_default_layer();
        let mut h = HistoryStack::new();
        assert!(h.undo(&mut doc).is_none());
        assert!(h.redo(&mut doc).is_none());
    }
}
