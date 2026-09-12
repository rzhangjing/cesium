//! Ported from `packages/engine/Source/Core/TerrainEncoding.js`.
//!
//! Encodes and decodes terrain mesh vertices.

use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::terrain_quantization::TerrainQuantization;
use crate::vertical_exaggeration::VerticalExaggeration;

/// Information about how a terrain mesh is encoded.
///
/// DEVIATION: the JS vertex layout is driven by
/// `_calculateStrideAndOffsets`, which charges **1** component for
/// `hasVertexNormals` (an oct-packed float) and 1 for `hasWebMercatorT`. This
/// port charges **2** components for the oct-encoded normal pair and 1 for the
/// water mask, matching what `HeightmapTerrainData::create_mesh` /
/// `createVerticesFromQuantizedTerrainMesh` actually push into `vertices`.
/// The layout is therefore `[X, Y, Z, H, U, V]`, then `(NX, NY)` when
/// `has_vertex_normals`, then `(W)` when `has_water_mask`, then
/// `(GNX, GNY, GNZ)` when `has_geodetic_surface_normals`.
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainEncoding {
    /// How the vertices of the mesh were compressed.
    ///
    /// Mirrors `TerrainEncoding.quantization`. Every construction site in this
    /// port produces [`TerrainQuantization::None`] (see the `decode_position`
    /// deviation note); the field exists so the JS branch structure is
    /// preserved rather than silently assumed away.
    pub quantization: TerrainQuantization,
    /// The center of the tile.
    ///
    /// Mirrors `TerrainEncoding.center`. Vertex positions are stored relative
    /// to it and [`Self::decode_position`] adds it back, which is what turns
    /// the buffer into world coordinates.
    pub center: Cartesian3,
    /// Whether the encoding includes vertex normals.
    pub has_vertex_normals: bool,
    /// Whether the encoding includes water mask.
    pub has_water_mask: bool,
    /// Whether the encoding includes geodetic surface normals, used for
    /// terrain exaggeration.
    ///
    /// Mirrors `TerrainEncoding.hasGeodeticSurfaceNormals`. No construction
    /// site in this port sets it yet, so [`Self::get_exaggerated_position`]
    /// currently reduces to [`Self::decode_position`] — exactly as it does in
    /// the JS for meshes without geodetic surface normals.
    pub has_geodetic_surface_normals: bool,
    /// The vertical exaggeration scale.
    pub exaggeration: f64,
    /// The height relative to which terrain is exaggerated.
    pub exaggeration_relative_height: f64,
    /// The stride (number of components per vertex).
    pub stride: usize,
    /// Offset of the geodetic surface normal within a vertex.
    ///
    /// Mirrors `_offsetGeodeticSurfaceNormal`. Only meaningful when
    /// `has_geodetic_surface_normals` is set.
    pub offset_geodetic_surface_normal: usize,
    /// Offset of the oct-encoded vertex normal within a vertex.
    ///
    /// Mirrors `_offsetVertexNormal`. Only meaningful when
    /// `has_vertex_normals` is set.
    pub offset_vertex_normal: usize,
}

impl TerrainEncoding {
    /// Creates a new TerrainEncoding whose vertices are relative to the
    /// origin.
    ///
    /// Vertex layout: `[X, Y, Z, H, U, V]` followed, when
    /// `has_vertex_normals`, by the oct-encoded normal pair (`NX, NY`).
    pub fn new(
        has_vertex_normals: bool,
        has_water_mask: bool,
        exaggeration: f64,
        exaggeration_relative_height: f64,
    ) -> Self {
        Self::new_with_center(
            &Cartesian3::ZERO,
            has_vertex_normals,
            has_water_mask,
            exaggeration,
            exaggeration_relative_height,
        )
    }

    /// Creates a new TerrainEncoding for vertices stored relative to `center`.
    ///
    /// Mirrors the JS constructor's `center` parameter, which the port's
    /// callers previously dropped: they kept the RTC centre on
    /// `TerrainMesh.center` only, so nothing could turn a vertex back into a
    /// world position. `TerrainPicker` needs that (see
    /// `TerrainPicker.js#getVertexPosition`).
    pub fn new_with_center(
        center: &Cartesian3,
        has_vertex_normals: bool,
        has_water_mask: bool,
        exaggeration: f64,
        exaggeration_relative_height: f64,
    ) -> Self {
        // Base stride: X, Y, Z, H, U, V = 6
        // + 2 for the oct-encoded normal pair if has_vertex_normals
        // + 1 for water mask if has_water_mask
        // + 3 for the geodetic surface normal if has_geodetic_surface_normals
        let mut stride = 6;
        let offset_vertex_normal = stride;
        if has_vertex_normals {
            stride += 2;
        }
        if has_water_mask {
            stride += 1;
        }
        let offset_geodetic_surface_normal = stride;

        Self {
            quantization: TerrainQuantization::None,
            center: *center,
            has_vertex_normals,
            has_water_mask,
            // DEVIATION: no construction site in this port supplies geodetic
            // surface normals, so the flag stays `false` and the offset above
            // is never dereferenced.
            has_geodetic_surface_normals: false,
            exaggeration,
            exaggeration_relative_height,
            stride,
            offset_geodetic_surface_normal,
            offset_vertex_normal,
        }
    }

    /// Decodes the position of a vertex stored in a packed vertex buffer, in
    /// world coordinates.
    ///
    /// Mirrors `TerrainEncoding.prototype.decodePosition`. The non-quantized
    /// arm reads the three RTC components and adds `this.center`; the addition
    /// is written out componentwise because the JS aliases `result` as both
    /// the `Cartesian3.add` input and output.
    ///
    /// DEVIATION: the JS `TerrainQuantization.BITS12` arm decompresses two
    /// packed floats and transforms by `fromScaledENU`. No construction site
    /// in this port selects `Bits12` — `QuantizedMeshTerrainData::create_mesh`
    /// is itself a stub (DEVIATION B4-5) — and the encoding carries neither
    /// `fromScaledENU` nor the `minimumHeight`/`maximumHeight` pair that arm
    /// needs, so it is rejected instead of silently returning garbage.
    pub fn decode_position<'a>(
        &self,
        buffer: &[f32],
        index: usize,
        result: &'a mut Cartesian3,
    ) -> &'a mut Cartesian3 {
        debug_assert_eq!(
            self.quantization,
            TerrainQuantization::None,
            "TerrainQuantization.BITS12 is not supported by this port"
        );

        let index = index * self.stride;
        result.x = buffer[index] as f64;
        result.y = buffer[index + 1] as f64;
        result.z = buffer[index + 2] as f64;
        // JS: `Cartesian3.add(result, this.center, result)`
        result.x += self.center.x;
        result.y += self.center.y;
        result.z += self.center.z;
        result
    }

    /// Decodes a position from the vertex buffer and applies vertical
    /// exaggeration.
    ///
    /// Mirrors `TerrainEncoding.prototype.getExaggeratedPosition`.
    pub fn get_exaggerated_position<'a>(
        &self,
        buffer: &[f32],
        index: usize,
        result: &'a mut Cartesian3,
    ) -> &'a mut Cartesian3 {
        self.decode_position(buffer, index, result);

        let exaggeration = self.exaggeration;
        let exaggeration_relative_height = self.exaggeration_relative_height;
        let has_exaggeration = exaggeration != 1.0;
        if has_exaggeration && self.has_geodetic_surface_normals {
            let mut geodetic_surface_normal = Cartesian3::default();
            self.decode_geodetic_surface_normal(buffer, index, &mut geodetic_surface_normal);
            let raw_height = self.decode_height(buffer, index);
            let height_difference = VerticalExaggeration::get_height(
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

    /// Decodes the geodetic surface normal of a vertex stored in a packed
    /// vertex buffer.
    ///
    /// Mirrors `TerrainEncoding.prototype.decodeGeodeticSurfaceNormal`.
    pub fn decode_geodetic_surface_normal<'a>(
        &self,
        buffer: &[f32],
        index: usize,
        result: &'a mut Cartesian3,
    ) -> &'a mut Cartesian3 {
        let index = index * self.stride + self.offset_geodetic_surface_normal;
        result.x = buffer[index] as f64;
        result.y = buffer[index + 1] as f64;
        result.z = buffer[index + 2] as f64;
        result
    }

    /// Decodes the height of a vertex stored in a packed vertex buffer.
    ///
    /// Mirrors `TerrainEncoding.prototype.decodeHeight`
    /// (`buffer[index * stride + 3]`; the height slot follows the XYZ
    /// position components).
    pub fn decode_height(&self, vertices: &[f32], index: usize) -> f64 {
        vertices[index * self.stride + 3] as f64
    }

    /// Decodes the texture coordinates (u, v) of a vertex stored in a packed
    /// vertex buffer.
    ///
    /// Mirrors `TerrainEncoding.prototype.decodeTextureCoordinates`
    /// (`buffer[index * stride + 4]` / `+ 5`).
    pub fn decode_texture_coordinates<'a>(
        &self,
        vertices: &[f32],
        index: usize,
        result: &'a mut Cartesian2,
    ) -> &'a mut Cartesian2 {
        result.x = vertices[index * self.stride + 4] as f64;
        result.y = vertices[index * self.stride + 5] as f64;
        result
    }
}
