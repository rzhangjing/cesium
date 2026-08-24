//! Ported from `packages/engine/Source/Core/CoplanarPolygonGeometry.js`.
//!
//! A description of a polygon composed of arbitrary coplanar positions.
//!
//! DEVIATION: JS `createGeometry` merges per-polygon geometries via
//! `GeometryInstance` + `GeometryPipeline.combineInstances`; the Rust port
//! merges them directly (same attribute/index concatenation result).

use std::collections::HashMap;

use crate::bounding_rectangle::BoundingRectangle;
use crate::bounding_sphere::BoundingSphere;
use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::component_datatype::ComponentDatatype;
use crate::coplanar_polygon_geometry_library::CoplanarPolygonGeometryLibrary;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::index_datatype::{IndexDatatype, IndexStorage};
use crate::math::CesiumMath;
use crate::matrix3::Matrix3;
use crate::polygon_geometry_library::PolygonGeometryLibrary;
use crate::polygon_hierarchy::PolygonHierarchy;
use crate::polygon_pipeline::PolygonPipeline;
use crate::primitive_type::PrimitiveType;
use crate::quaternion::Quaternion;
use crate::vertex_format::VertexFormat;
use crate::winding_order::WindingOrder;

/// A hierarchy of linear rings of [`Cartesian2`] points, mirroring the JS
/// `PolygonHierarchy` used for texture coordinates.
#[derive(Debug, Clone, Default)]
pub struct PolygonHierarchy2D {
    /// A linear ring defining the outer boundary of the polygon or hole.
    pub positions: Vec<Cartesian2>,
    /// An array of hierarchies defining holes.
    pub holes: Vec<PolygonHierarchy2D>,
}

impl PolygonHierarchy2D {
    /// Creates a new `PolygonHierarchy2D`.
    pub fn new(positions: Vec<Cartesian2>, holes: Vec<PolygonHierarchy2D>) -> Self {
        Self { positions, holes }
    }
}

/// A description of a polygon composed of arbitrary coplanar positions.
#[derive(Debug, Clone)]
pub struct CoplanarPolygonGeometry {
    polygon_hierarchy: PolygonHierarchy,
    vertex_format: VertexFormat,
    st_rotation: f64,
    ellipsoid: Ellipsoid,
    texture_coordinates: Option<PolygonHierarchy2D>,
}

impl CoplanarPolygonGeometry {
    /// Creates a new `CoplanarPolygonGeometry` from a polygon hierarchy
    /// (JS constructor).
    pub fn from_hierarchy(
        polygon_hierarchy: PolygonHierarchy,
        vertex_format: Option<VertexFormat>,
        st_rotation: Option<f64>,
        ellipsoid: Option<Ellipsoid>,
        texture_coordinates: Option<PolygonHierarchy2D>,
    ) -> Self {
        Self {
            polygon_hierarchy,
            vertex_format: vertex_format.unwrap_or_default(),
            st_rotation: st_rotation.unwrap_or(0.0),
            ellipsoid: ellipsoid.unwrap_or(Ellipsoid::WGS84.clone()),
            texture_coordinates,
        }
    }

    /// Creates a new `CoplanarPolygonGeometry` from a flat ring of positions.
    ///
    /// Kept for backwards compatibility; equivalent to JS
    /// `CoplanarPolygonGeometry.fromPositions`.
    pub fn new(positions: Vec<Cartesian3>, vertex_format: Option<VertexFormat>) -> Self {
        Self::from_positions(positions, vertex_format, None, None, None)
    }

    /// A description of a coplanar polygon from an array of positions (JS
    /// `CoplanarPolygonGeometry.fromPositions`).
    pub fn from_positions(
        positions: Vec<Cartesian3>,
        vertex_format: Option<VertexFormat>,
        st_rotation: Option<f64>,
        ellipsoid: Option<Ellipsoid>,
        texture_coordinates: Option<PolygonHierarchy2D>,
    ) -> Self {
        Self::from_hierarchy(
            PolygonHierarchy::new(positions, Vec::new()),
            vertex_format,
            st_rotation,
            ellipsoid,
            texture_coordinates,
        )
    }

    /// The polygon hierarchy (JS `_polygonHierarchy`).
    pub fn polygon_hierarchy(&self) -> &PolygonHierarchy {
        &self.polygon_hierarchy
    }

    /// The vertex format (JS `_vertexFormat`).
    pub fn vertex_format(&self) -> &VertexFormat {
        &self.vertex_format
    }

    /// The texture coordinate rotation (JS `_stRotation`).
    pub fn st_rotation(&self) -> f64 {
        self.st_rotation
    }

    /// The ellipsoid (JS `_ellipsoid`).
    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }

    /// The texture coordinates hierarchy (JS `_textureCoordinates`).
    pub fn texture_coordinates(&self) -> Option<&PolygonHierarchy2D> {
        self.texture_coordinates.as_ref()
    }

    /// The number of elements used to pack the object into an array (JS
    /// instance property `packedLength`).
    pub fn packed_length(&self) -> usize {
        let mut length = hierarchy_packed_length_3d(&self.polygon_hierarchy)
            + VertexFormat::PACKED_LENGTH
            + Ellipsoid::PACKED_LENGTH;
        length += match &self.texture_coordinates {
            Some(texture_coordinates) => hierarchy_packed_length_2d(texture_coordinates),
            None => 1,
        };
        length + 2
    }

    /// Stores this instance into `array` (JS `CoplanarPolygonGeometry.pack`).
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        let mut si = starting_index.unwrap_or(0);

        si = pack_hierarchy_3d(&self.polygon_hierarchy, array, si);

        Ellipsoid::pack(&self.ellipsoid, array, Some(si));
        si += Ellipsoid::PACKED_LENGTH;

        self.vertex_format.pack(array, si);
        si += VertexFormat::PACKED_LENGTH;

        array[si] = self.st_rotation;
        si += 1;

        match &self.texture_coordinates {
            Some(texture_coordinates) => {
                si = pack_hierarchy_2d(texture_coordinates, array, si);
            }
            None => {
                array[si] = -1.0;
                si += 1;
            }
        }
        array[si] = self.packed_length() as f64;
    }

    /// Retrieves an instance from a packed array (JS
    /// `CoplanarPolygonGeometry.unpack`).
    pub fn unpack(array: &[f64], starting_index: Option<usize>, result: Option<&mut Self>) -> Self {
        let mut si = starting_index.unwrap_or(0);

        let (polygon_hierarchy, next) = unpack_hierarchy_3d(array, si);
        si = next;

        let ellipsoid = Ellipsoid::unpack(array, Some(si));
        si += Ellipsoid::PACKED_LENGTH;

        let vertex_format = VertexFormat::unpack(array, si, None);
        si += VertexFormat::PACKED_LENGTH;

        let st_rotation = array[si];
        si += 1;

        let texture_coordinates: Option<PolygonHierarchy2D> = if array[si] == -1.0 {
            si += 1;
            None
        } else {
            let (texture_coordinates, next) = unpack_hierarchy_2d(array, si);
            si = next;
            Some(texture_coordinates)
        };
        let _packed_length = array[si];

        match result {
            None => Self::from_hierarchy(
                polygon_hierarchy,
                Some(vertex_format),
                Some(st_rotation),
                Some(ellipsoid),
                texture_coordinates,
            ),
            Some(r) => {
                r.polygon_hierarchy = polygon_hierarchy;
                r.ellipsoid = ellipsoid;
                r.vertex_format = vertex_format;
                r.st_rotation = st_rotation;
                r.texture_coordinates = texture_coordinates;
                r.clone()
            }
        }
    }

    /// Computes the geometric representation of an arbitrary coplanar
    /// polygon, including its vertices, indices, and a bounding sphere (JS
    /// `CoplanarPolygonGeometry.createGeometry`).
    pub fn create_geometry(&self) -> Option<Geometry> {
        let vertex_format = &self.vertex_format;
        let polygon_hierarchy = &self.polygon_hierarchy;
        let st_rotation = self.st_rotation;
        let texture_coordinates = self.texture_coordinates.as_ref();
        let has_texture_coordinates = texture_coordinates.is_some();

        let mut outer_positions = crate::array_remove_duplicates::array_remove_duplicates(
            &polygon_hierarchy.positions,
            |a: &Cartesian3, b: &Cartesian3, eps: f64| {
                Cartesian3::equals_epsilon(Some(a), Some(b), Some(eps), Some(eps))
            },
            true,
            None,
        )
        .unwrap_or_else(|| polygon_hierarchy.positions.clone());
        if outer_positions.len() < 3 {
            return None;
        }

        let mut center = Cartesian3::default();
        let mut axis1 = Cartesian3::default();
        let mut axis2 = Cartesian3::default();

        let valid_geometry = CoplanarPolygonGeometryLibrary::compute_project_to_2d_arguments(
            &outer_positions,
            &mut center,
            &mut axis1,
            &mut axis2,
        );
        if !valid_geometry {
            return None;
        }

        let mut normal = Cartesian3::default();
        Cartesian3::cross(&axis1, &axis2, &mut normal);
        let mut normalized = Cartesian3::default();
        Cartesian3::normalize(&normal, &mut normalized);
        normal = normalized;

        if !Cartesian3::equals_epsilon(
            Some(&center),
            Some(&Cartesian3::ZERO),
            Some(CesiumMath::EPSILON6),
            Some(CesiumMath::EPSILON6),
        ) {
            let mut surface_normal = Cartesian3::default();
            self.ellipsoid
                .geodetic_surface_normal(&center, &mut surface_normal);
            if Cartesian3::dot(&normal, &surface_normal) < 0.0 {
                let mut negated = Cartesian3::default();
                Cartesian3::negate(&normal, &mut negated);
                normal = negated;
                let mut negated = Cartesian3::default();
                Cartesian3::negate(&axis1, &mut negated);
                axis1 = negated;
            }
        }

        let project_points = CoplanarPolygonGeometryLibrary::create_project_points_to_2d_function(
            &center, &axis1, &axis2,
        );
        let project_point = CoplanarPolygonGeometryLibrary::create_project_point_to_2d_function(
            &center, &axis1, &axis2,
        );

        let mut tangent = Cartesian3::default();
        let mut bitangent = Cartesian3::default();
        if vertex_format.tangent {
            tangent = axis1;
        }
        if vertex_format.bitangent {
            bitangent = axis2;
        }

        // JS passes `hasTextureCoordinates` as `keepDuplicates` and `false`
        // for `scaleToEllipsoidSurface` (ellipsoid/splitPolygons undefined).
        let project_points_adapter =
            |positions: &[Cartesian3]| -> Option<Vec<Cartesian2>> { Some(project_points(positions)) };
        let results = PolygonGeometryLibrary::polygons_from_hierarchy(
            polygon_hierarchy,
            has_texture_coordinates,
            &project_points_adapter,
            false,
            &self.ellipsoid,
            None,
        );
        let hierarchy = &results.hierarchy;
        let polygons = &results.polygons;

        let texture_coordinate_polygons: Option<Vec<Vec<Cartesian2>>> =
            texture_coordinates.map(flatten_hierarchy_2d);

        if hierarchy.is_empty() {
            return None;
        }
        outer_positions = hierarchy[0].outer_ring.clone();

        let bounding_sphere = BoundingSphere::from_points(&outer_positions, None);
        let bounding_rectangle = PolygonGeometryLibrary::compute_bounding_rectangle(
            &normal,
            &project_point,
            &outer_positions,
            st_rotation,
        );

        let mut geometries: Vec<Geometry> = Vec::new();
        for (i, polygon) in polygons.iter().enumerate() {
            let hardcoded_texture_coordinates = if has_texture_coordinates {
                texture_coordinate_polygons.as_ref().map(|polygons| &polygons[i])
            } else {
                None
            };
            let geometry = create_geometry_from_polygon(
                polygon,
                vertex_format,
                &bounding_rectangle,
                st_rotation,
                hardcoded_texture_coordinates,
                &project_point,
                &mut normal,
                &mut tangent,
                &mut bitangent,
            );
            geometries.push(geometry);
        }

        // JS merges via GeometryPipeline.combineInstances(instances)[0]; the
        // geometries share attribute layout, so direct concatenation yields
        // the same result.
        let mut geometry = merge_geometries(geometries);

        let num_vertices = geometry
            .attributes
            .get("position")
            .map(|a| a.values.len() / 3)
            .unwrap_or(0);
        if let Some(indices) = &geometry.indices {
            let index_count = indices.len();
            let mut new_indices = IndexDatatype::create_typed_array(num_vertices, index_count);
            for i in 0..index_count {
                let value = match indices {
                    IndexStorage::U16(v) => v[i] as u32,
                    IndexStorage::U32(v) => v[i],
                };
                match &mut new_indices {
                    IndexStorage::U16(v) => v[i] = value as u16,
                    IndexStorage::U32(v) => v[i] = value,
                }
            }
            geometry.indices = Some(new_indices);
        }

        if !vertex_format.position {
            geometry.attributes.remove("position");
        }

        Some(Geometry::with_all(
            geometry.attributes,
            geometry.indices,
            Some(geometry.primitive_type),
            Some(bounding_sphere),
            crate::geometry_type::GeometryType::None,
            None,
            None,
        ))
    }
}

/// Mirrors the private JS `createGeometryFromPolygon` helper.
#[allow(clippy::too_many_arguments)]
fn create_geometry_from_polygon(
    polygon: &crate::polygon_geometry_library::PolygonResultEntry,
    vertex_format: &VertexFormat,
    bounding_rectangle: &BoundingRectangle,
    st_rotation: f64,
    hardcoded_texture_coordinates: Option<&Vec<Cartesian2>>,
    project_point_to_2d: &dyn Fn(&Cartesian3, &mut Cartesian2),
    normal: &mut Cartesian3,
    tangent: &mut Cartesian3,
    bitangent: &mut Cartesian3,
) -> Geometry {
    let positions = &polygon.positions;
    let mut indices = PolygonPipeline::triangulate(&polygon.positions_2d, Some(&polygon.holes));

    // If polygon is completely unrenderable, just use the first three vertices.
    if indices.len() < 3 {
        indices = vec![0, 1, 2];
    }

    let mut new_indices = IndexDatatype::create_typed_array(positions.len(), indices.len());
    for (i, &index) in indices.iter().enumerate() {
        match &mut new_indices {
            IndexStorage::U16(v) => v[i] = index as u16,
            IndexStorage::U32(v) => v[i] = index as u32,
        }
    }

    let texture_matrix: Matrix3;
    if st_rotation != 0.0 {
        let rotation = Quaternion::from_axis_angle_new(normal, st_rotation);
        let mut matrix = Matrix3::default();
        Matrix3::from_quaternion(&rotation, &mut matrix);
        texture_matrix = matrix;

        if vertex_format.tangent || vertex_format.bitangent {
            let rotation = Quaternion::from_axis_angle_new(normal, -st_rotation);
            let mut tangent_rotation = Matrix3::default();
            Matrix3::from_quaternion(&rotation, &mut tangent_rotation);

            let mut rotated = Cartesian3::default();
            Matrix3::multiply_by_vector(&tangent_rotation, tangent, &mut rotated);
            let mut normalized = Cartesian3::default();
            Cartesian3::normalize(&rotated, &mut normalized);
            *tangent = normalized;
            if vertex_format.bitangent {
                let mut cross = Cartesian3::default();
                Cartesian3::cross(normal, tangent, &mut cross);
                let mut normalized = Cartesian3::default();
                Cartesian3::normalize(&cross, &mut normalized);
                *bitangent = normalized;
            }
        }
    } else {
        texture_matrix = Matrix3::IDENTITY;
    }

    let mut st_origin = Cartesian2::default();
    if vertex_format.st {
        st_origin.x = bounding_rectangle.x;
        st_origin.y = bounding_rectangle.y;
    }

    let length = positions.len();
    let size = length * 3;
    let mut flat_positions = vec![0.0f64; size];
    let mut normals: Option<Vec<f64>> = if vertex_format.normal { Some(vec![0.0; size]) } else { None };
    let mut tangents: Option<Vec<f64>> = if vertex_format.tangent { Some(vec![0.0; size]) } else { None };
    let mut bitangents: Option<Vec<f64>> =
        if vertex_format.bitangent { Some(vec![0.0; size]) } else { None };
    let mut texture_coordinates: Option<Vec<f64>> =
        if vertex_format.st { Some(vec![0.0; length * 2]) } else { None };

    let mut position_index = 0usize;
    let mut normal_index = 0usize;
    let mut bitangent_index = 0usize;
    let mut tangent_index = 0usize;
    let mut st_index = 0usize;

    for (i, position) in positions.iter().enumerate() {
        flat_positions[position_index] = position.x;
        position_index += 1;
        flat_positions[position_index] = position.y;
        position_index += 1;
        flat_positions[position_index] = position.z;
        position_index += 1;

        if let Some(texture_coordinates) = &mut texture_coordinates {
            if let Some(hardcoded) = hardcoded_texture_coordinates {
                if hardcoded.len() == length {
                    texture_coordinates[st_index] = hardcoded[i].x;
                    st_index += 1;
                    texture_coordinates[st_index] = hardcoded[i].y;
                    st_index += 1;
                } else {
                    compute_st(
                        &texture_matrix,
                        position,
                        project_point_to_2d,
                        &st_origin,
                        bounding_rectangle,
                        texture_coordinates,
                        &mut st_index,
                    );
                }
            } else {
                compute_st(
                    &texture_matrix,
                    position,
                    project_point_to_2d,
                    &st_origin,
                    bounding_rectangle,
                    texture_coordinates,
                    &mut st_index,
                );
            }
        }

        if let Some(normals) = &mut normals {
            normals[normal_index] = normal.x;
            normal_index += 1;
            normals[normal_index] = normal.y;
            normal_index += 1;
            normals[normal_index] = normal.z;
            normal_index += 1;
        }

        if let Some(tangents) = &mut tangents {
            tangents[tangent_index] = tangent.x;
            tangent_index += 1;
            tangents[tangent_index] = tangent.y;
            tangent_index += 1;
            tangents[tangent_index] = tangent.z;
            tangent_index += 1;
        }

        if let Some(bitangents) = &mut bitangents {
            bitangents[bitangent_index] = bitangent.x;
            bitangent_index += 1;
            bitangents[bitangent_index] = bitangent.y;
            bitangent_index += 1;
            bitangents[bitangent_index] = bitangent.z;
            bitangent_index += 1;
        }
    }

    let mut attributes: HashMap<String, GeometryAttribute> = HashMap::new();

    if vertex_format.position {
        attributes.insert(
            "position".to_string(),
            GeometryAttribute::new(ComponentDatatype::Double, 3, false, flat_positions),
        );
    }

    if let Some(normals) = normals {
        attributes.insert(
            "normal".to_string(),
            GeometryAttribute::new(ComponentDatatype::Float, 3, false, normals),
        );
    }

    if let Some(tangents) = tangents {
        attributes.insert(
            "tangent".to_string(),
            GeometryAttribute::new(ComponentDatatype::Float, 3, false, tangents),
        );
    }

    if let Some(bitangents) = bitangents {
        attributes.insert(
            "bitangent".to_string(),
            GeometryAttribute::new(ComponentDatatype::Float, 3, false, bitangents),
        );
    }

    if let Some(texture_coordinates) = texture_coordinates {
        attributes.insert(
            "st".to_string(),
            GeometryAttribute::new(ComponentDatatype::Float, 2, false, texture_coordinates),
        );
    }

    Geometry::with_all(
        attributes,
        Some(new_indices),
        Some(PrimitiveType::Triangles),
        None,
        crate::geometry_type::GeometryType::None,
        None,
        None,
    )
}

/// Computes texture coordinates for one position (inlined from the JS
/// `createGeometryFromPolygon` loop body).
fn compute_st(
    texture_matrix: &Matrix3,
    position: &Cartesian3,
    project_point_to_2d: &dyn Fn(&Cartesian3, &mut Cartesian2),
    st_origin: &Cartesian2,
    bounding_rectangle: &BoundingRectangle,
    texture_coordinates: &mut Vec<f64>,
    st_index: &mut usize,
) {
    let mut rotated = Cartesian3::default();
    Matrix3::multiply_by_vector(texture_matrix, position, &mut rotated);
    let mut st = Cartesian2::default();
    project_point_to_2d(&rotated, &mut st);
    let mut st_shifted = Cartesian2::default();
    Cartesian2::subtract(&st, st_origin, &mut st_shifted);
    st = st_shifted;

    let stx = CesiumMath::clamp(st.x / bounding_rectangle.width, 0.0, 1.0);
    let sty = CesiumMath::clamp(st.y / bounding_rectangle.height, 0.0, 1.0);
    texture_coordinates[*st_index] = stx;
    *st_index += 1;
    texture_coordinates[*st_index] = sty;
    *st_index += 1;
}

/// Flattens a [`PolygonHierarchy2D`] into per-polygon 2D position lists,
/// mirroring JS
/// `PolygonGeometryLibrary.polygonsFromHierarchy(textureCoordinates, true,
/// dummyFunction, false)` (identity projection, keep duplicates).
fn flatten_hierarchy_2d(hierarchy: &PolygonHierarchy2D) -> Vec<Vec<Cartesian2>> {
    let mut polygons: Vec<Vec<Cartesian2>> = Vec::new();
    let mut queue: Vec<PolygonHierarchy2D> = vec![hierarchy.clone()];
    // Breadth-first like the JS Queue.
    let mut front = 0usize;
    while front < queue.len() {
        let outer_node = queue[front].clone();
        front += 1;

        let mut outer_ring = outer_node.positions;
        if outer_ring.len() < 3 {
            continue;
        }

        // JS reverses clockwise rings (identity projection ⇒ winding order of
        // the 2D positions themselves).
        if PolygonPipeline::compute_winding_order_2d(&outer_ring) == WindingOrder::Clockwise {
            outer_ring.reverse();
        }

        let mut positions = outer_ring.clone();
        for hole in &outer_node.holes {
            let mut hole_positions = hole.positions.clone();
            if hole_positions.len() < 3 {
                continue;
            }
            if PolygonPipeline::compute_winding_order_2d(&hole_positions)
                == WindingOrder::Clockwise
            {
                hole_positions.reverse();
            }
            positions.extend(hole_positions);

            for grandchild in &hole.holes {
                queue.push(grandchild.clone());
            }
        }

        polygons.push(positions);
    }

    polygons
}

/// Merges per-polygon geometries into one (mirrors the attribute/index
/// concatenation of `GeometryPipeline.combineInstances`).
fn merge_geometries(geometries: Vec<Geometry>) -> Geometry {
    if geometries.len() == 1 {
        return geometries.into_iter().next().unwrap();
    }

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
            let (dt, comp, normalize) = geometries
                .first()
                .and_then(|g| g.attributes.get(key))
                .map(|a| (a.component_datatype, a.components_per_attribute, a.normalize))
                .unwrap_or((ComponentDatatype::Double, 3, false));
            merged_attrs.insert(
                key.clone(),
                GeometryAttribute::new(dt, comp, normalize, merged_values),
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
                let v = match indices {
                    IndexStorage::U16(v) => v[i] as u32,
                    IndexStorage::U32(v) => v[i],
                };
                merged_indices_vec.push(v + vertex_offset);
            }
        }
        vertex_offset += pos_len as u32;
    }

    let total_vertices = vertex_offset as usize;
    let mut merged_indices =
        IndexDatatype::create_typed_array(total_vertices, merged_indices_vec.len());
    for (i, &v) in merged_indices_vec.iter().enumerate() {
        match &mut merged_indices {
            IndexStorage::U16(v) => v[i] = merged_indices_vec[i] as u16,
            IndexStorage::U32(v) => v[i] = merged_indices_vec[i],
        }
    }

    Geometry::with_all(
        merged_attrs,
        Some(merged_indices),
        Some(PrimitiveType::Triangles),
        None,
        crate::geometry_type::GeometryType::None,
        None,
        None,
    )
}

// --- Polygon hierarchy pack helpers (Cartesian3 / Cartesian2 variants) ---

/// Public wrapper of [`pack_hierarchy_3d`] for reuse by
/// `CoplanarPolygonOutlineGeometry`.
pub fn pack_hierarchy_3d_pub(
    hierarchy: &PolygonHierarchy,
    array: &mut [f64],
    starting_index: usize,
) -> usize {
    pack_hierarchy_3d(hierarchy, array, starting_index)
}

/// Public wrapper of [`unpack_hierarchy_3d`] for reuse by
/// `CoplanarPolygonOutlineGeometry`.
pub fn unpack_hierarchy_3d_pub(
    array: &[f64],
    starting_index: usize,
) -> (PolygonHierarchy, usize) {
    unpack_hierarchy_3d(array, starting_index)
}

/// Public wrapper of [`hierarchy_packed_length_2d`] for reuse by
/// `PolygonGeometry`.
pub fn hierarchy_packed_length_2d_pub(hierarchy: &PolygonHierarchy2D) -> usize {
    hierarchy_packed_length_2d(hierarchy)
}

/// Public wrapper of [`pack_hierarchy_2d`] for reuse by `PolygonGeometry`.
pub fn pack_hierarchy_2d_pub(
    hierarchy: &PolygonHierarchy2D,
    array: &mut [f64],
    starting_index: usize,
) -> usize {
    pack_hierarchy_2d(hierarchy, array, starting_index)
}

/// Public wrapper of [`unpack_hierarchy_2d`] for reuse by `PolygonGeometry`.
pub fn unpack_hierarchy_2d_pub(
    array: &[f64],
    starting_index: usize,
) -> (PolygonHierarchy2D, usize) {
    unpack_hierarchy_2d(array, starting_index)
}

fn hierarchy_packed_length_3d(hierarchy: &PolygonHierarchy) -> usize {
    let mut num_components = 0;
    let mut stack = vec![hierarchy];
    while let Some(h) = stack.pop() {
        num_components += 2 + h.positions.len() * Cartesian3::PACKED_LENGTH;
        for hole in &h.holes {
            stack.push(hole);
        }
    }
    num_components
}

fn hierarchy_packed_length_2d(hierarchy: &PolygonHierarchy2D) -> usize {
    let mut num_components = 0;
    let mut stack = vec![hierarchy];
    while let Some(h) = stack.pop() {
        num_components += 2 + h.positions.len() * Cartesian2::PACKED_LENGTH;
        for hole in &h.holes {
            stack.push(hole);
        }
    }
    num_components
}

/// Mirrors JS `PolygonGeometryLibrary.packPolygonHierarchy` with `Cartesian3`.
///
/// DEVIATION: depth-first with holes pushed onto a stack in order (the Rust
/// `pack_polygon_hierarchy` in `polygon_geometry_library.rs` uses the same
/// traversal, so layouts are mutually compatible).
fn pack_hierarchy_3d(hierarchy: &PolygonHierarchy, array: &mut [f64], starting_index: usize) -> usize {
    let mut si = starting_index;
    let mut stack = vec![hierarchy];
    while let Some(h) = stack.pop() {
        array[si] = h.positions.len() as f64;
        si += 1;
        array[si] = h.holes.len() as f64;
        si += 1;

        for position in &h.positions {
            Cartesian3::pack(position, array, Some(si));
            si += Cartesian3::PACKED_LENGTH;
        }

        for hole in &h.holes {
            stack.push(hole);
        }
    }
    si
}

/// Mirrors JS `PolygonGeometryLibrary.packPolygonHierarchy` with `Cartesian2`.
fn pack_hierarchy_2d(hierarchy: &PolygonHierarchy2D, array: &mut [f64], starting_index: usize) -> usize {
    let mut si = starting_index;
    let mut stack = vec![hierarchy];
    while let Some(h) = stack.pop() {
        array[si] = h.positions.len() as f64;
        si += 1;
        array[si] = h.holes.len() as f64;
        si += 1;

        for position in &h.positions {
            Cartesian2::pack(position, array, Some(si));
            si += Cartesian2::PACKED_LENGTH;
        }

        for hole in &h.holes {
            stack.push(hole);
        }
    }
    si
}

/// Mirrors JS `PolygonGeometryLibrary.unpackPolygonHierarchy` with
/// `Cartesian3`; returns the hierarchy and the next starting index.
fn unpack_hierarchy_3d(array: &[f64], starting_index: usize) -> (PolygonHierarchy, usize) {
    let mut si = starting_index;
    let positions_length = array[si] as usize;
    si += 1;
    let holes_length = array[si] as usize;
    si += 1;

    let mut positions = Vec::with_capacity(positions_length);
    for _ in 0..positions_length {
        positions.push(Cartesian3::unpack_new(array, Some(si)));
        si += Cartesian3::PACKED_LENGTH;
    }

    let mut holes = Vec::with_capacity(holes_length);
    for _ in 0..holes_length {
        let (hole, next) = unpack_hierarchy_3d(array, si);
        si = next;
        holes.push(hole);
    }

    (PolygonHierarchy::new(positions, holes), si)
}

/// Mirrors JS `PolygonGeometryLibrary.unpackPolygonHierarchy` with
/// `Cartesian2`; returns the hierarchy and the next starting index.
fn unpack_hierarchy_2d(array: &[f64], starting_index: usize) -> (PolygonHierarchy2D, usize) {
    let mut si = starting_index;
    let positions_length = array[si] as usize;
    si += 1;
    let holes_length = array[si] as usize;
    si += 1;

    let mut positions = Vec::with_capacity(positions_length);
    for _ in 0..positions_length {
        positions.push(Cartesian2::unpack_new(array, Some(si)));
        si += Cartesian2::PACKED_LENGTH;
    }

    let mut holes = Vec::with_capacity(holes_length);
    for _ in 0..holes_length {
        let (hole, next) = unpack_hierarchy_2d(array, si);
        si = next;
        holes.push(hole);
    }

    (PolygonHierarchy2D::new(positions, holes), si)
}
