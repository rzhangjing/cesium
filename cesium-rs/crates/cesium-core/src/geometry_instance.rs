//! Ported from `packages/engine/Source/Core/GeometryInstance.js`.
//!
//! Geometry instancing allows one Geometry object to be placed at several
//! different locations and colored uniquely.

use std::collections::HashMap;

use crate::geometry::Geometry;
use crate::geometry_instance_attribute::GeometryInstanceAttribute;
use crate::matrix4::Matrix4;

/// A single instance of a geometry, with its own model matrix and attributes.
#[derive(Debug, Clone)]
pub struct GeometryInstance {
    /// The geometry source for this instance.
    ///
    /// DEVIATION: JS allows assigning `westHemisphereGeometry` /
    /// `eastHemisphereGeometry` after `GeometryPipeline.splitLongitude` and
    /// clears `geometry` (sets it `undefined`). Rust models that with the
    /// dedicated `Option<Geometry>` fields below and resets `geometry` to
    /// [`GeometryInstanceGeometry::Placeholder`].
    pub geometry: GeometryInstanceGeometry,

    /// West hemisphere geometry produced by `GeometryPipeline.splitLongitude`.
    pub west_hemisphere_geometry: Option<Geometry>,

    /// East hemisphere geometry produced by `GeometryPipeline.splitLongitude`.
    pub east_hemisphere_geometry: Option<Geometry>,

    /// 4×4 transform from model to world coordinates.
    pub model_matrix: Matrix4,

    /// User-defined id for picking.
    pub id: Option<String>,

    /// Per-instance attributes (color, show, …).
    pub attributes: HashMap<String, GeometryInstanceAttribute>,
}

/// Enum wrapping the two possible geometry sources for an instance.
#[derive(Debug, Clone)]
pub enum GeometryInstanceGeometry {
    /// A pre-built geometry.
    Geometry(Box<Geometry>),
    /// No geometry (either not yet built, or cleared after a longitude split).
    Placeholder,
}

impl GeometryInstanceGeometry {
    /// Returns the wrapped geometry, if any.
    pub fn as_geometry(&self) -> Option<&Geometry> {
        match self {
            GeometryInstanceGeometry::Geometry(g) => Some(g),
            GeometryInstanceGeometry::Placeholder => None,
        }
    }

    /// Returns the wrapped geometry mutably, if any.
    pub fn as_geometry_mut(&mut self) -> Option<&mut Geometry> {
        match self {
            GeometryInstanceGeometry::Geometry(g) => Some(g),
            GeometryInstanceGeometry::Placeholder => None,
        }
    }
}

impl GeometryInstance {
    /// Creates a new `GeometryInstance`.
    pub fn new(
        geometry: GeometryInstanceGeometry,
        model_matrix: Option<Matrix4>,
        id: Option<String>,
        attributes: Option<HashMap<String, GeometryInstanceAttribute>>,
    ) -> Self {
        Self {
            geometry,
            west_hemisphere_geometry: None,
            east_hemisphere_geometry: None,
            model_matrix: model_matrix.unwrap_or(Matrix4::IDENTITY.clone()),
            id,
            attributes: attributes.unwrap_or_default(),
        }
    }
}
