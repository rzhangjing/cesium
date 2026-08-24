//! Ported from `packages/engine/Source/Core/PolygonGeometryLibrary.js`.
//!
//! Library of functions for computing polygon geometry.
//!
//! DEVIATION: the JS source reuses module-level scratch objects (scratch
//! cartographics, scratch rhumb line, scratch cartesian); the Rust port uses
//! local values instead.
//!
//! DEVIATION: JS mutates the input polygon hierarchy in place when scaling
//! positions to the ellipsoid surface; the Rust port works on cloned data
//! and never mutates its inputs.

use std::collections::HashMap;

use crate::arc_type::ArcType;
use crate::array_remove_duplicates::array_remove_duplicates;
use crate::bounding_rectangle::BoundingRectangle;
use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::component_datatype::ComponentDatatype;
use crate::ellipsoid::Ellipsoid;
use crate::ellipsoid_rhumb_line::EllipsoidRhumbLine;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_pipeline::normals::compute_normal;
use crate::index_datatype::{IndexDatatype, IndexStorage};
use crate::intersection_tests::IntersectionTests;
use crate::math::CesiumMath;
use crate::matrix3::Matrix3;
use crate::plane::Plane;
use crate::polygon_hierarchy::PolygonHierarchy;
use crate::polygon_pipeline::PolygonPipeline;
use crate::primitive_type::PrimitiveType;
use crate::quaternion::Quaternion;
use crate::queue::Queue;
use crate::vertex_format::VertexFormat;
use crate::winding_order::WindingOrder;

/// Library of functions for computing polygon geometry.
pub struct PolygonGeometryLibrary {
    _private: (),
}

impl PolygonGeometryLibrary {
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Computes the number of components required to pack a polygon hierarchy.
    pub fn compute_hierarchy_packed_length(polygon_hierarchy: &PolygonHierarchy) -> usize {
        let mut num_components = 0;
        let mut stack = vec![polygon_hierarchy];
        while let Some(hierarchy) = stack.pop() {
            num_components += 2;

            let positions = &hierarchy.positions;
            let holes = &hierarchy.holes;

            if !positions.is_empty() {
                num_components += positions.len() * Cartesian3::PACKED_LENGTH;
            }

            for hole in holes {
                stack.push(hole);
            }
        }

        num_components
    }

    /// Packs a polygon hierarchy into a flat array.
    ///
    /// DEVIATION: JS takes a `startingIndex` and returns the next index; the
    /// Rust port appends to the provided vector.
    pub fn pack_polygon_hierarchy(polygon_hierarchy: &PolygonHierarchy, array: &mut Vec<f64>) {
        let mut stack = vec![polygon_hierarchy];
        while let Some(hierarchy) = stack.pop() {
            let positions = &hierarchy.positions;
            let holes = &hierarchy.holes;

            array.push(positions.len() as f64);
            array.push(holes.len() as f64);

            for position in positions {
                let starting_index = array.len();
                Cartesian3::pack(position, array, Some(starting_index));
            }

            for hole in holes {
                stack.push(hole);
            }
        }
    }

    /// Unpacks a polygon hierarchy from a flat array.
    ///
    /// DEVIATION: JS embeds a transient `startingIndex` property in the
    /// returned object; the Rust port returns a tuple of the hierarchy and
    /// the next starting index.
    pub fn unpack_polygon_hierarchy(
        array: &[f64],
        starting_index: usize,
    ) -> (PolygonHierarchy, usize) {
        let mut starting_index = starting_index;
        let positions_length = array[starting_index] as usize;
        starting_index += 1;
        let holes_length = array[starting_index] as usize;
        starting_index += 1;

        let mut positions = Vec::with_capacity(positions_length);
        for _ in 0..positions_length {
            let mut position = Cartesian3::default();
            Cartesian3::unpack(array, Some(starting_index), &mut position);
            starting_index += Cartesian3::PACKED_LENGTH;
            positions.push(position);
        }

        let mut holes = Vec::with_capacity(holes_length);
        for _ in 0..holes_length {
            let (hole, next_index) =
                PolygonGeometryLibrary::unpack_polygon_hierarchy(array, starting_index);
            starting_index = next_index;
            holes.push(hole);
        }

        (
            PolygonHierarchy { positions, holes },
            starting_index,
        )
    }

    /// Port of `PolygonGeometryLibrary.subdivideLineCount`.
    pub fn subdivide_line_count(p0: &Cartesian3, p1: &Cartesian3, min_distance: f64) -> usize {
        let distance = Cartesian3::distance(p0, p1);
        let n = distance / min_distance;
        let count_divide = (0.0f64.max(CesiumMath::log2(n).ceil())) as i32;
        2usize.pow(count_divide as u32)
    }

    /// Port of `PolygonGeometryLibrary.subdivideRhumbLineCount`.
    pub fn subdivide_rhumb_line_count(
        ellipsoid: &Ellipsoid,
        p0: &Cartesian3,
        p1: &Cartesian3,
        min_distance: f64,
    ) -> usize {
        let mut c0 = Cartographic::default();
        let mut c1 = Cartographic::default();
        ellipsoid.cartesian_to_cartographic(p0, &mut c0);
        ellipsoid.cartesian_to_cartographic(p1, &mut c1);
        let rhumb = EllipsoidRhumbLine::new(Some(c0), Some(c1), None, Some(*ellipsoid));
        let n = rhumb.rhumb_distance() / min_distance;
        let count_divide = (0.0f64.max(CesiumMath::log2(n).ceil())) as i32;
        2usize.pow(count_divide as u32)
    }

    /// Subdivides texture coordinates based on the subdivision of the
    /// associated world positions.
    pub fn subdivide_texcoord_line(
        t0: &Cartesian2,
        t1: &Cartesian2,
        p0: &Cartesian3,
        p1: &Cartesian3,
        min_distance: f64,
    ) -> Vec<f64> {
        // Compute the number of subdivisions.
        let subdivisions = PolygonGeometryLibrary::subdivide_line_count(p0, p1, min_distance);

        // Compute the distance between each subdivided point.
        let length_2d = Cartesian2::distance(t0, t1);
        let distance_between_coords = length_2d / subdivisions as f64;

        // Compute texture coordinates using linear interpolation.
        let mut texcoords = Vec::with_capacity(subdivisions * 2);
        for i in 0..subdivisions {
            let t = get_point_at_distance_2d(t0, t1, i as f64 * distance_between_coords, length_2d);
            texcoords.push(t.0);
            texcoords.push(t.1);
        }

        texcoords
    }

    /// Port of `PolygonGeometryLibrary.subdivideLine`.
    pub fn subdivide_line(p0: &Cartesian3, p1: &Cartesian3, min_distance: f64) -> Vec<f64> {
        let num_vertices = PolygonGeometryLibrary::subdivide_line_count(p0, p1, min_distance);
        let length = Cartesian3::distance(p0, p1);
        let distance_between_vertices = length / num_vertices as f64;

        let mut positions = Vec::with_capacity(num_vertices * 3);
        for i in 0..num_vertices {
            let p = get_point_at_distance(p0, p1, i as f64 * distance_between_vertices, length);
            positions.push(p.0);
            positions.push(p.1);
            positions.push(p.2);
        }

        positions
    }

    /// Subdivides texture coordinates based on the subdivision of the
    /// associated world positions using a rhumb line.
    pub fn subdivide_texcoord_rhumb_line(
        t0: &Cartesian2,
        t1: &Cartesian2,
        ellipsoid: &Ellipsoid,
        p0: &Cartesian3,
        p1: &Cartesian3,
        min_distance: f64,
    ) -> Vec<f64> {
        // Compute the surface distance.
        let mut c0 = Cartographic::default();
        let mut c1 = Cartographic::default();
        ellipsoid.cartesian_to_cartographic(p0, &mut c0);
        ellipsoid.cartesian_to_cartographic(p1, &mut c1);
        let rhumb = EllipsoidRhumbLine::new(Some(c0), Some(c1), None, Some(*ellipsoid));
        let n = rhumb.rhumb_distance() / min_distance;

        // Compute the number of subdivisions.
        let count_divide = (0.0f64.max(CesiumMath::log2(n).ceil())) as i32;
        let subdivisions = 2usize.pow(count_divide as u32);

        // Compute the distance between each subdivided point.
        let length_2d = Cartesian2::distance(t0, t1);
        let distance_between_coords = length_2d / subdivisions as f64;

        // Compute texture coordinates using linear interpolation.
        let mut texcoords = Vec::with_capacity(subdivisions * 2);
        for i in 0..subdivisions {
            let t = get_point_at_distance_2d(t0, t1, i as f64 * distance_between_coords, length_2d);
            texcoords.push(t.0);
            texcoords.push(t.1);
        }

        texcoords
    }

    /// Subdivide the line between 2 points every `min_distance` length.
    /// If the points are already closer than `min_distance` the first point
    /// will be returned.
    pub fn subdivide_rhumb_line(
        ellipsoid: &Ellipsoid,
        p0: &Cartesian3,
        p1: &Cartesian3,
        min_distance: f64,
    ) -> Vec<f64> {
        let mut c0 = Cartographic::default();
        let mut c1 = Cartographic::default();
        ellipsoid.cartesian_to_cartographic(p0, &mut c0);
        ellipsoid.cartesian_to_cartographic(p1, &mut c1);
        let rhumb = EllipsoidRhumbLine::new(Some(c0), Some(c1), None, Some(*ellipsoid));

        if rhumb.rhumb_distance() <= min_distance {
            // no need to try and subdivide a line that's already shorter than
            // the min distance; this also inherently handles duplicated
            // points which would have 0 distance
            return vec![p0.x, p0.y, p0.z];
        }

        let n = rhumb.rhumb_distance() / min_distance;
        let count_divide = (0.0f64.max(CesiumMath::log2(n).ceil())) as i32;
        let num_vertices = 2usize.pow(count_divide as u32);
        let distance_between_vertices = rhumb.rhumb_distance() / num_vertices as f64;

        let mut positions = Vec::with_capacity(num_vertices * 3);
        for i in 0..num_vertices {
            let c = rhumb.interpolate_using_surface_distance(i as f64 * distance_between_vertices);
            let mut p = Cartesian3::default();
            ellipsoid.cartographic_to_cartesian(&c, &mut p);
            positions.push(p.x);
            positions.push(p.y);
            positions.push(p.z);
        }

        positions
    }

    /// Port of `PolygonGeometryLibrary.scaleToGeodeticHeightExtruded`.
    pub fn scale_to_geodetic_height_extruded(
        geometry: Option<&mut Geometry>,
        max_height: f64,
        min_height: f64,
        ellipsoid: Option<Ellipsoid>,
        per_position_height: bool,
    ) -> Option<&mut Geometry> {
        let ellipsoid = ellipsoid.unwrap_or(Ellipsoid::WGS84);

        if let Some(geometry) = geometry {
            let has_position = geometry
                .attributes
                .get("position")
                .map(|a| !a.values.is_empty())
                .unwrap_or(false);
            if has_position {
                let positions = &mut geometry.attributes.get_mut("position").unwrap().values;
                let length = positions.len() / 2;

                for i in (0..length).step_by(3) {
                    let mut p = Cartesian3::default();
                    Cartesian3::from_array(positions, Some(i), &mut p);

                    let mut n1 = Cartesian3::default();
                    ellipsoid.geodetic_surface_normal(&p, &mut n1);
                    let mut p2 = Cartesian3::default();
                    ellipsoid.scale_to_geodetic_surface(&p, &mut p2);
                    let n2 = Cartesian3::multiply_by_scalar_new(&n1, min_height);
                    let n2 = Cartesian3::add_new(&p2, &n2);
                    positions[i + length] = n2.x;
                    positions[i + 1 + length] = n2.y;
                    positions[i + 2 + length] = n2.z;

                    let p2 = if per_position_height { p } else { p2 };
                    let n2 = Cartesian3::multiply_by_scalar_new(&n1, max_height);
                    let n2 = Cartesian3::add_new(&p2, &n2);
                    positions[i] = n2.x;
                    positions[i + 1] = n2.y;
                    positions[i + 2] = n2.z;
                }
            }
            return Some(geometry);
        }
        None
    }

    /// Port of `PolygonGeometryLibrary.polygonOutlinesFromHierarchy`.
    pub fn polygon_outlines_from_hierarchy(
        polygon_hierarchy: &PolygonHierarchy,
        scale_to_ellipsoid_surface: bool,
        ellipsoid: &Ellipsoid,
    ) -> Vec<Vec<Cartesian3>> {
        // create from a polygon hierarchy
        // Algorithm adapted from http://www.geometrictools.com/Documentation/TriangulationByEarClipping.pdf
        let mut polygons: Vec<Vec<Cartesian3>> = vec![];
        let mut queue: Queue<PolygonHierarchy> = Queue::new();
        queue.enqueue(polygon_hierarchy.clone());
        while queue.length() != 0 {
            let outer_node = queue.dequeue().unwrap();
            let mut outer_ring = outer_node.positions;
            if scale_to_ellipsoid_surface {
                for position in outer_ring.iter_mut() {
                    let mut scaled = Cartesian3::default();
                    ellipsoid.scale_to_geodetic_surface(position, &mut scaled);
                    *position = scaled;
                }
            }
            outer_ring = array_remove_duplicates(
                &outer_ring,
                |a, b, eps| {
                    Cartesian3::equals_epsilon(Some(a), Some(b), Some(eps), None)
                },
                true,
                None,
            )
            .unwrap_or(outer_ring);
            if outer_ring.len() < 3 {
                continue;
            }

            // The outer polygon contains inner polygons
            for hole in outer_node.holes.iter() {
                let mut hole_positions = hole.positions.clone();
                if scale_to_ellipsoid_surface {
                    for position in hole_positions.iter_mut() {
                        let mut scaled = Cartesian3::default();
                        ellipsoid.scale_to_geodetic_surface(position, &mut scaled);
                        *position = scaled;
                    }
                }
                hole_positions = array_remove_duplicates(
                    &hole_positions,
                    |a, b, eps| {
                        Cartesian3::equals_epsilon(Some(a), Some(b), Some(eps), None)
                    },
                    true,
                    None,
                )
                .unwrap_or(hole_positions);
                if hole_positions.len() < 3 {
                    continue;
                }
                polygons.push(hole_positions);

                for grandchild in hole.holes.iter() {
                    queue.enqueue(grandchild.clone());
                }
            }

            polygons.push(outer_ring);
        }

        polygons
    }

    /// Splits an array of polygons, defined as a list of Cartesian3 positions
    /// in counter-clockwise winding order, along the equator.
    ///
    /// DEVIATION: JS takes an optional `result` array; the Rust port always
    /// returns a new vector.
    pub fn split_polygons_on_equator(
        outer_rings: &[Vec<Cartesian3>],
        ellipsoid: &Ellipsoid,
        arc_type: ArcType,
    ) -> Vec<Vec<Cartesian3>> {
        let mut result: Vec<Vec<Cartesian3>> = outer_rings.to_vec();

        let mut current_polygon = 0;
        while current_polygon < result.len() {
            // Adapted from https://www.sciencedirect.com/science/article/abs/pii/B9780125434577500589
            let outer_ring = result[current_polygon].clone();
            let mut positions = outer_ring.clone();

            if outer_ring.len() < 3 {
                result[current_polygon] = positions;
                current_polygon += 1;
                continue;
            }

            // Step 1: Get all edges which intersect the split line, splicing
            // any found intersection points into the list of positions
            let mut edges_on_plane = compute_edges_on_plane(&mut positions, ellipsoid, arc_type);
            // If nothing intersected (no points were added), or there is only
            // a single point on the plane, use the original polygon
            if positions.len() == outer_ring.len() || edges_on_plane.len() <= 1 {
                result[current_polygon] = positions;
                current_polygon += 1;
                continue;
            }

            // Step 2: Sort the edges along the split line by the distance
            // between their starting points and the starting point of the
            // split line.
            edges_on_plane.sort_by(|a, b| a.theta.partial_cmp(&b.theta).unwrap());

            // Step 3: Rewire polygons, splicing each polygon into the array
            // of results
            let north = positions[0].z >= 0.0;
            current_polygon = wire_polygon(
                &mut result,
                current_polygon as isize,
                &positions,
                &mut edges_on_plane,
                1,
                0,
                north,
            ) as usize;
        }

        result
    }

    /// Port of `PolygonGeometryLibrary.polygonsFromHierarchy`.
    ///
    /// DEVIATION: JS mutates the hierarchy positions in place when scaling to
    /// the ellipsoid surface; the Rust port clones instead.
    pub fn polygons_from_hierarchy(
        polygon_hierarchy: &PolygonHierarchy,
        keep_duplicates: bool,
        project_points_to_2d: &dyn Fn(&[Cartesian3]) -> Option<Vec<Cartesian2>>,
        scale_to_ellipsoid_surface: bool,
        ellipsoid: &Ellipsoid,
        split_polygons: Option<&dyn Fn(Vec<Vec<Cartesian3>>) -> Vec<Vec<Cartesian3>>>,
    ) -> PolygonsFromHierarchyResult {
        // create from a polygon hierarchy
        // Algorithm adapted from http://www.geometrictools.com/Documentation/TriangulationByEarClipping.pdf
        let mut hierarchy: Vec<HierarchyResultEntry> = vec![];
        let mut polygons: Vec<PolygonResultEntry> = vec![];

        let mut queue: Queue<PolygonHierarchy> = Queue::new();
        queue.enqueue(polygon_hierarchy.clone());

        let mut split = split_polygons.is_some();

        while queue.length() != 0 {
            let outer_node = queue.dequeue().unwrap();
            let mut outer_ring = outer_node.positions;
            let holes = outer_node.holes;

            if scale_to_ellipsoid_surface {
                for position in outer_ring.iter_mut() {
                    let mut scaled = Cartesian3::default();
                    ellipsoid.scale_to_geodetic_surface(position, &mut scaled);
                    *position = scaled;
                }
            }

            if !keep_duplicates {
                outer_ring = array_remove_duplicates(
                    &outer_ring,
                    |a, b, eps| {
                        Cartesian3::equals_epsilon(Some(a), Some(b), Some(eps), None)
                    },
                    true,
                    None,
                )
                .unwrap_or(outer_ring);
            }
            if outer_ring.len() < 3 {
                continue;
            }

            let mut positions_2d = match project_points_to_2d(&outer_ring) {
                Some(positions_2d) => positions_2d,
                None => continue,
            };
            let mut hole_indices: Vec<usize> = vec![];

            let original_winding_order =
                PolygonPipeline::compute_winding_order_2d(&positions_2d);
            if original_winding_order == WindingOrder::Clockwise {
                positions_2d.reverse();
                outer_ring = outer_ring.into_iter().rev().collect();
            }

            if split {
                split = false;
                let split_polygons = split_polygons.unwrap();
                let split_polygons = split_polygons(vec![outer_ring.clone()]);

                if split_polygons.len() > 1 {
                    for positions in split_polygons {
                        queue.enqueue(PolygonHierarchy::new(positions, holes.clone()));
                    }

                    continue;
                }
            }

            let mut positions = outer_ring.clone();
            let mut polygon_holes: Vec<Vec<Cartesian3>> = vec![];

            for hole in holes.iter() {
                let mut hole_positions = hole.positions.clone();
                if scale_to_ellipsoid_surface {
                    for position in hole_positions.iter_mut() {
                        let mut scaled = Cartesian3::default();
                        ellipsoid.scale_to_geodetic_surface(position, &mut scaled);
                        *position = scaled;
                    }
                }

                if !keep_duplicates {
                    hole_positions = array_remove_duplicates(
                        &hole_positions,
                        |a, b, eps| {
                            Cartesian3::equals_epsilon(Some(a), Some(b), Some(eps), None)
                        },
                        true,
                        None,
                    )
                    .unwrap_or(hole_positions);
                }
                if hole_positions.len() < 3 {
                    continue;
                }

                let mut hole_positions_2d = match project_points_to_2d(&hole_positions) {
                    Some(hole_positions_2d) => hole_positions_2d,
                    None => continue,
                };

                let original_winding_order =
                    PolygonPipeline::compute_winding_order_2d(&hole_positions_2d);
                if original_winding_order == WindingOrder::Clockwise {
                    hole_positions_2d.reverse();
                    hole_positions = hole_positions.into_iter().rev().collect();
                }

                polygon_holes.push(hole_positions.clone());
                hole_indices.push(positions.len());
                positions.extend(hole_positions);
                positions_2d.extend(hole_positions_2d);

                for grandchild in hole.holes.iter() {
                    queue.enqueue(grandchild.clone());
                }
            }

            hierarchy.push(HierarchyResultEntry {
                outer_ring,
                holes: polygon_holes,
            });
            polygons.push(PolygonResultEntry {
                positions,
                positions_2d,
                holes: hole_indices,
            });
        }

        PolygonsFromHierarchyResult { hierarchy, polygons }
    }

    /// Port of `PolygonGeometryLibrary.computeBoundingRectangle`.
    pub fn compute_bounding_rectangle(
        plane_normal: &Cartesian3,
        project_point_to_2d: &dyn Fn(&Cartesian3, &mut Cartesian2),
        positions: &[Cartesian3],
        angle: f64,
    ) -> BoundingRectangle {
        let rotation = Quaternion::from_axis_angle_new(plane_normal, angle);
        let mut texture_matrix = Matrix3::default();
        Matrix3::from_quaternion(&rotation, &mut texture_matrix);

        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for position in positions {
            let mut p = *position;
            let mut rotated = Cartesian3::default();
            Matrix3::multiply_by_vector(&texture_matrix, &p, &mut rotated);
            p = rotated;
            let mut st = Cartesian2::default();
            project_point_to_2d(&p, &mut st);

            min_x = min_x.min(st.x);
            max_x = max_x.max(st.x);

            min_y = min_y.min(st.y);
            max_y = max_y.max(st.y);
        }

        BoundingRectangle {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        }
    }

    /// Port of `PolygonGeometryLibrary.createGeometryFromPositions`.
    pub fn create_geometry_from_positions(
        ellipsoid: &Ellipsoid,
        polygon: &PolygonResultEntry,
        texture_coordinates: Option<&PolygonTextureCoordinates>,
        granularity: f64,
        per_position_height: bool,
        vertex_format: &VertexFormat,
        arc_type: ArcType,
    ) -> Geometry {
        let mut indices = PolygonPipeline::triangulate(&polygon.positions_2d, Some(&polygon.holes));

        /* If polygon is completely unrenderable, just use the first three vertices */
        if indices.len() < 3 {
            indices = vec![0, 1, 2];
        }

        let positions = &polygon.positions;

        let has_texcoords = texture_coordinates.is_some();
        let texcoords = texture_coordinates.map(|tc| &tc.positions);

        if per_position_height {
            let length = positions.len();
            let mut flattened_positions = Vec::with_capacity(length * 3);
            for p in positions {
                flattened_positions.push(p.x);
                flattened_positions.push(p.y);
                flattened_positions.push(p.z);
            }

            let mut attributes = HashMap::new();
            attributes.insert(
                "position".to_string(),
                GeometryAttribute::new(
                    ComponentDatatype::Double,
                    3,
                    false,
                    flattened_positions,
                ),
            );

            if has_texcoords {
                attributes.insert(
                    "st".to_string(),
                    GeometryAttribute::new(
                        ComponentDatatype::Float,
                        2,
                        false,
                        Cartesian2::pack_array(texcoords.unwrap(), None),
                    ),
                );
            }

            let index_storage = IndexStorage::U32(indices.iter().map(|&i| i as u32).collect());
            let mut geometry = Geometry::new(
                attributes,
                Some(index_storage),
                Some(PrimitiveType::Triangles),
                None,
            );

            if vertex_format.normal {
                compute_normal(&mut geometry);
            }

            return geometry;
        }

        let indices_u32: Vec<u32> = indices.iter().map(|&i| i as u32).collect();
        let texcoords_slice: Option<&[Cartesian2]> = texcoords.map(|t| t.as_slice());
        if arc_type == ArcType::Geodesic {
            return PolygonPipeline::compute_subdivision(
                ellipsoid,
                positions,
                &indices_u32,
                texcoords_slice,
                Some(granularity),
            );
        } else if arc_type == ArcType::Rhumb {
            return PolygonPipeline::compute_rhumb_line_subdivision(
                ellipsoid,
                positions,
                &indices_u32,
                texcoords_slice,
                Some(granularity),
            );
        }

        // DEVIATION: JS returns `undefined` when `arcType` is neither
        // GEODESIC nor RHUMB; callers never reach this branch, so the Rust
        // port falls back to geodesic subdivision.
        PolygonPipeline::compute_subdivision(
            ellipsoid,
            positions,
            &indices_u32,
            texcoords_slice,
            Some(granularity),
        )
    }

    /// Port of `PolygonGeometryLibrary.computeWallGeometry`.
    pub fn compute_wall_geometry(
        positions: &[Cartesian3],
        texture_coordinates: Option<&PolygonTextureCoordinates>,
        ellipsoid: &Ellipsoid,
        granularity: f64,
        per_position_height: bool,
        arc_type: ArcType,
    ) -> Geometry {
        let mut index = 0;
        let mut texture_index = 0;

        let has_texcoords = texture_coordinates.is_some();
        let texcoords = texture_coordinates.map(|tc| &tc.positions);

        let top_edge_length;
        let mut edge_positions: Vec<f64>;
        let mut top_edge_texcoord_length = 0usize;
        let mut edge_texcoords: Vec<f64> = vec![];

        let length = positions.len();

        if !per_position_height {
            let min_distance =
                CesiumMath::chord_length(granularity, ellipsoid.maximum_radius());

            let mut num_vertices = 0;
            if arc_type == ArcType::Geodesic {
                for i in 0..length {
                    num_vertices += PolygonGeometryLibrary::subdivide_line_count(
                        &positions[i],
                        &positions[(i + 1) % length],
                        min_distance,
                    );
                }
            } else if arc_type == ArcType::Rhumb {
                for i in 0..length {
                    num_vertices += PolygonGeometryLibrary::subdivide_rhumb_line_count(
                        ellipsoid,
                        &positions[i],
                        &positions[(i + 1) % length],
                        min_distance,
                    );
                }
            }

            top_edge_length = (num_vertices + length) * 3;
            edge_positions = vec![0.0; top_edge_length * 2];

            if has_texcoords {
                top_edge_texcoord_length = (num_vertices + length) * 2;
                edge_texcoords = vec![0.0; top_edge_texcoord_length * 2];
            }

            for i in 0..length {
                let p1 = &positions[i];
                let p2 = &positions[(i + 1) % length];

                let t1 = if has_texcoords {
                    texcoords.unwrap()[i]
                } else {
                    Cartesian2::default()
                };
                let t2 = if has_texcoords {
                    texcoords.unwrap()[(i + 1) % length]
                } else {
                    Cartesian2::default()
                };

                let temp_positions: Vec<f64>;
                let mut temp_texcoords: Vec<f64> = vec![];

                if arc_type == ArcType::Geodesic {
                    temp_positions =
                        PolygonGeometryLibrary::subdivide_line(p1, p2, min_distance);
                    if has_texcoords {
                        temp_texcoords = PolygonGeometryLibrary::subdivide_texcoord_line(
                            &t1, &t2, p1, p2, min_distance,
                        );
                    }
                } else {
                    // DEVIATION: JS leaves `tempPositions` undefined when
                    // `arcType` is neither GEODESIC nor RHUMB; callers always
                    // pass RHUMB here otherwise.
                    temp_positions = PolygonGeometryLibrary::subdivide_rhumb_line(
                        ellipsoid, p1, p2, min_distance,
                    );
                    if has_texcoords {
                        temp_texcoords = PolygonGeometryLibrary::subdivide_texcoord_rhumb_line(
                            &t1, &t2, ellipsoid, p1, p2, min_distance,
                        );
                    }
                }

                for (j, value) in temp_positions.iter().enumerate() {
                    edge_positions[index + j] = *value;
                    edge_positions[index + j + top_edge_length] = *value;
                }
                index += temp_positions.len();

                edge_positions[index] = p2.x;
                edge_positions[index + top_edge_length] = p2.x;
                index += 1;

                edge_positions[index] = p2.y;
                edge_positions[index + top_edge_length] = p2.y;
                index += 1;

                edge_positions[index] = p2.z;
                edge_positions[index + top_edge_length] = p2.z;
                index += 1;

                if has_texcoords {
                    for (k, value) in temp_texcoords.iter().enumerate() {
                        edge_texcoords[texture_index + k] = *value;
                        edge_texcoords[texture_index + k + top_edge_texcoord_length] = *value;
                    }
                    texture_index += temp_texcoords.len();

                    edge_texcoords[texture_index] = t2.x;
                    edge_texcoords[texture_index + top_edge_texcoord_length] = t2.x;
                    texture_index += 1;

                    edge_texcoords[texture_index] = t2.y;
                    edge_texcoords[texture_index + top_edge_texcoord_length] = t2.y;
                    texture_index += 1;
                }
            }
        } else {
            top_edge_length = length * 3 * 2;
            edge_positions = vec![0.0; top_edge_length * 2];

            if has_texcoords {
                top_edge_texcoord_length = length * 2 * 2;
                edge_texcoords = vec![0.0; top_edge_texcoord_length * 2];
            }

            for i in 0..length {
                let p1 = &positions[i];
                let p2 = &positions[(i + 1) % length];
                edge_positions[index] = p1.x;
                edge_positions[index + top_edge_length] = p1.x;
                index += 1;
                edge_positions[index] = p1.y;
                edge_positions[index + top_edge_length] = p1.y;
                index += 1;
                edge_positions[index] = p1.z;
                edge_positions[index + top_edge_length] = p1.z;
                index += 1;
                edge_positions[index] = p2.x;
                edge_positions[index + top_edge_length] = p2.x;
                index += 1;
                edge_positions[index] = p2.y;
                edge_positions[index + top_edge_length] = p2.y;
                index += 1;
                edge_positions[index] = p2.z;
                edge_positions[index + top_edge_length] = p2.z;
                index += 1;

                if has_texcoords {
                    let t1 = texcoords.unwrap()[i];
                    let t2 = texcoords.unwrap()[(i + 1) % length];
                    edge_texcoords[texture_index] = t1.x;
                    edge_texcoords[texture_index + top_edge_texcoord_length] = t1.x;
                    texture_index += 1;
                    edge_texcoords[texture_index] = t1.y;
                    edge_texcoords[texture_index + top_edge_texcoord_length] = t1.y;
                    texture_index += 1;
                    edge_texcoords[texture_index] = t2.x;
                    edge_texcoords[texture_index + top_edge_texcoord_length] = t2.x;
                    texture_index += 1;
                    edge_texcoords[texture_index] = t2.y;
                    edge_texcoords[texture_index + top_edge_texcoord_length] = t2.y;
                    texture_index += 1;
                }
            }
        }

        let total_length = edge_positions.len();
        // DEVIATION: JS pre-allocates the index array with
        // `totalLength - positions.len() * 6` entries, leaving zero-filled
        // entries for skipped corners; `IndexStorage` has no random-write
        // API, so the Rust port only pushes the used indices.
        let mut indices = IndexDatatype::create_typed_array(total_length / 3, 0);
        let length = total_length / 6;

        for i in 0..length {
            let ul = i;
            let ur = ul + 1;
            let ll = ul + length;
            let lr = ll + 1;

            let mut p1 = Cartesian3::default();
            let mut p2 = Cartesian3::default();
            Cartesian3::from_array(&edge_positions, Some(ul * 3), &mut p1);
            Cartesian3::from_array(&edge_positions, Some(ur * 3), &mut p2);
            if Cartesian3::equals_epsilon(
                Some(&p1),
                Some(&p2),
                Some(CesiumMath::EPSILON10),
                Some(CesiumMath::EPSILON10),
            ) {
                // skip corner
                continue;
            }

            indices.push(ul as u32);
            indices.push(ll as u32);
            indices.push(ur as u32);
            indices.push(ur as u32);
            indices.push(ll as u32);
            indices.push(lr as u32);
        }

        let mut attributes = HashMap::new();
        attributes.insert(
            "position".to_string(),
            GeometryAttribute::new(ComponentDatatype::Double, 3, false, edge_positions),
        );

        if has_texcoords {
            attributes.insert(
                "st".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 2, false, edge_texcoords),
            );
        }

        Geometry::new(
            attributes,
            Some(indices),
            Some(PrimitiveType::Triangles),
            None,
        )
    }
}

impl Default for PolygonGeometryLibrary {
    fn default() -> Self {
        Self::new()
    }
}

/// Texture coordinates associated with a polygon (positions of the `st`
/// attribute as `Cartesian2`s).
#[derive(Debug, Clone, Default)]
pub struct PolygonTextureCoordinates {
    /// The texture coordinate positions.
    pub positions: Vec<Cartesian2>,
}

/// An entry of the `hierarchy` array returned by
/// [`PolygonGeometryLibrary::polygons_from_hierarchy`].
#[derive(Debug, Clone, Default)]
pub struct HierarchyResultEntry {
    /// The outer ring positions of the polygon.
    pub outer_ring: Vec<Cartesian3>,
    /// The positions of each hole in the polygon.
    pub holes: Vec<Vec<Cartesian3>>,
}

/// An entry of the `polygons` array returned by
/// [`PolygonGeometryLibrary::polygons_from_hierarchy`].
#[derive(Debug, Clone, Default)]
pub struct PolygonResultEntry {
    /// The positions of the polygon (outer ring followed by holes).
    pub positions: Vec<Cartesian3>,
    /// The 2D-projected positions of the polygon.
    pub positions_2d: Vec<Cartesian2>,
    /// Indices into `positions` where each hole begins.
    pub holes: Vec<usize>,
}

/// Result of [`PolygonGeometryLibrary::polygons_from_hierarchy`].
#[derive(Debug, Clone, Default)]
pub struct PolygonsFromHierarchyResult {
    /// The hierarchy of outer rings and holes.
    pub hierarchy: Vec<HierarchyResultEntry>,
    /// The flattened polygons with 2D positions and hole indices.
    pub polygons: Vec<PolygonResultEntry>,
}

/// An edge of a polygon which lies on the split plane, used by
/// `splitPolygonsOnEquator`.
#[derive(Debug, Clone)]
struct EdgeOnPlane {
    /// Index of the edge's start position in the positions array.
    position: usize,
    /// The sign of the z coordinate of the edge's start position.
    edge_type: f64,
    /// Whether the edge has already been visited during wiring.
    visited: bool,
    /// The sign of the z coordinate of the edge's end position.
    next: f64,
    /// The longitude of the edge's point on the split plane.
    theta: f64,
}

fn get_point_at_distance_2d(p0: &Cartesian2, p1: &Cartesian2, distance: f64, length: f64) -> (f64, f64) {
    let mut distance_2d = Cartesian2::subtract_new(p1, p0);
    distance_2d = Cartesian2::multiply_by_scalar_new(&distance_2d, distance / length);
    distance_2d = Cartesian2::add_new(p0, &distance_2d);
    (distance_2d.x, distance_2d.y)
}

fn get_point_at_distance(p0: &Cartesian3, p1: &Cartesian3, distance: f64, length: f64) -> (f64, f64, f64) {
    let mut distance_scratch = Cartesian3::subtract_new(p1, p0);
    distance_scratch = Cartesian3::multiply_by_scalar_new(&distance_scratch, distance / length);
    distance_scratch = Cartesian3::add_new(p0, &distance_scratch);
    (distance_scratch.x, distance_scratch.y, distance_scratch.z)
}

fn compute_equator_intersection_rhumb(
    start: &Cartesian3,
    end: &Cartesian3,
    ellipsoid: &Ellipsoid,
) -> Option<Cartesian3> {
    let mut c0 = Cartographic::default();
    let mut c1 = Cartographic::default();
    ellipsoid.cartesian_to_cartographic(start, &mut c0);
    ellipsoid.cartesian_to_cartographic(end, &mut c1);

    if CesiumMath::sign(c0.latitude) == CesiumMath::sign(c1.latitude) {
        return None;
    }

    let rhumb = EllipsoidRhumbLine::new(Some(c0), Some(c1), None, Some(*ellipsoid));

    let intersection = rhumb.find_intersection_with_latitude(0.0)?;

    let mut min_longitude = c0.longitude.min(c1.longitude);
    let mut max_longitude = c0.longitude.max(c1.longitude);

    if (max_longitude - min_longitude).abs() > CesiumMath::PI {
        // Crosses IDL, flip min and max
        let swap = min_longitude;
        min_longitude = max_longitude;
        max_longitude = swap;
    }

    if intersection.longitude < min_longitude || intersection.longitude > max_longitude {
        return None;
    }

    let mut result = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(&intersection, &mut result);
    Some(result)
}

fn compute_equator_intersection(
    start: &Cartesian3,
    end: &Cartesian3,
    ellipsoid: &Ellipsoid,
    arc_type: ArcType,
) -> Option<Cartesian3> {
    if arc_type == ArcType::Rhumb {
        return compute_equator_intersection_rhumb(start, end, ellipsoid);
    }

    let intersection =
        IntersectionTests::line_segment_plane(start, end, &Plane::ORIGIN_XY_PLANE)?;

    let mut scaled = Cartesian3::default();
    ellipsoid.scale_to_geodetic_surface(&intersection, &mut scaled);
    Some(scaled)
}

fn compute_edges_on_plane(
    positions: &mut Vec<Cartesian3>,
    ellipsoid: &Ellipsoid,
    arc_type: ArcType,
) -> Vec<EdgeOnPlane> {
    let mut edges_on_plane: Vec<EdgeOnPlane> = vec![];
    let mut i = 0;
    let get_longitude = |position: &Cartesian3| -> f64 {
        let mut cartographic = Cartographic::default();
        ellipsoid.cartesian_to_cartographic(position, &mut cartographic);
        cartographic.longitude
    };
    while i < positions.len() {
        let start_point = positions[i];
        let end_point = positions[(i + 1) % positions.len()];

        let edge_type = CesiumMath::sign(start_point.z);
        let next = CesiumMath::sign(end_point.z);

        if edge_type == 0.0 {
            // The start position is on the split
            edges_on_plane.push(EdgeOnPlane {
                position: i,
                edge_type,
                visited: false,
                next,
                theta: get_longitude(&start_point),
            });
        } else if next != 0.0 {
            let intersection =
                compute_equator_intersection(&start_point, &end_point, ellipsoid, arc_type);

            i += 1;
            let intersection = match intersection {
                Some(intersection) => intersection,
                // The line segment is entirely above or below
                None => continue,
            };

            // The line segment passed through
            positions.insert(i, intersection);
            edges_on_plane.push(EdgeOnPlane {
                position: i,
                edge_type,
                visited: false,
                next,
                theta: get_longitude(&intersection),
            });
        }

        i += 1;
    }

    edges_on_plane
}

fn wire_polygon(
    polygons: &mut Vec<Vec<Cartesian3>>,
    polygon_index: isize,
    positions: &[Cartesian3],
    edges_on_plane: &mut Vec<EdgeOnPlane>,
    to_delete: usize,
    start_index: isize,
    above_plane: bool,
) -> isize {
    let mut polygon: Vec<Cartesian3> = vec![];
    let mut i = start_index;
    let mut polygons_to_wire: Vec<isize> = vec![];
    loop {
        let position = positions[i as usize];
        polygon.push(position);

        let edge_index = edges_on_plane
            .iter()
            .position(|edge| edge.position as isize == i);
        let edge_index = match edge_index {
            Some(edge_index) => edge_index,
            None => {
                // The current segment does not intersect
                i += 1;
                continue;
            }
        };

        let has_been_visited = edges_on_plane[edge_index].visited;
        let edge_type = edges_on_plane[edge_index].edge_type;
        let next = edges_on_plane[edge_index].next;
        edges_on_plane[edge_index].visited = true;

        if edge_type == 0.0 {
            if next == 0.0 {
                // Special case where we'll need to backtrack along the edge
                let previous_edge_index =
                    edge_index as isize + if above_plane { -1 } else { 1 };
                let previous_edge_ok = previous_edge_index >= 0
                    && (previous_edge_index as usize) < edges_on_plane.len()
                    && edges_on_plane[previous_edge_index as usize].position as isize == i + 1;
                if previous_edge_ok {
                    edges_on_plane[previous_edge_index as usize].visited = true;
                } else {
                    i += 1;
                    continue;
                }
            }

            // Special case where 3 polygons meet
            if (!has_been_visited && above_plane && next > 0.0)
                || (start_index == i && !above_plane && next < 0.0)
            {
                i += 1;
                continue;
            }
        }

        let follow_edge = if above_plane { edge_type >= 0.0 } else { edge_type <= 0.0 };
        if !follow_edge {
            i += 1;
            continue;
        }

        if !has_been_visited {
            // Wire another polygon starting at this position on the other
            // side of the edge
            polygons_to_wire.push(i);
        }

        // Continue counter-clockwise to the next edge
        let next_edge_index = edge_index as isize + if above_plane { 1 } else { -1 };
        if next_edge_index < 0 || next_edge_index as usize >= edges_on_plane.len() {
            i += 1;
            continue;
        }

        i = edges_on_plane[next_edge_index as usize].position as isize;

        if !(i < positions.len() as isize && i >= 0 && i != start_index && (polygon.len() as isize) < positions.len() as isize) {
            break;
        }
    }

    polygons.splice(polygon_index as usize..polygon_index as usize + to_delete, [polygon]);

    let mut polygon_index = polygon_index;
    for index in polygons_to_wire {
        polygon_index = wire_polygon(
            polygons,
            polygon_index + 1,
            positions,
            edges_on_plane,
            0,
            index,
            !above_plane,
        );
    }

    polygon_index
}
