//! Agent-facing **intent protocol** (plan P0).
//!
//! An external agent (LLM tool-caller, script, service) never touches the
//! renderer, vertices or GPU — it produces a small, stable, JSON-serialisable
//! [`AgentAction`]. [`compile`] turns an action into a reversible
//! [`PlotCommand`] by filling in the facts the server owns and the agent must
//! *not* guess: the `before` state it reads from the live [`Document`], the
//! freshly-minted element id, and the resolved target layer. [`apply_action`]
//! then applies + records the command as one undo step.
//!
//! Why a façade instead of exposing [`PlotCommand`] directly: commands carry
//! `before` / `id` / `layer`, are an internal recording unit, and would freeze
//! implementation details into the public contract. The intent layer keeps the
//! outside stable while the inside evolves, and it is where every write is
//! *validated* (unknown id, too-few points, no active layer) before it can touch
//! the document.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::geo::GeoPoint;
use crate::model::geometry::Geometry;
use crate::model::ids::{ElementId, LayerId};
use crate::model::{Document, HeightReference, Rgba, Style};
use crate::ops::transform;
use crate::ops::{commit_draft, DrawKind, HistoryStack, PlotCommand};

/// A minimal, incremental style edit — every field optional, only the present
/// ones are copied onto the element's current [`Style`] by
/// [`StylePatch::apply_to`]. Mirrors the agent-relevant subset of `Style`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StylePatch {
    pub color: Option<Rgba>,
    pub opacity: Option<f32>,
    pub width_px: Option<f32>,
    /// Polygon fill colour; a `Some(_)` sets it, `None` leaves it untouched.
    pub fill: Option<Rgba>,
    pub point_size_px: Option<f32>,
    pub z_order: Option<i32>,
    pub height_reference: Option<HeightReference>,
    pub depth_test: Option<bool>,
    pub show_in_flat: Option<bool>,
    pub show_in_globe: Option<bool>,
}

impl StylePatch {
    /// Fold this patch over `base`, returning the new style.
    pub fn apply_to(&self, base: &Style) -> Style {
        let mut s = base.clone();
        if let Some(v) = self.color {
            s.color = v;
        }
        if let Some(v) = self.opacity {
            s.opacity = v;
        }
        if let Some(v) = self.width_px {
            s.width_px = v;
        }
        if let Some(v) = self.fill {
            s.fill = Some(v);
        }
        if let Some(v) = self.point_size_px {
            s.point_size_px = v;
        }
        if let Some(v) = self.z_order {
            s.z_order = v;
        }
        if let Some(v) = self.height_reference {
            s.height_reference = v;
        }
        if let Some(v) = self.depth_test {
            s.depth_test = v;
        }
        if let Some(v) = self.show_in_flat {
            s.show_in_flat = v;
        }
        if let Some(v) = self.show_in_globe {
            s.show_in_globe = v;
        }
        s
    }
}

/// A single high-level editing intent. It deliberately carries **no** server-owned
/// facts (no `before`, no minted id, no resolved layer): [`compile`] supplies
/// those and validates the request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentAction {
    /// Create a primitive from a draw kind + its control points, folded through
    /// the same [`commit_draft`] rules the interactive draw tool uses. Richer
    /// geometry is placed with a follow-up [`AgentAction::SetGeometry`].
    Create {
        kind: DrawKind,
        positions: Vec<GeoPoint>,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        style: Option<StylePatch>,
        #[serde(default)]
        attributes: Option<Map<String, Value>>,
        /// Target layer; defaults to the document's active layer.
        #[serde(default)]
        layer: Option<LayerId>,
    },
    /// Delete an element by id (kept whole in the command so undo restores it).
    Delete { target: ElementId },
    /// Translate every vertex of an element by `(dlon, dlat)` degrees.
    Move {
        target: ElementId,
        delta_lonlat: [f64; 2],
    },
    /// Replace an element's whole geometry (any [`Geometry`] variant).
    SetGeometry {
        target: ElementId,
        geometry: Geometry,
    },
    /// Patch an element's style (only the present fields change).
    Style {
        target: ElementId,
        patch: StylePatch,
    },
    /// Merge free-form business attributes (敌我 / 番号 / 状态 …); existing keys
    /// with the same name are overwritten, others kept.
    SetAttributes {
        target: ElementId,
        merge: Map<String, Value>,
    },
    /// Flip the manual visibility flag.
    SetVisible {
        target: ElementId,
        visible: bool,
    },
    /// Several actions compiled into one [`PlotCommand::Composite`] — a single
    /// undo step. Sub-actions must reference pre-existing elements (a batch does
    /// not resolve references to elements created by an earlier sub-action in the
    /// same batch); use sequential [`apply_action`] calls for that.
    Batch { actions: Vec<AgentAction> },
}

/// Why an action cannot be compiled. Every failure is detected in [`compile`],
/// *before* the document is mutated, so a rejected action leaves the scene
/// unchanged (aside from an invisibly-consumed id counter).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ActionError {
    #[error("no element with id {0:?}")]
    UnknownTarget(ElementId),
    #[error("{kind:?} needs at least {need} points, got {got}")]
    TooFewPoints {
        kind: DrawKind,
        need: usize,
        got: usize,
    },
    #[error("no active layer to add into — set one or create a layer first")]
    NoActiveLayer,
    #[error("geometry rejected by validation: {0}")]
    InvalidGeometry(String),
    #[error("value out of range for field `{field}`")]
    OutOfRange { field: &'static str },
}

/// What an applied action did — enough for a host to reconcile its view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Applied {
    /// Every leaf element id the resulting command touched.
    pub touched_ids: Vec<ElementId>,
    /// The id of a newly created element, when this action created one.
    pub new_id: Option<ElementId>,
}

/// Validate `action` against `doc` and produce the reversible command it maps to
/// — **without** applying it. This reads `before` state, resolves the target
/// layer, and (for a create) mints a fresh id via [`Document::make_element`].
pub fn compile(doc: &mut Document, action: &AgentAction) -> Result<PlotCommand, ActionError> {
    match action {
        AgentAction::Create {
            kind,
            positions,
            name,
            style,
            attributes,
            layer,
        } => {
            let target = (*layer)
                .or_else(|| doc.active_layer())
                .ok_or(ActionError::NoActiveLayer)?;
            let geometry = commit_draft(*kind, positions).ok_or(ActionError::TooFewPoints {
                kind: *kind,
                need: kind.min_points(),
                got: positions.len(),
            })?;
            let mut ne = doc.make_element(name.clone().unwrap_or_default(), geometry);
            if let Some(patch) = style {
                ne.element.style = patch.apply_to(&ne.element.style);
            }
            if let Some(attrs) = attributes {
                for (k, v) in attrs {
                    ne.element.attributes.insert(k.clone(), v.clone());
                }
            }
            Ok(PlotCommand::AddElement {
                layer: target,
                element: Box::new(ne.element),
            })
        }
        AgentAction::Delete { target } => {
            let el = doc
                .element(*target)
                .ok_or(ActionError::UnknownTarget(*target))?
                .clone();
            let layer = doc
                .element_context(*target)
                .map(|(l, _)| l)
                .ok_or(ActionError::UnknownTarget(*target))?;
            Ok(PlotCommand::RemoveElement {
                element: Box::new(el),
                layer,
            })
        }
        AgentAction::Move {
            target,
            delta_lonlat,
        } => {
            let before = doc
                .element(*target)
                .ok_or(ActionError::UnknownTarget(*target))?
                .geometry
                .clone();
            let after = transform::translate(&before, delta_lonlat[0], delta_lonlat[1]);
            Ok(PlotCommand::UpdateGeometry {
                id: *target,
                before: Box::new(before),
                after: Box::new(after),
            })
        }
        AgentAction::SetGeometry { target, geometry } => {
            let before = doc
                .element(*target)
                .ok_or(ActionError::UnknownTarget(*target))?
                .geometry
                .clone();
            Ok(PlotCommand::UpdateGeometry {
                id: *target,
                before: Box::new(before),
                after: Box::new(geometry.clone()),
            })
        }
        AgentAction::Style { target, patch } => {
            let before = doc
                .element(*target)
                .ok_or(ActionError::UnknownTarget(*target))?
                .style
                .clone();
            let after = patch.apply_to(&before);
            Ok(PlotCommand::SetStyle {
                id: *target,
                before: Box::new(before),
                after: Box::new(after),
            })
        }
        AgentAction::SetAttributes { target, merge } => {
            let before = doc
                .element(*target)
                .ok_or(ActionError::UnknownTarget(*target))?
                .attributes
                .clone();
            let mut after = before.clone();
            for (k, v) in merge {
                after.insert(k.clone(), v.clone());
            }
            Ok(PlotCommand::SetAttributes {
                id: *target,
                before,
                after,
            })
        }
        AgentAction::SetVisible { target, visible } => {
            let before = doc
                .element(*target)
                .ok_or(ActionError::UnknownTarget(*target))?
                .flags
                .visible_manual;
            Ok(PlotCommand::SetVisibilityFlag {
                id: *target,
                before,
                after: *visible,
            })
        }
        AgentAction::Batch { actions } => {
            let mut steps = Vec::with_capacity(actions.len());
            for a in actions {
                steps.push(compile(doc, a)?);
            }
            Ok(PlotCommand::Composite { steps })
        }
    }
}

/// Compile, apply and record `action` on `doc` as one undo step. Validation
/// failures short-circuit before any mutation and are **not** recorded.
pub fn apply_action(
    doc: &mut Document,
    history: &mut HistoryStack,
    action: &AgentAction,
) -> Result<Applied, ActionError> {
    let command = compile(doc, action)?;
    let new_id = match &command {
        PlotCommand::AddElement { element, .. } => Some(element.id),
        _ => None,
    };
    command.apply(doc);
    let touched_ids = command.targets();
    history.record(command);
    Ok(Applied {
        touched_ids,
        new_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ids::ElementId;

    fn pt(lon: f64, lat: f64) -> GeoPoint {
        GeoPoint::surface(lon, lat)
    }

    fn doc() -> Document {
        Document::with_default_layer()
    }

    #[test]
    fn create_adds_and_undo_removes() {
        let mut doc = doc();
        let mut h = HistoryStack::new();
        let applied = apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Create {
                kind: DrawKind::Point,
                positions: vec![pt(3.0, 4.0)],
                name: Some("watch".into()),
                style: None,
                attributes: None,
                layer: None,
            },
        )
        .unwrap();
        let id = applied.new_id.expect("created id reported");
        assert_eq!(doc.element_count(), 1);
        assert_eq!(doc.element(id).unwrap().name, "watch");
        // A single undo removes it again.
        h.undo(&mut doc);
        assert_eq!(doc.element_count(), 0);
    }

    #[test]
    fn create_applies_style_and_attributes() {
        let mut doc = doc();
        let mut h = HistoryStack::new();
        let mut attrs = Map::new();
        attrs.insert("side".into(), Value::String("hostile".into()));
        let applied = apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Create {
                kind: DrawKind::Point,
                positions: vec![pt(0.0, 0.0)],
                name: None,
                style: Some(StylePatch {
                    color: Some([1.0, 0.0, 0.0, 1.0]),
                    ..Default::default()
                }),
                attributes: Some(attrs),
                layer: None,
            },
        )
        .unwrap();
        let e = doc.element(applied.new_id.unwrap()).unwrap();
        assert_eq!(e.style.color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(e.attributes["side"], Value::String("hostile".into()));
        assert_eq!(e.name, "");
    }

    #[test]
    fn move_then_undo_restores_geometry() {
        let mut doc = doc();
        let mut h = HistoryStack::new();
        let id = apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Create {
                kind: DrawKind::Point,
                positions: vec![pt(1.0, 1.0)],
                name: None,
                style: None,
                attributes: None,
                layer: None,
            },
        )
        .unwrap()
        .new_id
        .unwrap();
        apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Move {
                target: id,
                delta_lonlat: [2.0, -3.0],
            },
        )
        .unwrap();
        match &doc.element(id).unwrap().geometry {
            Geometry::Point(p) => assert_eq!((p.lon_deg, p.lat_deg), (3.0, -2.0)),
            g => panic!("{g:?}"),
        }
        h.undo(&mut doc);
        match &doc.element(id).unwrap().geometry {
            Geometry::Point(p) => assert_eq!((p.lon_deg, p.lat_deg), (1.0, 1.0)),
            g => panic!("{g:?}"),
        }
    }

    #[test]
    fn set_geometry_and_style_are_undoable() {
        let mut doc = doc();
        let mut h = HistoryStack::new();
        let id = apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Create {
                kind: DrawKind::Point,
                positions: vec![pt(0.0, 0.0)],
                name: None,
                style: None,
                attributes: None,
                layer: None,
            },
        )
        .unwrap()
        .new_id
        .unwrap();
        // Set geometry to a polyline.
        apply_action(
            &mut doc,
            &mut h,
            &AgentAction::SetGeometry {
                target: id,
                geometry: Geometry::Polyline(crate::model::geometry::Polyline {
                    positions: vec![pt(0.0, 0.0), pt(1.0, 1.0)],
                }),
            },
        )
        .unwrap();
        assert_eq!(doc.element(id).unwrap().geometry.kind(), crate::model::GeometryKind::Line);
        h.undo(&mut doc);
        assert_eq!(doc.element(id).unwrap().geometry.kind(), crate::model::GeometryKind::Point);
        // Style patch colour then undo.
        apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Style {
                target: id,
                patch: StylePatch {
                    width_px: Some(9.0),
                    ..Default::default()
                },
            },
        )
        .unwrap();
        assert_eq!(doc.element(id).unwrap().style.width_px, 9.0);
        h.undo(&mut doc);
        assert_ne!(doc.element(id).unwrap().style.width_px, 9.0);
    }

    #[test]
    fn set_attributes_merges_and_undo_restores() {
        let mut doc = doc();
        let mut h = HistoryStack::new();
        let id = apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Create {
                kind: DrawKind::Point,
                positions: vec![pt(0.0, 0.0)],
                name: None,
                style: None,
                attributes: Some({
                    let mut m = Map::new();
                    m.insert("a".into(), Value::from(1));
                    m
                }),
                layer: None,
            },
        )
        .unwrap()
        .new_id
        .unwrap();
        apply_action(
            &mut doc,
            &mut h,
            &AgentAction::SetAttributes {
                target: id,
                merge: {
                    let mut m = Map::new();
                    m.insert("b".into(), Value::from(2));
                    m
                },
            },
        )
        .unwrap();
        let e = doc.element(id).unwrap();
        assert_eq!(e.attributes["a"], Value::from(1));
        assert_eq!(e.attributes["b"], Value::from(2));
        h.undo(&mut doc);
        // Undo drops the merged key but keeps the original.
        let e = doc.element(id).unwrap();
        assert!(e.attributes.contains_key("a"));
        assert!(!e.attributes.contains_key("b"));
    }

    #[test]
    fn set_visible_roundtrips() {
        let mut doc = doc();
        let mut h = HistoryStack::new();
        let id = apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Create {
                kind: DrawKind::Point,
                positions: vec![pt(0.0, 0.0)],
                name: None,
                style: None,
                attributes: None,
                layer: None,
            },
        )
        .unwrap()
        .new_id
        .unwrap();
        apply_action(&mut doc, &mut h, &AgentAction::SetVisible { target: id, visible: false })
            .unwrap();
        assert!(!doc.element(id).unwrap().flags.visible_manual);
        h.undo(&mut doc);
        assert!(doc.element(id).unwrap().flags.visible_manual);
    }

    #[test]
    fn delete_then_undo_restores_the_element() {
        let mut doc = doc();
        let mut h = HistoryStack::new();
        let id = apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Create {
                kind: DrawKind::Point,
                positions: vec![pt(5.0, 6.0)],
                name: None,
                style: None,
                attributes: None,
                layer: None,
            },
        )
        .unwrap()
        .new_id
        .unwrap();
        apply_action(&mut doc, &mut h, &AgentAction::Delete { target: id }).unwrap();
        assert!(doc.element(id).is_none());
        h.undo(&mut doc);
        let back = doc.element(id).expect("restored by undo");
        match &back.geometry {
            Geometry::Point(p) => assert_eq!((p.lon_deg, p.lat_deg), (5.0, 6.0)),
            g => panic!("{g:?}"),
        }
    }

    #[test]
    fn batch_is_a_single_undo_step() {
        let mut doc = doc();
        let mut h = HistoryStack::new();
        let mk = |lon: f64| AgentAction::Create {
            kind: DrawKind::Point,
            positions: vec![pt(lon, 0.0)],
            name: None,
            style: None,
            attributes: None,
            layer: None,
        };
        apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Batch {
                actions: vec![mk(1.0), mk(2.0), mk(3.0)],
            },
        )
        .unwrap();
        assert_eq!(doc.element_count(), 3);
        assert_eq!(h.undo_len(), 1, "batch is one undo step");
        h.undo(&mut doc);
        assert_eq!(doc.element_count(), 0);
    }

    #[test]
    fn unknown_target_errors_without_mutation() {
        let mut doc = doc();
        let mut h = HistoryStack::new();
        let err = apply_action(&mut doc, &mut h, &AgentAction::Delete { target: ElementId(999) })
            .unwrap_err();
        assert_eq!(err, ActionError::UnknownTarget(ElementId(999)));
        assert_eq!(doc.element_count(), 0);
        assert!(!h.can_undo(), "a failed action is not recorded");
    }

    #[test]
    fn too_few_points_errors() {
        let mut doc = doc();
        let mut h = HistoryStack::new();
        let err = apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Create {
                kind: DrawKind::Polygon,
                positions: vec![pt(0.0, 0.0), pt(1.0, 1.0)],
                name: None,
                style: None,
                attributes: None,
                layer: None,
            },
        )
        .unwrap_err();
        assert_eq!(
            err,
            ActionError::TooFewPoints {
                kind: DrawKind::Polygon,
                need: 3,
                got: 2
            }
        );
        assert_eq!(doc.element_count(), 0);
        assert!(!h.can_undo());
    }

    #[test]
    fn no_active_layer_errors() {
        // A document with no layers at all.
        let mut doc = Document::default();
        let mut h = HistoryStack::new();
        let err = apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Create {
                kind: DrawKind::Point,
                positions: vec![pt(0.0, 0.0)],
                name: None,
                style: None,
                attributes: None,
                layer: None,
            },
        )
        .unwrap_err();
        assert_eq!(err, ActionError::NoActiveLayer);
    }

    #[test]
    fn actions_roundtrip_through_json() {
        let a = AgentAction::Create {
            kind: DrawKind::Polyline,
            positions: vec![pt(0.0, 0.0), pt(1.0, 2.0)],
            name: Some("route".into()),
            style: Some(StylePatch {
                color: Some([0.0, 1.0, 0.0, 1.0]),
                ..Default::default()
            }),
            attributes: None,
            layer: None,
        };
        let s = serde_json::to_string(&a).unwrap();
        let back: AgentAction = serde_json::from_str(&s).unwrap();
        assert_eq!(a, back);
    }
}
