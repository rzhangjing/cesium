//! The tree edge type shared by layers and groups: a member is either an
//! element (leaf) or a nested group (branch).

use serde::{Deserialize, Serialize};

use super::ids::{ElementId, GroupId};

/// One child slot in a [`Layer`](super::layer::Layer) or [`Group`](super::group::Group).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Node {
    /// A leaf element (resolved through the document's element table).
    Element(ElementId),
    /// A nested group (resolved through the document's group table).
    Group(GroupId),
}
