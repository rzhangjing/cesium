//! Heightmap tessellation: creates a mesh from a heightmap image.
//!
//! Maps to CesiumJS `Core/HeightmapTessellator.js`

use cesium_geospatial::bounding::{AxisAlignedBoundingBox, BoundingSphere};
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::math_utils;
use cesium_geospatial::projection::WebMercatorProjection;
use cesium_geospatial::rectangle::Rectangle;
use cesium_geospatial::transforms;
use glam::{DVec2, DVec3};

use crate::terrain_encoding::TerrainEncoding;

/// Default structure of a heightmap.
/// Maps to CesiumJS `HeightmapTessellator.DEFAULT_STRUCTURE`
#[derive(Debug, Clone)]
pub struct HeightmapStructure {
    /// The factor by which to multiply height samples.
    pub height_scale: f64,
    /// The offset to add to the scaled height.
    pub height_offset: f64,
    /// The number of elements that make up a single height sample.
    pub elements_per_height: usize,
    /// The number of elements to skip between heights.
    pub stride: usize,
    /// The multiplier used to compute height when stride > 1.
    pub element_multiplier: f64,
    /// Indicates endianness when elementsPerHeight > 1.
    pub is_big_endian: bool,
}

impl Default for HeightmapStructure {
    fn default() -> Self {
        Self {
            height_scale: 1.0,
            height_offset: 0.0,
            elements_per_height: 1,
            stride: 1,
            element_multiplier: 256.0,
            is_big_endian: false,
        }
    }
}

/// Options for `compute_vertices`.
pub struct ComputeVerticesOptions {
    /// The heightmap data.
    pub heightmap: Vec<f64>,
    /// Width in height samples.
    pub width: usize,
    /// Height in height samples.
    pub height: usize,
    /// Height of skirts at edges.
    pub skirt_height: f64,
    /// Rectangle in native coordinates (degrees for geographic, meters for web mercator).
    pub native_rectangle: Rectangle,
    /// Optional rectangle in geodetic radians.
    pub rectangle: Option<Rectangle>,
    /// True if geographic projection (default), false for web mercator.
    pub is_geographic: bool,
    /// The ellipsoid.
    pub ellipsoid: Ellipsoid,
    /// Optional center for relative positions.
    pub relative_to_center: Option<DVec3>,
    /// Height structure descriptor.
    pub structure: Option<HeightmapStructure>,
    /// Whether to include web mercator T coordinate.
    pub include_web_mercator_t: bool,
    /// Terrain exaggeration scale.
    pub exaggeration: f64,
    /// Height from which exaggeration is applied.
    pub exaggeration_relative_height: f64,
}

impl ComputeVerticesOptions {
    /// Creates options with required fields and sensible defaults.
    pub fn new(
        heightmap: Vec<f64>,
        width: usize,
        height: usize,
        skirt_height: f64,
        native_rectangle: Rectangle,
    ) -> Self {
        Self {
            heightmap,
            width,
            height,
            skirt_height,
            native_rectangle,
            rectangle: None,
            is_geographic: true,
            ellipsoid: Ellipsoid::WGS84,
            relative_to_center: None,
            structure: None,
            include_web_mercator_t: false,
            exaggeration: 1.0,
            exaggeration_relative_height: 0.0,
        }
    }
}

/// Result of `compute_vertices`.
pub struct TessellatedVertices {
    /// The vertex buffer (stride floats per vertex).
    pub vertices: Vec<f64>,
    /// Minimum height.
    pub minimum_height: f64,
    /// Maximum height.
    pub maximum_height: f64,
    /// The terrain encoding used.
    pub encoding: TerrainEncoding,
    /// Bounding sphere of the mesh.
    pub bounding_sphere_3d: BoundingSphere,
}

/// Fills an array of vertices from a heightmap image.
///
/// Maps to CesiumJS `HeightmapTessellator.computeVertices`
pub fn compute_vertices(options: &ComputeVerticesOptions) -> TessellatedVertices {
    let heightmap = &options.heightmap;
    let width = options.width;
    let height = options.height;
    let skirt_height = options.skirt_height;
    let has_skirts = skirt_height > 0.0;

    let is_geographic = options.is_geographic;
    let ellipsoid = &options.ellipsoid;
    let one_over_globe_semimajor_axis = 1.0 / ellipsoid.maximum_radius();

    let native_rectangle = options.native_rectangle;

    let (geographic_west, geographic_south, geographic_east, geographic_north) =
        if let Some(ref rect) = options.rectangle {
            (rect.west, rect.south, rect.east, rect.north)
        } else if is_geographic {
            (
                math_utils::to_radians(native_rectangle.west),
                math_utils::to_radians(native_rectangle.south),
                math_utils::to_radians(native_rectangle.east),
                math_utils::to_radians(native_rectangle.north),
            )
        } else {
            let pi_over_two = math_utils::PI_OVER_TWO;
            (
                native_rectangle.west * one_over_globe_semimajor_axis,
                pi_over_two
                    - 2.0 * (-native_rectangle.south * one_over_globe_semimajor_axis)
                        .exp()
                        .atan(),
                native_rectangle.east * one_over_globe_semimajor_axis,
                pi_over_two
                    - 2.0 * (-native_rectangle.north * one_over_globe_semimajor_axis)
                        .exp()
                        .atan(),
            )
        };

    let relative_to_center = options.relative_to_center.unwrap_or(DVec3::ZERO);
    let has_relative_to_center = options.relative_to_center.is_some();
    let _include_web_mercator_t = options.include_web_mercator_t;

    let exaggeration = options.exaggeration;
    let exaggeration_relative_height = options.exaggeration_relative_height;
    let has_exaggeration = (exaggeration - 1.0).abs() > f64::EPSILON;
    let _include_geodetic_surface_normals = has_exaggeration;

    let structure = options.structure.clone().unwrap_or_default();
    let height_scale = structure.height_scale;
    let height_offset = structure.height_offset;
    let elements_per_height = structure.elements_per_height;
    let stride = structure.stride;
    let element_multiplier = structure.element_multiplier;
    let is_big_endian = structure.is_big_endian;

    let rectangle_width = native_rectangle.east - native_rectangle.west;
    let rectangle_height = native_rectangle.north - native_rectangle.south;

    let granularity_x = rectangle_width / (width - 1) as f64;
    let granularity_y = rectangle_height / (height - 1) as f64;

    let radii_squared = ellipsoid.radii_squared();
    let radii_squared_x = radii_squared.x;
    let radii_squared_y = radii_squared.y;
    let radii_squared_z = radii_squared.z;

    let mut minimum_height = 65536.0f64;
    let mut maximum_height = -65536.0f64;

    let from_enu = transforms::east_north_up_to_fixed_frame(relative_to_center, ellipsoid);
    let to_enu = transforms::inverse_transformation(&from_enu);

    let mut minimum = DVec3::splat(f64::INFINITY);
    let mut maximum = DVec3::splat(f64::NEG_INFINITY);
    let mut h_min = f64::INFINITY;

    let grid_vertex_count = width * height;
    let edge_vertex_count = if has_skirts {
        width * 2 + height * 2
    } else {
        0
    };
    let vertex_count = grid_vertex_count + edge_vertex_count;

    let mut positions: Vec<Option<DVec3>> = vec![None; vertex_count];
    let mut heights_arr: Vec<f64> = vec![0.0; vertex_count];
    let mut uvs: Vec<DVec2> = vec![DVec2::ZERO; vertex_count];

    let (start_row, end_row, start_col, end_col) = if has_skirts {
        (
            -1i32,
            height as i32 + 1,
            -1i32,
            width as i32 + 1,
        )
    } else {
        (0, height as i32, 0, width as i32)
    };

    // Note: CesiumJS applies a tiny skirt_offset_percentage (0.00001) to lat/lon
    // for z-fighting prevention. We skip this as it's a rendering optimization
    // that doesn't affect geometric correctness.

    for row_index in start_row..end_row {
        let mut row = row_index;
        if row < 0 {
            row = 0;
        }
        if row >= height as i32 {
            row = height as i32 - 1;
        }

        let mut latitude = native_rectangle.north - granularity_y * row as f64;

        if !is_geographic {
            latitude = math_utils::PI_OVER_TWO
                - 2.0 * (-latitude * one_over_globe_semimajor_axis).exp().atan();
        } else {
            latitude = math_utils::to_radians(latitude);
        }

        let mut v =
            (latitude - geographic_south) / (geographic_north - geographic_south);
        v = math_utils::clamp(v, 0.0, 1.0);

        let is_north_edge = row_index == start_row;
        let is_south_edge = row_index == end_row - 1;

        let cos_latitude = latitude.cos();
        let n_z = latitude.sin();
        let k_z = radii_squared_z * n_z;

        for col_index in start_col..end_col {
            let mut col = col_index;
            if col < 0 {
                col = 0;
            }
            if col >= width as i32 {
                col = width as i32 - 1;
            }

            let terrain_offset = row as usize * (width * stride) + col as usize * stride;

            let height_sample = if elements_per_height == 1 {
                heightmap[terrain_offset]
            } else if is_big_endian {
                let mut sample = 0.0f64;
                for element_offset in 0..elements_per_height {
                    sample = sample * element_multiplier + heightmap[terrain_offset + element_offset];
                }
                sample
            } else {
                let mut sample = 0.0f64;
                for element_offset in (0..elements_per_height).rev() {
                    sample = sample * element_multiplier + heightmap[terrain_offset + element_offset];
                }
                sample
            };

            let height_sample = height_sample * height_scale + height_offset;

            maximum_height = maximum_height.max(height_sample);
            minimum_height = minimum_height.min(height_sample);

            let mut longitude = native_rectangle.west + granularity_x * col as f64;

            if !is_geographic {
                longitude *= one_over_globe_semimajor_axis;
            } else {
                longitude = math_utils::to_radians(longitude);
            }

            let mut u = (longitude - geographic_west) / (geographic_east - geographic_west);
            u = math_utils::clamp(u, 0.0, 1.0);

            let mut index = row as usize * width + col as usize;

            if skirt_height > 0.0 {
                let is_west_edge = col_index == start_col;
                let is_east_edge = col_index == end_col - 1;
                let is_edge = is_north_edge || is_south_edge || is_west_edge || is_east_edge;
                let is_corner =
                    (is_north_edge || is_south_edge) && (is_west_edge || is_east_edge);
                if is_corner {
                    continue;
                } else if is_edge {
                    let height_sample = height_sample - skirt_height;

                    if is_west_edge {
                        index = grid_vertex_count + (height - row as usize - 1);
                    } else if is_south_edge {
                        index = grid_vertex_count + height + (width - col as usize - 1);
                    } else if is_east_edge {
                        index = grid_vertex_count + height + width + row as usize;
                    } else if is_north_edge {
                        index = grid_vertex_count + height + width + height + col as usize;
                    }

                    let n_x = cos_latitude * longitude.cos();
                    let n_y = cos_latitude * longitude.sin();
                    let k_x = radii_squared_x * n_x;
                    let k_y = radii_squared_y * n_y;
                    let gamma = (k_x * n_x + k_y * n_y + k_z * n_z).sqrt();
                    let one_over_gamma = 1.0 / gamma;
                    let r_surface_x = k_x * one_over_gamma;
                    let r_surface_y = k_y * one_over_gamma;
                    let r_surface_z = k_z * one_over_gamma;

                    let position = DVec3::new(
                        r_surface_x + n_x * height_sample,
                        r_surface_y + n_y * height_sample,
                        r_surface_z + n_z * height_sample,
                    );

                    let enu_pos = to_enu.transform_point3(position);
                    minimum = minimum.min(enu_pos);
                    maximum = maximum.max(enu_pos);
                    h_min = h_min.min(height_sample);

                    positions[index] = Some(position);
                    uvs[index] = DVec2::new(u, v);
                    heights_arr[index] = height_sample;
                    continue;
                }
            }

            let n_x = cos_latitude * longitude.cos();
            let n_y = cos_latitude * longitude.sin();
            let k_x = radii_squared_x * n_x;
            let k_y = radii_squared_y * n_y;
            let gamma = (k_x * n_x + k_y * n_y + k_z * n_z).sqrt();
            let one_over_gamma = 1.0 / gamma;
            let r_surface_x = k_x * one_over_gamma;
            let r_surface_y = k_y * one_over_gamma;
            let r_surface_z = k_z * one_over_gamma;

            let position = DVec3::new(
                r_surface_x + n_x * height_sample,
                r_surface_y + n_y * height_sample,
                r_surface_z + n_z * height_sample,
            );

            let enu_pos = to_enu.transform_point3(position);
            minimum = minimum.min(enu_pos);
            maximum = maximum.max(enu_pos);
            h_min = h_min.min(height_sample);

            positions[index] = Some(position);
            uvs[index] = DVec2::new(u, v);
            heights_arr[index] = height_sample;
        }
    }

    // Compute bounding sphere from positions
    let valid_positions: Vec<DVec3> = positions.iter().filter_map(|p| *p).collect();
    let bounding_sphere_3d = BoundingSphere::from_points(&valid_positions);

    let aa_box = AxisAlignedBoundingBox::new(minimum, maximum);
    let encoding = TerrainEncoding::new(
        relative_to_center,
        &aa_box,
        h_min,
        maximum_height,
        from_enu,
        false,
        options.include_web_mercator_t,
        has_exaggeration,
        exaggeration,
        exaggeration_relative_height,
    );

    let mut vertices: Vec<f64> = Vec::with_capacity(vertex_count * encoding.stride);
    for j in 0..vertex_count {
        let pos = positions[j].unwrap_or(DVec3::ZERO);
        encoding.encode(
            &mut vertices,
            pos,
            uvs[j],
            heights_arr[j],
            None,
            None,
            None,
        );
    }

    TessellatedVertices {
        vertices,
        minimum_height,
        maximum_height,
        encoding,
        bounding_sphere_3d,
    }
}
