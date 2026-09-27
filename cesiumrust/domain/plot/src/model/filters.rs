//! The independent visibility "dimensions" a user toggles (plan §10). Each is
//! folded with logical AND inside `eval_visibility`; all default to pass-through
//! so a bare document renders fully.

use std::collections::{BTreeSet, HashSet};

use serde::{Deserialize, Serialize};

use super::geometry::GeometryKind;
use super::ids::ElementId;

/// Filter switches evaluated per element by `crate::visibility::eval_visibility`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Filters {
    /// §10.1 master overlay switch. `false` hides the whole overlay.
    pub overlay_enabled: bool,
    /// §10.5 type dimension. `None` = every kind passes; `Some(set)` allows
    /// only the listed kinds.
    pub enabled_types: Option<HashSet<GeometryKind>>,
    /// §10.8 selection focus. When `true`, only ids present in `selected` are
    /// shown (the rest are treated as hidden, not merely dimmed, in M1).
    pub only_selected: bool,
    /// The current selection, consulted when `only_selected` is set.
    pub selected: BTreeSet<ElementId>,
    /// §10.9 attribute dimension. A list of `key == value` predicates; an
    /// element passes only if it satisfies **all** of them (AND). Empty = pass.
    pub attribute_predicates: Vec<(String, serde_json::Value)>,
}

impl Filters {
    /// Everything on: overlay enabled, no type restriction, no selection focus,
    /// no attribute predicates.
    pub fn none() -> Self {
        Self::default()
    }

    /// A filter set with the master switch on (the default `none()` has
    /// `overlay_enabled == false` because `Default` is all-false).
    pub fn enabled() -> Self {
        Self {
            overlay_enabled: true,
            ..Self::default()
        }
    }

    /// Toggle the §10.1 master overlay switch.
    pub fn toggle_overlay(&mut self) {
        self.overlay_enabled = !self.overlay_enabled;
    }

    /// Whether `kind` passes the type dimension (§10.5). `None` (no restriction)
    /// means every kind is on.
    pub fn type_enabled(&self, kind: GeometryKind) -> bool {
        match &self.enabled_types {
            None => true,
            Some(set) => set.contains(&kind),
        }
    }

    /// Flip one type switch. A `None` (all-on) restriction materialises into the
    /// full set first so a single kind can be turned off in isolation.
    pub fn toggle_type(&mut self, kind: GeometryKind) {
        let set = self
            .enabled_types
            .get_or_insert_with(|| GeometryKind::ALL.iter().copied().collect());
        if !set.remove(&kind) {
            set.insert(kind);
        }
    }

    /// Toggle the §10.8 selection-focus switch.
    pub fn toggle_only_selected(&mut self) {
        self.only_selected = !self.only_selected;
    }

    /// Add or replace a §10.9 `key == value` attribute predicate. Keys are
    /// unique: re-adding an existing key overwrites its value in place.
    pub fn add_attribute_predicate(&mut self, key: impl Into<String>, value: serde_json::Value) {
        let key = key.into();
        if let Some(slot) = self
            .attribute_predicates
            .iter_mut()
            .find(|(k, _)| *k == key)
        {
            slot.1 = value;
        } else {
            self.attribute_predicates.push((key, value));
        }
    }

    /// Remove every predicate on `key`; returns whether anything was dropped.
    pub fn remove_attribute_predicate(&mut self, key: &str) -> bool {
        let before = self.attribute_predicates.len();
        self.attribute_predicates.retain(|(k, _)| k != key);
        self.attribute_predicates.len() != before
    }

    /// Drop all attribute predicates.
    pub fn clear_attribute_predicates(&mut self) {
        self.attribute_predicates.clear();
    }

    /// Whether any §10.5 / §10.8 / §10.9 restriction is active (used by the
    /// panel to badge the filter controls).
    pub fn is_restrictive(&self) -> bool {
        self.enabled_types.is_some() || self.only_selected || !self.attribute_predicates.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_disables_overlay() {
        assert!(!Filters::default().overlay_enabled);
    }

    #[test]
    fn enabled_turns_master_on() {
        let f = Filters::enabled();
        assert!(f.overlay_enabled);
        assert!(f.enabled_types.is_none());
        assert!(f.attribute_predicates.is_empty());
    }

    #[test]
    fn type_toggle_materialises_all_then_isolates_one() {
        let mut f = Filters::enabled();
        // No restriction yet → every kind reads as enabled.
        for k in GeometryKind::all() {
            assert!(f.type_enabled(*k));
        }
        // Turning one off materialises the full set minus that kind.
        f.toggle_type(GeometryKind::Polygon);
        assert!(!f.type_enabled(GeometryKind::Polygon));
        assert!(f.type_enabled(GeometryKind::Point));
        // Turning it back on re-enables it.
        f.toggle_type(GeometryKind::Polygon);
        assert!(f.type_enabled(GeometryKind::Polygon));
    }

    #[test]
    fn overlay_and_focus_toggles_flip() {
        let mut f = Filters::enabled();
        f.toggle_overlay();
        assert!(!f.overlay_enabled);
        f.toggle_only_selected();
        assert!(f.only_selected);
    }

    #[test]
    fn attribute_predicates_add_replace_remove() {
        let mut f = Filters::enabled();
        assert!(!f.is_restrictive());
        f.add_attribute_predicate("side", serde_json::json!("friend"));
        assert_eq!(f.attribute_predicates.len(), 1);
        assert!(f.is_restrictive());
        // Re-adding the same key replaces (never duplicates).
        f.add_attribute_predicate("side", serde_json::json!("hostile"));
        assert_eq!(f.attribute_predicates.len(), 1);
        assert_eq!(f.attribute_predicates[0].1, serde_json::json!("hostile"));
        // A second key appends.
        f.add_attribute_predicate("alt", serde_json::json!(12));
        assert_eq!(f.attribute_predicates.len(), 2);
        assert!(f.remove_attribute_predicate("side"));
        assert!(!f.remove_attribute_predicate("missing"));
        assert_eq!(f.attribute_predicates.len(), 1);
        f.clear_attribute_predicates();
        assert!(f.attribute_predicates.is_empty());
    }
}
