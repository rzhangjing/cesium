//! The serialisable scene-document model (plan §5).
//!
//! `Document` (a tree of layers → groups → elements) plus the value types each
//! node is built from. Everything here is plain data (`Serialize`/`Deserialize`)
//! with no engine dependency, so it is diffable and unit-testable headless.

pub mod document;
pub mod element;
pub mod filters;
pub mod geometry;
pub mod group;
pub mod ids;
pub mod layer;
pub mod node;
pub mod pick;
pub mod style;
pub mod view;

pub use document::{Document, NewElement};
pub use element::{Element, ElementFlags, ScaleVisibility};
pub use filters::Filters;
pub use geometry::{
    Arc3, Circle, Composite, Ellipse, Geometry, GeometryKind, IconGeometry, LabelAnchor,
    LabelGeometry, Path, PathSegment, Polyline, Polygon, Rectangle, Ring, SymbolKind,
};
pub use group::{Group, GroupTransform};
pub use ids::{ElementId, GroupId, LayerId};
pub use layer::Layer;
pub use node::Node;
pub use pick::{pick_best, Part, PickHit, RANK_LINE, RANK_MARKER, RANK_POLY_BODY, RANK_POLY_EDGE};
pub use style::{HeightReference, IconStyle, Outline, Rgba, Style, TextStyle, TRANSPARENT, WHITE};
pub use view::{ViewContext, ViewMode};
