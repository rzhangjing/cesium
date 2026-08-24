//! Ported from `packages/engine/Source/Core/Geometry.js`.
//!
//! A geometry representation with attributes forming vertices and optional index
//! data defining primitives.

use std::collections::HashMap;

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_type::GeometryType;
use crate::index_datatype::IndexStorage;
use crate::matrix2::Matrix2;
use crate::matrix3::Matrix3;
use crate::matrix4::Matrix4;
use crate::primitive_type::PrimitiveType;
use crate::quaternion::Quaternion;
use crate::rectangle::Rectangle;
use crate::transforms;

/// A geometry representation with attributes forming vertices and optional index
/// data defining primitives. Geometries and an `Appearance`, which describes the
/// shading, can be assigned to a `Primitive` for visualization.
#[derive(Debug, Clone)]
pub struct Geometry {
    /// Attributes which make up the geometry's vertices. Each property
    /// corresponds to a [`GeometryAttribute`] containing the attribute's data.
    pub attributes: HashMap<String, GeometryAttribute>,
    /// Optional index data that — along with `primitive_type` — determines the
    /// primitives in the geometry.
    pub indices: Option<IndexStorage>,
    /// The type of primitives in the geometry.
    pub primitive_type: PrimitiveType,
    /// An optional bounding sphere that fully encloses the geometry.
    pub bounding_sphere: Option<BoundingSphere>,
    /// Internal geometry type identifier (private).
    pub geometry_type: GeometryType,
    /// Bounding sphere in Columbus View (private).
    pub bounding_sphere_cv: Option<BoundingSphere>,
    /// Used for computing the bounding sphere for geometry using the applyOffset
    /// vertex attribute (private).
    pub offset_attribute: Option<String>,
}

impl Geometry {
    /// Creates a new `Geometry` from the given options.
    pub fn new(
        attributes: HashMap<String, GeometryAttribute>,
        indices: Option<IndexStorage>,
        primitive_type: Option<PrimitiveType>,
        bounding_sphere: Option<BoundingSphere>,
    ) -> Self {
        debug_assert!(
            !attributes.is_empty(),
            "options.attributes is required and must not be empty"
        );
        Self {
            attributes,
            indices,
            primitive_type: primitive_type.unwrap_or(PrimitiveType::Triangles),
            bounding_sphere,
            geometry_type: GeometryType::None,
            bounding_sphere_cv: None,
            offset_attribute: None,
        }
    }

    /// Creates a new `Geometry` with all options.
    pub fn with_all(
        attributes: HashMap<String, GeometryAttribute>,
        indices: Option<IndexStorage>,
        primitive_type: Option<PrimitiveType>,
        bounding_sphere: Option<BoundingSphere>,
        geometry_type: GeometryType,
        bounding_sphere_cv: Option<BoundingSphere>,
        offset_attribute: Option<String>,
    ) -> Self {
        Self {
            attributes,
            indices,
            primitive_type: primitive_type.unwrap_or(PrimitiveType::Triangles),
            bounding_sphere,
            geometry_type,
            bounding_sphere_cv,
            offset_attribute,
        }
    }

    /// Computes the number of vertices in a geometry. The runtime is linear with
    /// respect to the number of attributes in a vertex, not the number of vertices.
    ///
    /// Returns `None` if there are no valid attributes.
    ///
    /// # Panics (debug)
    /// Panics if attribute lists have inconsistent vertex counts.
    pub fn compute_number_of_vertices(&self) -> Option<usize> {
        let mut number_of_vertices: Option<usize> = None;

        for (name, attr) in &self.attributes {
            if attr.values.is_empty() {
                continue;
            }
            let num = attr.values.len() / attr.components_per_attribute as usize;
            if let Some(prev) = number_of_vertices {
                debug_assert!(
                    prev == num,
                    "All attribute lists must have the same number of attributes (mismatch on '{name}')"
                );
            }
            number_of_vertices = Some(num);
        }

        number_of_vertices
    }

    /// Port of the private static `Geometry._textureCoordinateRotationPoints`.
    ///
    /// For remapping texture coordinates when rendering GroundPrimitives with
    /// materials. GroundPrimitive texture coordinates are computed to align
    /// with the cartographic coordinate system on the globe; however
    /// `EllipseGeometry`, `RectangleGeometry`, and `PolygonGeometry` all bake
    /// rotations into per-vertex texture coordinates using different
    /// strategies. This method is used by `EllipseGeometry` and
    /// `PolygonGeometry` to approximate the same visual effect.
    ///
    /// Returns 6 values specifying [minimum point, u extent, v extent] as
    /// points in the "cartographic" texture coordinate system.
    pub fn texture_coordinate_rotation_points(
        positions: &[Cartesian3],
        st_rotation: f64,
        ellipsoid: &Ellipsoid,
        bounding_rectangle: &Rectangle,
    ) -> [f64; 6] {
        // Create a local east-north-up coordinate system centered on the
        // polygon's bounding rectangle. Project the southwest, northwest, and
        // southeast corners of the bounding rectangle into the plane of ENU
        // as 2D points. These are the equivalents of (0,0), (0,1), and (1,0)
        // in the texture coordinate system computed in
        // ShadowVolumeAppearanceFS, aka "ENU texture space."
        let rectangle_center = Rectangle::center(bounding_rectangle);
        let mut enu_center = Cartesian3::default();
        Cartesian3::from_radians(
            rectangle_center.longitude,
            rectangle_center.latitude,
            Some(rectangle_center.height),
            Some(ellipsoid.radii_squared()),
            &mut enu_center,
        );
        let mut enu_to_fixed_frame = Matrix4::default();
        transforms::east_north_up_to_fixed_frame(
            &enu_center,
            Some(ellipsoid),
            &mut enu_to_fixed_frame,
        );
        let mut fixed_frame_to_enu = Matrix4::default();
        Matrix4::inverse(&enu_to_fixed_frame, &mut fixed_frame_to_enu);

        let bounding_points_carto = [
            Cartographic::new(
                bounding_rectangle.west,
                bounding_rectangle.south,
                0.0,
            ),
            Cartographic::new(
                bounding_rectangle.west,
                bounding_rectangle.north,
                0.0,
            ),
            Cartographic::new(
                bounding_rectangle.east,
                bounding_rectangle.south,
                0.0,
            ),
        ];

        let mut bounding_points_enu = [Cartesian2::default(); 3];
        for i in 0..3 {
            let carto = &bounding_points_carto[i];
            let mut pos_enu = Cartesian3::default();
            Cartesian3::from_radians(
                carto.longitude,
                carto.latitude,
                Some(carto.height),
                Some(ellipsoid.radii_squared()),
                &mut pos_enu,
            );
            let mut transformed = Cartesian3::default();
            Matrix4::multiply_by_point_as_vector(
                &fixed_frame_to_enu,
                &pos_enu,
                &mut transformed,
            );
            bounding_points_enu[i].x = transformed.x;
            bounding_points_enu[i].y = transformed.y;
        }

        // Rotate each point in the polygon around the up vector in the ENU by
        // -stRotation and project into ENU as 2D. Compute the bounding box of
        // these rotated points in the 2D ENU plane. Rotate the corners back
        // by stRotation, then compute their equivalents in the ENU texture
        // space using the corners computed earlier.
        let rotation = Quaternion::from_axis_angle_new(&Cartesian3::UNIT_Z, -st_rotation);
        let texture_matrix = Matrix3::from_quaternion_new(&rotation);

        let mut enu_min_x = f64::INFINITY;
        let mut enu_min_y = f64::INFINITY;
        let mut enu_max_x = f64::NEG_INFINITY;
        let mut enu_max_y = f64::NEG_INFINITY;
        for position in positions {
            let mut pos_enu = Cartesian3::default();
            Matrix4::multiply_by_point_as_vector(
                &fixed_frame_to_enu,
                position,
                &mut pos_enu,
            );
            let rotated = Matrix3::multiply_by_vector_new(&texture_matrix, &pos_enu);

            enu_min_x = enu_min_x.min(rotated.x);
            enu_min_y = enu_min_y.min(rotated.y);
            enu_max_x = enu_max_x.max(rotated.x);
            enu_max_y = enu_max_y.max(rotated.y);
        }

        let to_desired_in_computed = Matrix2::from_rotation_new(st_rotation);

        let mut points_2d = [
            Cartesian2::new(enu_min_x, enu_min_y),
            Cartesian2::new(enu_min_x, enu_max_y),
            Cartesian2::new(enu_max_x, enu_min_y),
        ];

        let bounding_enu_min = bounding_points_enu[0];
        let bounding_points_width = bounding_points_enu[2].x - bounding_enu_min.x;
        let bounding_points_height = bounding_points_enu[1].y - bounding_enu_min.y;

        for point_2d in points_2d.iter_mut() {
            // rotate back
            let rotated_back =
                Matrix2::multiply_by_vector_new(&to_desired_in_computed, point_2d);

            // Convert point into east-north texture coordinate space
            point_2d.x = (rotated_back.x - bounding_enu_min.x) / bounding_points_width;
            point_2d.y = (rotated_back.y - bounding_enu_min.y) / bounding_points_height;
        }

        let mut result = [0.0f64; 6];
        Cartesian2::pack(&points_2d[0], &mut result, None);
        Cartesian2::pack(&points_2d[1], &mut result, Some(2));
        Cartesian2::pack(&points_2d[2], &mut result, Some(4));

        result
    }
}
