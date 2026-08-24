//! Ported from `packages/engine/Source/Core/SimplePolylineGeometry.js`.
//!
//! A description of a polyline modeled as a line strip; the first two positions
//! define a line segment, and each additional position defines a line segment
//! from the previous position.

pub use crate::arc_type::ArcType;

use std::collections::HashMap;

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::color::Color;
use crate::component_datatype::ComponentDatatype;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::index_datatype::{IndexDatatype, IndexStorage};
use crate::math::CesiumMath;
use crate::polyline_pipeline::{GenerateArcHeight, GenerateArcOptions, PolylinePipeline};
use crate::primitive_type::PrimitiveType;

/// A polyline described by a sequence of positions.
#[derive(Debug, Clone)]
pub struct SimplePolylineGeometry {
    positions: Vec<Cartesian3>,
    colors: Option<Vec<[f64; 4]>>,
    colors_per_vertex: bool,
    arc_type: ArcType,
    granularity: f64,
    ellipsoid: Ellipsoid,
}

impl SimplePolylineGeometry {
    /// Creates a new `SimplePolylineGeometry`.
    ///
    /// # Panics (debug)
    /// Mirrors the JS `DeveloperError` checks behind `debug_assertions`:
    /// at least two positions are required and `colors` must have a valid
    /// length.
    pub fn new(
        positions: Vec<Cartesian3>,
        colors: Option<Vec<[f64; 4]>>,
        colors_per_vertex: Option<bool>,
        arc_type: Option<ArcType>,
        granularity: Option<f64>,
        ellipsoid: Option<Ellipsoid>,
    ) -> Self {
        let colors_per_vertex = colors_per_vertex.unwrap_or(false);

        if cfg!(debug_assertions) {
            if positions.len() < 2 {
                crate::developer_error::throw_developer_error(
                    "At least two positions are required.",
                );
            }
            if let Some(c) = &colors {
                if (colors_per_vertex && c.len() < positions.len())
                    || (!colors_per_vertex && c.len() < positions.len() - 1)
                {
                    crate::developer_error::throw_developer_error(
                        "colors has an invalid length.",
                    );
                }
            }
        }

        Self {
            positions,
            colors,
            colors_per_vertex,
            arc_type: arc_type.unwrap_or(ArcType::Geodesic),
            granularity: granularity.unwrap_or(CesiumMath::RADIANS_PER_DEGREE),
            ellipsoid: ellipsoid.unwrap_or(Ellipsoid::WGS84.clone()),
        }
    }

    /// The positions.
    pub fn positions(&self) -> &[Cartesian3] {
        &self.positions
    }

    /// The colors.
    pub fn colors(&self) -> Option<&Vec<[f64; 4]>> {
        self.colors.as_ref()
    }

    /// Whether colors are per-vertex (JS `_colorsPerVertex`).
    pub fn colors_per_vertex(&self) -> bool {
        self.colors_per_vertex
    }

    /// The arc type (JS `_arcType`).
    pub fn arc_type(&self) -> ArcType {
        self.arc_type
    }

    /// The granularity (JS `_granularity`).
    pub fn granularity(&self) -> f64 {
        self.granularity
    }

    /// The ellipsoid (JS `_ellipsoid`).
    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }

    /// The number of elements used to pack the object into an array.
    ///
    /// DEVIATION: JS exposes `packedLength` as an instance property computed
    /// in the constructor; Rust computes it on demand (always equal).
    pub fn packed_length(&self) -> usize {
        let mut num_components = 1 + self.positions.len() * Cartesian3::PACKED_LENGTH;
        num_components += match &self.colors {
            Some(colors) => 1 + colors.len() * Color::PACKED_LENGTH,
            None => 1,
        };
        num_components + Ellipsoid::PACKED_LENGTH + 3
    }

    /// Stores this instance into `array` (JS `SimplePolylineGeometry.pack`).
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        let mut si = starting_index.unwrap_or(0);

        let length = self.positions.len();
        array[si] = length as f64;
        si += 1;

        for position in &self.positions {
            Cartesian3::pack(position, array, Some(si));
            si += Cartesian3::PACKED_LENGTH;
        }

        let colors_length = self.colors.as_ref().map_or(0, |c| c.len());
        array[si] = colors_length as f64;
        si += 1;

        if let Some(colors) = &self.colors {
            for color in colors {
                Color::pack(
                    &Color::new(color[0], color[1], color[2], color[3]),
                    array,
                    si,
                );
                si += Color::PACKED_LENGTH;
            }
        }

        Ellipsoid::pack(&self.ellipsoid, array, Some(si));
        si += Ellipsoid::PACKED_LENGTH;

        array[si] = if self.colors_per_vertex { 1.0 } else { 0.0 };
        si += 1;
        array[si] = self.arc_type as i32 as f64;
        si += 1;
        array[si] = self.granularity;
    }

    /// Retrieves an instance from a packed array (JS
    /// `SimplePolylineGeometry.unpack`).
    pub fn unpack(array: &[f64], starting_index: Option<usize>, result: Option<&mut Self>) -> Self {
        let mut si = starting_index.unwrap_or(0);

        let mut length = array[si] as usize;
        si += 1;
        let mut positions = Vec::with_capacity(length);
        for _ in 0..length {
            positions.push(Cartesian3::unpack_new(array, Some(si)));
            si += Cartesian3::PACKED_LENGTH;
        }

        length = array[si] as usize;
        si += 1;
        let mut colors: Option<Vec<[f64; 4]>> = if length > 0 {
            Some(Vec::with_capacity(length))
        } else {
            None
        };
        for _ in 0..length {
            let color = Color::unpack(array, si);
            if let Some(c) = &mut colors {
                c.push([color.red, color.green, color.blue, color.alpha]);
            }
            si += Color::PACKED_LENGTH;
        }

        let ellipsoid = Ellipsoid::unpack(array, Some(si));
        si += Ellipsoid::PACKED_LENGTH;

        let colors_per_vertex = array[si] == 1.0;
        si += 1;
        let arc_type_raw = array[si];
        si += 1;
        let granularity = array[si];

        let arc_type = match arc_type_raw as i32 {
            0 => ArcType::None,
            2 => ArcType::Rhumb,
            _ => ArcType::Geodesic,
        };

        match result {
            None => Self::new(
                positions,
                colors,
                Some(colors_per_vertex),
                Some(arc_type),
                Some(granularity),
                Some(ellipsoid),
            ),
            Some(r) => {
                // JS assigns the fields directly (no constructor re-validation).
                r.positions = positions;
                r.colors = colors;
                r.ellipsoid = ellipsoid;
                r.colors_per_vertex = colors_per_vertex;
                r.arc_type = arc_type;
                r.granularity = granularity;
                r.clone()
            }
        }
    }

    /// Computes the geometric representation of a simple polyline, including
    /// its vertices, indices, and a bounding sphere (JS
    /// `SimplePolylineGeometry.createGeometry`).
    pub fn create_geometry(&self) -> Option<Geometry> {
        let positions = &self.positions;
        let colors = self.colors.as_ref();
        let colors_per_vertex = self.colors_per_vertex;
        let arc_type = self.arc_type;
        let granularity = self.granularity;
        let ellipsoid = &self.ellipsoid;

        let min_distance = CesiumMath::chord_length(granularity, ellipsoid.maximum_radius());
        let per_segment_colors = colors.is_some() && !colors_per_vertex;

        let length = positions.len();

        let position_values: Vec<f64>;
        let mut color_values: Option<Vec<f64>> = None;

        if arc_type == ArcType::Geodesic || arc_type == ArcType::Rhumb {
            let (subdivision_size, is_geodesic) = if arc_type == ArcType::Geodesic {
                (
                    CesiumMath::chord_length(granularity, ellipsoid.maximum_radius()),
                    true,
                )
            } else {
                (granularity, false)
            };

            let heights = PolylinePipeline::extract_heights(positions, ellipsoid);

            let mut generate_arc_options = GenerateArcOptions::default();
            if is_geodesic {
                generate_arc_options.min_distance = Some(min_distance);
            } else {
                generate_arc_options.granularity = Some(granularity);
            }
            generate_arc_options.ellipsoid = Some(ellipsoid.clone());

            if per_segment_colors {
                let mut position_count = 0usize;
                for i in 0..length - 1 {
                    position_count += if is_geodesic {
                        PolylinePipeline::number_of_points(
                            &positions[i],
                            &positions[i + 1],
                            subdivision_size,
                        )
                    } else {
                        // DEVIATION: JS passes the Cartesian3 positions
                        // straight into `numberOfPointsRhumbLine` (which
                        // reads `longitude`/`latitude`), producing NaN and
                        // eventually a RangeError. We convert to
                        // Cartographic for a meaningful count instead.
                        let mut c0 = Cartographic::default();
                        let mut c1 = Cartographic::default();
                        ellipsoid.cartesian_to_cartographic(&positions[i], &mut c0);
                        ellipsoid.cartesian_to_cartographic(&positions[i + 1], &mut c1);
                        PolylinePipeline::number_of_points_rhumb_line(
                            &c0,
                            &c1,
                            subdivision_size,
                        )
                    } + 1;
                }

                let mut position_buf = vec![0.0f64; position_count * 3];
                let mut color_buf = vec![0.0f64; position_count * 4];

                let mut offset = 0usize;
                let mut ci = 0usize;
                for i in 0..length - 1 {
                    generate_arc_options.positions =
                        vec![positions[i], positions[i + 1]];
                    generate_arc_options.height = Some(GenerateArcHeight::Array(vec![
                        heights[i],
                        heights[i + 1],
                    ]));

                    let pos = if is_geodesic {
                        PolylinePipeline::generate_arc(Some(&generate_arc_options))
                    } else {
                        PolylinePipeline::generate_rhumb_arc(Some(&generate_arc_options))
                    };

                    if let Some(colors) = colors {
                        let seg_len = pos.len() / 3;
                        let color = &colors[i];
                        for _ in 0..seg_len {
                            color_buf[ci] = Color::float_to_byte(color[0]) as f64;
                            ci += 1;
                            color_buf[ci] = Color::float_to_byte(color[1]) as f64;
                            ci += 1;
                            color_buf[ci] = Color::float_to_byte(color[2]) as f64;
                            ci += 1;
                            color_buf[ci] = Color::float_to_byte(color[3]) as f64;
                            ci += 1;
                        }
                    }

                    position_buf[offset..offset + pos.len()].copy_from_slice(&pos);
                    offset += pos.len();
                }

                position_values = position_buf;
                color_values = Some(color_buf);
            } else {
                generate_arc_options.positions = positions.clone();
                generate_arc_options.height = Some(GenerateArcHeight::Array(heights));
                position_values = if is_geodesic {
                    PolylinePipeline::generate_arc(Some(&generate_arc_options))
                } else {
                    PolylinePipeline::generate_rhumb_arc(Some(&generate_arc_options))
                };

                if let Some(colors) = colors {
                    let mut color_buf = vec![0.0f64; (position_values.len() / 3) * 4];
                    let mut offset = 0usize;

                    for i in 0..length - 1 {
                        offset = interpolate_colors(
                            &positions[i],
                            &positions[i + 1],
                            &colors[i],
                            &colors[i + 1],
                            min_distance,
                            &mut color_buf,
                            offset,
                            is_geodesic,
                        );
                    }

                    let last_color = &colors[length - 1];
                    color_buf[offset] = Color::float_to_byte(last_color[0]) as f64;
                    offset += 1;
                    color_buf[offset] = Color::float_to_byte(last_color[1]) as f64;
                    offset += 1;
                    color_buf[offset] = Color::float_to_byte(last_color[2]) as f64;
                    offset += 1;
                    color_buf[offset] = Color::float_to_byte(last_color[3]) as f64;

                    color_values = Some(color_buf);
                }
            }
        } else {
            let number_of_positions = if per_segment_colors {
                length * 2 - 2
            } else {
                length
            };
            let mut position_buf = vec![0.0f64; number_of_positions * 3];
            let mut color_buf: Option<Vec<f64>> =
                colors.map(|_| vec![0.0f64; number_of_positions * 4]);

            let mut position_index = 0usize;
            let mut color_index = 0usize;

            for i in 0..length {
                let p = &positions[i];

                if per_segment_colors && i > 0 {
                    Cartesian3::pack(p, &mut position_buf, Some(position_index));
                    position_index += 3;

                    if let (Some(colors), Some(color_buf)) = (colors, &mut color_buf) {
                        let color = &colors[i - 1];
                        color_buf[color_index] = Color::float_to_byte(color[0]) as f64;
                        color_index += 1;
                        color_buf[color_index] = Color::float_to_byte(color[1]) as f64;
                        color_index += 1;
                        color_buf[color_index] = Color::float_to_byte(color[2]) as f64;
                        color_index += 1;
                        color_buf[color_index] = Color::float_to_byte(color[3]) as f64;
                        color_index += 1;
                    }
                }

                if per_segment_colors && i == length - 1 {
                    break;
                }

                Cartesian3::pack(p, &mut position_buf, Some(position_index));
                position_index += 3;

                if let (Some(colors), Some(color_buf)) = (colors, &mut color_buf) {
                    let color = &colors[i];
                    color_buf[color_index] = Color::float_to_byte(color[0]) as f64;
                    color_index += 1;
                    color_buf[color_index] = Color::float_to_byte(color[1]) as f64;
                    color_index += 1;
                    color_buf[color_index] = Color::float_to_byte(color[2]) as f64;
                    color_index += 1;
                    color_buf[color_index] = Color::float_to_byte(color[3]) as f64;
                    color_index += 1;
                }
            }

            position_values = position_buf;
            color_values = color_buf;
        }

        let mut attributes: HashMap<String, GeometryAttribute> = HashMap::new();
        attributes.insert(
            "position".to_string(),
            GeometryAttribute::new(ComponentDatatype::Double, 3, false, position_values.clone()),
        );

        if let Some(color_values) = &color_values {
            attributes.insert(
                "color".to_string(),
                GeometryAttribute::new(ComponentDatatype::UnsignedByte, 4, true, color_values.clone()),
            );
        }

        let number_of_positions = position_values.len() / 3;
        let number_of_indices = (number_of_positions - 1) * 2;
        let mut indices: IndexStorage =
            IndexDatatype::create_typed_array(number_of_positions, number_of_indices);

        for i in 0..number_of_positions - 1 {
            indices.push(i as u32);
            indices.push((i + 1) as u32);
        }

        Some(Geometry::with_all(
            attributes,
            Some(indices),
            Some(PrimitiveType::Lines),
            Some(BoundingSphere::from_points(positions, None)),
            crate::geometry_type::GeometryType::None,
            None,
            None,
        ))
    }
}

/// Mirrors the private JS `interpolateColors` helper.
///
/// DEVIATION: the JS function always counts points with
/// `PolylinePipeline.numberOfPoints` (geodesic chord length) regardless of
/// arc type; `is_geodesic` keeps the port faithful if callers ever diverge —
/// for `!is_geodesic` we mirror the same JS behavior (geodesic counting), so
/// both branches currently agree.
#[allow(clippy::too_many_arguments)]
fn interpolate_colors(
    p0: &Cartesian3,
    p1: &Cartesian3,
    color0: &[f64; 4],
    color1: &[f64; 4],
    min_distance: f64,
    array: &mut [f64],
    offset: usize,
    _is_geodesic: bool,
) -> usize {
    // JS always uses PolylinePipeline.numberOfPoints here.
    let num_points = PolylinePipeline::number_of_points(p0, p1, min_distance);

    let r0 = color0[0];
    let g0 = color0[1];
    let b0 = color0[2];
    let a0 = color0[3];

    let r1 = color1[0];
    let g1 = color1[1];
    let b1 = color1[2];
    let a1 = color1[3];

    let mut index = offset;
    if color_equals(color0, color1) {
        for _ in 0..num_points {
            array[index] = Color::float_to_byte(r0) as f64;
            index += 1;
            array[index] = Color::float_to_byte(g0) as f64;
            index += 1;
            array[index] = Color::float_to_byte(b0) as f64;
            index += 1;
            array[index] = Color::float_to_byte(a0) as f64;
            index += 1;
        }
        return index;
    }

    let red_per_vertex = (r1 - r0) / num_points as f64;
    let green_per_vertex = (g1 - g0) / num_points as f64;
    let blue_per_vertex = (b1 - b0) / num_points as f64;
    let alpha_per_vertex = (a1 - a0) / num_points as f64;

    for i in 0..num_points {
        let i = i as f64;
        array[index] = Color::float_to_byte(r0 + i * red_per_vertex) as f64;
        index += 1;
        array[index] = Color::float_to_byte(g0 + i * green_per_vertex) as f64;
        index += 1;
        array[index] = Color::float_to_byte(b0 + i * blue_per_vertex) as f64;
        index += 1;
        array[index] = Color::float_to_byte(a0 + i * alpha_per_vertex) as f64;
        index += 1;
    }

    index
}

/// Strict component equality, mirroring JS `Color.equals` used by
/// `interpolateColors`.
fn color_equals(left: &[f64; 4], right: &[f64; 4]) -> bool {
    left[0] == right[0] && left[1] == right[1] && left[2] == right[2] && left[3] == right[3]
}
