//! Ported from `packages/engine/Source/Core/PolygonPipeline.js`.
//!
//! Polygon processing utilities: 2D area/winding order, earcut triangulation,
//! geodesic and rhumb-line subdivision, and height scaling.
//!
//! DEVIATION: JS keeps shared edges in a string-keyed object
//! (`"${min} ${max}"`); the Rust port uses a `HashMap<(u32, u32), u32>` with
//! the normalized `(min, max)` tuple key — semantically identical.
//!
//! DEVIATION: JS `computeRhumbLineSubdivision` reuses three module-level
//! `EllipsoidRhumbLine` instances via `setEndPoints`; the Rust port constructs
//! a fresh instance per edge since the type is immutable after construction.

use std::collections::HashMap;

use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::component_datatype::ComponentDatatype;
use crate::developer_error::throw_developer_error;
use crate::ellipsoid::Ellipsoid;
use crate::ellipsoid_rhumb_line::EllipsoidRhumbLine;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::index_datatype::IndexDatatype;
use crate::math::CesiumMath;
use crate::primitive_type::PrimitiveType;
use crate::winding_order::WindingOrder;

/// Pipeline for processing polygon geometry.
pub struct PolygonPipeline {
    _private: (),
}

impl PolygonPipeline {
    /// Creates a new PolygonPipeline.
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Port of `PolygonPipeline.computeArea2D`.
    ///
    /// # Panics
    /// Debug builds panic with a `DeveloperError` when fewer than three
    /// positions are provided.
    pub fn compute_area_2d(positions: &[Cartesian2]) -> f64 {
        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            if positions.len() < 3 {
                throw_developer_error("At least three positions are required.");
            }
        }
        //>>includeEnd('debug');

        let length = positions.len();
        let mut area = 0.0;

        let mut i0 = length - 1;
        for i1 in 0..length {
            let v0 = &positions[i0];
            let v1 = &positions[i1];

            area += v0.x * v1.y - v1.x * v0.y;
            i0 = i1;
        }

        area * 0.5
    }

    /// Port of `PolygonPipeline.computeWindingOrder2D`.
    pub fn compute_winding_order_2d(positions: &[Cartesian2]) -> WindingOrder {
        let area = PolygonPipeline::compute_area_2d(positions);
        if area > 0.0 {
            WindingOrder::CounterClockwise
        } else {
            WindingOrder::Clockwise
        }
    }

    /// Port of `PolygonPipeline.triangulate`.
    ///
    /// Triangulates a polygon. `holes` is an array of the starting indices of
    /// the holes.
    pub fn triangulate(positions: &[Cartesian2], holes: Option<&[usize]>) -> Vec<usize> {
        //>>includeStart('debug', pragmas.debug);
        // DEVIATION: JS checks `positions` is defined; the Rust parameter is
        // non-optional.
        //>>includeEnd('debug');

        let flattened_positions = Cartesian2::pack_array(positions, None);
        // DEVIATION: `earcutr` returns a `Result`; JS `earcut` yields an empty
        // array for degenerate input, so `unwrap_or_default` mirrors that.
        earcutr::earcut(&flattened_positions, holes.unwrap_or(&[]), 2).unwrap_or_default()
    }

    /// Port of `PolygonPipeline.computeSubdivision`.
    ///
    /// Subdivides positions and raises points to the surface of the ellipsoid.
    ///
    /// # Panics
    /// Debug builds panic with a `DeveloperError` when the index count is
    /// invalid or the granularity is not positive.
    pub fn compute_subdivision(
        ellipsoid: &Ellipsoid,
        positions: &[Cartesian3],
        indices: &[u32],
        texcoords: Option<&[Cartesian2]>,
        granularity: Option<f64>,
    ) -> Geometry {
        let granularity = granularity.unwrap_or(CesiumMath::RADIANS_PER_DEGREE);

        let has_texcoords = texcoords.is_some();

        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            if indices.len() < 3 {
                throw_developer_error("At least three indices are required.");
            }
            if indices.len() % 3 != 0 {
                throw_developer_error("The number of indices must be divisable by three.");
            }
            if granularity <= 0.0 {
                throw_developer_error("granularity must be greater than zero.");
            }
        }
        //>>includeEnd('debug');

        // triangles that need (or might need) to be subdivided.
        let mut triangles: Vec<u32> = indices.to_vec();

        // New positions due to edge splits are appended to the positions list.
        let length = positions.len();
        let mut subdivided_positions: Vec<f64> = Vec::with_capacity(length * 3);
        let mut subdivided_texcoords: Vec<f64> = Vec::with_capacity(length * 2);
        for i in 0..length {
            let item = &positions[i];
            subdivided_positions.push(item.x);
            subdivided_positions.push(item.y);
            subdivided_positions.push(item.z);

            if let Some(texcoords) = texcoords {
                let texcoord_item = &texcoords[i];
                subdivided_texcoords.push(texcoord_item.x);
                subdivided_texcoords.push(texcoord_item.y);
            }
        }

        let mut subdivided_indices: Vec<u32> = Vec::new();

        // Used to make sure shared edges are not split more than once.
        let mut edges: HashMap<(u32, u32), u32> = HashMap::new();

        let radius = ellipsoid.maximum_radius();
        let min_distance = CesiumMath::chord_length(granularity, radius);
        let min_distance_sqrd = min_distance * min_distance;

        while !triangles.is_empty() {
            let i2 = triangles.pop().unwrap();
            let i1 = triangles.pop().unwrap();
            let i0 = triangles.pop().unwrap();

            let mut v0 = Cartesian3::default();
            Cartesian3::from_array(&subdivided_positions, Some(i0 as usize * 3), &mut v0);
            let mut v1 = Cartesian3::default();
            Cartesian3::from_array(&subdivided_positions, Some(i1 as usize * 3), &mut v1);
            let mut v2 = Cartesian3::default();
            Cartesian3::from_array(&subdivided_positions, Some(i2 as usize * 3), &mut v2);

            let (t0, t1, t2) = if has_texcoords {
                let mut t0 = Cartesian2::default();
                Cartesian2::from_array(&subdivided_texcoords, Some(i0 as usize * 2), &mut t0);
                let mut t1 = Cartesian2::default();
                Cartesian2::from_array(&subdivided_texcoords, Some(i1 as usize * 2), &mut t1);
                let mut t2 = Cartesian2::default();
                Cartesian2::from_array(&subdivided_texcoords, Some(i2 as usize * 2), &mut t2);
                (Some(t0), Some(t1), Some(t2))
            } else {
                (None, None, None)
            };

            let s0 =
                Cartesian3::multiply_by_scalar_new(&Cartesian3::normalize_new(&v0), radius);
            let s1 =
                Cartesian3::multiply_by_scalar_new(&Cartesian3::normalize_new(&v1), radius);
            let s2 =
                Cartesian3::multiply_by_scalar_new(&Cartesian3::normalize_new(&v2), radius);

            let g0 = Cartesian3::magnitude_squared(&Cartesian3::subtract_new(&s0, &s1));
            let g1 = Cartesian3::magnitude_squared(&Cartesian3::subtract_new(&s1, &s2));
            let g2 = Cartesian3::magnitude_squared(&Cartesian3::subtract_new(&s2, &s0));

            let max = g0.max(g1).max(g2);

            // if the max length squared of a triangle edge is greater than the
            // chord length of squared of the granularity, subdivide the triangle
            if max > min_distance_sqrd {
                if g0 == max {
                    let edge = (i0.min(i1), i0.max(i1));

                    let i = match edges.get(&edge) {
                        Some(&i) => i,
                        None => {
                            let mid = Cartesian3::multiply_by_scalar_new(
                                &Cartesian3::add_new(&v0, &v1),
                                0.5,
                            );
                            subdivided_positions.push(mid.x);
                            subdivided_positions.push(mid.y);
                            subdivided_positions.push(mid.z);
                            let i = (subdivided_positions.len() / 3 - 1) as u32;
                            edges.insert(edge, i);

                            if has_texcoords {
                                let mid_texcoord = Cartesian2::multiply_by_scalar_new(
                                    &Cartesian2::add_new(&t0.unwrap(), &t1.unwrap()),
                                    0.5,
                                );
                                subdivided_texcoords.push(mid_texcoord.x);
                                subdivided_texcoords.push(mid_texcoord.y);
                            }
                            i
                        }
                    };

                    triangles.extend_from_slice(&[i0, i, i2]);
                    triangles.extend_from_slice(&[i, i1, i2]);
                } else if g1 == max {
                    let edge = (i1.min(i2), i1.max(i2));

                    let i = match edges.get(&edge) {
                        Some(&i) => i,
                        None => {
                            let mid = Cartesian3::multiply_by_scalar_new(
                                &Cartesian3::add_new(&v1, &v2),
                                0.5,
                            );
                            subdivided_positions.push(mid.x);
                            subdivided_positions.push(mid.y);
                            subdivided_positions.push(mid.z);
                            let i = (subdivided_positions.len() / 3 - 1) as u32;
                            edges.insert(edge, i);

                            if has_texcoords {
                                let mid_texcoord = Cartesian2::multiply_by_scalar_new(
                                    &Cartesian2::add_new(&t1.unwrap(), &t2.unwrap()),
                                    0.5,
                                );
                                subdivided_texcoords.push(mid_texcoord.x);
                                subdivided_texcoords.push(mid_texcoord.y);
                            }
                            i
                        }
                    };

                    triangles.extend_from_slice(&[i1, i, i0]);
                    triangles.extend_from_slice(&[i, i2, i0]);
                } else if g2 == max {
                    let edge = (i2.min(i0), i2.max(i0));

                    let i = match edges.get(&edge) {
                        Some(&i) => i,
                        None => {
                            let mid = Cartesian3::multiply_by_scalar_new(
                                &Cartesian3::add_new(&v2, &v0),
                                0.5,
                            );
                            subdivided_positions.push(mid.x);
                            subdivided_positions.push(mid.y);
                            subdivided_positions.push(mid.z);
                            let i = (subdivided_positions.len() / 3 - 1) as u32;
                            edges.insert(edge, i);

                            if has_texcoords {
                                let mid_texcoord = Cartesian2::multiply_by_scalar_new(
                                    &Cartesian2::add_new(&t2.unwrap(), &t0.unwrap()),
                                    0.5,
                                );
                                subdivided_texcoords.push(mid_texcoord.x);
                                subdivided_texcoords.push(mid_texcoord.y);
                            }
                            i
                        }
                    };

                    triangles.extend_from_slice(&[i2, i, i1]);
                    triangles.extend_from_slice(&[i, i0, i1]);
                }
            } else {
                subdivided_indices.push(i0);
                subdivided_indices.push(i1);
                subdivided_indices.push(i2);
            }
        }

        let mut attributes: HashMap<String, GeometryAttribute> = HashMap::new();
        attributes.insert(
            "position".to_string(),
            GeometryAttribute::new(
                ComponentDatatype::Double,
                3,
                false,
                subdivided_positions.clone(),
            ),
        );

        if has_texcoords {
            attributes.insert(
                "st".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 2, false, subdivided_texcoords),
            );
        }

        let number_of_vertices = subdivided_positions.len() / 3;
        let mut index_storage = IndexDatatype::create_typed_array(number_of_vertices, 0);
        for index in &subdivided_indices {
            index_storage.push(*index);
        }

        Geometry::new(
            attributes,
            Some(index_storage),
            Some(PrimitiveType::Triangles),
            None,
        )
    }

    /// Port of `PolygonPipeline.computeRhumbLineSubdivision`.
    ///
    /// Subdivides positions on rhumb lines and raises points to the surface of
    /// the ellipsoid.
    ///
    /// # Panics
    /// Debug builds panic with a `DeveloperError` when the index count is
    /// invalid or the granularity is not positive.
    pub fn compute_rhumb_line_subdivision(
        ellipsoid: &Ellipsoid,
        positions: &[Cartesian3],
        indices: &[u32],
        texcoords: Option<&[Cartesian2]>,
        granularity: Option<f64>,
    ) -> Geometry {
        let granularity = granularity.unwrap_or(CesiumMath::RADIANS_PER_DEGREE);

        let has_texcoords = texcoords.is_some();

        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            if indices.len() < 3 {
                throw_developer_error("At least three indices are required.");
            }
            if indices.len() % 3 != 0 {
                throw_developer_error("The number of indices must be divisable by three.");
            }
            if granularity <= 0.0 {
                throw_developer_error("granularity must be greater than zero.");
            }
        }
        //>>includeEnd('debug');

        // triangles that need (or might need) to be subdivided.
        let mut triangles: Vec<u32> = indices.to_vec();

        // New positions due to edge splits are appended to the positions list.
        let length = positions.len();
        let mut subdivided_positions: Vec<f64> = Vec::with_capacity(length * 3);
        let mut subdivided_texcoords: Vec<f64> = Vec::with_capacity(length * 2);
        for i in 0..length {
            let item = &positions[i];
            subdivided_positions.push(item.x);
            subdivided_positions.push(item.y);
            subdivided_positions.push(item.z);

            if let Some(texcoords) = texcoords {
                let texcoord_item = &texcoords[i];
                subdivided_texcoords.push(texcoord_item.x);
                subdivided_texcoords.push(texcoord_item.y);
            }
        }

        let mut subdivided_indices: Vec<u32> = Vec::new();

        // Used to make sure shared edges are not split more than once.
        let mut edges: HashMap<(u32, u32), u32> = HashMap::new();

        let radius = ellipsoid.maximum_radius();
        let min_distance = CesiumMath::chord_length(granularity, radius);

        while !triangles.is_empty() {
            let i2 = triangles.pop().unwrap();
            let i1 = triangles.pop().unwrap();
            let i0 = triangles.pop().unwrap();

            let mut v0 = Cartesian3::default();
            Cartesian3::from_array(&subdivided_positions, Some(i0 as usize * 3), &mut v0);
            let mut v1 = Cartesian3::default();
            Cartesian3::from_array(&subdivided_positions, Some(i1 as usize * 3), &mut v1);
            let mut v2 = Cartesian3::default();
            Cartesian3::from_array(&subdivided_positions, Some(i2 as usize * 3), &mut v2);

            let (t0, t1, t2) = if has_texcoords {
                let mut t0 = Cartesian2::default();
                Cartesian2::from_array(&subdivided_texcoords, Some(i0 as usize * 2), &mut t0);
                let mut t1 = Cartesian2::default();
                Cartesian2::from_array(&subdivided_texcoords, Some(i1 as usize * 2), &mut t1);
                let mut t2 = Cartesian2::default();
                Cartesian2::from_array(&subdivided_texcoords, Some(i2 as usize * 2), &mut t2);
                (Some(t0), Some(t1), Some(t2))
            } else {
                (None, None, None)
            };

            let mut c0 = Cartographic::default();
            ellipsoid.cartesian_to_cartographic(&v0, &mut c0);
            let mut c1 = Cartographic::default();
            ellipsoid.cartesian_to_cartographic(&v1, &mut c1);
            let mut c2 = Cartographic::default();
            ellipsoid.cartesian_to_cartographic(&v2, &mut c2);

            let rhumb0 =
                EllipsoidRhumbLine::new(Some(c0), Some(c1), None, Some(ellipsoid.clone()));
            let g0 = rhumb0.rhumb_distance();
            let rhumb1 =
                EllipsoidRhumbLine::new(Some(c1), Some(c2), None, Some(ellipsoid.clone()));
            let g1 = rhumb1.rhumb_distance();
            let rhumb2 =
                EllipsoidRhumbLine::new(Some(c2), Some(c0), None, Some(ellipsoid.clone()));
            let g2 = rhumb2.rhumb_distance();

            let max = g0.max(g1).max(g2);

            // if the max length of a triangle edge is greater than the chord
            // length of the granularity, subdivide the triangle
            if max > min_distance {
                if g0 == max {
                    let edge = (i0.min(i1), i0.max(i1));

                    let i = match edges.get(&edge) {
                        Some(&i) => i,
                        None => {
                            let mid = rhumb0.interpolate_using_fraction(0.5);
                            let mid_height = (c0.height + c1.height) * 0.5;
                            let mut mid_cartesian3 = Cartesian3::default();
                            Cartesian3::from_radians(
                                mid.longitude,
                                mid.latitude,
                                Some(mid_height),
                                Some(ellipsoid.radii_squared()),
                                &mut mid_cartesian3,
                            );
                            subdivided_positions.push(mid_cartesian3.x);
                            subdivided_positions.push(mid_cartesian3.y);
                            subdivided_positions.push(mid_cartesian3.z);
                            let i = (subdivided_positions.len() / 3 - 1) as u32;
                            edges.insert(edge, i);

                            if has_texcoords {
                                let mid_texcoord = Cartesian2::multiply_by_scalar_new(
                                    &Cartesian2::add_new(&t0.unwrap(), &t1.unwrap()),
                                    0.5,
                                );
                                subdivided_texcoords.push(mid_texcoord.x);
                                subdivided_texcoords.push(mid_texcoord.y);
                            }
                            i
                        }
                    };

                    triangles.extend_from_slice(&[i0, i, i2]);
                    triangles.extend_from_slice(&[i, i1, i2]);
                } else if g1 == max {
                    let edge = (i1.min(i2), i1.max(i2));

                    let i = match edges.get(&edge) {
                        Some(&i) => i,
                        None => {
                            let mid = rhumb1.interpolate_using_fraction(0.5);
                            let mid_height = (c1.height + c2.height) * 0.5;
                            let mut mid_cartesian3 = Cartesian3::default();
                            Cartesian3::from_radians(
                                mid.longitude,
                                mid.latitude,
                                Some(mid_height),
                                Some(ellipsoid.radii_squared()),
                                &mut mid_cartesian3,
                            );
                            subdivided_positions.push(mid_cartesian3.x);
                            subdivided_positions.push(mid_cartesian3.y);
                            subdivided_positions.push(mid_cartesian3.z);
                            let i = (subdivided_positions.len() / 3 - 1) as u32;
                            edges.insert(edge, i);

                            if has_texcoords {
                                let mid_texcoord = Cartesian2::multiply_by_scalar_new(
                                    &Cartesian2::add_new(&t1.unwrap(), &t2.unwrap()),
                                    0.5,
                                );
                                subdivided_texcoords.push(mid_texcoord.x);
                                subdivided_texcoords.push(mid_texcoord.y);
                            }
                            i
                        }
                    };

                    triangles.extend_from_slice(&[i1, i, i0]);
                    triangles.extend_from_slice(&[i, i2, i0]);
                } else if g2 == max {
                    let edge = (i2.min(i0), i2.max(i0));

                    let i = match edges.get(&edge) {
                        Some(&i) => i,
                        None => {
                            let mid = rhumb2.interpolate_using_fraction(0.5);
                            let mid_height = (c2.height + c0.height) * 0.5;
                            let mut mid_cartesian3 = Cartesian3::default();
                            Cartesian3::from_radians(
                                mid.longitude,
                                mid.latitude,
                                Some(mid_height),
                                Some(ellipsoid.radii_squared()),
                                &mut mid_cartesian3,
                            );
                            subdivided_positions.push(mid_cartesian3.x);
                            subdivided_positions.push(mid_cartesian3.y);
                            subdivided_positions.push(mid_cartesian3.z);
                            let i = (subdivided_positions.len() / 3 - 1) as u32;
                            edges.insert(edge, i);

                            if has_texcoords {
                                let mid_texcoord = Cartesian2::multiply_by_scalar_new(
                                    &Cartesian2::add_new(&t2.unwrap(), &t0.unwrap()),
                                    0.5,
                                );
                                subdivided_texcoords.push(mid_texcoord.x);
                                subdivided_texcoords.push(mid_texcoord.y);
                            }
                            i
                        }
                    };

                    triangles.extend_from_slice(&[i2, i, i1]);
                    triangles.extend_from_slice(&[i, i0, i1]);
                }
            } else {
                subdivided_indices.push(i0);
                subdivided_indices.push(i1);
                subdivided_indices.push(i2);
            }
        }

        let mut attributes: HashMap<String, GeometryAttribute> = HashMap::new();
        attributes.insert(
            "position".to_string(),
            GeometryAttribute::new(
                ComponentDatatype::Double,
                3,
                false,
                subdivided_positions.clone(),
            ),
        );

        if has_texcoords {
            attributes.insert(
                "st".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 2, false, subdivided_texcoords),
            );
        }

        let number_of_vertices = subdivided_positions.len() / 3;
        let mut index_storage = IndexDatatype::create_typed_array(number_of_vertices, 0);
        for index in &subdivided_indices {
            index_storage.push(*index);
        }

        Geometry::new(
            attributes,
            Some(index_storage),
            Some(PrimitiveType::Triangles),
            None,
        )
    }

    /// Port of `PolygonPipeline.scaleToGeodeticHeight`.
    ///
    /// Scales each position of a geometry's position attribute to a height, in
    /// place.
    pub fn scale_to_geodetic_height(
        positions: Option<&mut Vec<f64>>,
        height: Option<f64>,
        ellipsoid: Option<&Ellipsoid>,
        scale_to_surface: Option<bool>,
    ) {
        let default_ellipsoid = Ellipsoid::WGS84;
        let ellipsoid = ellipsoid.unwrap_or(&default_ellipsoid);

        let height = height.unwrap_or(0.0);
        let scale_to_surface = scale_to_surface.unwrap_or(true);

        if let Some(positions) = positions {
            let length = positions.len();

            let mut i = 0;
            while i < length {
                let mut p = Cartesian3::default();
                Cartesian3::from_array(positions, Some(i), &mut p);

                if scale_to_surface {
                    let mut scaled = Cartesian3::default();
                    ellipsoid.scale_to_geodetic_surface(&p, &mut scaled);
                    p = scaled;
                }

                if height != 0.0 {
                    let mut n = Cartesian3::default();
                    ellipsoid.geodetic_surface_normal(&p, &mut n);

                    let n_scaled = Cartesian3::multiply_by_scalar_new(&n, height);
                    p = Cartesian3::add_new(&p, &n_scaled);
                }

                positions[i] = p.x;
                positions[i + 1] = p.y;
                positions[i + 2] = p.z;
                i += 3;
            }
        }
    }
}

impl Default for PolygonPipeline {
    fn default() -> Self {
        Self::new()
    }
}
