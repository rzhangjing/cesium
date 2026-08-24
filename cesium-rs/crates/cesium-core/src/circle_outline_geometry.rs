//! Ported from `packages/engine/Source/Core/CircleOutlineGeometry.js`.
//!
//! A description of the outline of a circle on the ellipsoid.
//!
//! `CircleOutlineGeometry` is a thin wrapper around `EllipseOutlineGeometry`
//! where `semi_major_axis == semi_minor_axis == radius`; all operations
//! delegate to the inner ellipse outline geometry, mirroring the JS
//! implementation.

use crate::cartesian3::Cartesian3;
use crate::ellipse_outline_geometry::EllipseOutlineGeometry;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::Geometry;

/// A description of the outline of a circle on the ellipsoid.
#[derive(Debug, Clone)]
pub struct CircleOutlineGeometry {
    ellipse_geometry: EllipseOutlineGeometry,
}

impl CircleOutlineGeometry {
    /// Creates a new `CircleOutlineGeometry`.
    pub fn new(
        center: Cartesian3,
        radius: f64,
        ellipsoid: Option<Ellipsoid>,
        height: Option<f64>,
        granularity: Option<f64>,
        extruded_height: Option<f64>,
        number_of_vertical_lines: Option<usize>,
    ) -> Self {
        let ellipse_geometry = EllipseOutlineGeometry::new(
            center,
            radius,
            radius,
            ellipsoid,
            height,
            extruded_height,
            None,
            granularity,
            number_of_vertical_lines,
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

    /// Access to the inner ellipse outline geometry (mirrors JS
    /// `_ellipseGeometry`).
    pub fn ellipse_geometry(&self) -> &EllipseOutlineGeometry {
        &self.ellipse_geometry
    }

    /// The number of `f64` elements needed to pack/unpack a
    /// `CircleOutlineGeometry`.
    pub const PACKED_LENGTH: usize = EllipseOutlineGeometry::PACKED_LENGTH;

    /// Packs the circle outline geometry into `array` starting at
    /// `starting_index`.
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        self.ellipse_geometry.pack(array, starting_index);
    }

    /// Unpacks a `CircleOutlineGeometry` from `array`.
    ///
    /// Mirrors the JS semantics: with `result == None` a new
    /// `CircleOutlineGeometry` is built from the ellipse's semi-major axis
    /// as the radius; with a provided `result` its inner ellipse outline
    /// geometry is rebuilt preserving both semi-axes.
    pub fn unpack(
        array: &[f64],
        starting_index: Option<usize>,
        result: Option<&mut Self>,
    ) -> Self {
        let ellipse_geometry = EllipseOutlineGeometry::unpack(array, starting_index, None);

        match result {
            None => Self::new(
                *ellipse_geometry.center(),
                ellipse_geometry.semi_major_axis(),
                Some(ellipse_geometry.ellipsoid().clone()),
                Some(ellipse_geometry.height()),
                Some(ellipse_geometry.granularity()),
                Some(ellipse_geometry.extruded_height()),
                Some(ellipse_geometry.number_of_vertical_lines()),
            ),
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

    /// Computes the geometric representation of an outline of a circle on an
    /// ellipsoid, including its vertices, indices, and a bounding sphere.
    pub fn create_geometry(circle_geometry: &Self) -> Option<Geometry> {
        crate::ellipse_outline_geometry::create_geometry(&circle_geometry.ellipse_geometry)
    }
}
