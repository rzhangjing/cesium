//! A layer: the top-level container in the tree — an ordered bucket of root
//! nodes (elements + groups) with visibility / editability / opacity flags and
//! the "active layer" marker new content falls into (plan §5, §9).

use serde::{Deserialize, Serialize};

use super::ids::LayerId;
use super::node::Node;

/// A plotting layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    /// Draw / pick priority; higher draws later (on top). Distinct from a
    /// member's own `style.z_order` (which orders within the layer).
    pub order: i32,
    /// Layer on/off switch (§10.2).
    pub visible: bool,
    /// Group opacity multiplier applied over members' colours (§9).
    pub opacity: f32,
    /// Editing allowed in this layer? Locked layers are read-only (§9).
    pub editable: bool,
    /// Elements here are pickable/selectable? (§9).
    pub selectable: bool,
    /// New / pasted elements land in the active layer (§9).
    pub active: bool,
    /// Root members in draw order.
    pub roots: Vec<Node>,
}

impl Layer {
    /// A visible, editable, selectable, empty layer.
    pub fn new(id: LayerId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            order: 0,
            visible: true,
            opacity: 1.0,
            editable: true,
            selectable: true,
            active: false,
            roots: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_layer_defaults() {
        let l = Layer::new(LayerId(1), "default");
        assert!(l.visible && l.editable && l.selectable);
        assert!(!l.active);
        assert_eq!(l.opacity, 1.0);
        assert!(l.roots.is_empty());
    }
}
