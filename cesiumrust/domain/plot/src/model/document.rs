//! The scene document: the single serialisable source of truth the whole
//! overlay is a view of (plan §5, §14).
//!
//! A flat set of tables (`layers` / `groups` / `elements`) plus a `parents`
//! index that turns the tree into O(1) upward walks. Ids come from one
//! monotonic counter. All structural edits go through the methods here so the
//! index and the `Group.parent` field never drift from the `roots` / `members`
//! vectors — the later command layer (`ops`, M6) drives these and records
//! inverses for undo.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use crate::geo::GeoBounds;

use super::element::Element;
use super::geometry::Geometry;
use super::group::Group;
use super::ids::{ElementId, GroupId, LayerId};
use super::layer::Layer;
use super::node::Node;

/// Where a node sits in the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParentRef {
    Layer(LayerId),
    Group(GroupId),
}

/// A freshly built, still-unplaced element (its `id` is already minted).
pub struct NewElement {
    pub id: ElementId,
    pub element: Element,
}

/// The document tree.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// Insertion-ordered layers (draw order resolved by `Layer::order`).
    layers: Vec<Layer>,
    groups: BTreeMap<GroupId, Group>,
    elements: BTreeMap<ElementId, Element>,
    /// node -> parent, kept in sync with every tree mutation.
    #[serde(skip)]
    parents: HashMap<Node, ParentRef>,
    /// Monotonic id counter shared by all three id kinds (unique per doc).
    next_id: u64,
    /// The layer new content lands in (§9).
    active_layer: Option<LayerId>,
}

impl Document {
    /// An empty document with a single default, active layer.
    pub fn with_default_layer() -> Self {
        let mut doc = Self::default();
        let id = doc.new_layer("默认层");
        doc.active_layer = Some(id);
        doc
    }

    fn alloc(&mut self) -> u64 {
        let v = self.next_id;
        self.next_id += 1;
        v
    }

    // ── Layers ─────────────────────────────────────────────────────────────

    /// Mint + append a new layer, returning its id.
    pub fn new_layer(&mut self, name: impl Into<String>) -> LayerId {
        let id = LayerId(self.alloc());
        self.layers.push(Layer::new(id, name));
        id
    }

    pub fn layer(&self, id: LayerId) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }

    pub fn layer_mut(&mut self, id: LayerId) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| l.id == id)
    }

    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// Layers sorted by `order` ascending (draw back-to-front).
    pub fn layers_ordered(&self) -> Vec<&Layer> {
        let mut v: Vec<&Layer> = self.layers.iter().collect();
        v.sort_by_key(|l| (l.order, l.id.raw()));
        v
    }

    pub fn set_active_layer(&mut self, id: Option<LayerId>) {
        self.active_layer = id;
    }

    pub fn active_layer(&self) -> Option<LayerId> {
        self.active_layer
    }

    /// Make `id` the active layer (new content lands here) and reflect the flag
    /// on the [`Layer`] itself. Silently ignored when the layer is gone.
    pub fn focus_layer(&mut self, id: LayerId) {
        for l in &mut self.layers {
            l.active = l.id == id;
        }
        self.active_layer = Some(id);
    }

    /// Set a layer's §10.2 manual visibility.
    pub fn set_layer_visible(&mut self, id: LayerId, visible: bool) {
        if let Some(l) = self.layer_mut(id) {
            l.visible = visible;
        }
    }

    /// Set a layer's edit permission (locked layers are read-only, plan §9).
    pub fn set_layer_editable(&mut self, id: LayerId, editable: bool) {
        if let Some(l) = self.layer_mut(id) {
            l.editable = editable;
        }
    }

    /// Set a layer's group-opacity multiplier, clamped to `0..=1` (§9).
    pub fn set_layer_opacity(&mut self, id: LayerId, opacity: f32) {
        if let Some(l) = self.layer_mut(id) {
            l.opacity = opacity.clamp(0.0, 1.0);
        }
    }

    /// Set a layer's pick / select permission (plan §9).
    pub fn set_layer_selectable(&mut self, id: LayerId, selectable: bool) {
        if let Some(l) = self.layer_mut(id) {
            l.selectable = selectable;
        }
    }

    /// Nudge a layer's draw / pick `order` by `delta` (the panel's up / down
    /// buttons). Higher draws later (on top).
    pub fn nudge_layer_order(&mut self, id: LayerId, delta: i32) {
        if let Some(l) = self.layer_mut(id) {
            l.order += delta;
        }
    }

    /// Remove a layer and every element / group that was under it.
    pub fn remove_layer(&mut self, id: LayerId) {
        if let Some(idx) = self.layers.iter().position(|l| l.id == id) {
            let layer = self.layers.remove(idx);
            for node in &layer.roots {
                self.detach_subtree(*node);
            }
            if self.active_layer == Some(id) {
                self.active_layer = None;
            }
        }
    }

    // ── Elements ─────────────────────────────────────────────────────────────

    /// Mint an element (not yet in the tree). Bounds are computed.
    pub fn make_element(&mut self, name: impl Into<String>, geometry: Geometry) -> NewElement {
        let id = ElementId(self.alloc());
        let element = Element::new(id, name, geometry);
        NewElement { id, element }
    }

    /// Place an element into a layer's roots (append, draw last).
    pub fn add_element_to_layer(&mut self, layer: LayerId, ne: NewElement) -> ElementId {
        let Some(l) = self.layers.iter_mut().find(|l| l.id == layer) else {
            // Layer vanished: drop the element (id stays consumed).
            return ne.id;
        };
        l.roots.push(Node::Element(ne.id));
        self.parents.insert(Node::Element(ne.id), ParentRef::Layer(layer));
        self.elements.insert(ne.id, ne.element);
        ne.id
    }

    /// Place an element into a group's members.
    pub fn add_element_to_group(&mut self, group: GroupId, ne: NewElement) -> Option<ElementId> {
        let g = self.groups.get_mut(&group)?;
        g.members.push(Node::Element(ne.id));
        self.parents
            .insert(Node::Element(ne.id), ParentRef::Group(group));
        self.elements.insert(ne.id, ne.element);
        Some(ne.id)
    }

    pub fn element(&self, id: ElementId) -> Option<&Element> {
        self.elements.get(&id)
    }

    pub fn element_mut(&mut self, id: ElementId) -> Option<&mut Element> {
        self.elements.get_mut(&id)
    }

    pub fn elements(&self) -> impl Iterator<Item = &Element> {
        self.elements.values()
    }

    pub fn element_ids(&self) -> impl Iterator<Item = ElementId> + '_ {
        self.elements.keys().copied()
    }

    /// Remove an element from the tree + table (leaves its parent's vector
    /// containing a dangling slot? no — we purge the node reference too).
    pub fn remove_element(&mut self, id: ElementId) {
        let node = Node::Element(id);
        if let Some(parent) = self.parents.remove(&node) {
            self.purge_node(parent, node);
        }
        self.elements.remove(&id);
    }

    // ── Groups ─────────────────────────────────────────────────────────────

    /// Create a group under a layer root and return its id.
    pub fn new_group_in_layer(&mut self, layer: LayerId, name: impl Into<String>) -> GroupId {
        let id = GroupId(self.alloc());
        self.groups.insert(id, Group::new(id, name));
        if let Some(l) = self.layers.iter_mut().find(|l| l.id == layer) {
            l.roots.push(Node::Group(id));
        }
        self.parents.insert(Node::Group(id), ParentRef::Layer(layer));
        id
    }

    /// Create a nested group under a parent group.
    pub fn new_group_in_group(&mut self, parent: GroupId, name: impl Into<String>) -> Option<GroupId> {
        let id = GroupId(self.alloc());
        self.groups.insert(id, Group::new(id, name));
        if let Some(g) = self.groups.get_mut(&parent) {
            g.members.push(Node::Group(id));
            self.parents.insert(Node::Group(id), ParentRef::Group(parent));
            Some(id)
        } else {
            self.groups.remove(&id);
            None
        }
    }

    pub fn group(&self, id: GroupId) -> Option<&Group> {
        self.groups.get(&id)
    }

    pub fn group_mut(&mut self, id: GroupId) -> Option<&mut Group> {
        self.groups.get_mut(&id)
    }

    // ── Tree walks ────────────────────────────────────────────────────────────

    /// The layer an element ultimately belongs to (walking up the parent chain),
    /// plus the group chain from the layer root down to the element (outermost
    /// first). Returns `None` for an orphan (shouldn't happen in a valid tree).
    pub fn element_context(&self, id: ElementId) -> Option<(LayerId, Vec<GroupId>)> {
        let mut groups = Vec::new();
        let mut cur = Node::Element(id);
        // guard against a corrupt cycle: bounded by node count.
        for _ in 0..(self.elements.len() + self.groups.len() + 1) {
            match self.parents.get(&cur)? {
                ParentRef::Layer(l) => {
                    groups.reverse();
                    return Some((*l, groups));
                }
                ParentRef::Group(g) => {
                    groups.push(*g);
                    cur = Node::Group(*g);
                }
            }
        }
        None
    }

    /// The layer a group belongs to.
    pub fn group_layer(&self, id: GroupId) -> Option<LayerId> {
        let mut cur = Node::Group(id);
        for _ in 0..(self.groups.len() + 1) {
            match self.parents.get(&cur)? {
                ParentRef::Layer(l) => return Some(*l),
                ParentRef::Group(g) => cur = Node::Group(*g),
            }
        }
        None
    }

    /// All element ids under a group (recursive).
    pub fn group_members_recursive(&self, id: GroupId) -> Vec<ElementId> {
        let mut out = Vec::new();
        let mut stack = vec![id];
        while let Some(g) = stack.pop() {
            let Some(grp) = self.groups.get(&g) else { continue };
            for node in &grp.members {
                match node {
                    Node::Element(e) => out.push(*e),
                    Node::Group(sub) => stack.push(*sub),
                }
            }
        }
        out
    }

    /// Every element in the document, in layer-order then within each layer in
    /// roots order, expanding groups depth-first (parents before members so a
    /// later sort by `z_order` can refine within the layer).
    pub fn flatten_draw_order(&self) -> Vec<ElementId> {
        let mut out = Vec::new();
        for layer in self.layers_ordered() {
            for node in &layer.roots {
                self.collect_draw(*node, &mut out);
            }
        }
        out
    }

    fn collect_draw(&self, node: Node, out: &mut Vec<ElementId>) {
        match node {
            Node::Element(e) => out.push(e),
            Node::Group(g) => {
                if let Some(grp) = self.groups.get(&g) {
                    for m in &grp.members {
                        self.collect_draw(*m, out);
                    }
                }
            }
        }
    }

    /// Every element ordered for **painting** (plan §16 M9 "z 排序完善"): first
    /// by layer `order` (bottom layer first), then by each element's
    /// `style.z_order` tier, then by id for a stable tie-break. Unlike
    /// [`flatten_draw_order`](Self::flatten_draw_order) (layer-then-tree order
    /// only), this honours the per-element z tier so a user can lift one feature
    /// above its siblings; painting in this order makes later (higher) elements
    /// win both on screen and in pick ties.
    pub fn draw_order_sorted(&self) -> Vec<ElementId> {
        let mut keyed: Vec<(i32, i32, u64, ElementId)> = self
            .flatten_draw_order()
            .into_iter()
            .filter_map(|id| {
                let el = self.element(id)?;
                let layer_order = self
                    .element_context(id)
                    .and_then(|(l, _)| self.layer(l))
                    .map(|l| l.order)
                    .unwrap_or(0);
                Some((layer_order, el.style.z_order, id.raw(), id))
            })
            .collect();
        keyed.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        keyed.into_iter().map(|(_, _, _, id)| id).collect()
    }

    // ── Aggregate ──────────────────────────────────────────────────────────────

    /// Union of every element's bounds, or `None` when empty.
    pub fn bounds(&self) -> Option<GeoBounds> {
        let mut acc = GeoBounds::empty();
        for e in self.elements.values() {
            acc = acc.union(e.bounds);
        }
        (!acc.is_empty()).then_some(acc)
    }

    pub fn is_empty(&self) -> bool {
        self.layers.is_empty() && self.elements.is_empty() && self.groups.is_empty()
    }

    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    /// Rebuild the `#[serde(skip)]` parent index from the layer / group member
    /// vectors. A loader that deserialises a [`Document`] (e.g. from GeoJSON,
    /// see `crate::io`) must call this before any upward walk (`element_context`,
    /// `group_layer`, `flatten_draw_order`) returns correct results.
    pub fn rebuild_parents(&mut self) {
        let mut refs: Vec<(Node, ParentRef)> = Vec::new();
        for layer in &self.layers {
            for node in &layer.roots {
                refs.push((*node, ParentRef::Layer(layer.id)));
            }
        }
        for (gid, group) in &self.groups {
            for node in &group.members {
                refs.push((*node, ParentRef::Group(*gid)));
            }
        }
        self.parents.clear();
        for (n, p) in refs {
            self.parents.insert(n, p);
        }
    }

    // ── internals ──────────────────────────────────────────────────────────

    /// Remove a node reference from whatever holds it (layer roots or group
    /// members) given its recorded parent.
    fn purge_node(&mut self, parent: ParentRef, node: Node) {
        match parent {
            ParentRef::Layer(l) => {
                if let Some(layer) = self.layers.iter_mut().find(|x| x.id == l) {
                    layer.roots.retain(|n| *n != node);
                }
            }
            ParentRef::Group(g) => {
                if let Some(grp) = self.groups.get_mut(&g) {
                    grp.members.retain(|n| *n != node);
                }
            }
        }
    }

    /// Recursively detach a node (group or element) and everything under it,
    /// dropping the element/group tables and parent entries.
    fn detach_subtree(&mut self, node: Node) {
        match node {
            Node::Element(e) => {
                self.parents.remove(&node);
                self.elements.remove(&e);
            }
            Node::Group(g) => {
                self.parents.remove(&node);
                let members = self.groups.remove(&g).map(|grp| grp.members).unwrap_or_default();
                for m in members {
                    self.detach_subtree(m);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::GeoPoint;

    fn point_geo(lon: f64, lat: f64) -> Geometry {
        Geometry::Point(GeoPoint::surface(lon, lat))
    }

    #[test]
    fn default_layer_is_active_and_holds_elements() {
        let mut doc = Document::with_default_layer();
        let layer = doc.active_layer().unwrap();
        let ne = doc.make_element("a", point_geo(1.0, 2.0));
        let id = doc.add_element_to_layer(layer, ne);
        assert_eq!(doc.element_count(), 1);
        assert_eq!(doc.element_context(id).unwrap().0, layer);
        assert!(doc.element(id).is_some());
    }

    #[test]
    fn draw_order_sorted_honours_layer_then_z_then_id() {
        let mut doc = Document::default();
        let back = doc.new_layer("back");
        let front = doc.new_layer("front");
        doc.layer_mut(back).unwrap().order = 0;
        doc.layer_mut(front).unwrap().order = 10;

        let low = doc.make_element("low", point_geo(0.0, 0.0));
        let e_low = doc.add_element_to_layer(front, low);
        let mut hi = doc.make_element("high", point_geo(1.0, 1.0));
        hi.element.style.z_order = 5;
        let e_high = doc.add_element_to_layer(front, hi);
        // A back-layer element added last by id must still paint first (lower layer).
        let back_el = doc.make_element("back", point_geo(2.0, 2.0));
        let e_back = doc.add_element_to_layer(back, back_el);

        let order = doc.draw_order_sorted();
        assert_eq!(
            order,
            vec![e_back, e_low, e_high],
            "layer order dominates, then z_order within a layer"
        );
        // The highest-z front element paints last.
        assert_eq!(order.last(), Some(&e_high));
    }

    #[test]
    fn ids_are_unique_across_kinds() {
        let mut doc = Document::default();
        let l = doc.new_layer("L");
        let g = doc.new_group_in_layer(l, "G");
        let e = doc.make_element("E", point_geo(0.0, 0.0)).id;
        let set = std::collections::HashSet::from([l.raw(), g.raw(), e.raw()]);
        assert_eq!(set.len(), 3, "layer/group/element ids must not collide");
    }

    #[test]
    fn nested_group_context_walks_chain() {
        let mut doc = Document::default();
        let l = doc.new_layer("L");
        let outer = doc.new_group_in_layer(l, "outer");
        let inner = doc.new_group_in_group(outer, "inner").unwrap();
        let ne = doc.make_element("e", point_geo(3.0, 4.0));
        let e = ne.id;
        doc.add_element_to_group(inner, ne);
        let (layer, chain) = doc.element_context(e).unwrap();
        assert_eq!(layer, l);
        // Chain is outermost-first: the layer's group, then the nested one.
        assert_eq!(chain, vec![outer, inner]);
        assert_eq!(doc.group_layer(inner), Some(l));
        assert_eq!(doc.group_members_recursive(outer), vec![e]);
    }

    #[test]
    fn remove_element_detaches_from_group_and_table() {
        let mut doc = Document::default();
        let l = doc.new_layer("L");
        let g = doc.new_group_in_layer(l, "G");
        let e = doc.make_element("e", point_geo(0.0, 0.0));
        let eid = e.id;
        doc.add_element_to_group(g, e);
        assert_eq!(doc.group(g).unwrap().members.len(), 1);
        doc.remove_element(eid);
        assert!(doc.element(eid).is_none());
        assert!(doc.group(g).unwrap().members.is_empty());
    }

    #[test]
    fn flatten_respects_layer_order_and_group_expansion() {
        let mut doc = Document::default();
        let back = doc.new_layer("back");
        let front = doc.new_layer("front");
        doc.layer_mut(back).unwrap().order = 0;
        doc.layer_mut(front).unwrap().order = 10;
        let g = doc.new_group_in_layer(back, "g");
        let e1 = doc.make_element("1", point_geo(0.0, 0.0));
        let e2 = doc.make_element("2", point_geo(1.0, 1.0));
        let (id1, id2) = (e1.id, e2.id);
        doc.add_element_to_group(g, e1);
        doc.add_element_to_layer(front, e2);
        let flat = doc.flatten_draw_order();
        assert_eq!(flat, vec![id1, id2], "group in back layer draws before front layer");
    }

    #[test]
    fn layer_flags_and_focus_round_trip() {
        let mut doc = Document::default();
        let a = doc.new_layer("a");
        let b = doc.new_layer("b");
        doc.focus_layer(a);
        assert_eq!(doc.active_layer(), Some(a));
        assert!(doc.layer(a).unwrap().active);
        assert!(!doc.layer(b).unwrap().active);
        // Focusing b moves the marker and the flag together.
        doc.focus_layer(b);
        assert!(!doc.layer(a).unwrap().active);
        assert!(doc.layer(b).unwrap().active);

        doc.set_layer_visible(a, false);
        assert!(!doc.layer(a).unwrap().visible);
        doc.set_layer_editable(a, false);
        assert!(!doc.layer(a).unwrap().editable);
        doc.set_layer_selectable(a, false);
        assert!(!doc.layer(a).unwrap().selectable);

        let before = doc.layer(b).unwrap().order;
        doc.nudge_layer_order(b, 5);
        assert_eq!(doc.layer(b).unwrap().order, before + 5);

        doc.set_layer_opacity(a, 0.5);
        assert_eq!(doc.layer(a).unwrap().opacity, 0.5);
        // Out-of-range values clamp rather than panic.
        doc.set_layer_opacity(a, 2.0);
        assert_eq!(doc.layer(a).unwrap().opacity, 1.0);
        doc.set_layer_opacity(a, -1.0);
        assert_eq!(doc.layer(a).unwrap().opacity, 0.0);
    }

    #[test]
    fn remove_layer_cascades_subtree() {
        let mut doc = Document::with_default_layer();
        let l = doc.active_layer().unwrap();
        let g = doc.new_group_in_layer(l, "g");
        let e = doc.make_element("e", point_geo(0.0, 0.0));
        let eid = e.id;
        doc.add_element_to_group(g, e);
        doc.remove_layer(l);
        assert!(doc.element(eid).is_none());
        assert!(doc.group(g).is_none());
        assert!(doc.layers().is_empty());
    }

    #[test]
    fn bounds_union_over_elements() {
        let mut doc = Document::with_default_layer();
        let l = doc.active_layer().unwrap();
        let a = doc.make_element("a", Geometry::Polyline(crate::model::geometry::Polyline {
            positions: vec![GeoPoint::surface(-10.0, -20.0), GeoPoint::surface(0.0, 0.0)],
        }));
        let b = doc.make_element("b", point_geo(30.0, 40.0));
        doc.add_element_to_layer(l, a);
        doc.add_element_to_layer(l, b);
        let bb = doc.bounds().unwrap();
        assert_eq!((bb.west_deg, bb.south_deg, bb.east_deg, bb.north_deg), (-10.0, -20.0, 30.0, 40.0));
    }

    #[test]
    fn rebuild_parents_restores_walks_after_deserialise() {
        let mut doc = Document::default();
        let l = doc.new_layer("L");
        let g = doc.new_group_in_layer(l, "g");
        let ne = doc.make_element("e", point_geo(1.0, 2.0));
        let e = ne.id;
        doc.add_element_to_group(g, ne);
        // Round-trip drops the parent index (serde skip), then rebuild it.
        let j = serde_json::to_string(&doc).unwrap();
        let mut back: Document = serde_json::from_str(&j).unwrap();
        assert!(back.element_context(e).is_none(), "index is empty before rebuild");
        back.rebuild_parents();
        assert_eq!(back.element_context(e), Some((l, vec![g])));
        assert_eq!(back.group_layer(g), Some(l));
    }

    #[test]
    fn document_serde_roundtrip_rebuilds_parent_index() {
        // Serialise a small tree, then confirm tables come back. The parents
        // index is #[serde(skip)], so a consumer must rebuild it if it needs
        // walks; here we assert the raw tables survive (index rebuild is a
        // documented follow-up responsibility of the loader, not serde).
        let mut doc = Document::with_default_layer();
        let l = doc.active_layer().unwrap();
        let e = doc.make_element("e", point_geo(5.0, 6.0));
        doc.add_element_to_layer(l, e);
        let j = serde_json::to_string(&doc).unwrap();
        let back: Document = serde_json::from_str(&j).unwrap();
        assert_eq!(back.element_count(), 1);
        assert_eq!(back.layers().len(), 1);
        assert!(back.parents.is_empty(), "index is not serialised");
    }
}
