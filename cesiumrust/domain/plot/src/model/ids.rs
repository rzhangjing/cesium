//! Numeric identity newtypes for the document tree.
//!
//! Ids are allocated monotonically by the [`crate`'s `Document`](super::document::Document)
//! from a single counter, so they are unique within one document and cheap to
//! serialise / diff / hash. They are deliberately opaque `u64` wrappers rather
//! than UUIDs: the document is in-memory authoring state, ids only need to be
//! stable within a session (and across a GeoJSON round-trip they are re-minted).

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! id_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub u64);

        impl $name {
            /// Wrap a raw counter value.
            #[inline]
            pub const fn new(v: u64) -> Self {
                Self(v)
            }
            /// The underlying value.
            #[inline]
            pub const fn raw(self) -> u64 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}#{}", stringify!($name), self.0)
            }
        }
    };
}

id_newtype!(
    /// Identity of a [`Element`](super::element::Element) in a document.
    ElementId
);
id_newtype!(
    /// Identity of a [`Group`](super::group::Group) in a document.
    GroupId
);
id_newtype!(
    /// Identity of a [`Layer`](super::layer::Layer) in a document.
    LayerId
);
