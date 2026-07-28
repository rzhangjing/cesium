//! Terrain vertex encoding/decoding.
//!
//! Data used to quantize and pack the terrain mesh. The position can be unpacked
//! for picking and all attributes are unpacked in the vertex shader.
//!
//! Maps to CesiumJS `Core/TerrainEncoding.js`

use crate::TerrainQuantization;
use cesium_geospatial::attribute_compression::{
    compress_texture_coordinates, decompress_texture_coordinates, oct_pack_float,
};
use cesium_geospatial::bounding::AxisAlignedBoundingBox;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::transforms::inverse_transformation;
use cesium_geospatial::vertical_exaggeration;
use glam::{DMat4, DVec2, DVec3};

const SHIFT_LEFT_12: f64 = 4096.0;

/// Component datatype size in bytes for the vertex buffer (FLOAT = 4 bytes).
const FLOAT_SIZE_IN_BYTES: usize = 4;

/// Descriptor of a single attribute stored in the terrain vertex buffer.
///
/// Maps to the attribute objects returned by `TerrainEncoding.prototype.getAttributes`.
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainAttribute {
    /// The attribute index (location) in the shader.
    pub index: u32,
    /// Number of components per vertex attribute.
    pub components_per_attribute: u32,
    /// Byte offset of this attribute within a vertex.
    pub offset_in_bytes: usize,
    /// Byte stride between consecutive vertices.
    pub stride_in_bytes: usize,
}

/// Indices pointing to the attribute locations in the vertex buffer.
///
/// Maps to the objects returned by `TerrainEncoding.prototype.getAttributeLocations`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainAttributeLocations {
    /// Index of the position 3D + height attribute (NONE quantization).
    pub position_3d_and_height: u32,
    /// Index of the texture coordinate + encoded normals attribute (NONE quantization).
    pub texture_coord_and_encoded_normals: u32,
    /// Index of the compressed0 attribute (BITS12 quantization).
    pub compressed0: u32,
    /// Index of the compressed1 attribute (BITS12 quantization).
    pub compressed1: u32,
    /// Index of the geodetic surface normal attribute.
    pub geodetic_surface_normal: u32,
}

const ATTRIBUTES_INDICES_NONE: TerrainAttributeLocations = TerrainAttributeLocations {
    position_3d_and_height: 0,
    texture_coord_and_encoded_normals: 1,
    geodetic_surface_normal: 2,
    compressed0: 0,
    compressed1: 1,
};

const ATTRIBUTES_INDICES_BITS12: TerrainAttributeLocations = TerrainAttributeLocations {
    position_3d_and_height: 0,
    texture_coord_and_encoded_normals: 1,
    compressed0: 0,
    compressed1: 1,
    geodetic_surface_normal: 2,
};

/// Data used to quantize and pack the terrain mesh. The position can be unpacked for
/// picking and all attributes are unpacked in the vertex shader.
///
/// Maps to CesiumJS `Core/TerrainEncoding.js`
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainEncoding {
    /// How the vertices of the mesh were compressed.
    pub quantization: TerrainQuantization,
    /// The minimum height of the tile including the skirts.
    pub minimum_height: Option<f64>,
    /// The maximum height of the tile.
    pub maximum_height: Option<f64>,
    /// The center of the tile.
    pub center: Option<DVec3>,
    /// A matrix that takes a vertex from the tile, transforms it to east-north-up at the
    /// center and scales it so each component is in the [0, 1] range.
    pub to_scaled_enu: Option<DMat4>,
    /// A matrix that restores a vertex transformed with toScaledENU back to the earth
    /// fixed reference frame.
    pub from_scaled_enu: Option<DMat4>,
    /// The matrix used to decompress the terrain vertices in the shader for RTE rendering.
    pub matrix: Option<DMat4>,
    /// The terrain mesh contains normals.
    pub has_vertex_normals: bool,
    /// The terrain mesh contains a vertical texture coordinate following the Web Mercator
    /// projection.
    pub has_web_mercator_t: bool,
    /// The terrain mesh contains geodetic surface normals, used for terrain exaggeration.
    pub has_geodetic_surface_normals: bool,
    /// A scalar used to exaggerate terrain.
    pub exaggeration: f64,
    /// The relative height from which terrain is exaggerated.
    pub exaggeration_relative_height: f64,
    /// The number of components in each vertex. This value can differ with different
    /// quantizations.
    pub stride: usize,

    offset_geodetic_surface_normal: usize,
    offset_vertex_normal: usize,
}

impl Default for TerrainEncoding {
    fn default() -> Self {
        let mut encoding = Self {
            quantization: TerrainQuantization::None,
            minimum_height: None,
            maximum_height: None,
            center: None,
            to_scaled_enu: None,
            from_scaled_enu: None,
            matrix: None,
            has_vertex_normals: false,
            has_web_mercator_t: false,
            has_geodetic_surface_normals: false,
            exaggeration: 1.0,
            exaggeration_relative_height: 0.0,
            stride: 0,
            offset_geodetic_surface_normal: 0,
            offset_vertex_normal: 0,
        };
        encoding.calculate_stride_and_offsets();
        encoding
    }
}

impl TerrainEncoding {
    /// Creates a terrain encoding from an axis aligned bounding box using default
    /// options (no web mercator T, no geodetic surface normals, exaggeration 1.0).
    ///
    /// Maps to the CesiumJS constructor called with the first six arguments.
    #[allow(clippy::too_many_arguments)]
    pub fn from_aabb(
        center: DVec3,
        axis_aligned_bounding_box: &AxisAlignedBoundingBox,
        minimum_height: f64,
        maximum_height: f64,
        from_enu: DMat4,
        has_vertex_normals: bool,
    ) -> Self {
        Self::new(
            center,
            axis_aligned_bounding_box,
            minimum_height,
            maximum_height,
            from_enu,
            has_vertex_normals,
            false,
            false,
            1.0,
            0.0,
        )
    }

    /// Creates a terrain encoding from an axis aligned bounding box.
    ///
    /// Maps to the CesiumJS `TerrainEncoding` constructor.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        center: DVec3,
        axis_aligned_bounding_box: &AxisAlignedBoundingBox,
        minimum_height: f64,
        maximum_height: f64,
        from_enu: DMat4,
        has_vertex_normals: bool,
        has_web_mercator_t: bool,
        has_geodetic_surface_normals: bool,
        exaggeration: f64,
        exaggeration_relative_height: f64,
    ) -> Self {
        let minimum = axis_aligned_bounding_box.minimum;
        let maximum = axis_aligned_bounding_box.maximum;

        // Scale and bias from [0,1] to [ENU min, ENU max].
        // Also compute the inverse of the scale and bias.
        let dimensions = maximum - minimum;
        let h_dim = maximum_height - minimum_height;
        let max_dim = dimensions.max_element().max(h_dim);

        let quantization = if max_dim < SHIFT_LEFT_12 - 1.0 {
            TerrainQuantization::Bits12
        } else {
            TerrainQuantization::None
        };

        let mut st = DMat4::from_scale(dimensions);
        st.w_axis = minimum.extend(1.0);

        let inv_scale = DMat4::from_scale(DVec3::new(
            1.0 / dimensions.x,
            1.0 / dimensions.y,
            1.0 / dimensions.z,
        ));
        let inv_st = inv_scale * DMat4::from_translation(-minimum);

        let rtc_offset = from_enu.w_axis.truncate() - center;
        let mut matrix = from_enu;
        matrix.w_axis = rtc_offset.extend(1.0);
        matrix = matrix * st;

        let to_scaled_enu = inv_st * inverse_transformation(&from_enu);
        let from_scaled_enu = from_enu * st;

        let mut encoding = Self {
            quantization,
            minimum_height: Some(minimum_height),
            maximum_height: Some(maximum_height),
            center: Some(center),
            to_scaled_enu: Some(to_scaled_enu),
            from_scaled_enu: Some(from_scaled_enu),
            matrix: Some(matrix),
            has_vertex_normals,
            has_web_mercator_t,
            has_geodetic_surface_normals,
            exaggeration,
            exaggeration_relative_height,
            stride: 0,
            offset_geodetic_surface_normal: 0,
            offset_vertex_normal: 0,
        };
        encoding.calculate_stride_and_offsets();
        encoding
    }

    /// Calculate the stride and offsets for sampling the vertex buffer.
    ///
    /// Maps to `TerrainEncoding.prototype._calculateStrideAndOffsets`
    fn calculate_stride_and_offsets(&mut self) {
        let mut vertex_stride = 0usize;

        match self.quantization {
            TerrainQuantization::Bits12 => vertex_stride += 3,
            _ => vertex_stride += 6,
        }
        if self.has_web_mercator_t {
            vertex_stride += 1;
        }
        if self.has_vertex_normals {
            self.offset_vertex_normal = vertex_stride;
            vertex_stride += 1;
        }
        if self.has_geodetic_surface_normals {
            self.offset_geodetic_surface_normal = vertex_stride;
            vertex_stride += 3;
        }

        self.stride = vertex_stride;
    }

    /// Encode information about the terrain at a given position into the vertex buffer.
    /// Position, texture coordinates, height, and (optionally) normal, projection
    /// information, and geodetic surface normal are all packed into the same buffer.
    ///
    /// Values are pushed onto `vertex_buffer`. Returns the new buffer length.
    ///
    /// Maps to `TerrainEncoding.prototype.encode`
    #[allow(clippy::too_many_arguments)]
    pub fn encode(
        &self,
        vertex_buffer: &mut Vec<f64>,
        position: DVec3,
        uv: DVec2,
        height: f64,
        normal_to_pack: Option<DVec2>,
        web_mercator_t: Option<f64>,
        geodetic_surface_normal: Option<DVec3>,
    ) -> usize {
        let u = uv.x;
        let v = uv.y;

        if self.quantization == TerrainQuantization::Bits12 {
            let to_scaled_enu = self.to_scaled_enu.unwrap();
            let mut position = to_scaled_enu.transform_point3(position);
            position.x = position.x.clamp(0.0, 1.0);
            position.y = position.y.clamp(0.0, 1.0);
            position.z = position.z.clamp(0.0, 1.0);

            let minimum_height = self.minimum_height.unwrap();
            let maximum_height = self.maximum_height.unwrap();
            let h_dim = maximum_height - minimum_height;
            let h = ((height - minimum_height) / h_dim).clamp(0.0, 1.0);

            let compressed0 = compress_texture_coordinates(DVec2::new(position.x, position.y));
            let compressed1 = compress_texture_coordinates(DVec2::new(position.z, h));
            let compressed2 = compress_texture_coordinates(DVec2::new(u, v));

            vertex_buffer.push(compressed0);
            vertex_buffer.push(compressed1);
            vertex_buffer.push(compressed2);

            if self.has_web_mercator_t {
                let compressed3 =
                    compress_texture_coordinates(DVec2::new(web_mercator_t.unwrap_or(0.0), 0.0));
                vertex_buffer.push(compressed3);
            }
        } else {
            let center = self.center.unwrap();
            vertex_buffer.push(position.x - center.x);
            vertex_buffer.push(position.y - center.y);
            vertex_buffer.push(position.z - center.z);
            vertex_buffer.push(height);
            vertex_buffer.push(u);
            vertex_buffer.push(v);

            if self.has_web_mercator_t {
                vertex_buffer.push(web_mercator_t.unwrap_or(0.0));
            }
        }

        if self.has_vertex_normals {
            vertex_buffer.push(oct_pack_float(normal_to_pack.unwrap_or(DVec2::ZERO)));
        }

        if self.has_geodetic_surface_normals {
            let normal = geodetic_surface_normal.unwrap_or(DVec3::ZERO);
            vertex_buffer.push(normal.x);
            vertex_buffer.push(normal.y);
            vertex_buffer.push(normal.z);
        }

        vertex_buffer.len()
    }

    /// Decode a position from the vertex buffer.
    ///
    /// Maps to `TerrainEncoding.prototype.decodePosition`
    pub fn decode_position(&self, buffer: &[f64], index: usize) -> DVec3 {
        let index = index * self.stride;

        if self.quantization == TerrainQuantization::Bits12 {
            let xy = decompress_texture_coordinates(buffer[index]);
            let zh = decompress_texture_coordinates(buffer[index + 1]);
            let mut result = DVec3::new(xy.x, xy.y, zh.x);
            let from_scaled_enu = self.from_scaled_enu.unwrap();
            result = from_scaled_enu.transform_point3(result);
            return result;
        }

        let result = DVec3::new(buffer[index], buffer[index + 1], buffer[index + 2]);
        result + self.center.unwrap()
    }

    /// Decode a position from the vertex buffer and apply vertical exaggeration.
    ///
    /// Maps to `TerrainEncoding.prototype.getExaggeratedPosition`
    pub fn get_exaggerated_position(&self, buffer: &[f64], index: usize) -> DVec3 {
        let mut result = self.decode_position(buffer, index);

        let exaggeration = self.exaggeration;
        let exaggeration_relative_height = self.exaggeration_relative_height;
        let has_exaggeration = (exaggeration - 1.0).abs() > f64::EPSILON;
        if has_exaggeration && self.has_geodetic_surface_normals {
            let geodetic_surface_normal = self.decode_geodetic_surface_normal(buffer, index);
            let raw_height = self.decode_height(buffer, index);
            let height_difference = vertical_exaggeration::get_height(
                raw_height,
                exaggeration,
                exaggeration_relative_height,
            ) - raw_height;

            // some math is unrolled for better performance
            result.x += geodetic_surface_normal.x * height_difference;
            result.y += geodetic_surface_normal.y * height_difference;
            result.z += geodetic_surface_normal.z * height_difference;
        }

        result
    }

    /// Decode texture coordinates from the vertex buffer.
    ///
    /// Maps to `TerrainEncoding.prototype.decodeTextureCoordinates`
    pub fn decode_texture_coordinates(&self, buffer: &[f64], index: usize) -> DVec2 {
        let index = index * self.stride;

        if self.quantization == TerrainQuantization::Bits12 {
            return decompress_texture_coordinates(buffer[index + 2]);
        }

        DVec2::new(buffer[index + 4], buffer[index + 5])
    }

    /// Decode a height from the vertex buffer.
    ///
    /// Maps to `TerrainEncoding.prototype.decodeHeight`
    pub fn decode_height(&self, buffer: &[f64], index: usize) -> f64 {
        let index = index * self.stride;

        if self.quantization == TerrainQuantization::Bits12 {
            let zh = decompress_texture_coordinates(buffer[index + 1]);
            let minimum_height = self.minimum_height.unwrap();
            let maximum_height = self.maximum_height.unwrap();
            return zh.y * (maximum_height - minimum_height) + minimum_height;
        }

        buffer[index + 3]
    }

    /// Decode a web mercator T coordinate from the vertex buffer.
    ///
    /// Maps to `TerrainEncoding.prototype.decodeWebMercatorT`
    pub fn decode_web_mercator_t(&self, buffer: &[f64], index: usize) -> f64 {
        let index = index * self.stride;

        if self.quantization == TerrainQuantization::Bits12 {
            return decompress_texture_coordinates(buffer[index + 3]).x;
        }

        buffer[index + 6]
    }

    /// Decode an oct-encoded normal from the vertex buffer.
    ///
    /// Maps to `TerrainEncoding.prototype.getOctEncodedNormal`
    pub fn get_oct_encoded_normal(&self, buffer: &[f64], index: usize) -> DVec2 {
        let index = index * self.stride + self.offset_vertex_normal;

        let temp = buffer[index] / 256.0;
        let x = temp.floor();
        let y = (temp - x) * 256.0;

        DVec2::new(x, y)
    }

    /// Decode a geodetic surface normal from the vertex buffer.
    ///
    /// Maps to `TerrainEncoding.prototype.decodeGeodeticSurfaceNormal`
    pub fn decode_geodetic_surface_normal(&self, buffer: &[f64], index: usize) -> DVec3 {
        let index = index * self.stride + self.offset_geodetic_surface_normal;

        DVec3::new(buffer[index], buffer[index + 1], buffer[index + 2])
    }

    /// Add geodetic surface normals to a terrain vertex buffer.
    /// The new buffer will be larger than the old buffer.
    ///
    /// Maps to `TerrainEncoding.prototype.addGeodeticSurfaceNormals`
    pub fn add_geodetic_surface_normals(
        &mut self,
        old_buffer: &[f64],
        ellipsoid: &Ellipsoid,
    ) -> Vec<f64> {
        if self.has_geodetic_surface_normals {
            return old_buffer.to_vec();
        }

        let old_stride = self.stride;
        let vertex_count = old_buffer.len() / old_stride;
        self.has_geodetic_surface_normals = true;
        self.calculate_stride_and_offsets();
        let new_stride = self.stride;

        let mut new_buffer = vec![0.0f64; vertex_count * new_stride];
        for index in 0..vertex_count {
            for offset in 0..old_stride {
                let old_index = index * old_stride + offset;
                let new_index = index * new_stride + offset;
                new_buffer[new_index] = old_buffer[old_index];
            }
            let position = self.decode_position(&new_buffer, index);
            let geodetic_surface_normal = ellipsoid
                .geodetic_surface_normal(position)
                .unwrap_or(DVec3::ZERO);

            let buffer_index = index * new_stride + self.offset_geodetic_surface_normal;
            new_buffer[buffer_index] = geodetic_surface_normal.x;
            new_buffer[buffer_index + 1] = geodetic_surface_normal.y;
            new_buffer[buffer_index + 2] = geodetic_surface_normal.z;
        }
        new_buffer
    }

    /// Remove geodetic surface normals from a terrain vertex buffer.
    ///
    /// Maps to `TerrainEncoding.prototype.removeGeodeticSurfaceNormals`
    pub fn remove_geodetic_surface_normals(&mut self, old_buffer: &[f64]) -> Vec<f64> {
        if !self.has_geodetic_surface_normals {
            return old_buffer.to_vec();
        }

        let old_stride = self.stride;
        let vertex_count = old_buffer.len() / old_stride;
        self.has_geodetic_surface_normals = false;
        self.calculate_stride_and_offsets();
        let new_stride = self.stride;

        let mut new_buffer = vec![0.0f64; vertex_count * new_stride];
        for index in 0..vertex_count {
            for offset in 0..new_stride {
                let old_index = index * old_stride + offset;
                let new_index = index * new_stride + offset;
                new_buffer[new_index] = old_buffer[old_index];
            }
        }
        new_buffer
    }

    /// Get descriptors of the attributes stored in the vertex buffer.
    ///
    /// Maps to `TerrainEncoding.prototype.getAttributes`
    pub fn get_attributes(&self) -> Vec<TerrainAttribute> {
        let stride_in_bytes = self.stride * FLOAT_SIZE_IN_BYTES;
        let mut offset_in_bytes = 0usize;
        let mut attributes = Vec::new();

        let mut add_attribute = |index: u32, components_per_attribute: u32| {
            attributes.push(TerrainAttribute {
                index,
                components_per_attribute,
                offset_in_bytes,
                stride_in_bytes,
            });
            offset_in_bytes += components_per_attribute as usize * FLOAT_SIZE_IN_BYTES;
        };

        if self.quantization == TerrainQuantization::None {
            add_attribute(ATTRIBUTES_INDICES_NONE.position_3d_and_height, 4);

            let mut components_tex_coord_and_normals = 2u32;
            if self.has_web_mercator_t {
                components_tex_coord_and_normals += 1;
            }
            if self.has_vertex_normals {
                components_tex_coord_and_normals += 1;
            }
            add_attribute(
                ATTRIBUTES_INDICES_NONE.texture_coord_and_encoded_normals,
                components_tex_coord_and_normals,
            );

            if self.has_geodetic_surface_normals {
                add_attribute(ATTRIBUTES_INDICES_NONE.geodetic_surface_normal, 3);
            }
        } else {
            // When there is no webMercatorT or vertex normals, the attribute only needs 3
            // components: x/y, z/h, u/v. WebMercatorT and vertex normals each take up one
            // component, so if only one of them is present the first attribute gets a 4th
            // component. If both are present, we need an additional attribute that has 1
            // component.
            let using_attribute_0_component_4 = self.has_web_mercator_t || self.has_vertex_normals;
            let using_attribute_1_component_1 = self.has_web_mercator_t && self.has_vertex_normals;
            add_attribute(
                ATTRIBUTES_INDICES_BITS12.compressed0,
                if using_attribute_0_component_4 { 4 } else { 3 },
            );

            if using_attribute_1_component_1 {
                add_attribute(ATTRIBUTES_INDICES_BITS12.compressed1, 1);
            }

            if self.has_geodetic_surface_normals {
                add_attribute(ATTRIBUTES_INDICES_BITS12.geodetic_surface_normal, 3);
            }
        }

        attributes
    }

    /// Get indices pointing to the attribute locations in the vertex buffer.
    ///
    /// Maps to `TerrainEncoding.prototype.getAttributeLocations`
    pub fn get_attribute_locations(&self) -> TerrainAttributeLocations {
        if self.quantization == TerrainQuantization::None {
            ATTRIBUTES_INDICES_NONE
        } else {
            ATTRIBUTES_INDICES_BITS12
        }
    }
}
