//! Ported from `packages/engine/Source/Core/PolylineGeometry.js`.
//!
//! A description of a polyline modeled as a line strip; the first two
//! positions define a line segment, and each additional position defines a
//! line segment from the previous position. The polyline is capable of
//! displaying with a material.

use std::collections::HashMap;

use crate::arc_type::ArcType;
use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::color::Color;
use crate::component_datatype::ComponentDatatype;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_type::GeometryType;
use crate::index_datatype::{IndexDatatype, IndexStorage};
use crate::math::CesiumMath;
use crate::polyline_pipeline::{GenerateArcHeight, GenerateArcOptions, PolylinePipeline};
use crate::primitive_type::PrimitiveType;
use crate::vertex_format::VertexFormat;

/// Mirrors JS `interpolateColors`.
fn interpolate_colors(color0: &Color, color1: &Color, num_points: usize) -> Vec<Color> {
    let mut colors = Vec::with_capacity(num_points);

    let r0 = color0.red;
    let g0 = color0.green;
    let b0 = color0.blue;
    let a0 = color0.alpha;

    let r1 = color1.red;
    let g1 = color1.green;
    let b1 = color1.blue;
    let a1 = color1.alpha;

    if Color::equals(color0, color1) {
        for _ in 0..num_points {
            colors.push(*color0);
        }
        return colors;
    }

    let red_per_vertex = (r1 - r0) / num_points as f64;
    let green_per_vertex = (g1 - g0) / num_points as f64;
    let blue_per_vertex = (b1 - b0) / num_points as f64;
    let alpha_per_vertex = (a1 - a0) / num_points as f64;

    for i in 0..num_points {
        let i = i as f64;
        colors.push(Color {
            red: r0 + i * red_per_vertex,
            green: g0 + i * green_per_vertex,
            blue: b0 + i * blue_per_vertex,
            alpha: a0 + i * alpha_per_vertex,
        });
    }

    colors
}

/// A description of a polyline modeled as a line strip.
///
/// DEVIATION: JS `packedLength` is an instance property computed in the
/// constructor; Rust exposes it as `packed_length(&self)`.
#[derive(Debug, Clone)]
pub struct PolylineGeometry {
    positions: Vec<Cartesian3>,
    colors: Option<Vec<Color>>,
    width: f64,
    colors_per_vertex: bool,
    vertex_format: VertexFormat,
    arc_type: ArcType,
    granularity: f64,
    ellipsoid: Ellipsoid,
}

impl PolylineGeometry {
    /// Creates a new `PolylineGeometry`.
    ///
    /// Retained for spec compatibility; the JS constructor takes an options
    /// object (see [`PolylineGeometry::from_options`]). Colors are given as
    /// `[red, green, blue, alpha]` arrays.
    pub fn new(
        positions: Vec<Cartesian3>,
        width: Option<f64>,
        colors: Option<Vec<[f64; 4]>>,
        colors_per_vertex: Option<bool>,
        arc_type: Option<ArcType>,
        granularity: Option<f64>,
        ellipsoid: Option<Ellipsoid>,
    ) -> Self {
        let colors = colors.map(|colors| {
            colors
                .into_iter()
                .map(|c| Color {
                    red: c[0],
                    green: c[1],
                    blue: c[2],
                    alpha: c[3],
                })
                .collect()
        });
        Self::from_options(
            positions,
            width,
            colors,
            colors_per_vertex,
            None,
            arc_type,
            granularity,
            ellipsoid,
        )
    }

    /// JS constructor equivalent: `new PolylineGeometry(options)`.
    pub fn from_options(
        positions: Vec<Cartesian3>,
        width: Option<f64>,
        colors: Option<Vec<Color>>,
        colors_per_vertex: Option<bool>,
        vertex_format: Option<VertexFormat>,
        arc_type: Option<ArcType>,
        granularity: Option<f64>,
        ellipsoid: Option<Ellipsoid>,
    ) -> Self {
        let width = width.unwrap_or(1.0);
        let colors_per_vertex = colors_per_vertex.unwrap_or(false);

        if cfg!(debug_assertions) {
            assert!(
                positions.len() >= 2,
                "At least two positions are required."
            );
            if let Some(colors) = &colors {
                assert!(
                    !((colors_per_vertex && colors.len() < positions.len())
                        || (!colors_per_vertex && colors.len() < positions.len() - 1)),
                    "colors has an invalid length."
                );
            }
        }

        Self {
            positions,
            colors,
            width,
            colors_per_vertex,
            vertex_format: vertex_format.unwrap_or_else(VertexFormat::default_format),
            arc_type: arc_type.unwrap_or(ArcType::Geodesic),
            granularity: granularity.unwrap_or(CesiumMath::RADIANS_PER_DEGREE),
            ellipsoid: ellipsoid.unwrap_or(Ellipsoid::WGS84),
        }
    }

    /// Accessors.
    pub fn positions(&self) -> &[Cartesian3] {
        &self.positions
    }

    pub fn width(&self) -> f64 {
        self.width
    }

    pub fn arc_type(&self) -> ArcType {
        self.arc_type
    }

    pub fn granularity(&self) -> f64 {
        self.granularity
    }

    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }

    pub fn vertex_format(&self) -> &VertexFormat {
        &self.vertex_format
    }

    /// The number of elements used to pack the object into an array.
    pub fn packed_length(&self) -> usize {
        let mut num_components = 1 + self.positions.len() * Cartesian3::PACKED_LENGTH;
        num_components += match &self.colors {
            Some(colors) => 1 + colors.len() * Color::PACKED_LENGTH,
            None => 1,
        };

        num_components
            + Ellipsoid::PACKED_LENGTH
            + VertexFormat::PACKED_LENGTH
            + 4
    }

    /// Stores the provided instance into the provided array.
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        let mut si = starting_index.unwrap_or(0);

        let positions = &self.positions;
        array[si] = positions.len() as f64;
        si += 1;

        for position in positions {
            Cartesian3::pack(position, array, Some(si));
            si += Cartesian3::PACKED_LENGTH;
        }

        let colors = &self.colors;
        let length = colors.as_ref().map(|c| c.len()).unwrap_or(0);
        array[si] = length as f64;
        si += 1;

        if let Some(colors) = colors {
            for color in colors {
                Color::pack(color, array, si);
                si += Color::PACKED_LENGTH;
            }
        }

        Ellipsoid::pack(&self.ellipsoid, array, Some(si));
        si += Ellipsoid::PACKED_LENGTH;

        self.vertex_format.pack(array, si);
        si += VertexFormat::PACKED_LENGTH;

        array[si] = self.width;
        si += 1;
        array[si] = if self.colors_per_vertex { 1.0 } else { 0.0 };
        si += 1;
        array[si] = self.arc_type as i32 as f64;
        si += 1;
        array[si] = self.granularity;
    }

    /// Retrieves an instance from a packed array.
    pub fn unpack(array: &[f64], starting_index: Option<usize>, result: Option<&mut Self>) -> Self {
        let mut si = starting_index.unwrap_or(0);

        let length = array[si] as usize;
        si += 1;
        let mut positions = Vec::with_capacity(length);
        for _ in 0..length {
            positions.push(Cartesian3::unpack_new(array, Some(si)));
            si += Cartesian3::PACKED_LENGTH;
        }

        let length = array[si] as usize;
        si += 1;
        let mut colors: Option<Vec<Color>> = if length > 0 {
            Some(Vec::with_capacity(length))
        } else {
            None
        };
        for _ in 0..length {
            if let Some(colors) = &mut colors {
                colors.push(Color::unpack(array, si));
            }
            si += Color::PACKED_LENGTH;
        }

        let ellipsoid = Ellipsoid::unpack(array, Some(si));
        si += Ellipsoid::PACKED_LENGTH;

        let vertex_format = VertexFormat::unpack(array, si, None);
        si += VertexFormat::PACKED_LENGTH;

        let width = array[si];
        si += 1;
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
            None => Self {
                positions,
                colors,
                width,
                colors_per_vertex,
                vertex_format,
                arc_type,
                granularity,
                ellipsoid,
            },
            Some(r) => {
                r.positions = positions;
                r.colors = colors;
                r.ellipsoid = ellipsoid;
                r.vertex_format = vertex_format;
                r.width = width;
                r.colors_per_vertex = colors_per_vertex;
                r.arc_type = arc_type;
                r.granularity = granularity;
                r.clone()
            }
        }
    }

    /// Computes the geometric representation of a polyline, including its
    /// vertices, indices, and a bounding sphere.
    pub fn create_geometry(polyline_geometry: &Self) -> Option<Geometry> {
        let width = polyline_geometry.width;
        let vertex_format = &polyline_geometry.vertex_format;
        let mut colors = polyline_geometry.colors.clone();
        let colors_per_vertex = polyline_geometry.colors_per_vertex;
        let arc_type = polyline_geometry.arc_type;
        let granularity = polyline_geometry.granularity;
        let ellipsoid = &polyline_geometry.ellipsoid;

        let mut removed_indices: Vec<usize> = Vec::new();
        let mut positions = crate::array_remove_duplicates::array_remove_duplicates(
            &polyline_geometry.positions,
            |a: &Cartesian3, b: &Cartesian3, eps: f64| {
                Cartesian3::equals_epsilon(Some(a), Some(b), Some(eps), None)
            },
            false,
            Some(&mut removed_indices),
        )
        .unwrap_or_else(|| polyline_geometry.positions.clone());

        if let Some(c) = &mut colors {
            if !removed_indices.is_empty() {
                let mut removed_array_index = 0usize;
                let mut next_removed_index = removed_indices[0];
                let mut filtered = Vec::with_capacity(c.len());
                for (index, color) in c.iter().enumerate() {
                    let remove = if colors_per_vertex {
                        index == next_removed_index || (index == 0 && next_removed_index == 1)
                    } else {
                        index + 1 == next_removed_index
                    };

                    if remove {
                        removed_array_index += 1;
                        if removed_array_index < removed_indices.len() {
                            next_removed_index = removed_indices[removed_array_index];
                        }
                    } else {
                        filtered.push(*color);
                    }
                }
                *c = filtered;
            }
        }

        let mut positions_length = positions.len();

        // A width of a pixel or less is not a valid geometry, but in order to
        // support external data that may have errors we treat this as an
        // empty geometry.
        if positions_length < 2 || width <= 0.0 {
            return None;
        }

        if arc_type == ArcType::Geodesic || arc_type == ArcType::Rhumb {
            let subdivision_size;
            if arc_type == ArcType::Geodesic {
                subdivision_size =
                    CesiumMath::chord_length(granularity, ellipsoid.maximum_radius());
            } else {
                subdivision_size = granularity;
            }

            let heights = PolylinePipeline::extract_heights(&positions, ellipsoid);

            if let Some(c) = &mut colors {
                // Number of points per segment; for RHUMB the JS passes
                // Cartesian3 into `numberOfPointsRhumbLine` (a bug yielding
                // NaN). DEVIATION: convert to Cartographic for a meaningful
                // count, consistent with `SimplePolylineGeometry`.
                let num_points_for = |p0: &Cartesian3, p1: &Cartesian3| -> usize {
                    if arc_type == ArcType::Geodesic {
                        PolylinePipeline::number_of_points(p0, p1, subdivision_size)
                    } else {
                        let mut c0 = Cartographic::default();
                        let mut c1 = Cartographic::default();
                        ellipsoid.cartesian_to_cartographic(p0, &mut c0);
                        ellipsoid.cartesian_to_cartographic(p1, &mut c1);
                        PolylinePipeline::number_of_points_rhumb_line(&c0, &c1, subdivision_size)
                    }
                };

                let mut color_length = 1usize;
                for i in 0..positions_length - 1 {
                    color_length += num_points_for(&positions[i], &positions[i + 1]);
                }

                let mut new_colors: Vec<Color> = Vec::with_capacity(color_length);

                for i in 0..positions_length - 1 {
                    let p0 = positions[i];
                    let p1 = positions[i + 1];
                    let c0 = c[i];

                    let num_colors = num_points_for(&p0, &p1);
                    if colors_per_vertex && i < color_length {
                        let c1 = c[i + 1];
                        let interpolated = interpolate_colors(&c0, &c1, num_colors);
                        new_colors.extend_from_slice(&interpolated);
                    } else {
                        for _ in 0..num_colors {
                            new_colors.push(c0);
                        }
                    }
                }

                new_colors.push(*c.last().unwrap());
                *c = new_colors;
            }

            if arc_type == ArcType::Geodesic {
                positions = PolylinePipeline::generate_cartesian_arc(Some(&GenerateArcOptions {
                    positions,
                    min_distance: Some(subdivision_size),
                    ellipsoid: Some(ellipsoid.clone()),
                    height: Some(GenerateArcHeight::Array(heights)),
                    ..Default::default()
                }));
            } else {
                positions = PolylinePipeline::generate_cartesian_rhumb_arc(Some(
                    &GenerateArcOptions {
                        positions,
                        granularity: Some(subdivision_size),
                        ellipsoid: Some(ellipsoid.clone()),
                        height: Some(GenerateArcHeight::Array(heights)),
                        ..Default::default()
                    },
                ));
            }
        }

        positions_length = positions.len();
        let size = positions_length * 4 - 4;

        let mut final_positions = vec![0.0f64; size * 3];
        let mut prev_positions = vec![0.0f64; size * 3];
        let mut next_positions = vec![0.0f64; size * 3];
        let mut expand_and_width = vec![0.0f64; size * 2];
        let mut st: Option<Vec<f64>> = if vertex_format.st {
            Some(vec![0.0f64; size * 2])
        } else {
            None
        };
        let mut final_colors: Option<Vec<f64>> = if colors.is_some() {
            Some(vec![0.0f64; size * 4])
        } else {
            None
        };

        let mut position_index = 0usize;
        let mut expand_and_width_index = 0usize;
        let mut st_index = 0usize;
        let mut color_index = 0usize;

        for j in 0..positions_length {
            let prev_position = if j == 0 {
                let mut tmp = Cartesian3::default();
                Cartesian3::subtract(&positions[0], &positions[1], &mut tmp);
                let mut out = Cartesian3::default();
                Cartesian3::add(&positions[0], &tmp, &mut out);
                out
            } else {
                positions[j - 1]
            };

            let scratch_position = positions[j];

            let next_position = if j == positions_length - 1 {
                let mut tmp = Cartesian3::default();
                Cartesian3::subtract(
                    &positions[positions_length - 1],
                    &positions[positions_length - 2],
                    &mut tmp,
                );
                let mut out = Cartesian3::default();
                Cartesian3::add(&positions[positions_length - 1], &tmp, &mut out);
                out
            } else {
                positions[j + 1]
            };

            let (color0, color1) = if final_colors.is_some() {
                let c = colors.as_ref().unwrap();
                let c0 = if j != 0 && !colors_per_vertex {
                    c[j - 1]
                } else {
                    c[j]
                };
                let c1 = if j != positions_length - 1 {
                    Some(c[j])
                } else {
                    None
                };
                (c0, c1)
            } else {
                (Color::default(), None)
            };

            let start_k = if j == 0 { 2 } else { 0 };
            let end_k = if j == positions_length - 1 { 2 } else { 4 };

            for k in start_k..end_k {
                Cartesian3::pack(&scratch_position, &mut final_positions, Some(position_index));
                Cartesian3::pack(&prev_position, &mut prev_positions, Some(position_index));
                Cartesian3::pack(&next_position, &mut next_positions, Some(position_index));
                position_index += 3;

                let direction = if (k as i32 - 2) < 0 { -1.0 } else { 1.0 };
                expand_and_width[expand_and_width_index] = 2.0 * (k % 2) as f64 - 1.0; // expand direction
                expand_and_width_index += 1;
                expand_and_width[expand_and_width_index] = direction * width;
                expand_and_width_index += 1;

                if let Some(st) = &mut st {
                    st[st_index] = j as f64 / (positions_length - 1) as f64;
                    st_index += 1;
                    st[st_index] = expand_and_width[expand_and_width_index - 2].max(0.0);
                    st_index += 1;
                }

                if let Some(final_colors) = &mut final_colors {
                    let color = if k < 2 { color0 } else { color1.unwrap_or(color0) };

                    final_colors[color_index] = Color::float_to_byte(color.red) as f64;
                    color_index += 1;
                    final_colors[color_index] = Color::float_to_byte(color.green) as f64;
                    color_index += 1;
                    final_colors[color_index] = Color::float_to_byte(color.blue) as f64;
                    color_index += 1;
                    final_colors[color_index] = Color::float_to_byte(color.alpha) as f64;
                    color_index += 1;
                }
            }
        }

        let mut attributes = HashMap::new();
        attributes.insert(
            "position".to_string(),
            GeometryAttribute::new(ComponentDatatype::Double, 3, false, final_positions),
        );
        attributes.insert(
            "prevPosition".to_string(),
            GeometryAttribute::new(ComponentDatatype::Double, 3, false, prev_positions),
        );
        attributes.insert(
            "nextPosition".to_string(),
            GeometryAttribute::new(ComponentDatatype::Double, 3, false, next_positions),
        );
        attributes.insert(
            "expandAndWidth".to_string(),
            GeometryAttribute::new(ComponentDatatype::Float, 2, false, expand_and_width),
        );

        if let Some(st) = st {
            attributes.insert(
                "st".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 2, false, st),
            );
        }

        if let Some(final_colors) = final_colors {
            attributes.insert(
                "color".to_string(),
                GeometryAttribute::new(ComponentDatatype::UnsignedByte, 4, true, final_colors),
            );
        }

        let mut indices: IndexStorage =
            IndexDatatype::create_typed_array(size, positions_length * 6 - 6);
        let mut index = 0usize;
        for _ in 0..positions_length - 1 {
            indices.push(index as u32);
            indices.push((index + 2) as u32);
            indices.push((index + 1) as u32);

            indices.push((index + 1) as u32);
            indices.push((index + 2) as u32);
            indices.push((index + 3) as u32);

            index += 4;
        }

        Some(Geometry::with_all(
            attributes,
            Some(indices),
            Some(PrimitiveType::Triangles),
            Some(BoundingSphere::from_points(&positions, None)),
            GeometryType::Polylines,
            None,
            None,
        ))
    }
}
