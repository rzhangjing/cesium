//! Ported from `packages/engine/Source/Core/PolylinePipeline.js`.
//!
//! Polyline subdivision utilities: arc generation over geodesics and rhumb
//! lines, height extraction, and the ±180° meridian wrap helper.
//!
//! DEVIATION: JS reuses module-level `EllipsoidGeodesic` / `EllipsoidRhumbLine`
//! instances via `setEndPoints`. The Rust port constructs a fresh instance per
//! segment call since those types are immutable after construction.
//!
//! DEVIATION: JS `options.height` accepts `number | number[]`; the Rust port
//! models this with the [`GenerateArcHeight`] enum.
//!
//! DEVIATION: the segment writers push into a growable `Vec<f64>` instead of
//! packing into a pre-sized array at absolute offsets; the produced values are
//! identical.

use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::ellipsoid_geodesic::EllipsoidGeodesic;
use crate::ellipsoid_rhumb_line::EllipsoidRhumbLine;
use crate::intersection_tests::IntersectionTests;
use crate::math::CesiumMath;
use crate::matrix4::Matrix4;
use crate::plane::Plane;

/// Pipeline for processing polyline geometry.
pub struct PolylinePipeline {
    _private: (),
}

/// Height input for arc generation: a single height applied to every position
/// or a per-position height array (mirrors JS `number | number[]`).
#[derive(Clone, Debug)]
pub enum GenerateArcHeight {
    Scalar(f64),
    Array(Vec<f64>),
}

/// Options for [`PolylinePipeline::generate_arc`] and friends.
#[derive(Clone, Default)]
pub struct GenerateArcOptions {
    /// The array of Cartesian3 positions (JS `options.positions`, required).
    pub positions: Vec<Cartesian3>,
    /// Heights of each position (JS `options.height`, default `0`).
    pub height: Option<GenerateArcHeight>,
    /// Distance, in radians, between each latitude and longitude
    /// (JS `options.granularity`, default `CesiumMath::RADIANS_PER_DEGREE`).
    pub granularity: Option<f64>,
    /// Minimum distance between points (JS `options.minDistance`); when
    /// undefined it is derived from the granularity.
    pub min_distance: Option<f64>,
    /// The ellipsoid on which the positions lie (JS `options.ellipsoid`,
    /// default `Ellipsoid.default`).
    pub ellipsoid: Option<Ellipsoid>,
}

/// Result of [`PolylinePipeline::wrap_longitude`].
#[derive(Clone, Debug, Default)]
pub struct WrapLongitudeResult {
    pub positions: Vec<Cartesian3>,
    pub lengths: Vec<usize>,
}

fn height_at(height: &GenerateArcHeight, index: usize) -> f64 {
    match height {
        GenerateArcHeight::Scalar(h) => *h,
        GenerateArcHeight::Array(a) => a[index],
    }
}

fn subdivide_heights(num_points: usize, h0: f64, h1: f64) -> Vec<f64> {
    let mut heights = vec![0.0; num_points];

    if h0 == h1 {
        for i in 0..num_points {
            heights[i] = h0;
        }
        return heights;
    }

    let d_height = h1 - h0;
    let height_per_vertex = d_height / num_points as f64;

    for i in 0..num_points {
        heights[i] = h0 + i as f64 * height_per_vertex;
    }

    heights
}

// Returns subdivided line scaled to ellipsoid surface starting at p0 and
// ending at p1. Result includes p0, but does not include p1. This function is
// called for a sequence of line segments, and this prevents duplication of the
// end point.
fn generate_cartesian_arc_segment(
    p0: &Cartesian3,
    p1: &Cartesian3,
    min_distance: f64,
    ellipsoid: &Ellipsoid,
    h0: f64,
    h1: f64,
    array: &mut Vec<f64>,
) {
    let mut first = Cartesian3::default();
    let mut last = Cartesian3::default();
    ellipsoid.scale_to_geodetic_surface(p0, &mut first);
    ellipsoid.scale_to_geodetic_surface(p1, &mut last);
    let num_points = PolylinePipeline::number_of_points(p0, p1, min_distance);
    let mut start = Cartographic::default();
    let mut end = Cartographic::default();
    ellipsoid.cartesian_to_cartographic(&first, &mut start);
    ellipsoid.cartesian_to_cartographic(&last, &mut end);
    let heights = subdivide_heights(num_points, h0, h1);

    let geodesic = EllipsoidGeodesic::new(
        Some(start),
        Some(end),
        None,
        None,
        Some(ellipsoid.clone()),
    );
    let surface_distance_between_points = geodesic.surface_distance() / num_points as f64;

    start.height = h0;
    let mut cart = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(&start, &mut cart);
    array.push(cart.x);
    array.push(cart.y);
    array.push(cart.z);

    for i in 1..num_points {
        let mut carto =
            geodesic.interpolate_using_surface_distance(i as f64 * surface_distance_between_points);
        carto.height = heights[i];
        ellipsoid.cartographic_to_cartesian(&carto, &mut cart);
        array.push(cart.x);
        array.push(cart.y);
        array.push(cart.z);
    }
}

// Returns subdivided line scaled to ellipsoid surface starting at p0 and
// ending at p1. Result includes p0, but does not include p1. This function is
// called for a sequence of line segments, and this prevents duplication of the
// end point.
fn generate_cartesian_rhumb_arc_segment(
    p0: &Cartesian3,
    p1: &Cartesian3,
    granularity: f64,
    ellipsoid: &Ellipsoid,
    h0: f64,
    h1: f64,
    array: &mut Vec<f64>,
) {
    let mut start = Cartographic::default();
    let mut end = Cartographic::default();
    ellipsoid.cartesian_to_cartographic(p0, &mut start);
    ellipsoid.cartesian_to_cartographic(p1, &mut end);
    let num_points = PolylinePipeline::number_of_points_rhumb_line(&start, &end, granularity);
    start.height = 0.0;
    end.height = 0.0;
    let heights = subdivide_heights(num_points, h0, h1);

    let rhumb = EllipsoidRhumbLine::new(Some(start), Some(end), None, Some(ellipsoid.clone()));
    let surface_distance_between_points = rhumb.rhumb_distance() / num_points as f64;

    start.height = h0;
    let mut cart = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(&start, &mut cart);
    array.push(cart.x);
    array.push(cart.y);
    array.push(cart.z);

    for i in 1..num_points {
        let mut carto = rhumb
            .interpolate_using_surface_distance(i as f64 * surface_distance_between_points);
        carto.height = heights[i];
        ellipsoid.cartographic_to_cartesian(&carto, &mut cart);
        array.push(cart.x);
        array.push(cart.y);
        array.push(cart.z);
    }
}

impl PolylinePipeline {
    /// Creates a new PolylinePipeline.
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Port of `PolylinePipeline.numberOfPoints`.
    pub fn number_of_points(p0: &Cartesian3, p1: &Cartesian3, min_distance: f64) -> usize {
        let distance = Cartesian3::distance(p0, p1);
        (distance / min_distance).ceil() as usize
    }

    /// Port of `PolylinePipeline.numberOfPointsRhumbLine`.
    pub fn number_of_points_rhumb_line(
        p0: &Cartographic,
        p1: &Cartographic,
        granularity: f64,
    ) -> usize {
        let radians_distance_squared = (p0.longitude - p1.longitude).powi(2)
            + (p0.latitude - p1.latitude).powi(2);

        1.max(
            ((radians_distance_squared / (granularity * granularity)).sqrt().ceil()) as usize,
        )
    }

    /// Port of `PolylinePipeline.extractHeights`.
    pub fn extract_heights(positions: &[Cartesian3], ellipsoid: &Ellipsoid) -> Vec<f64> {
        let length = positions.len();
        let mut heights = vec![0.0; length];
        for i in 0..length {
            let p = &positions[i];
            let mut carto = Cartographic::default();
            ellipsoid.cartesian_to_cartographic(p, &mut carto);
            heights[i] = carto.height;
        }
        heights
    }

    /// Port of `PolylinePipeline.wrapLongitude`.
    ///
    /// Breaks a polyline into segments such that it does not cross the ±180
    /// degree meridian of an ellipsoid.
    pub fn wrap_longitude(
        positions: Option<&[Cartesian3]>,
        model_matrix: Option<&Matrix4>,
    ) -> WrapLongitudeResult {
        let mut cartesians: Vec<Cartesian3> = Vec::new();
        let mut segments: Vec<usize> = Vec::new();

        if let Some(positions) = positions {
            if !positions.is_empty() {
                let model_matrix = model_matrix.unwrap_or(&Matrix4::IDENTITY);
                let mut inverse_model_matrix = Matrix4::default();
                Matrix4::inverse_transformation(model_matrix, &mut inverse_model_matrix);

                let mut origin = Cartesian3::default();
                Matrix4::multiply_by_point(
                    &inverse_model_matrix,
                    &Cartesian3::ZERO,
                    &mut origin,
                );
                let mut xz_normal = Cartesian3::default();
                Matrix4::multiply_by_point_as_vector(
                    &inverse_model_matrix,
                    &Cartesian3::UNIT_Y,
                    &mut xz_normal,
                );
                xz_normal = Cartesian3::normalize_new(&xz_normal);
                let xz_plane = Plane::from_point_normal_new(&origin, &xz_normal);
                let mut yz_normal = Cartesian3::default();
                Matrix4::multiply_by_point_as_vector(
                    &inverse_model_matrix,
                    &Cartesian3::UNIT_X,
                    &mut yz_normal,
                );
                yz_normal = Cartesian3::normalize_new(&yz_normal);
                let yz_plane = Plane::from_point_normal_new(&origin, &yz_normal);

                let mut count: usize = 1;
                cartesians.push(positions[0]);

                let mut prev = positions[0];
                let length = positions.len();
                for i in 1..length {
                    let cur = positions[i];

                    // intersects the IDL if either endpoint is on the negative
                    // side of the yz-plane
                    if Plane::get_point_distance(&yz_plane, &prev) < 0.0
                        || Plane::get_point_distance(&yz_plane, &cur) < 0.0
                    {
                        // and intersects the xz-plane
                        let intersection =
                            IntersectionTests::line_segment_plane(&prev, &cur, &xz_plane);
                        if let Some(intersection) = intersection {
                            // move point on the xz-plane slightly away from the plane
                            let mut offset =
                                Cartesian3::multiply_by_scalar_new(&xz_normal, 5.0e-9);
                            if Plane::get_point_distance(&xz_plane, &prev) < 0.0 {
                                offset = Cartesian3::negate_new(&offset);
                            }

                            cartesians.push(Cartesian3::add_new(&intersection, &offset));
                            segments.push(count + 1);

                            let negated_offset = Cartesian3::negate_new(&offset);
                            cartesians.push(Cartesian3::add_new(&intersection, &negated_offset));
                            count = 1;
                        }
                    }

                    cartesians.push(positions[i]);
                    count += 1;

                    prev = cur;
                }

                segments.push(count);
            }
        }

        WrapLongitudeResult {
            positions: cartesians,
            lengths: segments,
        }
    }

    /// Port of `PolylinePipeline.generateArc`.
    ///
    /// Subdivides the polyline and raises all points to the specified height.
    /// Returns an array of numbers to represent the positions.
    pub fn generate_arc(options: Option<&GenerateArcOptions>) -> Vec<f64> {
        let default_options = GenerateArcOptions::default();
        let options = options.unwrap_or(&default_options);
        let positions = &options.positions;
        //>>includeStart('debug', pragmas.debug);
        // DEVIATION: JS throws when `options.positions` is undefined; in the
        // Rust port positions is a non-optional field (always defined).
        //>>includeEnd('debug');

        let length = positions.len();
        let default_ellipsoid = Ellipsoid::WGS84;
        let ellipsoid = options.ellipsoid.as_ref().unwrap_or(&default_ellipsoid);
        let default_height = GenerateArcHeight::Scalar(0.0);
        let height = options.height.as_ref().unwrap_or(&default_height);

        if length < 1 {
            return Vec::new();
        } else if length == 1 {
            let mut p = Cartesian3::default();
            ellipsoid.scale_to_geodetic_surface(&positions[0], &mut p);
            let h = height_at(height, 0);
            if h != 0.0 {
                let mut n = Cartesian3::default();
                ellipsoid.geodetic_surface_normal(&p, &mut n);
                let n_scaled = Cartesian3::multiply_by_scalar_new(&n, h);
                p = Cartesian3::add_new(&p, &n_scaled);
            }

            return vec![p.x, p.y, p.z];
        }

        let min_distance = match options.min_distance {
            Some(d) => d,
            None => {
                let granularity = options
                    .granularity
                    .unwrap_or(CesiumMath::RADIANS_PER_DEGREE);
                CesiumMath::chord_length(granularity, ellipsoid.maximum_radius())
            }
        };

        let mut num_points: usize = 0;
        for i in 0..length - 1 {
            num_points +=
                PolylinePipeline::number_of_points(&positions[i], &positions[i + 1], min_distance);
        }

        let array_length = (num_points + 1) * 3;
        let mut new_positions: Vec<f64> = Vec::with_capacity(array_length);

        for i in 0..length - 1 {
            let p0 = &positions[i];
            let p1 = &positions[i + 1];

            let h0 = height_at(height, i);
            let h1 = height_at(height, i + 1);

            generate_cartesian_arc_segment(p0, p1, min_distance, ellipsoid, h0, h1, &mut new_positions);
        }

        let last_point = &positions[length - 1];
        let mut carto = Cartographic::default();
        ellipsoid.cartesian_to_cartographic(last_point, &mut carto);
        carto.height = height_at(height, length - 1);
        let mut cart = Cartesian3::default();
        ellipsoid.cartographic_to_cartesian(&carto, &mut cart);
        new_positions.push(cart.x);
        new_positions.push(cart.y);
        new_positions.push(cart.z);

        new_positions
    }

    /// Port of `PolylinePipeline.generateRhumbArc`.
    ///
    /// Subdivides the polyline and raises all points to the specified height
    /// using rhumb lines. Returns an array of numbers to represent the
    /// positions.
    pub fn generate_rhumb_arc(options: Option<&GenerateArcOptions>) -> Vec<f64> {
        let default_options = GenerateArcOptions::default();
        let options = options.unwrap_or(&default_options);
        let positions = &options.positions;
        //>>includeStart('debug', pragmas.debug);
        // DEVIATION: JS throws when `options.positions` is undefined; in the
        // Rust port positions is a non-optional field (always defined).
        //>>includeEnd('debug');

        let length = positions.len();
        let default_ellipsoid = Ellipsoid::WGS84;
        let ellipsoid = options.ellipsoid.as_ref().unwrap_or(&default_ellipsoid);
        let default_height = GenerateArcHeight::Scalar(0.0);
        let height = options.height.as_ref().unwrap_or(&default_height);

        if length < 1 {
            return Vec::new();
        } else if length == 1 {
            let mut p = Cartesian3::default();
            ellipsoid.scale_to_geodetic_surface(&positions[0], &mut p);
            let h = height_at(height, 0);
            if h != 0.0 {
                let mut n = Cartesian3::default();
                ellipsoid.geodetic_surface_normal(&p, &mut n);
                let n_scaled = Cartesian3::multiply_by_scalar_new(&n, h);
                p = Cartesian3::add_new(&p, &n_scaled);
            }

            return vec![p.x, p.y, p.z];
        }

        let granularity = options
            .granularity
            .unwrap_or(CesiumMath::RADIANS_PER_DEGREE);

        let mut num_points: usize = 0;

        let mut c0 = Cartographic::default();
        ellipsoid.cartesian_to_cartographic(&positions[0], &mut c0);
        let mut c1 = Cartographic::default();
        for i in 0..length - 1 {
            ellipsoid.cartesian_to_cartographic(&positions[i + 1], &mut c1);
            num_points += PolylinePipeline::number_of_points_rhumb_line(&c0, &c1, granularity);
            c0 = c1;
        }

        let array_length = (num_points + 1) * 3;
        let mut new_positions: Vec<f64> = Vec::with_capacity(array_length);

        for i in 0..length - 1 {
            let p0 = &positions[i];
            let p1 = &positions[i + 1];

            let h0 = height_at(height, i);
            let h1 = height_at(height, i + 1);

            generate_cartesian_rhumb_arc_segment(
                p0,
                p1,
                granularity,
                ellipsoid,
                h0,
                h1,
                &mut new_positions,
            );
        }

        let last_point = &positions[length - 1];
        let mut carto = Cartographic::default();
        ellipsoid.cartesian_to_cartographic(last_point, &mut carto);
        carto.height = height_at(height, length - 1);
        let mut cart = Cartesian3::default();
        ellipsoid.cartographic_to_cartesian(&carto, &mut cart);
        new_positions.push(cart.x);
        new_positions.push(cart.y);
        new_positions.push(cart.z);

        new_positions
    }

    /// Port of `PolylinePipeline.generateCartesianArc`.
    ///
    /// Subdivides the polyline and raises all points to the specified height.
    /// Returns an array of new Cartesian3 positions.
    pub fn generate_cartesian_arc(options: Option<&GenerateArcOptions>) -> Vec<Cartesian3> {
        let number_array = PolylinePipeline::generate_arc(options);
        let size = number_array.len() / 3;
        let mut new_positions = Vec::with_capacity(size);
        for i in 0..size {
            let mut c = Cartesian3::default();
            Cartesian3::unpack(&number_array, Some(i * 3), &mut c);
            new_positions.push(c);
        }
        new_positions
    }

    /// Port of `PolylinePipeline.generateCartesianRhumbArc`.
    ///
    /// Subdivides the polyline and raises all points to the specified height
    /// using rhumb lines. Returns an array of new Cartesian3 positions.
    pub fn generate_cartesian_rhumb_arc(options: Option<&GenerateArcOptions>) -> Vec<Cartesian3> {
        let number_array = PolylinePipeline::generate_rhumb_arc(options);
        let size = number_array.len() / 3;
        let mut new_positions = Vec::with_capacity(size);
        for i in 0..size {
            let mut c = Cartesian3::default();
            Cartesian3::unpack(&number_array, Some(i * 3), &mut c);
            new_positions.push(c);
        }
        new_positions
    }
}

impl Default for PolylinePipeline {
    fn default() -> Self {
        Self::new()
    }
}
