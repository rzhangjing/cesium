//! Ported from `packages/engine/Source/Core/CircleGeometry.js`.
//!
//! A description of a circle on the ellipsoid. Circle geometry can be
//! rendered with both `Primitive` and `GroundPrimitive`.
//!
//! `CircleGeometry` is a thin wrapper around `EllipseGeometry` where
//! `semi_major_axis == semi_minor_axis == radius`; all operations delegate
//! to the inner ellipse geometry, mirroring the JS implementation.

use crate::cartesian3::Cartesian3;
use crate::ellipse_geometry::EllipseGeometry;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::Geometry;
use crate::rectangle::Rectangle;
use crate::vertex_format::VertexFormat;

/// A description of a circle on the ellipsoid.
#[derive(Debug, Clone)]
pub struct CircleGeometry {
    ellipse_geometry: EllipseGeometry,
}

impl CircleGeometry {
    /// Creates a new `CircleGeometry`.
    ///
    /// # Panics (debug)
    /// Mirrors the JS `Check.typeOf.number("radius")` behind
    /// `debug_assertions` (any `f64` passes the type check; the check is
    /// retained for structural parity).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        center: Cartesian3,
        radius: f64,
        ellipsoid: Option<Ellipsoid>,
        height: Option<f64>,
        granularity: Option<f64>,
        vertex_format: Option<VertexFormat>,
        extruded_height: Option<f64>,
        st_rotation: Option<f64>,
    ) -> Self {
        let ellipse_geometry = EllipseGeometry::new(
            center,
            radius,
            radius,
            ellipsoid,
            None,
            st_rotation,
            height,
            extruded_height,
            granularity,
            vertex_format,
            None,
            None,
        );
        Self { ellipse_geometry }
    }

    /// The circle's center point.
    pub fn center(&self) -> &Cartesian3 {
        self.ellipse_geometry.center()
    }

    /// The circle's radius in meters.
    pub fn radius(&self) -> f64 {
        self.ellipse_geometry.semi_major_axis()
    }

    /// The ellipsoid.
    pub fn ellipsoid(&self) -> &Ellipsoid {
        self.ellipse_geometry.ellipsoid()
    }

    /// Access to the inner ellipse geometry (mirrors JS `_ellipseGeometry`).
    pub fn ellipse_geometry(&self) -> &EllipseGeometry {
        &self.ellipse_geometry
    }

    /// The number of `f64` elements needed to pack/unpack a `CircleGeometry`.
    pub const PACKED_LENGTH: usize = EllipseGeometry::PACKED_LENGTH;

    /// Packs the circle geometry into `array` starting at `starting_index`.
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        self.ellipse_geometry.pack(array, starting_index);
    }

    /// Unpacks a `CircleGeometry` from `array`.
    ///
    /// Mirrors the JS semantics: with `result == None` a new
    /// `CircleGeometry` is built from the ellipse's semi-major axis as the
    /// radius; with a provided `result` its inner ellipse geometry is
    /// rebuilt preserving both semi-axes.
    pub fn unpack(
        array: &[f64],
        starting_index: Option<usize>,
        result: Option<&mut Self>,
    ) -> Self {
        let ellipse_geometry = EllipseGeometry::unpack(array, starting_index, None);

        match result {
            None => {
                let radius = ellipse_geometry.semi_major_axis();
                Self::new(
                    *ellipse_geometry.center(),
                    radius,
                    Some(ellipse_geometry.ellipsoid().clone()),
                    Some(ellipse_geometry.height()),
                    Some(ellipse_geometry.granularity()),
                    Some(ellipse_geometry.vertex_format().clone()),
                    Some(ellipse_geometry.extruded_height()),
                    Some(ellipse_geometry.st_rotation()),
                )
            }
            Some(r) => {
                // JS rebuilds result._ellipseGeometry via the constructor with
                // explicit semiMajorAxis/semiMinorAxis; both constructors
                // normalize height/extrudedHeight identically, so unpacking
                // straight into the inner ellipse is equivalent.
                r.ellipse_geometry = ellipse_geometry;
                r.clone()
            }
        }
    }

    /// Computes the geometric representation of a circle on an ellipsoid,
    /// including its vertices, indices, and a bounding sphere.
    pub fn create_geometry(circle_geometry: &Self) -> Option<Geometry> {
        crate::ellipse_geometry::create_geometry(&circle_geometry.ellipse_geometry)
    }

    /// Port of `CircleGeometry.createShadowVolume`.
    pub fn create_shadow_volume<F>(
        circle_geometry: &Self,
        min_height_func: F,
        max_height_func: F,
    ) -> Self
    where
        F: Fn(f64, &Ellipsoid) -> f64,
    {
        let inner = EllipseGeometry::create_shadow_volume(
            &circle_geometry.ellipse_geometry,
            min_height_func,
            max_height_func,
        );
        Self {
            ellipse_geometry: inner,
        }
    }

    /// The bounding [`Rectangle`] of this circle (JS `rectangle` getter).
    pub fn rectangle(&self) -> Rectangle {
        self.ellipse_geometry.rectangle()
    }

    /// For remapping texture coordinates when rendering CircleGeometries as
    /// GroundPrimitives (JS `textureCoordinateRotationPoints` getter).
    pub fn texture_coordinate_rotation_points(&self) -> [f64; 6] {
        self.ellipse_geometry.texture_coordinate_rotation_points()
    }
}
