//! Ported from `packages/engine/Source/Core/PolygonGeometry.js`.
//!
//! A description of a polygon on an ellipsoid.

use std::collections::HashMap;
use std::f64::consts::PI;

use crate::arc_type::ArcType;
use crate::bounding_rectangle::BoundingRectangle;
use crate::bounding_sphere::BoundingSphere;
use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::component_datatype::ComponentDatatype;
use crate::coplanar_polygon_geometry::{
    hierarchy_packed_length_2d_pub, pack_hierarchy_2d_pub, pack_hierarchy_3d_pub,
    unpack_hierarchy_2d_pub, unpack_hierarchy_3d_pub, PolygonHierarchy2D,
};
use crate::ellipsoid::Ellipsoid;
use crate::ellipsoid_tangent_plane::EllipsoidTangentPlane;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_instance::{GeometryInstance, GeometryInstanceGeometry};
use crate::geometry_offset_attribute::GeometryOffsetAttribute;
use crate::geometry_pipeline::GeometryPipeline;
use crate::geometry_type::GeometryType;
use crate::index_datatype::{IndexDatatype, IndexStorage};
use crate::math::CesiumMath;
use crate::matrix3::Matrix3;
use crate::polygon_geometry_library::{
    HierarchyResultEntry, PolygonGeometryLibrary, PolygonResultEntry, PolygonTextureCoordinates,
};
use crate::polygon_hierarchy::PolygonHierarchy;
use crate::polygon_pipeline::PolygonPipeline;
use crate::primitive_type::PrimitiveType;
use crate::quaternion::Quaternion;
use crate::rectangle::Rectangle;
use crate::stereographic::Stereographic;
use crate::vertex_format::VertexFormat;
use crate::winding_order::WindingOrder;

/// A description of a polygon on an ellipsoid. Polygon geometry can be
/// rendered with both `Primitive` and `GroundPrimitive`.
#[derive(Debug, Clone)]
pub struct PolygonGeometry {
    polygon_hierarchy: PolygonHierarchy,
    ellipsoid: Ellipsoid,
    vertex_format: VertexFormat,
    height: f64,
    extruded_height: f64,
    granularity: f64,
    st_rotation: f64,
    per_position_height: bool,
    close_top: bool,
    close_bottom: bool,
    offset_attribute: Option<GeometryOffsetAttribute>,
    arc_type: ArcType,
    shadow_volume: bool,
    per_position_height_extrude: bool,
    texture_coordinates: Option<PolygonHierarchy2D>,
}

impl PolygonGeometry {
    /// Creates a new `PolygonGeometry` from a flat list of positions (the
    /// Rust equivalent of the JS constructor with a `polygonHierarchy`
    /// built from `positions`).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        positions: Vec<Cartesian3>,
        ellipsoid: Option<Ellipsoid>,
        vertex_format: Option<VertexFormat>,
        height: Option<f64>,
        extruded_height: Option<f64>,
        granularity: Option<f64>,
        st_rotation: Option<f64>,
        per_position_height: Option<bool>,
        close_top: Option<bool>,
        close_bottom: Option<bool>,
        offset_attribute: Option<GeometryOffsetAttribute>,
        arc_type: Option<ArcType>,
    ) -> Self {
        Self::from_hierarchy(
            PolygonHierarchy::new(positions, Vec::new()),
            ellipsoid,
            vertex_format,
            height,
            extruded_height,
            granularity,
            st_rotation,
            per_position_height,
            close_top,
            close_bottom,
            offset_attribute,
            arc_type,
            None,
            None,
        )
    }

    /// Creates a new `PolygonGeometry` from a polygon hierarchy; the Rust
    /// equivalent of the JS `PolygonGeometry` constructor.
    #[allow(clippy::too_many_arguments)]
    pub fn from_hierarchy(
        polygon_hierarchy: PolygonHierarchy,
        ellipsoid: Option<Ellipsoid>,
        vertex_format: Option<VertexFormat>,
        height: Option<f64>,
        extruded_height: Option<f64>,
        granularity: Option<f64>,
        st_rotation: Option<f64>,
        per_position_height: Option<bool>,
        close_top: Option<bool>,
        close_bottom: Option<bool>,
        offset_attribute: Option<GeometryOffsetAttribute>,
        arc_type: Option<ArcType>,
        shadow_volume: Option<bool>,
        texture_coordinates: Option<PolygonHierarchy2D>,
    ) -> Self {
        if cfg!(debug_assertions) {
            if per_position_height == Some(true) && height.is_some() {
                panic!("Cannot use both options.perPositionHeight and options.height");
            }
            if let Some(at) = arc_type {
                if at != ArcType::Geodesic && at != ArcType::Rhumb {
                    panic!("Invalid arcType. Valid options are ArcType.GEODESIC and ArcType.RHUMB.");
                }
            }
        }

        let per_position_height = per_position_height.unwrap_or(false);
        let per_position_height_extrude = per_position_height && extruded_height.is_some();
        let height = height.unwrap_or(0.0);
        let extruded_height = extruded_height.unwrap_or(height);
        let (height, extruded_height) = if per_position_height_extrude {
            (height, extruded_height)
        } else {
            (height.max(extruded_height), height.min(extruded_height))
        };

        Self {
            polygon_hierarchy,
            ellipsoid: ellipsoid.unwrap_or(Ellipsoid::WGS84),
            vertex_format: vertex_format.unwrap_or_default(),
            height,
            extruded_height,
            granularity: granularity.unwrap_or(CesiumMath::RADIANS_PER_DEGREE),
            st_rotation: st_rotation.unwrap_or(0.0),
            per_position_height,
            close_top: close_top.unwrap_or(true),
            close_bottom: close_bottom.unwrap_or(true),
            offset_attribute,
            arc_type: arc_type.unwrap_or(ArcType::Geodesic),
            shadow_volume: shadow_volume.unwrap_or(false),
            per_position_height_extrude,
            texture_coordinates,
        }
    }

    /// Creates a polygon geometry from positions (JS static
    /// `PolygonGeometry.fromPositions`).
    #[allow(clippy::too_many_arguments)]
    pub fn from_positions(
        positions: Vec<Cartesian3>,
        ellipsoid: Option<Ellipsoid>,
        vertex_format: Option<VertexFormat>,
        height: Option<f64>,
        extruded_height: Option<f64>,
        granularity: Option<f64>,
        st_rotation: Option<f64>,
        per_position_height: Option<bool>,
        close_top: Option<bool>,
        close_bottom: Option<bool>,
        offset_attribute: Option<GeometryOffsetAttribute>,
        arc_type: Option<ArcType>,
        texture_coordinates: Option<PolygonHierarchy2D>,
    ) -> Self {
        Self::from_hierarchy(
            PolygonHierarchy::new(positions, Vec::new()),
            ellipsoid,
            vertex_format,
            height,
            extruded_height,
            granularity,
            st_rotation,
            per_position_height,
            close_top,
            close_bottom,
            offset_attribute,
            arc_type,
            None,
            texture_coordinates,
        )
    }

    /// The number of elements used to pack the object into an array.
    ///
    /// DEVIATION: JS `packedLength` is an instance property computed in the
    /// constructor; Rust exposes it as `packed_length(&self)`.
    pub fn packed_length(&self) -> usize {
        PolygonGeometryLibrary::compute_hierarchy_packed_length(&self.polygon_hierarchy)
            + Ellipsoid::PACKED_LENGTH
            + VertexFormat::PACKED_LENGTH
            + match &self.texture_coordinates {
                Some(texture_coordinates) => hierarchy_packed_length_2d_pub(texture_coordinates),
                None => 1,
            }
            + 12
    }

    /// Stores the provided instance into the provided array (JS static
    /// `PolygonGeometry.pack`).
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        let mut si = starting_index.unwrap_or(0);

        si = pack_hierarchy_3d_pub(&self.polygon_hierarchy, array, si);

        Ellipsoid::pack(&self.ellipsoid, array, Some(si));
        si += Ellipsoid::PACKED_LENGTH;

        self.vertex_format.pack(array, si);
        si += VertexFormat::PACKED_LENGTH;

        array[si] = self.height;
        si += 1;
        array[si] = self.extruded_height;
        si += 1;
        array[si] = self.granularity;
        si += 1;
        array[si] = self.st_rotation;
        si += 1;
        array[si] = if self.per_position_height_extrude { 1.0 } else { 0.0 };
        si += 1;
        array[si] = if self.per_position_height { 1.0 } else { 0.0 };
        si += 1;
        array[si] = if self.close_top { 1.0 } else { 0.0 };
        si += 1;
        array[si] = if self.close_bottom { 1.0 } else { 0.0 };
        si += 1;
        array[si] = if self.shadow_volume { 1.0 } else { 0.0 };
        si += 1;
        array[si] = match &self.offset_attribute {
            Some(v) => *v as u32 as f64,
            None => -1.0,
        };
        si += 1;
        array[si] = self.arc_type as i32 as f64;
        si += 1;

        match &self.texture_coordinates {
            Some(texture_coordinates) => {
                si = pack_hierarchy_2d_pub(texture_coordinates, array, si);
            }
            None => {
                array[si] = -1.0;
                si += 1;
            }
        }
        array[si] = self.packed_length() as f64;
    }

    /// Retrieves an instance from a packed array (JS static
    /// `PolygonGeometry.unpack`).
    ///
    /// DEVIATION: JS assigns the packed `packedLength` back onto the
    /// instance; Rust recomputes it on demand.
    pub fn unpack(array: &[f64], starting_index: Option<usize>, result: Option<&mut Self>) -> Self {
        let mut si = starting_index.unwrap_or(0);

        let (polygon_hierarchy, next) = unpack_hierarchy_3d_pub(array, si);
        si = next;

        let ellipsoid = Ellipsoid::unpack(array, Some(si));
        si += Ellipsoid::PACKED_LENGTH;

        let vertex_format = VertexFormat::unpack(array, si, None);
        si += VertexFormat::PACKED_LENGTH;

        let height = array[si];
        si += 1;
        let extruded_height = array[si];
        si += 1;
        let granularity = array[si];
        si += 1;
        let st_rotation = array[si];
        si += 1;
        let per_position_height_extrude = array[si] == 1.0;
        si += 1;
        let per_position_height = array[si] == 1.0;
        si += 1;
        let close_top = array[si] == 1.0;
        si += 1;
        let close_bottom = array[si] == 1.0;
        si += 1;
        let shadow_volume = array[si] == 1.0;
        si += 1;
        let offset_attribute_raw = array[si];
        si += 1;
        let arc_type_raw = array[si];
        si += 1;

        let texture_coordinates: Option<PolygonHierarchy2D> = if array[si] == -1.0 {
            si += 1;
            None
        } else {
            let (texture_coordinates, next) = unpack_hierarchy_2d_pub(array, si);
            si = next;
            Some(texture_coordinates)
        };
        let _packed_length = array[si];

        let offset_attribute = if offset_attribute_raw == -1.0 {
            None
        } else {
            GeometryOffsetAttribute::try_from_u32(offset_attribute_raw as u32)
        };
        let arc_type = match arc_type_raw as i32 {
            0 => ArcType::None,
            2 => ArcType::Rhumb,
            _ => ArcType::Geodesic,
        };

        // JS assigns raw field values on both paths (no constructor
        // normalization), so the two paths behave identically.
        fn assign(
            target: &mut PolygonGeometry,
            polygon_hierarchy: PolygonHierarchy,
            ellipsoid: Ellipsoid,
            vertex_format: VertexFormat,
            height: f64,
            extruded_height: f64,
            granularity: f64,
            st_rotation: f64,
            per_position_height_extrude: bool,
            per_position_height: bool,
            close_top: bool,
            close_bottom: bool,
            shadow_volume: bool,
            offset_attribute: Option<GeometryOffsetAttribute>,
            arc_type: ArcType,
            texture_coordinates: Option<PolygonHierarchy2D>,
        ) {
            target.polygon_hierarchy = polygon_hierarchy;
            target.ellipsoid = ellipsoid;
            target.vertex_format = vertex_format;
            target.height = height;
            target.extruded_height = extruded_height;
            target.granularity = granularity;
            target.st_rotation = st_rotation;
            target.per_position_height_extrude = per_position_height_extrude;
            target.per_position_height = per_position_height;
            target.close_top = close_top;
            target.close_bottom = close_bottom;
            target.shadow_volume = shadow_volume;
            target.offset_attribute = offset_attribute;
            target.arc_type = arc_type;
            target.texture_coordinates = texture_coordinates;
        }

        match result {
            None => {
                let mut g = Self::from_hierarchy(
                    PolygonHierarchy::new(Vec::new(), Vec::new()),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
                assign(
                    &mut g,
                    polygon_hierarchy,
                    ellipsoid,
                    vertex_format,
                    height,
                    extruded_height,
                    granularity,
                    st_rotation,
                    per_position_height_extrude,
                    per_position_height,
                    close_top,
                    close_bottom,
                    shadow_volume,
                    offset_attribute,
                    arc_type,
                    texture_coordinates,
                );
                g
            }
            Some(r) => {
                assign(
                    r,
                    polygon_hierarchy,
                    ellipsoid,
                    vertex_format,
                    height,
                    extruded_height,
                    granularity,
                    st_rotation,
                    per_position_height_extrude,
                    per_position_height,
                    close_top,
                    close_bottom,
                    shadow_volume,
                    offset_attribute,
                    arc_type,
                    texture_coordinates,
                );
                r.clone()
            }
        }
    }

    /// Creates a shadow volume polygon geometry from this geometry (JS
    /// private `PolygonGeometry.createShadowVolume`).
    pub fn create_shadow_volume(
        polygon_geometry: &Self,
        min_height_func: &dyn Fn(f64, &Ellipsoid) -> f64,
        max_height_func: &dyn Fn(f64, &Ellipsoid) -> f64,
    ) -> Self {
        let granularity = polygon_geometry.granularity;
        let ellipsoid = polygon_geometry.ellipsoid.clone();

        let min_height = min_height_func(granularity, &ellipsoid);
        let max_height = max_height_func(granularity, &ellipsoid);

        Self::from_hierarchy(
            polygon_geometry.polygon_hierarchy.clone(),
            Some(ellipsoid),
            Some(VertexFormat::position_only()),
            Some(max_height),
            Some(min_height),
            Some(granularity),
            Some(polygon_geometry.st_rotation),
            Some(false),
            None,
            None,
            None,
            Some(polygon_geometry.arc_type),
            Some(true),
            None,
        )
    }

    /// JS `Object.defineProperties` `rectangle` getter.
    ///
    /// DEVIATION: JS caches the result on first access; Rust recomputes on
    /// each call.
    pub fn rectangle(&self) -> Rectangle {
        Self::compute_rectangle_from_positions(
            &self.polygon_hierarchy.positions,
            Some(self.ellipsoid.clone()),
            Some(self.arc_type),
            None,
        )
    }

    /// Computes a rectangle which encloses the polygon defined by the list
    /// of positions, including cases over the international date line and
    /// the poles (JS static `PolygonGeometry.computeRectangleFromPositions`).
    pub fn compute_rectangle_from_positions(
        positions: &[Cartesian3],
        ellipsoid: Option<Ellipsoid>,
        arc_type: Option<ArcType>,
        result: Option<Rectangle>,
    ) -> Rectangle {
        let mut result = result.unwrap_or_default();

        if positions.len() < 3 {
            return result;
        }

        let arc_type = arc_type.unwrap_or(ArcType::Geodesic);

        result.west = f64::INFINITY;
        result.east = f64::NEG_INFINITY;
        result.south = f64::INFINITY;
        result.north = f64::NEG_INFINITY;

        let mut polygon = PolygonAngleScratch {
            north_angle: 0.0,
            south_angle: 0.0,
            west_over_idl: f64::INFINITY,
            east_over_idl: f64::NEG_INFINITY,
        };

        let mut last_polar_position = Stereographic::from_cartesian(&positions[0], None);
        for position in positions.iter().skip(1) {
            let polar_position = Stereographic::from_cartesian(position, None);
            expand_rectangle(
                &polar_position,
                &last_polar_position,
                ellipsoid.as_ref(),
                arc_type,
                &mut polygon,
                &mut result,
            );
            last_polar_position = polar_position;
        }

        expand_rectangle(
            &Stereographic::from_cartesian(&positions[0], None),
            &last_polar_position,
            ellipsoid.as_ref(),
            arc_type,
            &mut polygon,
            &mut result,
        );

        if result.east - result.west > polygon.east_over_idl - polygon.west_over_idl {
            result.west = polygon.west_over_idl;
            result.east = polygon.east_over_idl;

            if result.east > std::f64::consts::PI {
                result.east -= CesiumMath::TWO_PI;
            }
            if result.west > std::f64::consts::PI {
                result.west -= CesiumMath::TWO_PI;
            }
        }

        // If either pole is inside the polygon, adjust the rectangle so the
        // pole is included.
        if CesiumMath::equals_epsilon(
            polygon.north_angle.abs(),
            CesiumMath::TWO_PI,
            Some(CesiumMath::EPSILON10),
            Some(CesiumMath::EPSILON10),
        ) {
            result.north = CesiumMath::PI_OVER_TWO;
            result.east = std::f64::consts::PI;
            result.west = -std::f64::consts::PI;
        }

        if CesiumMath::equals_epsilon(
            polygon.south_angle.abs(),
            CesiumMath::TWO_PI,
            Some(CesiumMath::EPSILON10),
            Some(CesiumMath::EPSILON10),
        ) {
            result.south = -CesiumMath::PI_OVER_TWO;
            result.east = std::f64::consts::PI;
            result.west = -std::f64::consts::PI;
        }

        result
    }

    /// For remapping texture coordinates when rendering PolygonGeometries
    /// as GroundPrimitives (JS `textureCoordinateRotationPoints` getter).
    ///
    /// DEVIATION: JS caches the result on first access; Rust recomputes on
    /// each call.
    pub fn texture_coordinate_rotation_points(&self) -> [f64; 6] {
        let st_rotation = -self.st_rotation;
        if st_rotation == 0.0 {
            return [0.0, 0.0, 0.0, 1.0, 1.0, 0.0];
        }
        let bounding_rectangle = self.rectangle();
        Geometry::texture_coordinate_rotation_points(
            &self.polygon_hierarchy.positions,
            st_rotation,
            &self.ellipsoid,
            &bounding_rectangle,
        )
    }
}

/// Scratch state accumulated while expanding a polygon's bounding rectangle
/// (JS module-level `polygon` object).
struct PolygonAngleScratch {
    north_angle: f64,
    south_angle: f64,
    west_over_idl: f64,
    east_over_idl: f64,
}

/// Port of the module-level `expandRectangle` helper.
fn expand_rectangle(
    polar: &Stereographic,
    last_polar: &Stereographic,
    ellipsoid: Option<&Ellipsoid>,
    arc_type: ArcType,
    polygon: &mut PolygonAngleScratch,
    result: &mut Rectangle,
) {
    let longitude = polar.longitude();
    let lon_adjusted = if longitude >= 0.0 {
        longitude
    } else {
        longitude + CesiumMath::TWO_PI
    };
    polygon.west_over_idl = polygon.west_over_idl.min(lon_adjusted);
    polygon.east_over_idl = polygon.east_over_idl.max(lon_adjusted);

    result.west = result.west.min(longitude);
    result.east = result.east.max(longitude);

    let latitude = polar.get_latitude(ellipsoid);
    let mut segment_latitude = latitude;

    result.south = result.south.min(latitude);
    result.north = result.north.max(latitude);

    if arc_type != ArcType::Rhumb {
        // Geodesics need to find the closest point on line. Rhumb lines do
        // not have a latitude greater in magnitude than either of their
        // endpoints.
        let segment = Cartesian2::subtract_new(&last_polar.position, &polar.position);
        let t = Cartesian2::dot(&last_polar.position, &segment)
            / Cartesian2::dot(&segment, &segment);
        if t > 0.0 && t < 1.0 {
            let projected = Cartesian2::add_new(
                &last_polar.position,
                &Cartesian2::multiply_by_scalar_new(&segment, -t),
            );
            let mut closest_polar = last_polar.clone();
            closest_polar.position = projected;
            let adjusted_latitude = closest_polar.get_latitude(ellipsoid);
            result.south = result.south.min(adjusted_latitude);
            result.north = result.north.max(adjusted_latitude);

            if latitude.abs() > adjusted_latitude.abs() {
                segment_latitude = adjusted_latitude;
            }
        }
    }
    let direction = last_polar.x() * polar.y() - polar.x() * last_polar.y();

    // The total internal angle in either hemisphere determines if the pole
    // is inside or outside the polygon.
    let mut angle = if direction > 0.0 {
        1.0
    } else if direction < 0.0 {
        -1.0
    } else {
        0.0
    };
    if angle != 0.0 {
        angle *= Cartesian2::angle_between(&last_polar.position, &polar.position);
    }

    if segment_latitude >= 0.0 {
        polygon.north_angle += angle;
    }

    if segment_latitude <= 0.0 {
        polygon.south_angle += angle;
    }
}

fn write_index(storage: &mut IndexStorage, index: usize, value: u32) {
    match storage {
        IndexStorage::U16(v) => v[index] = value as u16,
        IndexStorage::U32(v) => v[index] = value,
    }
}

fn read_index(storage: &IndexStorage, index: usize) -> u32 {
    match storage {
        IndexStorage::U16(v) => v[index] as u32,
        IndexStorage::U32(v) => v[index],
    }
}

/// Port of the module-level `adjustPosHeightsForNormal` helper.
fn adjust_pos_heights_for_normal(
    position: &Cartesian3,
    p1: &mut Cartesian3,
    p2: &mut Cartesian3,
    ellipsoid: &Ellipsoid,
) {
    let mut carto1 = Cartographic::default();
    ellipsoid.cartesian_to_cartographic(position, &mut carto1);
    let height = carto1.height;

    let mut p1_carto = Cartographic::default();
    ellipsoid.cartesian_to_cartographic(p1, &mut p1_carto);
    p1_carto.height = height;
    ellipsoid.cartographic_to_cartesian(&p1_carto, p1);

    let mut p2_carto = Cartographic::default();
    ellipsoid.cartesian_to_cartographic(p2, &mut p2_carto);
    p2_carto.height = height - 100.0;
    ellipsoid.cartographic_to_cartesian(&p2_carto, p2);
}

/// Port of the module-level `getTangentPlane` helper.
fn get_tangent_plane(
    rectangle: &Rectangle,
    positions: &[Cartesian3],
    ellipsoid: &Ellipsoid,
) -> Option<EllipsoidTangentPlane> {
    if rectangle.height() >= PI || rectangle.width() >= PI {
        let polar = Stereographic::from_cartesian(&positions[0], None);
        return Some(polar.tangent_plane());
    }

    // Use a local tangent plane for smaller extents
    EllipsoidTangentPlane::from_points(positions, Some(ellipsoid.clone()))
}

/// Port of the module-level `createProjectTo2d` helper; returns the
/// multi-point projection closure used by `polygonsFromHierarchy`.
fn create_project_to_2d(
    rectangle: Rectangle,
    outer_positions: Vec<Cartesian3>,
    ellipsoid: Ellipsoid,
) -> Box<dyn Fn(&[Cartesian3]) -> Option<Vec<Cartesian2>>> {
    Box::new(move |positions: &[Cartesian3]| -> Option<Vec<Cartesian2>> {
        // If the polygon positions span a large enough extent, use a
        // specialized projection
        if rectangle.height() >= PI || rectangle.width() >= PI {
            // polygons that cross the equator must use cylindrical
            // coordinates to correctly compute winding order.
            if rectangle.south < 0.0 && rectangle.north > 0.0 {
                let mut results = Vec::with_capacity(positions.len());
                let mut cartographic = Cartographic::default();
                for position in positions {
                    ellipsoid.cartesian_to_cartographic(position, &mut cartographic);
                    results.push(Cartesian2::new(
                        cartographic.longitude / PI,
                        cartographic.latitude / CesiumMath::PI_OVER_TWO,
                    ));
                }
                return Some(results);
            }

            return Some(
                Stereographic::from_cartesian_array(positions, None)
                    .iter()
                    .map(|s| s.position)
                    .collect(),
            );
        }

        // Use a local tangent plane for smaller extents
        let tangent_plane =
            EllipsoidTangentPlane::from_points(&outer_positions, Some(ellipsoid.clone()))?;
        Some(tangent_plane.project_points_onto_plane(positions))
    })
}

/// Port of the module-level `createProjectPositionTo2d` helper; returns the
/// single-point projection closure used by `computeAttributes`.
///
/// DEVIATION: JS returns closures with inconsistent array/point semantics
/// (`projectPointsOntoPlane` for the tangent plane path, `fromCartesian`
/// for the stereographic path) which `computeAttributes` calls with a
/// one-element array; this port normalizes both paths to single-point
/// semantics producing the same 2D coordinates.
fn create_project_position_to_2d(
    rectangle: Rectangle,
    outer_ring: Vec<Cartesian3>,
    ellipsoid: Ellipsoid,
) -> Box<dyn Fn(&Cartesian3) -> Cartesian2> {
    // If the polygon positions span a large enough extent, use a
    // specialized projection
    if rectangle.height() >= PI || rectangle.width() >= PI {
        return Box::new(move |position: &Cartesian3| -> Cartesian2 {
            // polygons that cross the equator must use cylindrical
            // coordinates to correctly compute winding order.
            if rectangle.south < 0.0 && rectangle.north > 0.0 {
                let mut cartographic = Cartographic::default();
                ellipsoid.cartesian_to_cartographic(position, &mut cartographic);
                return Cartesian2::new(
                    cartographic.longitude / PI,
                    cartographic.latitude / CesiumMath::PI_OVER_TWO,
                );
            }

            Stereographic::from_cartesian(position, None).position
        });
    }

    let tangent_plane =
        EllipsoidTangentPlane::from_points(&outer_ring, Some(ellipsoid.clone()));
    Box::new(move |position: &Cartesian3| -> Cartesian2 {
        // Use a local tangent plane for smaller extents
        match &tangent_plane {
            Some(plane) => plane.project_point_onto_plane(position).unwrap_or_default(),
            None => Cartesian2::default(),
        }
    })
}

/// Port of the module-level `createSplitPolygons` helper.
fn create_split_polygons(
    rectangle: Rectangle,
    ellipsoid: Ellipsoid,
    arc_type: ArcType,
    per_position_height: bool,
) -> Box<dyn Fn(Vec<Vec<Cartesian3>>) -> Vec<Vec<Cartesian3>>> {
    Box::new(move |polygons: Vec<Vec<Cartesian3>>| -> Vec<Vec<Cartesian3>> {
        if !per_position_height
            && (rectangle.height() >= CesiumMath::PI_OVER_TWO
                || rectangle.width() >= 2.0 * CesiumMath::PI_OVER_THREE)
        {
            return PolygonGeometryLibrary::split_polygons_on_equator(
                &polygons,
                &ellipsoid,
                arc_type,
            );
        }

        polygons
    })
}

/// Port of the module-level `computeBoundingRectangle` helper.
fn compute_bounding_rectangle(
    outer_ring: &[Cartesian3],
    rectangle: &Rectangle,
    ellipsoid: &Ellipsoid,
    st_rotation: f64,
) -> BoundingRectangle {
    if rectangle.height() >= PI || rectangle.width() >= PI {
        return BoundingRectangle::from_rectangle(Some(rectangle), None);
    }

    let tangent_plane =
        EllipsoidTangentPlane::from_points(outer_ring, Some(ellipsoid.clone()));
    match tangent_plane {
        Some(plane) => {
            let normal = plane.plane().normal;
            PolygonGeometryLibrary::compute_bounding_rectangle(
                &normal,
                &|position: &Cartesian3, result: &mut Cartesian2| {
                    if let Some(projected) = plane.project_point_onto_plane(position) {
                        *result = projected;
                    }
                },
                outer_ring,
                st_rotation,
            )
        }
        None => BoundingRectangle::default(),
    }
}

/// Converts a 2D texture-coordinate hierarchy into a 3D hierarchy with
/// zero heights so it can flow through `polygons_from_hierarchy` (JS
/// `dummyFunction` identity projection path).
fn polygon_hierarchy_2d_to_3d(hierarchy: &PolygonHierarchy2D) -> PolygonHierarchy {
    PolygonHierarchy::new(
        hierarchy
            .positions
            .iter()
            .map(|p| Cartesian3::new(p.x, p.y, 0.0))
            .collect(),
        hierarchy
            .holes
            .iter()
            .map(polygon_hierarchy_2d_to_3d)
            .collect(),
    )
}

/// Computes the geometric representation of a polygon, including vertices,
/// indices, and a bounding sphere.
///
/// Port of `PolygonGeometry.createGeometry`.
///
/// DEVIATION: JS converts the combined position values to a fresh
/// `Float64Array`; Rust position values are already `f64` (no-op).
pub fn create_geometry(polygon_geometry: &PolygonGeometry) -> Option<Geometry> {
    let vertex_format = &polygon_geometry.vertex_format;
    let ellipsoid = &polygon_geometry.ellipsoid;
    let granularity = polygon_geometry.granularity;
    let st_rotation = polygon_geometry.st_rotation;
    let polygon_hierarchy = &polygon_geometry.polygon_hierarchy;
    let per_position_height = polygon_geometry.per_position_height;
    let close_top = polygon_geometry.close_top;
    let close_bottom = polygon_geometry.close_bottom;
    let arc_type = polygon_geometry.arc_type;
    let texture_coordinates = &polygon_geometry.texture_coordinates;

    let has_texture_coordinates = texture_coordinates.is_some();

    let outer_positions = &polygon_hierarchy.positions;
    if outer_positions.len() < 3 {
        return None;
    }

    let rectangle = polygon_geometry.rectangle();
    let project_to_2d = create_project_to_2d(
        rectangle,
        outer_positions.clone(),
        ellipsoid.clone(),
    );
    let split_polygons = create_split_polygons(
        rectangle,
        ellipsoid.clone(),
        arc_type,
        per_position_height,
    );
    let split_polygons_ref: &dyn Fn(Vec<Vec<Cartesian3>>) -> Vec<Vec<Cartesian3>> =
        &*split_polygons;
    let results = PolygonGeometryLibrary::polygons_from_hierarchy(
        polygon_hierarchy,
        has_texture_coordinates,
        &*project_to_2d,
        !per_position_height,
        ellipsoid,
        Some(split_polygons_ref),
    );

    let hierarchy = &results.hierarchy;
    let polygons = &results.polygons;

    let dummy_function: &dyn Fn(&[Cartesian3]) -> Option<Vec<Cartesian2>> =
        &|positions: &[Cartesian3]| {
            Some(
                positions
                    .iter()
                    .map(|p| Cartesian2::new(p.x, p.y))
                    .collect(),
            )
        };

    let texture_coordinate_polygons: Option<Vec<PolygonTextureCoordinates>> =
        if has_texture_coordinates {
            let tc_hierarchy = polygon_hierarchy_2d_to_3d(texture_coordinates.as_ref().unwrap());
            let tc_results = PolygonGeometryLibrary::polygons_from_hierarchy(
                &tc_hierarchy,
                true,
                dummy_function,
                false,
                ellipsoid,
                None,
            );
            Some(
                tc_results
                    .polygons
                    .iter()
                    .map(|p| PolygonTextureCoordinates {
                        positions: p.positions_2d.clone(),
                    })
                    .collect(),
            )
        } else {
            None
        };

    if hierarchy.is_empty() {
        return None;
    }

    let outer_ring = &hierarchy[0].outer_ring;
    let bounding_rectangle =
        compute_bounding_rectangle(outer_ring, &rectangle, ellipsoid, st_rotation);

    let mut geometries: Vec<GeometryInstance> = Vec::new();

    let height = polygon_geometry.height;
    let extruded_height = polygon_geometry.extruded_height;
    let extrude = polygon_geometry.per_position_height_extrude
        || !CesiumMath::equals_epsilon(
            height,
            extruded_height,
            Some(0.0),
            Some(CesiumMath::EPSILON2),
        );

    let rotation_axis_plane = get_tangent_plane(&rectangle, outer_ring, ellipsoid);
    // DEVIATION: JS would throw when the tangent plane cannot be built
    // (origin at the ellipsoid center); this port falls back to UNIT_Z.
    let fallback_axis = Cartesian3::UNIT_Z;
    let rotation_axis = rotation_axis_plane
        .as_ref()
        .map(|p| p.plane().normal)
        .unwrap_or(fallback_axis);
    let project_position_to_2d = create_project_position_to_2d(
        rectangle,
        outer_ring.clone(),
        ellipsoid.clone(),
    );

    let mut options = ComputeAttributesOptions {
        per_position_height,
        vertex_format,
        shadow_volume: false,
        rotation_axis: &rotation_axis,
        project_to_2d: &*project_position_to_2d,
        bounding_rectangle: &bounding_rectangle,
        ellipsoid,
        st_rotation,
        bottom: false,
        top: true,
        wall: false,
        extrude: false,
        offset_attribute: polygon_geometry.offset_attribute,
    };

    if extrude {
        options.extrude = true;
        options.top = close_top;
        options.bottom = close_bottom;
        options.shadow_volume = polygon_geometry.shadow_volume;
        for i in 0..polygons.len() {
            let mut split_geometry = create_geometry_from_positions_extruded(
                ellipsoid,
                &polygons[i],
                texture_coordinate_polygons.as_ref().map(|t| &t[i]),
                granularity,
                &hierarchy[i],
                per_position_height,
                close_top,
                close_bottom,
                vertex_format,
                arc_type,
            );

            if let Some(top_and_bottom) = split_geometry.top_and_bottom.as_mut() {
                let geo = top_and_bottom.geometry.as_geometry_mut().unwrap();
                if close_top && close_bottom {
                    PolygonGeometryLibrary::scale_to_geodetic_height_extruded(
                        Some(geo),
                        height,
                        extruded_height,
                        Some(ellipsoid.clone()),
                        per_position_height,
                    );
                } else if close_top {
                    let mut vals = geo
                        .attributes
                        .get("position")
                        .map(|a| a.values.clone())
                        .unwrap_or_default();
                    PolygonPipeline::scale_to_geodetic_height(
                        Some(&mut vals),
                        Some(height),
                        Some(ellipsoid),
                        Some(!per_position_height),
                    );
                    if let Some(attr) = geo.attributes.get_mut("position") {
                        attr.values = vals;
                    }
                } else if close_bottom {
                    let mut vals = geo
                        .attributes
                        .get("position")
                        .map(|a| a.values.clone())
                        .unwrap_or_default();
                    PolygonPipeline::scale_to_geodetic_height(
                        Some(&mut vals),
                        Some(extruded_height),
                        Some(ellipsoid),
                        Some(true),
                    );
                    if let Some(attr) = geo.attributes.get_mut("position") {
                        attr.values = vals;
                    }
                }
                if close_top || close_bottom {
                    options.wall = false;
                    compute_attributes(&options, geo);
                    geometries.push(top_and_bottom.clone());
                }
            }

            options.wall = true;
            for wall in split_geometry.walls.iter_mut() {
                let geo = wall.geometry.as_geometry_mut().unwrap();
                PolygonGeometryLibrary::scale_to_geodetic_height_extruded(
                    Some(geo),
                    height,
                    extruded_height,
                    Some(ellipsoid.clone()),
                    per_position_height,
                );
                compute_attributes(&options, geo);
                geometries.push(wall.clone());
            }
        }
    } else {
        for i in 0..polygons.len() {
            let mut geometry = PolygonGeometryLibrary::create_geometry_from_positions(
                ellipsoid,
                &polygons[i],
                texture_coordinate_polygons.as_ref().map(|t| &t[i]),
                granularity,
                per_position_height,
                vertex_format,
                arc_type,
            );
            let mut vals = geometry
                .attributes
                .get("position")
                .map(|a| a.values.clone())
                .unwrap_or_default();
            PolygonPipeline::scale_to_geodetic_height(
                Some(&mut vals),
                Some(height),
                Some(ellipsoid),
                Some(!per_position_height),
            );
            if let Some(attr) = geometry.attributes.get_mut("position") {
                attr.values = vals;
            }

            compute_attributes(&options, &mut geometry);

            if polygon_geometry.offset_attribute.is_some() {
                let length = geometry
                    .attributes
                    .get("position")
                    .map(|a| a.values.len())
                    .unwrap_or(0);
                let offset_value =
                    if polygon_geometry.offset_attribute == Some(GeometryOffsetAttribute::None) {
                        0
                    } else {
                        1
                    };
                let apply_offset = vec![offset_value as f64; length / 3];
                geometry.attributes.insert(
                    "applyOffset".to_string(),
                    GeometryAttribute::new(
                        ComponentDatatype::UnsignedByte,
                        1,
                        false,
                        apply_offset,
                    ),
                );
            }

            geometries.push(GeometryInstance::new(
                GeometryInstanceGeometry::Geometry(Box::new(geometry)),
                None,
                None,
                None,
            ));
        }
    }

    if geometries.is_empty() {
        return None;
    }

    let mut combined = GeometryPipeline::combine_instances(&geometries);
    let mut geometry = combined.remove(0);

    // JS: geometry.indices = IndexDatatype.createTypedArray(vertexCount, indices)
    let position_count = geometry
        .attributes
        .get("position")
        .map(|a| a.values.len() / 3)
        .unwrap_or(0);
    if let Some(indices) = &geometry.indices {
        let mut values = Vec::with_capacity(indices.len());
        for i in 0..indices.len() {
            values.push(read_index(indices, i));
        }
        let mut storage = IndexDatatype::create_typed_array(position_count, values.len());
        for (i, &v) in values.iter().enumerate() {
            write_index(&mut storage, i, v);
        }
        geometry.indices = Some(storage);
    }

    let bounding_sphere = {
        let pos_values = geometry
            .attributes
            .get("position")
            .map(|a| a.values.clone())
            .unwrap_or_default();
        BoundingSphere::from_vertices(&pos_values, None, Some(3), None)
    };

    if !vertex_format.position {
        geometry.attributes.remove("position");
    }

    Some(Geometry::with_all(
        geometry.attributes,
        geometry.indices,
        Some(geometry.primitive_type),
        Some(bounding_sphere),
        GeometryType::None,
        None,
        polygon_geometry
            .offset_attribute
            .map(|_| "applyOffset".to_string()),
    ))
}

/// The options object consumed by [`compute_attributes`] (JS anonymous
/// `options` object minus the `geometry` field, which is passed separately).
struct ComputeAttributesOptions<'a> {
    vertex_format: &'a VertexFormat,
    shadow_volume: bool,
    wall: bool,
    top: bool,
    bottom: bool,
    bounding_rectangle: &'a BoundingRectangle,
    rotation_axis: &'a Cartesian3,
    project_to_2d: &'a dyn Fn(&Cartesian3) -> Cartesian2,
    ellipsoid: &'a Ellipsoid,
    st_rotation: f64,
    per_position_height: bool,
    extrude: bool,
    offset_attribute: Option<GeometryOffsetAttribute>,
}

/// Port of the module-level `computeAttributes` function.
fn compute_attributes(options: &ComputeAttributesOptions, geometry: &mut Geometry) {
    let vertex_format = options.vertex_format;
    let flat_positions = geometry
        .attributes
        .get("position")
        .map(|a| a.values.clone())
        .unwrap_or_default();
    let flat_texcoords = geometry.attributes.get("st").map(|a| a.values.clone());

    let mut length = flat_positions.len();
    let wall = options.wall;
    let top = options.top || wall;
    let bottom = options.bottom || wall;
    if vertex_format.st
        || vertex_format.normal
        || vertex_format.tangent
        || vertex_format.bitangent
        || options.shadow_volume
    {
        // PERFORMANCE_IDEA: Compute before subdivision, then just interpolate
        // during subdivision.
        let bounding_rectangle = options.bounding_rectangle;
        let ellipsoid = options.ellipsoid;

        let origin = Cartesian2::new(bounding_rectangle.x, bounding_rectangle.y);

        let mut texture_coordinates = if vertex_format.st {
            vec![0.0; 2 * (length / 3)]
        } else {
            Vec::new()
        };
        let mut normals: Vec<f64> = if vertex_format.normal {
            if options.per_position_height && top && !wall {
                geometry
                    .attributes
                    .get("normal")
                    .map(|a| a.values.clone())
                    .unwrap_or_default()
            } else {
                vec![0.0; length]
            }
        } else {
            Vec::new()
        };
        let mut tangents = if vertex_format.tangent {
            vec![0.0; length]
        } else {
            Vec::new()
        };
        let mut bitangents = if vertex_format.bitangent {
            vec![0.0; length]
        } else {
            Vec::new()
        };
        let mut extrude_normals = if options.shadow_volume {
            vec![0.0; length]
        } else {
            Vec::new()
        };

        let mut texture_coord_index = 0usize;
        let mut attr_index = 0usize;

        let mut recompute_normal = true;

        let mut texture_matrix = Matrix3::IDENTITY;
        let mut tangent_rotation_matrix = Matrix3::IDENTITY;
        if options.st_rotation != 0.0 {
            let rotation = Quaternion::from_axis_angle_new(
                options.rotation_axis,
                options.st_rotation,
            );
            Matrix3::from_quaternion(&rotation, &mut texture_matrix);

            let rotation = Quaternion::from_axis_angle_new(
                options.rotation_axis,
                -options.st_rotation,
            );
            Matrix3::from_quaternion(&rotation, &mut tangent_rotation_matrix);
        }

        let mut bottom_offset = 0usize;
        let mut bottom_offset2 = 0usize;

        if top && bottom {
            bottom_offset = length / 2;
            bottom_offset2 = length / 3;

            length /= 2;
        }

        let mut i = 0usize;
        while i < length {
            let position = Cartesian3::new(
                flat_positions[i],
                flat_positions[i + 1],
                flat_positions[i + 2],
            );

            if vertex_format.st && flat_texcoords.is_none() {
                let mut p = Cartesian3::default();
                Matrix3::multiply_by_vector(&texture_matrix, &position, &mut p);
                let mut scaled = Cartesian3::default();
                if ellipsoid.scale_to_geodetic_surface(&p, &mut scaled) {
                    p = scaled;
                }
                // DEVIATION: JS calls the projection closure with a
                // one-element array; this port uses the single-point closure
                // directly (see `create_project_position_to_2d`).
                let st = (options.project_to_2d)(&p);
                let st = Cartesian2::subtract_new(&st, &origin);

                let stx = CesiumMath::clamp(st.x / bounding_rectangle.width, 0.0, 1.0);
                let sty = CesiumMath::clamp(st.y / bounding_rectangle.height, 0.0, 1.0);
                if bottom {
                    texture_coordinates[texture_coord_index + bottom_offset2] = stx;
                    texture_coordinates[texture_coord_index + 1 + bottom_offset2] = sty;
                }
                if top {
                    texture_coordinates[texture_coord_index] = stx;
                    texture_coordinates[texture_coord_index + 1] = sty;
                }

                texture_coord_index += 2;
            }

            if vertex_format.normal
                || vertex_format.tangent
                || vertex_format.bitangent
                || options.shadow_volume
            {
                let attr_index1 = attr_index + 1;
                let attr_index2 = attr_index + 2;

                let mut normal = Cartesian3::default();
                let mut tangent = Cartesian3::default();
                let mut bitangent = Cartesian3::default();
                let mut per_pos_tangent = Cartesian3::default();
                let mut per_pos_bitangent = Cartesian3::default();

                if wall {
                    if i + 3 < length {
                        let mut p1 = Cartesian3::new(
                            flat_positions[i + 3],
                            flat_positions[i + 4],
                            flat_positions[i + 5],
                        );

                        if recompute_normal {
                            let mut p2 = Cartesian3::new(
                                flat_positions[i + length],
                                flat_positions[i + length + 1],
                                flat_positions[i + length + 2],
                            );
                            if options.per_position_height {
                                adjust_pos_heights_for_normal(
                                    &position, &mut p1, &mut p2, ellipsoid,
                                );
                            }
                            let p1_delta = Cartesian3::subtract_new(&p1, &position);
                            let p2_delta = Cartesian3::subtract_new(&p2, &position);
                            normal = Cartesian3::normalize_new(&Cartesian3::cross_new(
                                &p2_delta, &p1_delta,
                            ));
                            // JS reuses the scratch: after the subtract `p1`
                            // holds the delta vector for the corner check.
                            p1 = p1_delta;
                            recompute_normal = false;
                        }

                        if Cartesian3::equals_epsilon(
                            Some(&p1),
                            Some(&position),
                            Some(CesiumMath::EPSILON10),
                            None,
                        ) {
                            // if we've reached a corner
                            recompute_normal = true;
                        }
                    }

                    if vertex_format.tangent || vertex_format.bitangent {
                        ellipsoid.geodetic_surface_normal(&position, &mut bitangent);
                        if vertex_format.tangent {
                            tangent = Cartesian3::normalize_new(&Cartesian3::cross_new(
                                &bitangent, &normal,
                            ));
                        }
                    }
                } else {
                    ellipsoid.geodetic_surface_normal(&position, &mut normal);
                    if vertex_format.tangent || vertex_format.bitangent {
                        if options.per_position_height {
                            let per_pos_normal = Cartesian3::new(
                                normals[attr_index],
                                normals[attr_index + 1],
                                normals[attr_index + 2],
                            );
                            let per_pos_t = Cartesian3::cross_new(
                                &Cartesian3::UNIT_Z,
                                &per_pos_normal,
                            );
                            let mut rotated = Cartesian3::default();
                            Matrix3::multiply_by_vector(
                                &tangent_rotation_matrix,
                                &per_pos_t,
                                &mut rotated,
                            );
                            per_pos_tangent = Cartesian3::normalize_new(&rotated);
                            if vertex_format.bitangent {
                                per_pos_bitangent = Cartesian3::normalize_new(
                                    &Cartesian3::cross_new(&per_pos_normal, &per_pos_tangent),
                                );
                            }
                        }

                        tangent = Cartesian3::cross_new(&Cartesian3::UNIT_Z, &normal);
                        let mut rotated = Cartesian3::default();
                        Matrix3::multiply_by_vector(
                            &tangent_rotation_matrix,
                            &tangent,
                            &mut rotated,
                        );
                        tangent = Cartesian3::normalize_new(&rotated);
                        if vertex_format.bitangent {
                            bitangent = Cartesian3::normalize_new(&Cartesian3::cross_new(
                                &normal, &tangent,
                            ));
                        }
                    }
                }

                if vertex_format.normal {
                    if options.wall {
                        normals[attr_index + bottom_offset] = normal.x;
                        normals[attr_index1 + bottom_offset] = normal.y;
                        normals[attr_index2 + bottom_offset] = normal.z;
                    } else if bottom {
                        normals[attr_index + bottom_offset] = -normal.x;
                        normals[attr_index1 + bottom_offset] = -normal.y;
                        normals[attr_index2 + bottom_offset] = -normal.z;
                    }

                    if (top && !options.per_position_height) || wall {
                        normals[attr_index] = normal.x;
                        normals[attr_index1] = normal.y;
                        normals[attr_index2] = normal.z;
                    }
                }

                if options.shadow_volume {
                    if wall {
                        ellipsoid.geodetic_surface_normal(&position, &mut normal);
                    }
                    extrude_normals[attr_index + bottom_offset] = -normal.x;
                    extrude_normals[attr_index1 + bottom_offset] = -normal.y;
                    extrude_normals[attr_index2 + bottom_offset] = -normal.z;
                }

                if vertex_format.tangent {
                    if options.wall {
                        tangents[attr_index + bottom_offset] = tangent.x;
                        tangents[attr_index1 + bottom_offset] = tangent.y;
                        tangents[attr_index2 + bottom_offset] = tangent.z;
                    } else if bottom {
                        tangents[attr_index + bottom_offset] = -tangent.x;
                        tangents[attr_index1 + bottom_offset] = -tangent.y;
                        tangents[attr_index2 + bottom_offset] = -tangent.z;
                    }

                    if top {
                        if options.per_position_height {
                            tangents[attr_index] = per_pos_tangent.x;
                            tangents[attr_index1] = per_pos_tangent.y;
                            tangents[attr_index2] = per_pos_tangent.z;
                        } else {
                            tangents[attr_index] = tangent.x;
                            tangents[attr_index1] = tangent.y;
                            tangents[attr_index2] = tangent.z;
                        }
                    }
                }

                if vertex_format.bitangent {
                    if bottom {
                        bitangents[attr_index + bottom_offset] = bitangent.x;
                        bitangents[attr_index1 + bottom_offset] = bitangent.y;
                        bitangents[attr_index2 + bottom_offset] = bitangent.z;
                    }
                    if top {
                        if options.per_position_height {
                            bitangents[attr_index] = per_pos_bitangent.x;
                            bitangents[attr_index1] = per_pos_bitangent.y;
                            bitangents[attr_index2] = per_pos_bitangent.z;
                        } else {
                            bitangents[attr_index] = bitangent.x;
                            bitangents[attr_index1] = bitangent.y;
                            bitangents[attr_index2] = bitangent.z;
                        }
                    }
                }
                attr_index += 3;
            }
            i += 3;
        }

        if vertex_format.st && flat_texcoords.is_none() {
            geometry.attributes.insert(
                "st".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 2, false, texture_coordinates),
            );
        }

        if vertex_format.normal {
            geometry.attributes.insert(
                "normal".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 3, false, normals),
            );
        }

        if vertex_format.tangent {
            geometry.attributes.insert(
                "tangent".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 3, false, tangents),
            );
        }

        if vertex_format.bitangent {
            geometry.attributes.insert(
                "bitangent".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 3, false, bitangents),
            );
        }

        if options.shadow_volume {
            geometry.attributes.insert(
                "extrudeDirection".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 3, false, extrude_normals),
            );
        }
    }

    if options.extrude && options.offset_attribute.is_some() {
        let size = flat_positions.len() / 3;
        let mut offset_attribute = vec![0.0; size];

        if options.offset_attribute == Some(GeometryOffsetAttribute::Top) {
            if (top && bottom) || wall {
                for v in offset_attribute.iter_mut().take(size / 2) {
                    *v = 1.0;
                }
            } else if top {
                for v in offset_attribute.iter_mut() {
                    *v = 1.0;
                }
            }
        } else {
            let offset_value =
                if options.offset_attribute == Some(GeometryOffsetAttribute::None) {
                    0.0
                } else {
                    1.0
                };
            offset_attribute.fill(offset_value);
        }

        geometry.attributes.insert(
            "applyOffset".to_string(),
            GeometryAttribute::new(
                ComponentDatatype::UnsignedByte,
                1,
                false,
                offset_attribute,
            ),
        );
    }
}

/// Result of [`create_geometry_from_positions_extruded`] (JS `geos` object).
struct ExtrudedGeometries {
    top_and_bottom: Option<GeometryInstance>,
    walls: Vec<GeometryInstance>,
}

/// Port of the module-level `createGeometryFromPositionsExtruded` function.
#[allow(clippy::too_many_arguments)]
fn create_geometry_from_positions_extruded(
    ellipsoid: &Ellipsoid,
    polygon: &PolygonResultEntry,
    texture_coordinates: Option<&PolygonTextureCoordinates>,
    granularity: f64,
    hierarchy: &HierarchyResultEntry,
    per_position_height: bool,
    close_top: bool,
    close_bottom: bool,
    vertex_format: &VertexFormat,
    arc_type: ArcType,
) -> ExtrudedGeometries {
    let mut geos = ExtrudedGeometries {
        top_and_bottom: None,
        walls: Vec::new(),
    };

    if close_top || close_bottom {
        let mut top_geo = PolygonGeometryLibrary::create_geometry_from_positions(
            ellipsoid,
            polygon,
            texture_coordinates,
            granularity,
            per_position_height,
            vertex_format,
            arc_type,
        );

        let edge_points = top_geo
            .attributes
            .get("position")
            .map(|a| a.values.clone())
            .unwrap_or_default();
        let indices = top_geo
            .indices
            .clone()
            .unwrap_or(IndexStorage::U32(Vec::new()));
        let index_count = indices.len();

        if close_top && close_bottom {
            let mut top_bottom_positions = edge_points.clone();
            top_bottom_positions.extend_from_slice(&edge_points);

            let num_positions = top_bottom_positions.len() / 3;

            let mut new_indices =
                IndexDatatype::create_typed_array(num_positions, index_count * 2);
            // newIndices.set(indices)
            for i in 0..index_count {
                write_index(&mut new_indices, i, read_index(&indices, i));
            }

            let length = num_positions / 2;

            let mut i = 0usize;
            while i < index_count {
                let i0 = read_index(&new_indices, i) + length as u32;
                let i1 = read_index(&new_indices, i + 1) + length as u32;
                let i2 = read_index(&new_indices, i + 2) + length as u32;

                write_index(&mut new_indices, i + index_count, i2);
                write_index(&mut new_indices, i + 1 + index_count, i1);
                write_index(&mut new_indices, i + 2 + index_count, i0);
                i += 3;
            }

            if let Some(attr) = top_geo.attributes.get_mut("position") {
                attr.values = top_bottom_positions.clone();
            }
            if per_position_height && vertex_format.normal {
                let normals = top_geo
                    .attributes
                    .get("normal")
                    .map(|a| a.values.clone())
                    .unwrap_or_default();
                let mut new_normals = vec![0.0; top_bottom_positions.len()];
                new_normals[..normals.len()].copy_from_slice(&normals);
                if let Some(attr) = top_geo.attributes.get_mut("normal") {
                    attr.values = new_normals;
                }
            }

            if vertex_format.st && texture_coordinates.is_some() {
                let texcoords = top_geo
                    .attributes
                    .get("st")
                    .map(|a| a.values.clone())
                    .unwrap_or_default();
                let mut new_texcoords = texcoords.clone();
                new_texcoords.extend_from_slice(&texcoords);
                new_texcoords.truncate(num_positions * 2);
                if let Some(attr) = top_geo.attributes.get_mut("st") {
                    attr.values = new_texcoords;
                }
            }

            top_geo.indices = Some(new_indices);
        } else if close_bottom {
            let num_positions = edge_points.len() / 3;
            let mut new_indices =
                IndexDatatype::create_typed_array(num_positions, index_count);

            let mut i = 0usize;
            while i < index_count {
                write_index(&mut new_indices, i, read_index(&indices, i + 2));
                write_index(&mut new_indices, i + 1, read_index(&indices, i + 1));
                write_index(&mut new_indices, i + 2, read_index(&indices, i));
                i += 3;
            }

            top_geo.indices = Some(new_indices);
        }

        geos.top_and_bottom = Some(GeometryInstance::new(
            GeometryInstanceGeometry::Geometry(Box::new(top_geo)),
            None,
            None,
            None,
        ));
    }

    let mut outer_ring = hierarchy.outer_ring.clone();
    let tangent_plane =
        EllipsoidTangentPlane::from_points(&outer_ring, Some(ellipsoid.clone()));
    if let Some(plane) = &tangent_plane {
        let positions_2d = plane.project_points_onto_plane(&outer_ring);
        let winding_order = PolygonPipeline::compute_winding_order_2d(&positions_2d);
        if winding_order == WindingOrder::Clockwise {
            outer_ring.reverse();
        }
    }

    let wall_geo = PolygonGeometryLibrary::compute_wall_geometry(
        &outer_ring,
        texture_coordinates,
        ellipsoid,
        granularity,
        per_position_height,
        arc_type,
    );
    geos.walls.push(GeometryInstance::new(
        GeometryInstanceGeometry::Geometry(Box::new(wall_geo)),
        None,
        None,
        None,
    ));

    for hole in &hierarchy.holes {
        let mut hole_ring = hole.clone();
        if let Some(plane) = &tangent_plane {
            let positions_2d = plane.project_points_onto_plane(&hole_ring);
            let winding_order = PolygonPipeline::compute_winding_order_2d(&positions_2d);
            if winding_order == WindingOrder::CounterClockwise {
                hole_ring.reverse();
            }
        }

        let wall_geo = PolygonGeometryLibrary::compute_wall_geometry(
            &hole_ring,
            texture_coordinates,
            ellipsoid,
            granularity,
            per_position_height,
            arc_type,
        );
        geos.walls.push(GeometryInstance::new(
            GeometryInstanceGeometry::Geometry(Box::new(wall_geo)),
            None,
            None,
            None,
        ));
    }

    geos
}
