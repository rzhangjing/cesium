//! Structural document operations (plan §8 / §14): the draw-tool draft commit
//! and the [`PlotCommand`]s that mutate the [`Document`].
//!
//! The command is a *pure* value: applying it to a document is a plain function,
//! so the same command drives the live bridge and (from M6) the history stack's
//! undo by carrying an inverse. M5 introduces the add / remove pair used by the
//! drawing flow; M6 extends with geometry / style / transform / group edits.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::geo::GeoPoint;
use crate::model::document::NewElement;
use crate::model::geometry::{Circle, Geometry, Polyline, Polygon, Rectangle};
use crate::model::ids::{ElementId, LayerId};
use crate::model::{Document, Element, Style};

/// The primitive the draw tool is producing. Drives how a draft vertex list is
/// folded into a concrete geometry ([`commit_draft`]) and how many clicks the
/// tool needs before it can finish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrawKind {
    /// A single point marker (1 click).
    Point,
    /// A free polyline (≥ 2 clicks, finished by Enter / double-click).
    Polyline,
    /// A closed polygon (≥ 3 clicks).
    Polygon,
    /// An axis-aligned rectangle from two opposite corners (2 clicks).
    Rectangle,
    /// A ground circle from centre + a radius point (2 clicks).
    Circle,
}

impl DrawKind {
    /// Fewest draft vertices that can still form this geometry.
    #[inline]
    pub fn min_points(self) -> usize {
        match self {
            DrawKind::Point => 1,
            DrawKind::Polyline => 2,
            DrawKind::Polygon => 3,
            DrawKind::Rectangle | DrawKind::Circle => 2,
        }
    }

    /// A fixed click count that auto-completes the draw (rect / circle / point),
    /// or `None` for open-ended kinds (polyline / polygon) finished by gesture.
    #[inline]
    pub fn fixed_points(self) -> Option<usize> {
        match self {
            DrawKind::Point => Some(1),
            DrawKind::Rectangle | DrawKind::Circle => Some(2),
            DrawKind::Polyline | DrawKind::Polygon => None,
        }
    }
}

/// Fold a completed draft into a concrete geometry, or `None` when there are too
/// few vertices. Rectangles take the two opposite corners' bounds; circles the
/// great-circle radius between centre and the radius point.
pub fn commit_draft(kind: DrawKind, draft: &[GeoPoint]) -> Option<Geometry> {
    if draft.len() < kind.min_points() {
        return None;
    }
    match kind {
        DrawKind::Point => Some(Geometry::Point(draft[0])),
        DrawKind::Polyline => Some(Geometry::Polyline(Polyline {
            positions: draft.to_vec(),
        })),
        DrawKind::Polygon => Some(Geometry::Polygon(Polygon {
            outer: draft.to_vec(),
            holes: Vec::new(),
        })),
        DrawKind::Rectangle => {
            let (a, b) = (draft[0], draft[1]);
            Some(Geometry::Rectangle(Rectangle {
                west: a.lon_deg.min(b.lon_deg),
                east: a.lon_deg.max(b.lon_deg),
                south: a.lat_deg.min(b.lat_deg),
                north: a.lat_deg.max(b.lat_deg),
            }))
        }
        DrawKind::Circle => {
            let center = draft[0];
            let radius_m = center.surface_distance(draft[1]);
            if radius_m <= 0.0 {
                return None;
            }
            Some(Geometry::Circle(Circle { center, radius_m }))
        }
    }
}

/// A reversible structural change to the document (plan §14). The bridge applies
/// it through [`PlotCommand::apply`]; the history stack records it and its
/// [`PlotCommand::inverse`] so an edit can be undone / redone (M6).
#[derive(Debug, Clone, PartialEq)]
pub enum PlotCommand {
    /// Insert a finished element into a layer.
    AddElement {
        layer: LayerId,
        element: Box<Element>,
    },
    /// Delete an element by id (the full element is kept so the op reverses).
    RemoveElement {
        element: Box<Element>,
        layer: LayerId,
    },
    /// Replace an element's geometry (the move / vertex-edit / rotate / scale
    /// result). Both sides are kept so undo restores the previous shape.
    UpdateGeometry {
        id: ElementId,
        before: Box<Geometry>,
        after: Box<Geometry>,
    },
    /// Replace an element's whole style bag (property-panel edits).
    SetStyle {
        id: ElementId,
        before: Box<Style>,
        after: Box<Style>,
    },
    /// Flip an element's manual visibility toggle (layer / panel eye button).
    SetVisibilityFlag {
        id: ElementId,
        before: bool,
        after: bool,
    },
    /// Replace an element's free-form business attributes (敌我 / 番号 / 状态 …).
    /// Both maps are kept so undo restores the previous metadata; the agent layer
    /// (M-agent) emits this for `SetAttributes` merges.
    SetAttributes {
        id: ElementId,
        before: Map<String, Value>,
        after: Map<String, Value>,
    },
    /// A group of commands applied as one undo step (multi-select edits).
    Composite {
        steps: Vec<PlotCommand>,
    },
}

impl PlotCommand {
    /// Mutate `doc` by this command.
    pub fn apply(&self, doc: &mut Document) {
        match self {
            PlotCommand::AddElement { layer, element } => {
                let ne = NewElement {
                    id: element.id,
                    element: (**element).clone(),
                };
                doc.add_element_to_layer(*layer, ne);
            }
            PlotCommand::RemoveElement { element, .. } => {
                doc.remove_element(element.id);
            }
            PlotCommand::UpdateGeometry { id, after, .. } => {
                if let Some(e) = doc.element_mut(*id) {
                    e.set_geometry((**after).clone());
                }
            }
            PlotCommand::SetStyle { id, after, .. } => {
                if let Some(e) = doc.element_mut(*id) {
                    e.style = (**after).clone();
                }
            }
            PlotCommand::SetVisibilityFlag { id, after, .. } => {
                if let Some(e) = doc.element_mut(*id) {
                    e.flags.visible_manual = *after;
                }
            }
            PlotCommand::SetAttributes { id, after, .. } => {
                if let Some(e) = doc.element_mut(*id) {
                    e.attributes = after.clone();
                }
            }
            PlotCommand::Composite { steps } => {
                for s in steps {
                    s.apply(doc);
                }
            }
        }
    }

    /// The command that exactly reverses this one (undo). Re-adding an element
    /// reuses its id, so the redo / undo pair is stable.
    pub fn inverse(&self) -> PlotCommand {
        match self {
            PlotCommand::AddElement { layer, element } => PlotCommand::RemoveElement {
                element: element.clone(),
                layer: *layer,
            },
            PlotCommand::RemoveElement { element, layer } => PlotCommand::AddElement {
                layer: *layer,
                element: element.clone(),
            },
            PlotCommand::UpdateGeometry { id, before, after } => PlotCommand::UpdateGeometry {
                id: *id,
                before: after.clone(),
                after: before.clone(),
            },
            PlotCommand::SetStyle { id, before, after } => PlotCommand::SetStyle {
                id: *id,
                before: after.clone(),
                after: before.clone(),
            },
            PlotCommand::SetVisibilityFlag { id, before, after } => PlotCommand::SetVisibilityFlag {
                id: *id,
                before: *after,
                after: *before,
            },
            PlotCommand::SetAttributes { id, before, after } => PlotCommand::SetAttributes {
                id: *id,
                before: after.clone(),
                after: before.clone(),
            },
            PlotCommand::Composite { steps } => PlotCommand::Composite {
                // Reverse the whole group: undo runs the inverses back-to-front.
                steps: steps.iter().rev().map(PlotCommand::inverse).collect(),
            },
        }
    }

    /// The id this command touches (for change-driven reconcile hints).
    /// A composite reports its first leaf target; an empty composite reports
    /// the reserved id `0` (nothing to reconcile).
    pub fn target(&self) -> ElementId {
        match self {
            PlotCommand::AddElement { element, .. } => element.id,
            PlotCommand::RemoveElement { element, .. } => element.id,
            PlotCommand::UpdateGeometry { id, .. } => *id,
            PlotCommand::SetStyle { id, .. } => *id,
            PlotCommand::SetVisibilityFlag { id, .. } => *id,
            PlotCommand::SetAttributes { id, .. } => *id,
            PlotCommand::Composite { steps } => {
                steps.first().map(PlotCommand::target).unwrap_or(ElementId(0))
            }
        }
    }

    /// Every leaf element id this command touches (a composite flattens; the
    /// bridge uses this to know which visuals to reconcile after an undo).
    pub fn targets(&self) -> Vec<ElementId> {
        match self {
            PlotCommand::Composite { steps } => steps.iter().flat_map(PlotCommand::targets).collect(),
            other => vec![other.target()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::GeoPoint;

    fn p(lon: f64, lat: f64) -> GeoPoint {
        GeoPoint::surface(lon, lat)
    }

    #[test]
    fn point_needs_one_click() {
        let g = commit_draft(DrawKind::Point, &[p(1.0, 2.0)]).unwrap();
        assert!(matches!(g, Geometry::Point(pt) if (pt.lon_deg - 1.0).abs() < 1e-9));
        assert!(commit_draft(DrawKind::Point, &[]).is_none());
    }

    #[test]
    fn polyline_and_polygon_need_their_minimum() {
        assert!(commit_draft(DrawKind::Polyline, &[p(0.0, 0.0)]).is_none());
        assert!(commit_draft(DrawKind::Polygon, &[p(0.0, 0.0), p(1.0, 0.0)]).is_none());
        let line = commit_draft(DrawKind::Polyline, &[p(0.0, 0.0), p(1.0, 1.0)]).unwrap();
        match line {
            Geometry::Polyline(pl) => assert_eq!(pl.positions.len(), 2),
            other => panic!("{other:?}"),
        }
        let poly = commit_draft(
            DrawKind::Polygon,
            &[p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0)],
        )
        .unwrap();
        match poly {
            Geometry::Polygon(pg) => assert_eq!(pg.outer.len(), 3),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn rectangle_orders_corners() {
        // Corners given out of order → west/south are still the minima.
        let g = commit_draft(DrawKind::Rectangle, &[p(10.0, 20.0), p(-5.0, 3.0)]).unwrap();
        match g {
            Geometry::Rectangle(r) => {
                assert_eq!((r.west, r.east), (-5.0, 10.0));
                assert_eq!((r.south, r.north), (3.0, 20.0));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn circle_radius_is_great_circle_distance() {
        // One degree of longitude on the equator ≈ 111.32 km.
        let g = commit_draft(
            DrawKind::Circle,
            &[p(0.0, 0.0), p(1.0, 0.0)],
        )
        .unwrap();
        match g {
            Geometry::Circle(c) => {
                assert!(c.radius_m > 111_000.0 && c.radius_m < 112_000.0, "{}", c.radius_m);
            }
            other => panic!("{other:?}"),
        }
        // Zero-radius circle (two identical clicks) is rejected.
        assert!(commit_draft(DrawKind::Circle, &[p(0.0, 0.0), p(0.0, 0.0)]).is_none());
    }

    #[test]
    fn add_then_remove_roundtrips_the_document() {
        let mut doc = Document::with_default_layer();
        let layer = doc.active_layer().unwrap();
        let element = Element::new(ElementId(99), "pt", Geometry::Point(p(1.0, 2.0)));
        let add = PlotCommand::AddElement {
            layer,
            element: Box::new(element.clone()),
        };
        add.apply(&mut doc);
        assert!(doc.element(ElementId(99)).is_some());
        // Its inverse removes it.
        add.inverse().apply(&mut doc);
        assert!(doc.element(ElementId(99)).is_none());
        // And the inverse-of-inverse re-adds (redo).
        add.inverse().inverse().apply(&mut doc);
        assert!(doc.element(ElementId(99)).is_some());
    }

    #[test]
    fn fixed_and_open_kinds_report_completion() {
        assert_eq!(DrawKind::Point.fixed_points(), Some(1));
        assert_eq!(DrawKind::Rectangle.fixed_points(), Some(2));
        assert_eq!(DrawKind::Circle.fixed_points(), Some(2));
        assert_eq!(DrawKind::Polyline.fixed_points(), None);
        assert_eq!(DrawKind::Polygon.fixed_points(), None);
    }

    fn doc_with_point(id: u64) -> Document {
        let mut doc = Document::with_default_layer();
        let layer = doc.active_layer().unwrap();
        let e = Element::new(ElementId(id), "pt", Geometry::Point(p(0.0, 0.0)));
        doc.add_element_to_layer(layer, NewElement { id: ElementId(id), element: e });
        doc
    }

    #[test]
    fn update_geometry_moves_and_undoes() {
        let mut doc = doc_with_point(7);
        let after = Geometry::Point(p(3.0, 4.0));
        let cmd = PlotCommand::UpdateGeometry {
            id: ElementId(7),
            before: Box::new(Geometry::Point(p(0.0, 0.0))),
            after: Box::new(after.clone()),
        };
        cmd.apply(&mut doc);
        assert_eq!(doc.element(ElementId(7)).unwrap().geometry, after);
        // Undo restores the original and refreshes the cached bounds.
        cmd.inverse().apply(&mut doc);
        let e = doc.element(ElementId(7)).unwrap();
        assert_eq!(e.geometry, Geometry::Point(p(0.0, 0.0)));
        assert_eq!((e.bounds.west_deg, e.bounds.north_deg), (0.0, 0.0));
    }

    #[test]
    fn set_style_and_visibility_reverse() {
        let mut doc = doc_with_point(8);
        let styled = Style::default().with_color([1.0, 0.0, 0.0, 1.0]);
        let set = PlotCommand::SetStyle {
            id: ElementId(8),
            before: Box::new(Style::default()),
            after: Box::new(styled.clone()),
        };
        set.apply(&mut doc);
        assert_eq!(doc.element(ElementId(8)).unwrap().style, styled);
        set.inverse().apply(&mut doc);
        assert_eq!(doc.element(ElementId(8)).unwrap().style, Style::default());

        let hide = PlotCommand::SetVisibilityFlag {
            id: ElementId(8),
            before: true,
            after: false,
        };
        hide.apply(&mut doc);
        assert!(!doc.element(ElementId(8)).unwrap().flags.visible_manual);
        hide.inverse().apply(&mut doc);
        assert!(doc.element(ElementId(8)).unwrap().flags.visible_manual);
    }

    #[test]
    fn composite_undoes_back_to_front() {
        let mut doc = Document::with_default_layer();
        let layer = doc.active_layer().unwrap();
        for id in 1..=3u64 {
            let e = Element::new(ElementId(id), "p", Geometry::Point(p(0.0, 0.0)));
            doc.add_element_to_layer(layer, NewElement { id: ElementId(id), element: e });
        }
        let step = |id: u64| PlotCommand::SetVisibilityFlag {
            id: ElementId(id),
            before: true,
            after: false,
        };
        let group = PlotCommand::Composite {
            steps: vec![step(1), step(2), step(3)],
        };
        group.apply(&mut doc);
        for id in 1..=3u64 {
            assert!(!doc.element(ElementId(id)).unwrap().flags.visible_manual);
        }
        // targets() flattens across leaves.
        assert_eq!(group.targets(), vec![ElementId(1), ElementId(2), ElementId(3)]);
        // Undo the group restores every leaf.
        group.inverse().apply(&mut doc);
        for id in 1..=3u64 {
            assert!(doc.element(ElementId(id)).unwrap().flags.visible_manual);
        }
    }
}
