//! Ported from `packages/engine/Source/Core/GroundPolylineGeometry.js`.
//!
//! A description of a polyline on terrain or 3D Tiles. Only to be used with
//! `GroundPolylinePrimitive`.

use std::collections::HashMap;

use crate::approximate_terrain_heights::get_minimum_maximum_heights;
use crate::arc_type::ArcType;
use crate::array_remove_duplicates::array_remove_duplicates;
use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::component_datatype::ComponentDatatype;
use crate::ellipsoid::Ellipsoid;
use crate::ellipsoid_geodesic::EllipsoidGeodesic;
use crate::ellipsoid_rhumb_line::EllipsoidRhumbLine;
use crate::encoded_cartesian3::EncodedCartesian3;
use crate::geographic_projection::GeographicProjection;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_type::GeometryType;
use crate::index_datatype::{IndexDatatype, IndexStorage};
use crate::intersection_tests::IntersectionTests;
use crate::math::CesiumMath;
use crate::matrix3::Matrix3;
use crate::plane::Plane;
use crate::primitive_type::PrimitiveType;
use crate::quaternion::Quaternion;
use crate::rectangle::Rectangle;
use crate::web_mercator_projection::WebMercatorProjection;

const MITER_BREAK_SMALL: f64 = 0.8660254037844387; // cos(toRadians(30.0))
const MITER_BREAK_LARGE: f64 = -0.8660254037844387; // cos(toRadians(150.0))

// Initial heights for constructing the wall.
// Keeping WALL_INITIAL_MIN_HEIGHT near the ellipsoid surface helps
// prevent precision problems with planes in the shader.
const WALL_INITIAL_MIN_HEIGHT: f64 = 0.0;
const WALL_INITIAL_MAX_HEIGHT: f64 = 1000.0;

/// Map projections supported by `GroundPolylineGeometry` (JS `PROJECTIONS`).
#[derive(Debug, Clone)]
pub enum GroundPolylineProjection {
    /// `GeographicProjection`.
    Geographic(GeographicProjection),
    /// `WebMercatorProjection`.
    WebMercator(WebMercatorProjection),
}

impl GroundPolylineProjection {
    fn ellipsoid(&self) -> &Ellipsoid {
        match self {
            Self::Geographic(p) => p.ellipsoid(),
            Self::WebMercator(p) => p.ellipsoid(),
        }
    }

    fn project(&self, cartographic: &Cartographic) -> Cartesian3 {
        match self {
            Self::Geographic(p) => p.project(cartographic),
            Self::WebMercator(p) => p.project(cartographic),
        }
    }
}

/// A description of a polyline on terrain or 3D Tiles. Only to be used with
/// `GroundPolylinePrimitive`.
#[derive(Debug, Clone)]
pub struct GroundPolylineGeometry {
    /// The screen space width in pixels.
    pub width: f64,
    positions: Vec<Cartesian3>,
    /// The distance interval used for interpolating options.points. Zero
    /// indicates no interpolation.
    pub granularity: f64,
    /// Whether during geometry creation a line segment will be added between
    /// the last and first line positions to make this Polyline a loop.
    pub r#loop: bool,
    /// The type of path the polyline must follow.
    pub arc_type: ArcType,
    ellipsoid: Ellipsoid,
    // MapProjections can't be packed, so store the index to a known
    // MapProjection.
    projection_index: usize,
    // Used by GroundPolylinePrimitive to signal worker that scenemode is 3D
    // only.
    scene3d_only: bool,
}

impl GroundPolylineGeometry {
    /// Creates a new `GroundPolylineGeometry`.
    ///
    /// JS `new GroundPolylineGeometry(options)`.
    ///
    /// # Panics
    /// Panics (debug) if fewer than two positions are provided or `arc_type`
    /// is not `GEODESIC`/`RHUMB`.
    pub fn new(
        positions: Vec<Cartesian3>,
        width: Option<f64>,
        granularity: Option<f64>,
        r#loop: Option<bool>,
        arc_type: Option<ArcType>,
    ) -> Self {
        if cfg!(debug_assertions) {
            debug_assert!(
                positions.len() >= 2,
                "At least two positions are required."
            );
            if let Some(arc_type) = arc_type {
                debug_assert!(
                    arc_type == ArcType::Geodesic || arc_type == ArcType::Rhumb,
                    "Valid options for arcType are ArcType.GEODESIC and ArcType.RHUMB."
                );
            }
        }

        Self {
            width: width.unwrap_or(1.0),
            positions,
            granularity: granularity.unwrap_or(9999.0),
            r#loop: r#loop.unwrap_or(false),
            arc_type: arc_type.unwrap_or(ArcType::Geodesic),
            ellipsoid: Ellipsoid::WGS84,
            projection_index: 0,
            scene3d_only: false,
        }
    }

    /// The number of elements used to pack the object into an array.
    pub fn packed_length(&self) -> usize {
        1 + self.positions.len() * 3 + 1 + 1 + 1 + Ellipsoid::PACKED_LENGTH + 1 + 1
    }

    /// The positions (JS `_positions`, read by the specs).
    pub fn positions(&self) -> &[Cartesian3] {
        &self.positions
    }

    /// The ellipsoid (JS `_ellipsoid`, read by the specs).
    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }

    /// The projection index (JS `_projectionIndex`, read by the specs).
    pub fn projection_index(&self) -> usize {
        self.projection_index
    }

    /// Whether the geometry is 3D only (JS `_scene3DOnly`).
    pub fn scene3d_only(&self) -> bool {
        self.scene3d_only
    }

    /// Sets the 3D-only flag (JS assigns `_scene3DOnly` directly).
    pub fn set_scene3d_only(&mut self, value: bool) {
        self.scene3d_only = value;
    }

    /// Set the GroundPolylineGeometry's projection and ellipsoid.
    /// Used by GroundPolylinePrimitive to signal scene information to the
    /// geometry for generating 2D attributes.
    ///
    /// JS `GroundPolylineGeometry.setProjectionAndEllipsoid` (private).
    pub fn set_projection_and_ellipsoid(
        &mut self,
        map_projection: &GroundPolylineProjection,
    ) {
        self.projection_index = match map_projection {
            GroundPolylineProjection::Geographic(_) => 0,
            GroundPolylineProjection::WebMercator(_) => 1,
        };
        self.ellipsoid = map_projection.ellipsoid().clone();
    }

    /// Stores the provided instance into the provided array.
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        let mut index = starting_index.unwrap_or(0);

        let positions = &self.positions;
        let positions_length = positions.len();

        array[index] = positions_length as f64;
        index += 1;

        for cartesian in positions {
            Cartesian3::pack(cartesian, array, Some(index));
            index += 3;
        }

        array[index] = self.granularity;
        index += 1;
        array[index] = if self.r#loop { 1.0 } else { 0.0 };
        index += 1;
        array[index] = self.arc_type as i32 as f64;
        index += 1;

        Ellipsoid::pack(&self.ellipsoid, array, Some(index));
        index += Ellipsoid::PACKED_LENGTH;

        array[index] = self.projection_index as f64;
        index += 1;
        array[index] = if self.scene3d_only { 1.0 } else { 0.0 };
    }

    /// Retrieves an instance from a packed array.
    ///
    /// Mirrors JS `unpack`: when `result` is provided it is written in
    /// place (JS mutates the caller-provided scratch); otherwise a fresh
    /// instance is allocated. The populated instance is returned either
    /// way (JS returns the same `result` reference).
    pub fn unpack(
        array: &[f64],
        starting_index: Option<usize>,
        result: Option<&mut Self>,
    ) -> Self {
        let mut index = starting_index.unwrap_or(0);

        let positions_length = array[index] as usize;
        index += 1;

        let mut positions = Vec::with_capacity(positions_length);
        for _ in 0..positions_length {
            positions.push(Cartesian3::unpack_new(array, Some(index)));
            index += 3;
        }

        let granularity = array[index];
        index += 1;
        let loop_flag = array[index] == 1.0;
        index += 1;
        let arc_type = match array[index] as i32 {
            2 => ArcType::Rhumb,
            1 => ArcType::Geodesic,
            _ => ArcType::None,
        };
        index += 1;

        let ellipsoid = Ellipsoid::unpack(array, Some(index));
        index += Ellipsoid::PACKED_LENGTH;

        let projection_index = array[index] as usize;
        index += 1;
        let scene3d_only = array[index] == 1.0;

        let mut owned;
        let result = match result {
            Some(result) => result,
            None => {
                owned = Self::new(positions.clone(), None, None, None, None);
                &mut owned
            }
        };

        result.positions = positions;
        result.granularity = granularity;
        result.r#loop = loop_flag;
        result.arc_type = arc_type;
        result.ellipsoid = ellipsoid;
        result.projection_index = projection_index;
        result.scene3d_only = scene3d_only;

        result.clone()
    }

    /// Computes shadow volumes for the ground polyline, consisting of its
    /// vertices, indices, and a bounding sphere. Vertices are "fat," packing
    /// all the data needed in each volume to describe a line on terrain or
    /// 3D Tiles.
    ///
    /// JS `GroundPolylineGeometry.createGeometry`.
    pub fn create_geometry(ground_polyline_geometry: &Self) -> Option<Geometry> {
        let compute_2d_attributes = !ground_polyline_geometry.scene3d_only;
        let mut loop_flag = ground_polyline_geometry.r#loop;
        let ellipsoid = ground_polyline_geometry.ellipsoid.clone();
        let granularity = ground_polyline_geometry.granularity;
        let arc_type = ground_polyline_geometry.arc_type;
        let projection = match ground_polyline_geometry.projection_index {
            1 => GroundPolylineProjection::WebMercator(WebMercatorProjection::new(Some(
                ellipsoid.clone(),
            ))),
            _ => GroundPolylineProjection::Geographic(GeographicProjection::new(Some(
                ellipsoid.clone(),
            ))),
        };

        let min_height = WALL_INITIAL_MIN_HEIGHT;
        let max_height = WALL_INITIAL_MAX_HEIGHT;

        let positions = &ground_polyline_geometry.positions;
        let positions_length = positions.len();

        if positions_length == 2 {
            loop_flag = false;
        }

        // Split positions across the IDL and the Prime Meridian as well.
        let xz_plane = Plane::from_point_normal_new(&Cartesian3::ZERO, &Cartesian3::UNIT_Y);
        let mut rhumb_line = EllipsoidRhumbLine::new(None, None, None, Some(ellipsoid.clone()));

        let mut split_positions = vec![positions[0]];
        for i in 0..positions_length - 1 {
            split_segment(
                &positions[i],
                &positions[i + 1],
                arc_type,
                &ellipsoid,
                &xz_plane,
                &mut rhumb_line,
                &mut split_positions,
            );
            split_positions.push(positions[i + 1]);
        }

        if loop_flag {
            let p0 = positions[positions_length - 1];
            let p1 = positions[0];
            split_segment(
                &p0,
                &p1,
                arc_type,
                &ellipsoid,
                &xz_plane,
                &mut rhumb_line,
                &mut split_positions,
            );
        }

        let cartographics_length = split_positions.len();
        let mut cartographics = Vec::with_capacity(cartographics_length);
        for position in &split_positions {
            let mut cartographic = Cartographic::default();
            if Cartographic::from_cartesian(position, Some(&ellipsoid.ellipsoid_params()), &mut cartographic) {
                cartographic.height = 0.0;
                cartographics.push(cartographic);
            }
        }

        let deduplicated = array_remove_duplicates(
            &cartographics,
            |left: &Cartographic, right: &Cartographic, epsilon: f64| {
                Cartographic::equals_epsilon(Some(left), Some(right), Some(epsilon))
            },
            false,
            None,
        );
        let cartographics = deduplicated.unwrap_or(cartographics);
        let cartographics_length = cartographics.len();

        if cartographics_length < 2 {
            return None;
        }

        /**** Build arrays for positions, interpolated cartographics, and normals ****/
        let mut cartographics_array: Vec<f64> = Vec::new();
        let mut normals_array: Vec<f64> = Vec::new();
        let mut bottom_positions_array: Vec<f64> = Vec::new();
        let mut top_positions_array: Vec<f64> = Vec::new();

        // First point - either loop or attach a "perpendicular" normal
        let start_cartographic = cartographics[0].clone();
        let next_cartographic = cartographics[1].clone();

        let prestart_cartographic = cartographics[cartographics_length - 1].clone();
        let previous_bottom = get_position(&ellipsoid, &prestart_cartographic, min_height);
        let next_bottom = get_position(&ellipsoid, &next_cartographic, min_height);
        let vertex_bottom = get_position(&ellipsoid, &start_cartographic, min_height);
        let vertex_top = get_position(&ellipsoid, &start_cartographic, max_height);

        let mut vertex_normal = Cartesian3::default();
        if loop_flag {
            compute_vertex_miter_normal(
                &previous_bottom,
                &vertex_bottom,
                &vertex_top,
                &next_bottom,
                &mut vertex_normal,
            );
        } else {
            compute_right_normal(
                &start_cartographic,
                &next_cartographic,
                max_height,
                &ellipsoid,
                &mut vertex_normal,
            );
        }

        pack_cartesian3_append(&vertex_normal, &mut normals_array);
        pack_cartesian3_append(&vertex_bottom, &mut bottom_positions_array);
        pack_cartesian3_append(&vertex_top, &mut top_positions_array);
        cartographics_array.push(start_cartographic.latitude);
        cartographics_array.push(start_cartographic.longitude);

        interpolate_segment(
            &start_cartographic,
            &next_cartographic,
            min_height,
            max_height,
            granularity,
            arc_type,
            &ellipsoid,
            &mut normals_array,
            &mut bottom_positions_array,
            &mut top_positions_array,
            &mut cartographics_array,
        );

        // All inbetween points
        let mut vertex_bottom = vertex_bottom;
        let mut next_bottom = next_bottom;
        let mut vertex_top;
        for i in 1..cartographics_length - 1 {
            let previous_bottom = vertex_bottom;
            vertex_bottom = next_bottom;
            let vertex_cartographic = &cartographics[i];
            vertex_top = get_position(&ellipsoid, vertex_cartographic, max_height);
            next_bottom = get_position(&ellipsoid, &cartographics[i + 1], min_height);

            compute_vertex_miter_normal(
                &previous_bottom,
                &vertex_bottom,
                &vertex_top,
                &next_bottom,
                &mut vertex_normal,
            );

            pack_cartesian3_append(&vertex_normal, &mut normals_array);
            pack_cartesian3_append(&vertex_bottom, &mut bottom_positions_array);
            pack_cartesian3_append(&vertex_top, &mut top_positions_array);
            cartographics_array.push(vertex_cartographic.latitude);
            cartographics_array.push(vertex_cartographic.longitude);

            interpolate_segment(
                &cartographics[i],
                &cartographics[i + 1],
                min_height,
                max_height,
                granularity,
                arc_type,
                &ellipsoid,
                &mut normals_array,
                &mut bottom_positions_array,
                &mut top_positions_array,
                &mut cartographics_array,
            );
        }

        // Last point - either loop or attach a normal "perpendicular" to the
        // wall.
        let end_cartographic = cartographics[cartographics_length - 1].clone();
        let pre_end_cartographic = cartographics[cartographics_length - 2].clone();

        let vertex_bottom = get_position(&ellipsoid, &end_cartographic, min_height);
        let vertex_top = get_position(&ellipsoid, &end_cartographic, max_height);

        if loop_flag {
            let post_end_cartographic = cartographics[0].clone();
            let previous_bottom = get_position(&ellipsoid, &pre_end_cartographic, min_height);
            let next_bottom = get_position(&ellipsoid, &post_end_cartographic, min_height);

            compute_vertex_miter_normal(
                &previous_bottom,
                &vertex_bottom,
                &vertex_top,
                &next_bottom,
                &mut vertex_normal,
            );
        } else {
            compute_right_normal(
                &pre_end_cartographic,
                &end_cartographic,
                max_height,
                &ellipsoid,
                &mut vertex_normal,
            );
        }

        pack_cartesian3_append(&vertex_normal, &mut normals_array);
        pack_cartesian3_append(&vertex_bottom, &mut bottom_positions_array);
        pack_cartesian3_append(&vertex_top, &mut top_positions_array);
        cartographics_array.push(end_cartographic.latitude);
        cartographics_array.push(end_cartographic.longitude);

        if loop_flag {
            interpolate_segment(
                &end_cartographic,
                &start_cartographic,
                min_height,
                max_height,
                granularity,
                arc_type,
                &ellipsoid,
                &mut normals_array,
                &mut bottom_positions_array,
                &mut top_positions_array,
                &mut cartographics_array,
            );
            let index = normals_array.len();
            for i in 0..3 {
                normals_array.push(normals_array[i]);
                bottom_positions_array.push(bottom_positions_array[i]);
                top_positions_array.push(top_positions_array[i]);
            }
            let _ = index;
            cartographics_array.push(start_cartographic.latitude);
            cartographics_array.push(start_cartographic.longitude);
        }

        Some(generate_geometry_attributes(
            loop_flag,
            &projection,
            &bottom_positions_array,
            &top_positions_array,
            &normals_array,
            &cartographics_array,
            compute_2d_attributes,
        ))
    }
}

/// Mirrors the JS IDL/prime-meridian split block used for one segment.
fn split_segment(
    p0: &Cartesian3,
    p1: &Cartesian3,
    arc_type: ArcType,
    ellipsoid: &Ellipsoid,
    xz_plane: &Plane,
    rhumb_line: &mut EllipsoidRhumbLine,
    split_positions: &mut Vec<Cartesian3>,
) {
    let mut intersection = match IntersectionTests::line_segment_plane(p0, p1, xz_plane) {
        Some(intersection) => intersection,
        None => return,
    };

    if Cartesian3::equals_epsilon(Some(&intersection), Some(p0), Some(CesiumMath::EPSILON7), None)
        || Cartesian3::equals_epsilon(
            Some(&intersection),
            Some(p1),
            Some(CesiumMath::EPSILON7),
            None,
        )
    {
        return;
    }

    if arc_type == ArcType::Geodesic {
        split_positions.push(intersection);
    } else if arc_type == ArcType::Rhumb {
        let mut intersection_carto = Cartographic::default();
        if !ellipsoid.cartesian_to_cartographic(&intersection, &mut intersection_carto) {
            return;
        }
        let intersection_longitude = intersection_carto.longitude;

        let mut c0 = Cartographic::default();
        let mut c1 = Cartographic::default();
        if !ellipsoid.cartesian_to_cartographic(p0, &mut c0)
            || !ellipsoid.cartesian_to_cartographic(p1, &mut c1)
        {
            return;
        }
        rhumb_line.set_end_points(&c0, &c1);
        if let Some(mut intersection_cartographic) =
            rhumb_line.find_intersection_with_longitude(intersection_longitude)
        {
            let mut new_intersection = Cartesian3::default();
            ellipsoid.cartographic_to_cartesian(&mut intersection_cartographic, &mut new_intersection);
            intersection = new_intersection;
            if !Cartesian3::equals_epsilon(
                Some(&intersection),
                Some(p0),
                Some(CesiumMath::EPSILON7),
                None,
            ) && !Cartesian3::equals_epsilon(
                Some(&intersection),
                Some(p1),
                Some(CesiumMath::EPSILON7),
                None,
            ) {
                split_positions.push(intersection);
            }
        }
    }
}

/// Appends a Cartesian3 to the end of a growable packed array.
///
/// In JS, `Cartesian3.pack(value, array, index)` writes past the array
/// length and the array grows implicitly; Rust `Vec` requires explicit
/// growth, so this helper extends the buffer before packing.
fn pack_cartesian3_append(value: &Cartesian3, array: &mut Vec<f64>) {
    let index = array.len();
    array.resize(index + 3, 0.0);
    Cartesian3::pack(value, array, Some(index));
}

/// Mirrors JS `getPosition(ellipsoid, cartographic, height, result)`.
fn get_position(ellipsoid: &Ellipsoid, cartographic: &Cartographic, height: f64) -> Cartesian3 {
    let mut heightless = cartographic.clone();
    heightless.height = height;
    let mut result = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(&heightless, &mut result);
    result
}

/// Mirrors JS `direction(target, origin, result)`.
fn direction(target: &Cartesian3, origin: &Cartesian3) -> Cartesian3 {
    let result = Cartesian3::subtract_new(target, origin);
    Cartesian3::normalize_new(&result)
}

/// Mirrors JS `computeRightNormal(start, end, maxHeight, ellipsoid, result)`.
fn compute_right_normal(
    start: &Cartographic,
    end: &Cartographic,
    max_height: f64,
    ellipsoid: &Ellipsoid,
    result: &mut Cartesian3,
) {
    let start_bottom = get_position(ellipsoid, start, 0.0);
    let start_top = get_position(ellipsoid, start, max_height);
    let end_bottom = get_position(ellipsoid, end, 0.0);

    let up = direction(&start_top, &start_bottom);
    let forward = direction(&end_bottom, &start_bottom);

    Cartesian3::cross(&forward, &up, result);
    let mut normalized = Cartesian3::default();
    Cartesian3::normalize(result, &mut normalized);
    *result = normalized;
}

/// Mirrors JS `tangentDirection(target, origin, up, result)`.
fn tangent_direction(target: &Cartesian3, origin: &Cartesian3, up: &Cartesian3) -> Cartesian3 {
    let mut result = direction(target, origin);

    // orthogonalize
    let mut tmp = Cartesian3::default();
    Cartesian3::cross(&result, up, &mut tmp);
    let mut normalized = Cartesian3::default();
    Cartesian3::normalize(&tmp, &mut normalized);
    let mut out = Cartesian3::default();
    Cartesian3::cross(up, &normalized, &mut out);
    result = out;
    result
}

/// Mirrors JS `computeVertexMiterNormal(previousBottom, vertexBottom,
/// vertexTop, nextBottom, result)`.
fn compute_vertex_miter_normal(
    previous_bottom: &Cartesian3,
    vertex_bottom: &Cartesian3,
    vertex_top: &Cartesian3,
    next_bottom: &Cartesian3,
    result: &mut Cartesian3,
) {
    let up = direction(vertex_top, vertex_bottom);

    // Compute vectors pointing towards neighboring points but tangent to this
    // point on the ellipsoid
    let to_previous = tangent_direction(previous_bottom, vertex_bottom, &up);
    let to_next = tangent_direction(next_bottom, vertex_bottom, &up);

    // Check if tangents are almost opposite - if so, no need to miter.
    if CesiumMath::equals_epsilon(
        Cartesian3::dot(&to_previous, &to_next),
        -1.0,
        Some(CesiumMath::EPSILON5),
        None,
    ) {
        Cartesian3::cross(&up, &to_previous, result);
        let mut normalized = Cartesian3::default();
        Cartesian3::normalize(result, &mut normalized);
        *result = normalized;
        return;
    }

    // Average directions to previous and to next in the plane of Up
    let mut averaged = Cartesian3::add_new(&to_next, &to_previous);
    let mut normalized = Cartesian3::default();
    Cartesian3::normalize(&averaged, &mut normalized);
    averaged = normalized;

    // Flip the normal if it isn't pointing roughly bound right (aka if
    // forward is pointing more "backwards")
    let mut forward = Cartesian3::default();
    Cartesian3::cross(&up, &averaged, &mut forward);
    if Cartesian3::dot(&to_next, &forward) < 0.0 {
        Cartesian3::negate(&averaged, result);
    } else {
        *result = averaged;
    }
}

/// Mirrors JS `interpolateSegment`.
#[allow(clippy::too_many_arguments)]
fn interpolate_segment(
    start: &Cartographic,
    end: &Cartographic,
    min_height: f64,
    max_height: f64,
    granularity: f64,
    arc_type: ArcType,
    ellipsoid: &Ellipsoid,
    normals_array: &mut Vec<f64>,
    bottom_positions_array: &mut Vec<f64>,
    top_positions_array: &mut Vec<f64>,
    cartographics_array: &mut Vec<f64>,
) {
    if granularity == 0.0 {
        return;
    }

    enum EllipsoidLine {
        Geodesic(EllipsoidGeodesic),
        Rhumb(EllipsoidRhumbLine),
    }

    let ellipsoid_line = match arc_type {
        ArcType::Geodesic => EllipsoidLine::Geodesic(EllipsoidGeodesic::new(
            Some(start.clone()),
            Some(end.clone()),
            None,
            None,
            Some(ellipsoid.clone()),
        )),
        ArcType::Rhumb => EllipsoidLine::Rhumb(EllipsoidRhumbLine::new(
            Some(start.clone()),
            Some(end.clone()),
            None,
            Some(ellipsoid.clone()),
        )),
        ArcType::None => {
            // JS would throw on `undefined.surfaceDistance`; this is guarded
            // by the constructor debug check.
            debug_assert!(false, "arcType must be GEODESIC or RHUMB");
            return;
        }
    };

    let surface_distance = match &ellipsoid_line {
        EllipsoidLine::Geodesic(line) => line.surface_distance(),
        EllipsoidLine::Rhumb(line) => line.rhumb_distance(),
    };
    if surface_distance < granularity {
        return;
    }

    // Compute rightwards normal applicable at all interpolated points
    let mut interpolated_normal = Cartesian3::default();
    compute_right_normal(start, end, max_height, ellipsoid, &mut interpolated_normal);

    let segments = (surface_distance / granularity).ceil() as usize;
    let interpoint_distance = surface_distance / segments as f64;
    let mut distance_from_start = interpoint_distance;
    let points_to_add = segments - 1;
    for _ in 0..points_to_add {
        let interpolated_cartographic = match &ellipsoid_line {
            EllipsoidLine::Geodesic(line) => {
                line.interpolate_using_surface_distance(distance_from_start)
            }
            EllipsoidLine::Rhumb(line) => {
                line.interpolate_using_surface_distance(distance_from_start)
            }
        };
        let interpolated_bottom = get_position(ellipsoid, &interpolated_cartographic, min_height);
        let interpolated_top = get_position(ellipsoid, &interpolated_cartographic, max_height);

        pack_cartesian3_append(&interpolated_normal, normals_array);
        pack_cartesian3_append(&interpolated_bottom, bottom_positions_array);
        pack_cartesian3_append(&interpolated_top, top_positions_array);
        cartographics_array.push(interpolated_cartographic.latitude);
        cartographics_array.push(interpolated_cartographic.longitude);

        distance_from_start += interpoint_distance;
    }
}

// If the end normal angle is too steep compared to the direction of the line
// segment, "break" the miter by rotating the normal 90 degrees around the
// "up" direction at the point.
/// Mirrors JS `breakMiter`.
fn break_miter(
    end_geometry_normal: &mut Cartesian3,
    start_bottom: &Cartesian3,
    end_bottom: &Cartesian3,
    end_top: &Cartesian3,
) -> bool {
    let line_direction = direction(end_bottom, start_bottom);

    let dot = Cartesian3::dot(&line_direction, end_geometry_normal);
    if dot > MITER_BREAK_SMALL || dot < MITER_BREAK_LARGE {
        let vertex_up = direction(end_top, end_bottom);
        let angle = if dot < MITER_BREAK_LARGE {
            CesiumMath::PI_OVER_TWO
        } else {
            -CesiumMath::PI_OVER_TWO
        };
        let quaternion = Quaternion::from_axis_angle_new(&vertex_up, angle);
        let mut rotation_matrix = Matrix3::default();
        Matrix3::from_quaternion(&quaternion, &mut rotation_matrix);
        let mut rotated = Cartesian3::default();
        Matrix3::multiply_by_vector(&rotation_matrix, end_geometry_normal, &mut rotated);
        *end_geometry_normal = rotated;
        return true;
    }
    false
}

/// Mirrors JS `projectNormal` (exposed as
/// `GroundPolylineGeometry._projectNormal` for testing).
pub fn project_normal(
    projection: &GroundPolylineProjection,
    cartographic: &Cartographic,
    normal: &Cartesian3,
    projected_position: &Cartesian3,
    result: &mut Cartesian3,
) {
    let ellipsoid = projection.ellipsoid().clone();
    let mut position = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(cartographic, &mut position);

    let mut normal_endpoint = Cartesian3::add_new(&position, normal);
    let mut flip_normal = false;

    let mut normal_endpoint_cartographic = Cartographic::default();
    ellipsoid.cartesian_to_cartographic(&normal_endpoint, &mut normal_endpoint_cartographic);

    // If normal crosses the IDL, go the other way and flip the result.
    if (cartographic.longitude - normal_endpoint_cartographic.longitude).abs()
        > CesiumMath::PI_OVER_TWO
    {
        flip_normal = true;
        normal_endpoint = Cartesian3::subtract_new(&position, normal);
        ellipsoid.cartesian_to_cartographic(&normal_endpoint, &mut normal_endpoint_cartographic);
    }

    normal_endpoint_cartographic.height = 0.0;
    let normal_endpoint_projected = projection.project(&normal_endpoint_cartographic);
    let mut diff = Cartesian3::subtract_new(&normal_endpoint_projected, projected_position);
    diff.z = 0.0;
    let mut normalized = Cartesian3::default();
    Cartesian3::normalize(&diff, &mut normalized);
    if flip_normal {
        Cartesian3::negate(&normalized, result);
    } else {
        *result = normalized;
    }
}

/// Mirrors JS `adjustHeights`.
fn adjust_heights(
    bottom: &Cartesian3,
    top: &Cartesian3,
    min_height: f64,
    max_height: f64,
    adjust_height_bottom: &mut Cartesian3,
    adjust_height_top: &mut Cartesian3,
) {
    // bottom and top should be at WALL_INITIAL_MIN_HEIGHT and
    // WALL_INITIAL_MAX_HEIGHT, respectively
    let mut adjust_height_normal = Cartesian3::subtract_new(top, bottom);
    let mut tmp = Cartesian3::default();
    Cartesian3::normalize(&adjust_height_normal, &mut tmp);
    adjust_height_normal = tmp;

    let distance_for_bottom = min_height - WALL_INITIAL_MIN_HEIGHT;
    let adjust_height_offset =
        Cartesian3::multiply_by_scalar_new(&adjust_height_normal, distance_for_bottom);
    Cartesian3::add(bottom, &adjust_height_offset, adjust_height_bottom);

    let distance_for_top = max_height - WALL_INITIAL_MAX_HEIGHT;
    let adjust_height_offset =
        Cartesian3::multiply_by_scalar_new(&adjust_height_normal, distance_for_top);
    Cartesian3::add(top, &adjust_height_offset, adjust_height_top);
}

/// Mirrors JS `nudgeXZ`.
fn nudge_xz(start: &mut Cartesian3, end: &mut Cartesian3) {
    let xz_plane = Plane::from_point_normal_new(&Cartesian3::ZERO, &Cartesian3::UNIT_Y);
    let start_to_xz_distance = Plane::get_point_distance(&xz_plane, start);
    let end_to_xz_distance = Plane::get_point_distance(&xz_plane, end);
    // Larger epsilon than what's used in GeometryPipeline, a centimeter in
    // world space
    if CesiumMath::equals_epsilon(start_to_xz_distance, 0.0, Some(CesiumMath::EPSILON2), None) {
        let mut offset = direction(end, start);
        offset = Cartesian3::multiply_by_scalar_new(&offset, CesiumMath::EPSILON2);
        let mut nudged = Cartesian3::default();
        Cartesian3::add(start, &offset, &mut nudged);
        *start = nudged;
    } else if CesiumMath::equals_epsilon(end_to_xz_distance, 0.0, Some(CesiumMath::EPSILON2), None)
    {
        let mut offset = direction(start, end);
        offset = Cartesian3::multiply_by_scalar_new(&offset, CesiumMath::EPSILON2);
        let mut nudged = Cartesian3::default();
        Cartesian3::add(end, &offset, &mut nudged);
        *end = nudged;
    }
}

// "Nudge" cartographic coordinates so start and end are on the same side of
// the IDL. Only used for 2D/CV.
/// Mirrors JS `nudgeCartographic`.
fn nudge_cartographic(start: &mut Cartographic, end: &mut Cartographic) -> i32 {
    let abs_start_lon = start.longitude.abs();
    let abs_end_lon = end.longitude.abs();
    if CesiumMath::equals_epsilon(
        abs_start_lon,
        std::f64::consts::PI,
        Some(CesiumMath::EPSILON11),
        None,
    ) {
        let end_sign = CesiumMath::sign(end.longitude);
        start.longitude = end_sign * (abs_start_lon - CesiumMath::EPSILON11);
        1
    } else if CesiumMath::equals_epsilon(
        abs_end_lon,
        std::f64::consts::PI,
        Some(CesiumMath::EPSILON11),
        None,
    ) {
        let start_sign = CesiumMath::sign(start.longitude);
        end.longitude = start_sign * (abs_end_lon - CesiumMath::EPSILON11);
        2
    } else {
        0
    }
}

// Winding order is reversed so each segment's volume is inside-out
const REFERENCE_INDICES: [u32; 36] = [
    0, 2, 1, 0, 3, 2, // right
    0, 7, 3, 0, 4, 7, // start
    0, 5, 4, 0, 1, 5, // bottom
    5, 7, 4, 5, 6, 7, // left
    5, 2, 6, 5, 1, 2, // end
    3, 6, 2, 3, 7, 6, // top
];

/// Mirrors JS `generateGeometryAttributes`.
#[allow(clippy::too_many_arguments)]
fn generate_geometry_attributes(
    loop_flag: bool,
    projection: &GroundPolylineProjection,
    bottom_positions_array: &[f64],
    top_positions_array: &[f64],
    normals_array: &[f64],
    cartographics_array: &[f64],
    compute_2d_attributes: bool,
) -> Geometry {
    let ellipsoid = projection.ellipsoid().clone();

    // Each segment will have 8 vertices
    let segment_count = bottom_positions_array.len() / 3 - 1;
    let vertex_count = segment_count * 8;
    let array_size_vec4 = vertex_count * 4;
    let index_count = segment_count * 36;

    let mut indices = IndexDatatype::create_typed_array(vertex_count, index_count);
    let mut positions_array = vec![0.0f64; vertex_count * 3];

    let mut start_hi_and_forward_offset_x = vec![0.0f64; array_size_vec4];
    let mut start_lo_and_forward_offset_y = vec![0.0f64; array_size_vec4];
    let mut start_normal_and_forward_offset_z = vec![0.0f64; array_size_vec4];
    let mut end_normal_and_texture_coordinate_normalization_x = vec![0.0f64; array_size_vec4];
    let mut right_normal_and_texture_coordinate_normalization_y = vec![0.0f64; array_size_vec4];

    let mut start_hi_lo_2d = if compute_2d_attributes { Some(vec![0.0f64; array_size_vec4]) } else { None };
    let mut offset_and_right_2d = if compute_2d_attributes { Some(vec![0.0f64; array_size_vec4]) } else { None };
    let mut start_end_normals_2d = if compute_2d_attributes { Some(vec![0.0f64; array_size_vec4]) } else { None };
    let mut texcoord_normalization_2d =
        if compute_2d_attributes { Some(vec![0.0f64; vertex_count * 2]) } else { None };

    /*** Compute total lengths for texture coordinate normalization ***/
    let cartographics_length = cartographics_array.len() / 2;
    let mut length_2d = 0.0;

    let mut start_cartographic = Cartographic::default();
    start_cartographic.height = 0.0;
    let mut end_cartographic = Cartographic::default();
    end_cartographic.height = 0.0;

    if compute_2d_attributes {
        let mut index = 0usize;
        for _ in 1..cartographics_length {
            // Don't clone anything from previous segment b/c possible IDL touch
            start_cartographic.latitude = cartographics_array[index];
            start_cartographic.longitude = cartographics_array[index + 1];
            end_cartographic.latitude = cartographics_array[index + 2];
            end_cartographic.longitude = cartographics_array[index + 3];

            let segment_start_cartesian = projection.project(&start_cartographic);
            let segment_end_cartesian = projection.project(&end_cartographic);
            length_2d += Cartesian3::distance(&segment_start_cartesian, &segment_end_cartesian);
            index += 2;
        }
    }

    // 3D
    let positions_length = top_positions_array.len() / 3;
    let mut segment_end_cartesian = Cartesian3::unpack_new(top_positions_array, Some(0));
    let mut length_3d = 0.0;

    let mut index = 3usize;
    for _ in 1..positions_length {
        let segment_start_cartesian = segment_end_cartesian;
        segment_end_cartesian = Cartesian3::unpack_new(top_positions_array, Some(index));
        length_3d += Cartesian3::distance(&segment_start_cartesian, &segment_end_cartesian);
        index += 3;
    }

    /*** Generate segments ***/
    index = 3;
    let mut cartographics_index = 0usize;
    let mut vec2s_write_index = 0usize;
    let mut vec3s_write_index = 0usize;
    let mut vec4s_write_index = 0usize;
    let mut miter_broken = false;

    let mut end_bottom = Cartesian3::unpack_new(bottom_positions_array, Some(0));
    let mut end_top = Cartesian3::unpack_new(top_positions_array, Some(0));
    let mut end_geometry_normal = Cartesian3::unpack_new(normals_array, Some(0));

    if loop_flag {
        let pre_end_bottom = Cartesian3::unpack_new(
            bottom_positions_array,
            Some(bottom_positions_array.len() - 6),
        );
        if break_miter(&mut end_geometry_normal, &pre_end_bottom, &end_bottom, &end_top) {
            // Miter broken as if for the last point in the loop, needs to be
            // inverted for first point (clone of endBottom)
            end_geometry_normal = Cartesian3::negate_new(&end_geometry_normal);
        }
    }

    let mut length_so_far_3d = 0.0;
    let mut length_so_far_2d = 0.0;

    // For translating bounding volume
    let mut sum_heights = 0.0;

    for _ in 0..segment_count {
        let start_bottom = end_bottom;
        let start_top = end_top;
        let mut start_geometry_normal = end_geometry_normal;

        if miter_broken {
            start_geometry_normal = Cartesian3::negate_new(&start_geometry_normal);
        }

        end_bottom = Cartesian3::unpack_new(bottom_positions_array, Some(index));
        end_top = Cartesian3::unpack_new(top_positions_array, Some(index));
        end_geometry_normal = Cartesian3::unpack_new(normals_array, Some(index));

        miter_broken =
            break_miter(&mut end_geometry_normal, &start_bottom, &end_bottom, &end_top);

        // 2D - don't clone anything from previous segment b/c possible IDL
        // touch
        start_cartographic.latitude = cartographics_array[cartographics_index];
        start_cartographic.longitude = cartographics_array[cartographics_index + 1];
        end_cartographic.latitude = cartographics_array[cartographics_index + 2];
        end_cartographic.longitude = cartographics_array[cartographics_index + 3];

        let mut start_geometry_normal_2d = Cartesian3::default();
        let mut end_geometry_normal_2d = Cartesian3::default();
        let mut start_2d = Cartesian3::default();
        let mut end_2d = Cartesian3::default();

        if compute_2d_attributes {
            let nudge_result = nudge_cartographic(&mut start_cartographic, &mut end_cartographic);
            start_2d = projection.project(&start_cartographic);
            end_2d = projection.project(&end_cartographic);
            let mut direction_2d = direction(&end_2d, &start_2d);
            direction_2d.y = direction_2d.y.abs();

            if nudge_result == 0
                || Cartesian3::dot(&direction_2d, &Cartesian3::UNIT_Y) > MITER_BREAK_SMALL
            {
                // No nudge - project the original normal
                project_normal(
                    projection,
                    &start_cartographic,
                    &start_geometry_normal,
                    &start_2d,
                    &mut start_geometry_normal_2d,
                );
                project_normal(
                    projection,
                    &end_cartographic,
                    &end_geometry_normal,
                    &end_2d,
                    &mut end_geometry_normal_2d,
                );
            } else if nudge_result == 1 {
                // Start is close to IDL - snap start normal to align with IDL
                project_normal(
                    projection,
                    &end_cartographic,
                    &end_geometry_normal,
                    &end_2d,
                    &mut end_geometry_normal_2d,
                );
                start_geometry_normal_2d.x = 0.0;
                start_geometry_normal_2d.y = CesiumMath::sign(
                    start_cartographic.longitude - end_cartographic.longitude.abs(),
                );
                start_geometry_normal_2d.z = 0.0;
            } else {
                // End is close to IDL - snap end normal to align with IDL
                project_normal(
                    projection,
                    &start_cartographic,
                    &start_geometry_normal,
                    &start_2d,
                    &mut start_geometry_normal_2d,
                );
                end_geometry_normal_2d.x = 0.0;
                end_geometry_normal_2d.y = CesiumMath::sign(
                    start_cartographic.longitude - end_cartographic.longitude,
                );
                end_geometry_normal_2d.z = 0.0;
            }
        }

        /* 3D */
        let segment_length_3d = Cartesian3::distance(&start_top, &end_top);

        let encoded_start = EncodedCartesian3::from_cartesian(&start_bottom);
        let forward_offset = Cartesian3::subtract_new(&end_bottom, &start_bottom);
        let mut forward = Cartesian3::default();
        Cartesian3::normalize(&forward_offset, &mut forward);

        let mut start_up = Cartesian3::subtract_new(&start_top, &start_bottom);
        let mut tmp = Cartesian3::default();
        Cartesian3::normalize(&start_up, &mut tmp);
        start_up = tmp;

        let mut right_normal = Cartesian3::default();
        Cartesian3::cross(&forward, &start_up, &mut right_normal);
        Cartesian3::normalize(&right_normal, &mut tmp);
        right_normal = tmp;

        let mut start_plane_normal = Cartesian3::default();
        Cartesian3::cross(&start_up, &start_geometry_normal, &mut start_plane_normal);
        Cartesian3::normalize(&start_plane_normal, &mut tmp);
        start_plane_normal = tmp;

        let mut end_up = Cartesian3::subtract_new(&end_top, &end_bottom);
        Cartesian3::normalize(&end_up, &mut tmp);
        end_up = tmp;

        let mut end_plane_normal = Cartesian3::default();
        Cartesian3::cross(&end_geometry_normal, &end_up, &mut end_plane_normal);
        Cartesian3::normalize(&end_plane_normal, &mut tmp);
        end_plane_normal = tmp;

        let texcoord_normalization_3d_x = segment_length_3d / length_3d;
        let texcoord_normalization_3d_y = length_so_far_3d / length_3d;

        /* 2D */
        let mut segment_length_2d = 0.0;
        let mut encoded_start_2d = EncodedCartesian3::default();
        let mut forward_offset_2d = Cartesian3::default();
        let mut right_2d = Cartesian3::default();
        let mut texcoord_normalization_2d_x = 0.0;
        let mut texcoord_normalization_2d_y = 0.0;
        if compute_2d_attributes {
            segment_length_2d = Cartesian3::distance(&start_2d, &end_2d);

            encoded_start_2d = EncodedCartesian3::from_cartesian(&start_2d);
            forward_offset_2d = Cartesian3::subtract_new(&end_2d, &start_2d);

            // Right direction is just forward direction rotated by -90 degrees
            // around Z. Similarly with plane normals
            let mut normalized = Cartesian3::default();
            Cartesian3::normalize(&forward_offset_2d, &mut normalized);
            right_2d = normalized;
            let swap = right_2d.x;
            right_2d.x = right_2d.y;
            right_2d.y = -swap;

            texcoord_normalization_2d_x = segment_length_2d / length_2d;
            texcoord_normalization_2d_y = length_so_far_2d / length_2d;
        }

        /* Pack */
        for j in 0..8usize {
            let vec4_index = vec4s_write_index + j * 4;
            let vec2_index = vec2s_write_index + j * 2;
            let w_index = vec4_index + 3;

            // Encode sidedness of vertex relative to right plane in texture
            // coordinate normalization X, whether vertex is top or bottom of
            // volume in sign/magnitude of normalization Y.
            let right_plane_side = if j < 4 { 1.0 } else { -1.0 };
            let top_bottom_side =
                if j == 2 || j == 3 || j == 6 || j == 7 { 1.0 } else { -1.0 };

            // 3D
            Cartesian3::pack(
                &encoded_start.high,
                &mut start_hi_and_forward_offset_x,
                Some(vec4_index),
            );
            start_hi_and_forward_offset_x[w_index] = forward_offset.x;

            Cartesian3::pack(
                &encoded_start.low,
                &mut start_lo_and_forward_offset_y,
                Some(vec4_index),
            );
            start_lo_and_forward_offset_y[w_index] = forward_offset.y;

            Cartesian3::pack(
                &start_plane_normal,
                &mut start_normal_and_forward_offset_z,
                Some(vec4_index),
            );
            start_normal_and_forward_offset_z[w_index] = forward_offset.z;

            Cartesian3::pack(
                &end_plane_normal,
                &mut end_normal_and_texture_coordinate_normalization_x,
                Some(vec4_index),
            );
            end_normal_and_texture_coordinate_normalization_x[w_index] =
                texcoord_normalization_3d_x * right_plane_side;

            Cartesian3::pack(
                &right_normal,
                &mut right_normal_and_texture_coordinate_normalization_y,
                Some(vec4_index),
            );

            let mut texcoord_normalization = texcoord_normalization_3d_y * top_bottom_side;
            if texcoord_normalization == 0.0 && top_bottom_side < 0.0 {
                texcoord_normalization = 9.0; // some value greater than 1.0
            }
            right_normal_and_texture_coordinate_normalization_y[w_index] = texcoord_normalization;

            // 2D
            if compute_2d_attributes {
                if let Some(start_hi_lo_2d) = start_hi_lo_2d.as_mut() {
                    start_hi_lo_2d[vec4_index] = encoded_start_2d.high.x;
                    start_hi_lo_2d[vec4_index + 1] = encoded_start_2d.high.y;
                    start_hi_lo_2d[vec4_index + 2] = encoded_start_2d.low.x;
                    start_hi_lo_2d[vec4_index + 3] = encoded_start_2d.low.y;
                }
                if let Some(start_end_normals_2d) = start_end_normals_2d.as_mut() {
                    start_end_normals_2d[vec4_index] = -start_geometry_normal_2d.y;
                    start_end_normals_2d[vec4_index + 1] = start_geometry_normal_2d.x;
                    start_end_normals_2d[vec4_index + 2] = end_geometry_normal_2d.y;
                    start_end_normals_2d[vec4_index + 3] = -end_geometry_normal_2d.x;
                }
                if let Some(offset_and_right_2d) = offset_and_right_2d.as_mut() {
                    offset_and_right_2d[vec4_index] = forward_offset_2d.x;
                    offset_and_right_2d[vec4_index + 1] = forward_offset_2d.y;
                    offset_and_right_2d[vec4_index + 2] = right_2d.x;
                    offset_and_right_2d[vec4_index + 3] = right_2d.y;
                }
                if let Some(texcoord_normalization_2d) = texcoord_normalization_2d.as_mut() {
                    texcoord_normalization_2d[vec2_index] =
                        texcoord_normalization_2d_x * right_plane_side;

                    let mut texcoord_normalization =
                        texcoord_normalization_2d_y * top_bottom_side;
                    if texcoord_normalization == 0.0 && top_bottom_side < 0.0 {
                        texcoord_normalization = 9.0; // some value greater than 1.0
                    }
                    texcoord_normalization_2d[vec2_index + 1] = texcoord_normalization;
                }
            }
        }

        // Adjust height of volume in 3D
        let get_height_cartographics = [start_cartographic, end_cartographic];
        let get_heights_rectangle = Rectangle::from_cartographic_array(&get_height_cartographics);
        let min_max_heights = get_minimum_maximum_heights(
            Some(&get_heights_rectangle),
            Some(&ellipsoid),
        );
        let min_height = min_max_heights.minimum_terrain_height;
        let max_height = min_max_heights.maximum_terrain_height;

        // Sum using abs() to properly account for negative elevations in
        // calculating bounding sphere radius
        sum_heights += min_height.abs();
        sum_heights += max_height.abs();

        let mut adjust_height_start_bottom = Cartesian3::default();
        let mut adjust_height_start_top = Cartesian3::default();
        adjust_heights(
            &start_bottom,
            &start_top,
            min_height,
            max_height,
            &mut adjust_height_start_bottom,
            &mut adjust_height_start_top,
        );
        let mut adjust_height_end_bottom = Cartesian3::default();
        let mut adjust_height_end_top = Cartesian3::default();
        adjust_heights(
            &end_bottom,
            &end_top,
            min_height,
            max_height,
            &mut adjust_height_end_bottom,
            &mut adjust_height_end_top,
        );

        // Nudge the positions away from the "polyline" a little bit to
        // prevent errors in GeometryPipeline
        let mut normal_nudge =
            Cartesian3::multiply_by_scalar_new(&right_normal, CesiumMath::EPSILON5);
        let mut nudged = Cartesian3::default();
        Cartesian3::add(&adjust_height_start_bottom, &normal_nudge, &mut nudged);
        adjust_height_start_bottom = nudged;
        Cartesian3::add(&adjust_height_end_bottom, &normal_nudge, &mut nudged);
        adjust_height_end_bottom = nudged;
        Cartesian3::add(&adjust_height_start_top, &normal_nudge, &mut nudged);
        adjust_height_start_top = nudged;
        Cartesian3::add(&adjust_height_end_top, &normal_nudge, &mut nudged);
        adjust_height_end_top = nudged;

        // If the segment is very close to the XZ plane, nudge the vertices
        // slightly to avoid touching it.
        nudge_xz(&mut adjust_height_start_bottom, &mut adjust_height_end_bottom);
        nudge_xz(&mut adjust_height_start_top, &mut adjust_height_end_top);

        Cartesian3::pack(
            &adjust_height_start_bottom,
            &mut positions_array,
            Some(vec3s_write_index),
        );
        Cartesian3::pack(
            &adjust_height_end_bottom,
            &mut positions_array,
            Some(vec3s_write_index + 3),
        );
        Cartesian3::pack(
            &adjust_height_end_top,
            &mut positions_array,
            Some(vec3s_write_index + 6),
        );
        Cartesian3::pack(
            &adjust_height_start_top,
            &mut positions_array,
            Some(vec3s_write_index + 9),
        );

        normal_nudge = Cartesian3::multiply_by_scalar_new(&right_normal, -2.0 * CesiumMath::EPSILON5);
        Cartesian3::add(&adjust_height_start_bottom, &normal_nudge, &mut nudged);
        adjust_height_start_bottom = nudged;
        Cartesian3::add(&adjust_height_end_bottom, &normal_nudge, &mut nudged);
        adjust_height_end_bottom = nudged;
        Cartesian3::add(&adjust_height_start_top, &normal_nudge, &mut nudged);
        adjust_height_start_top = nudged;
        Cartesian3::add(&adjust_height_end_top, &normal_nudge, &mut nudged);
        adjust_height_end_top = nudged;

        nudge_xz(&mut adjust_height_start_bottom, &mut adjust_height_end_bottom);
        nudge_xz(&mut adjust_height_start_top, &mut adjust_height_end_top);

        Cartesian3::pack(
            &adjust_height_start_bottom,
            &mut positions_array,
            Some(vec3s_write_index + 12),
        );
        Cartesian3::pack(
            &adjust_height_end_bottom,
            &mut positions_array,
            Some(vec3s_write_index + 15),
        );
        Cartesian3::pack(
            &adjust_height_end_top,
            &mut positions_array,
            Some(vec3s_write_index + 18),
        );
        Cartesian3::pack(
            &adjust_height_start_top,
            &mut positions_array,
            Some(vec3s_write_index + 21),
        );

        cartographics_index += 2;
        index += 3;

        vec2s_write_index += 16;
        vec3s_write_index += 24;
        vec4s_write_index += 32;

        length_so_far_3d += segment_length_3d;
        length_so_far_2d += segment_length_2d;
    }

    index = 0;
    let mut index_offset = 0u32;
    for _ in 0..segment_count {
        for j in 0..REFERENCE_INDICES.len() {
            write_index(&mut indices, index + j, REFERENCE_INDICES[j] + index_offset);
        }
        index_offset += 8;
        index += REFERENCE_INDICES.len();
    }

    let bottom_bs = BoundingSphere::from_vertices(
        bottom_positions_array,
        Some(&Cartesian3::ZERO),
        Some(3),
        None,
    );
    let top_bs = BoundingSphere::from_vertices(
        top_positions_array,
        Some(&Cartesian3::ZERO),
        Some(3),
        None,
    );
    let mut bounding_sphere = BoundingSphere::from_bounding_spheres(&[bottom_bs, top_bs], None);

    // Adjust bounding sphere height and radius to cover more of the volume
    bounding_sphere.radius += sum_heights / (segment_count as f64 * 2.0);

    let mut attributes = HashMap::new();
    attributes.insert(
        "position".to_string(),
        GeometryAttribute::new(ComponentDatatype::Double, 3, false, positions_array),
    );
    attributes.insert(
        "startHiAndForwardOffsetX".to_string(),
        get_vec4_geometry_attribute(start_hi_and_forward_offset_x),
    );
    attributes.insert(
        "startLoAndForwardOffsetY".to_string(),
        get_vec4_geometry_attribute(start_lo_and_forward_offset_y),
    );
    attributes.insert(
        "startNormalAndForwardOffsetZ".to_string(),
        get_vec4_geometry_attribute(start_normal_and_forward_offset_z),
    );
    attributes.insert(
        "endNormalAndTextureCoordinateNormalizationX".to_string(),
        get_vec4_geometry_attribute(end_normal_and_texture_coordinate_normalization_x),
    );
    attributes.insert(
        "rightNormalAndTextureCoordinateNormalizationY".to_string(),
        get_vec4_geometry_attribute(right_normal_and_texture_coordinate_normalization_y),
    );

    if compute_2d_attributes {
        if let Some(start_hi_lo_2d) = start_hi_lo_2d {
            attributes.insert(
                "startHiLo2D".to_string(),
                get_vec4_geometry_attribute(start_hi_lo_2d),
            );
        }
        if let Some(offset_and_right_2d) = offset_and_right_2d {
            attributes.insert(
                "offsetAndRight2D".to_string(),
                get_vec4_geometry_attribute(offset_and_right_2d),
            );
        }
        if let Some(start_end_normals_2d) = start_end_normals_2d {
            attributes.insert(
                "startEndNormals2D".to_string(),
                get_vec4_geometry_attribute(start_end_normals_2d),
            );
        }
        if let Some(texcoord_normalization_2d) = texcoord_normalization_2d {
            attributes.insert(
                "texcoordNormalization2D".to_string(),
                GeometryAttribute::new(
                    ComponentDatatype::Float,
                    2,
                    false,
                    texcoord_normalization_2d,
                ),
            );
        }
    }

    Geometry::with_all(
        attributes,
        Some(indices),
        Some(PrimitiveType::Triangles),
        Some(bounding_sphere),
        GeometryType::None,
        None,
        None,
    )
}

/// Mirrors JS `getVec4GeometryAttribute`.
fn get_vec4_geometry_attribute(values: Vec<f64>) -> GeometryAttribute {
    GeometryAttribute::new(ComponentDatatype::Float, 4, false, values)
}

/// Index write helper for [`IndexStorage`].
fn write_index(indices: &mut IndexStorage, i: usize, v: u32) {
    match indices {
        IndexStorage::U16(vec) => vec[i] = v as u16,
        IndexStorage::U32(vec) => vec[i] = v,
    }
}
