//! Ported from `packages/engine/Source/Core/PolygonGeometry.js`.
//!
//! A description of a polygon on an ellipsoid.
//!
//! DEVIATION: JS `createGeometry` uses `EllipsoidTangentPlane` for 2D
//! projection and `GeometryPipeline.combineInstances` for merging. The
//! Rust port uses a cartographic (lon/lat) projection and manual merging.
//!
//! DEVIATION: JS `createProjectTo2d` has special handling for polygons
//! spanning large extents or crossing the equator. The Rust port uses a
//! simple cartographic projection for all cases.

use std::collections::HashMap;

use crate::arc_type::ArcType;
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
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_offset_attribute::GeometryOffsetAttribute;
use crate::geometry_type::GeometryType;
use crate::index_datatype::{IndexDatatype, IndexStorage};
use crate::math::CesiumMath;
use crate::polygon_geometry_library::{
    PolygonGeometryLibrary, PolygonResultEntry,
};
use crate::polygon_hierarchy::PolygonHierarchy;
use crate::polygon_pipeline::PolygonPipeline;
use crate::primitive_type::PrimitiveType;
use crate::rectangle::Rectangle;
use crate::stereographic::Stereographic;
use crate::vertex_format::VertexFormat;

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

/// Computes the geometric representation of a polygon, including vertices,
/// indices, and a bounding sphere.
///
/// Port of `PolygonGeometry.createGeometry`.
pub fn create_geometry(polygon_geometry: &PolygonGeometry) -> Option<Geometry> {
    let ellipsoid = &polygon_geometry.ellipsoid;
    let polygon_hierarchy = &polygon_geometry.polygon_hierarchy;
    let per_position_height = polygon_geometry.per_position_height;
    let vertex_format = &polygon_geometry.vertex_format;

    let outer_positions = &polygon_hierarchy.positions;
    if outer_positions.len() < 3 {
        return None;
    }

    // Project positions to 2D using cartographic (lon/lat) projection
    let project_fn = |positions: &[Cartesian3]| -> Option<Vec<Cartesian2>> {
        let mut result = Vec::with_capacity(positions.len());
        let mut carto = Cartographic::default();
        for p in positions {
            ellipsoid.cartesian_to_cartographic(p, &mut carto);
            result.push(Cartesian2::new(carto.longitude, carto.latitude));
        }
        Some(result)
    };

    let results = PolygonGeometryLibrary::polygons_from_hierarchy(
        polygon_hierarchy,
        false,
        &project_fn,
        !per_position_height,
        ellipsoid,
        None,
    );

    if results.hierarchy.is_empty() {
        return None;
    }

    let height = polygon_geometry.height;
    let extruded_height = polygon_geometry.extruded_height;
    let extrude = !CesiumMath::equals_epsilon(
        height,
        extruded_height,
        Some(0.0),
        Some(CesiumMath::EPSILON2),
    );

    let polygons = &results.polygons;
    let mut geometries: Vec<Geometry> = Vec::new();

    for polygon in polygons {
        let geo = PolygonGeometryLibrary::create_geometry_from_positions(
            ellipsoid,
            polygon,
            None,
            polygon_geometry.granularity,
            per_position_height,
            vertex_format,
            polygon_geometry.arc_type,
        );
        geometries.push(geo);
    }

    if geometries.is_empty() {
        return None;
    }

    // Scale to height (non-extruded case)
    if !extrude {
        for geo in &mut geometries {
            if let Some(pos_attr) = geo.attributes.get_mut("position") {
                let mut vals = pos_attr.values.clone();
                PolygonPipeline::scale_to_geodetic_height(
                    Some(&mut vals),
                    Some(height),
                    Some(ellipsoid),
                    Some(true),
                );
                pos_attr.values = vals;
            }
        }

        // Add offset attribute if needed
        if let Some(offset_attr) = polygon_geometry.offset_attribute {
            for geo in &mut geometries {
                let length = geo.attributes.get("position").map(|a| a.values.len()).unwrap_or(0);
                let offset_value = if offset_attr == GeometryOffsetAttribute::None { 0 } else { 1 };
                let apply_offset = vec![offset_value as f64; length / 3];
                geo.attributes.insert(
                    "applyOffset".to_string(),
                    GeometryAttribute::new(
                        ComponentDatatype::UnsignedByte,
                        1,
                        false,
                        apply_offset,
                    ),
                );
            }
        }
    } else {
        // Extruded case: scale to geodetic height extruded
        for geo in &mut geometries {
            PolygonGeometryLibrary::scale_to_geodetic_height_extruded(
                Some(geo),
                height,
                extruded_height,
                Some(ellipsoid.clone()),
                per_position_height,
            );

            if let Some(offset_attr) = polygon_geometry.offset_attribute {
                let length = geo.attributes.get("position").map(|a| a.values.len()).unwrap_or(0);
                let vertex_count = length / 3;
                let apply_offset: Vec<f64> = if offset_attr == GeometryOffsetAttribute::Top {
                    let mut v = vec![0.0f64; vertex_count];
                    for i in 0..vertex_count / 2 {
                        v[i] = 1.0;
                    }
                    v
                } else {
                    let ov = if offset_attr == GeometryOffsetAttribute::None { 0 } else { 1 };
                    vec![ov as f64; vertex_count]
                };
                geo.attributes.insert(
                    "applyOffset".to_string(),
                    GeometryAttribute::new(
                        ComponentDatatype::UnsignedByte,
                        1,
                        false,
                        apply_offset,
                    ),
                );
            }
        }
    }

    // Merge geometries if multiple
    let final_geometry = if geometries.len() == 1 {
        geometries.into_iter().next().unwrap()
    } else {
        merge_geometries(geometries)
    };

    // Remove position if vertex_format doesn't request it
    if !vertex_format.position {
        let mut geo = final_geometry;
        geo.attributes.remove("position");
        // Re-add a dummy position for Geometry validity
        let pos = geo.attributes.values().next();
        if pos.is_none() {
            return None;
        }
        Some(Geometry::with_all(
            geo.attributes,
            geo.indices,
            Some(geo.primitive_type),
            geo.bounding_sphere,
            GeometryType::None,
            None,
            polygon_geometry.offset_attribute.map(|_| "applyOffset".to_string()),
        ))
    } else {
        let mut geo = final_geometry;
        // Update bounding sphere from position
        let pos_values = geo.attributes.get("position").map(|a| a.values.clone()).unwrap_or_default();
        let bounding_sphere = BoundingSphere::from_vertices(&pos_values, None, Some(3), None);
        geo.bounding_sphere = Some(bounding_sphere);
        geo.offset_attribute = polygon_geometry.offset_attribute.map(|_| "applyOffset".to_string());
        Some(geo)
    }
}

/// Merge multiple geometries into one, combining attributes and indices.
fn merge_geometries(geometries: Vec<Geometry>) -> Geometry {
    let mut merged_attrs: HashMap<String, GeometryAttribute> = HashMap::new();
    let mut merged_indices_vec: Vec<u32> = Vec::new();
    let mut vertex_offset = 0u32;

    let attr_keys: Vec<String> = geometries
        .first()
        .map(|g| g.attributes.keys().cloned().collect())
        .unwrap_or_default();

    for key in &attr_keys {
        let mut merged_values = Vec::new();
        for geo in &geometries {
            if let Some(attr) = geo.attributes.get(key) {
                merged_values.extend_from_slice(&attr.values);
            }
        }
        if !merged_values.is_empty() {
            let (dt, comp) = geometries
                .first()
                .and_then(|g| g.attributes.get(key))
                .map(|a| (a.component_datatype, a.components_per_attribute))
                .unwrap_or((ComponentDatatype::Double, 3));
            merged_attrs.insert(
                key.clone(),
                GeometryAttribute::new(dt, comp, false, merged_values),
            );
        }
    }

    for geo in &geometries {
        let pos_len = geo
            .attributes
            .get("position")
            .map(|a| a.values.len() / 3)
            .unwrap_or(0);
        if let Some(indices) = &geo.indices {
            for i in 0..indices.len() {
                let v = read_index(indices, i);
                merged_indices_vec.push(v + vertex_offset);
            }
        }
        vertex_offset += pos_len as u32;
    }

    let total_vertices = vertex_offset as usize;
    let mut merged_indices =
        IndexDatatype::create_typed_array(total_vertices, merged_indices_vec.len());
    for (i, &v) in merged_indices_vec.iter().enumerate() {
        write_index(&mut merged_indices, i, v);
    }

    let pos_values = merged_attrs
        .get("position")
        .map(|a| a.values.clone())
        .unwrap_or_default();
    let bounding_sphere = BoundingSphere::from_vertices(&pos_values, None, Some(3), None);

    Geometry::with_all(
        merged_attrs,
        Some(merged_indices),
        Some(PrimitiveType::Triangles),
        Some(bounding_sphere),
        GeometryType::None,
        None,
        None,
    )
}
