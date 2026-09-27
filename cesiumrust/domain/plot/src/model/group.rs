//! Groups: named, nestable collections of members with an optional group-level
//! transform and visibility/lock inheritance (plan §5, §9).

use serde::{Deserialize, Serialize};

use super::ids::GroupId;
use super::node::Node;

/// A transform applied to a whole group *without* burning it into member
/// geometry (plan §17.4: group keeps the transform; single-element edits bake).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GroupTransform {
    /// Rotation about the group pivot, degrees clockwise.
    pub rotate_deg: f64,
    /// Uniform scale factor.
    pub scale: f64,
    /// Geographic offset (lon, lat degrees) added to members.
    pub offset_lonlat: [f64; 2],
}

impl Default for GroupTransform {
    fn default() -> Self {
        Self {
            rotate_deg: 0.0,
            scale: 1.0,
            offset_lonlat: [0.0, 0.0],
        }
    }
}

impl GroupTransform {
    /// The identity transform.
    pub fn identity() -> Self {
        Self::default()
    }
    /// True when the transform changes nothing.
    pub fn is_identity(&self) -> bool {
        self.rotate_deg == 0.0
            && self.scale == 1.0
            && self.offset_lonlat == [0.0, 0.0]
    }
}

/// A group of elements and/or child groups.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub id: GroupId,
    pub name: String,
    /// Enclosing group, if nested (the document owns the tree consistency).
    pub parent: Option<GroupId>,
    /// Members in draw order.
    pub members: Vec<Node>,
    /// Optional group transform; `None` == identity.
    pub transform: Option<GroupTransform>,
    /// Manual visibility (inherited down the chain).
    pub visible: bool,
    /// Locked groups cannot be edited (and their members can't be individually
    /// moved) though they may still be selected as a whole.
    pub locked: bool,
    /// When true, member styles are governed by the group and locked from
    /// per-member edits (style-propagation hook).
    pub style_lock: bool,
}

impl Group {
    /// An empty, visible, unlocked group.
    pub fn new(id: GroupId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            parent: None,
            members: Vec::new(),
            transform: None,
            visible: true,
            locked: false,
            style_lock: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_group_is_empty_and_editable() {
        let g = Group::new(GroupId(7), "taskforce");
        assert!(g.members.is_empty());
        assert!(g.visible && !g.locked);
        assert_eq!(g.transform, None);
    }

    #[test]
    fn identity_transform_predicate() {
        assert!(GroupTransform::identity().is_identity());
        assert!(!GroupTransform {
            scale: 2.0,
            ..GroupTransform::identity()
        }
        .is_identity());
    }
}
