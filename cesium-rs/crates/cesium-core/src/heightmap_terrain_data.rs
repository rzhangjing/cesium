//! Ported from `packages/engine/Source/Core/HeightmapTerrainData.js` (904 lines).
//!
//! Terrain data for a single tile where the terrain data is represented as a
//! heightmap. A heightmap is a rectangular array of heights in row-major order
//! from north to south and west to east.
//!
//! ## Method-level alignment table
//!
//! | JS | Rust | Notes |
//! |---|---|---|
//! | `constructor` | [`HeightmapTerrainData::new`] | structure `??` merge mirrored |
//! | `credits` | — | always `undefined` in JS |
//! | `waterMask` | [`HeightmapTerrainData::water_mask`] | |
//! | `childTileMask` | [`HeightmapTerrainData::child_tile_mask`] | |
//! | `createMesh` | [`HeightmapTerrainData::create_mesh`] | throttling mirrored via a global active-task counter; see DEVIATION below |
//! | `_createMeshSync` | — | folded into [`HeightmapTerrainData::create_mesh`] |
//! | `interpolateHeight` | [`HeightmapTerrainData::interpolate_height`] | `undefined` → `None` |
//! | `upsample` | [`HeightmapTerrainData::upsample`] | synchronous (`Promise.resolve` unwrapped) |
//! | `isChildAvailable` | [`HeightmapTerrainData::is_child_available`] | |
//! | `wasCreatedByUpsampling` | [`HeightmapTerrainData::was_created_by_upsampling`] | |
//! | `interpolateHeight` (private) | [`interpolate_buffer_height`] | |
//! | `interpolateMeshHeight` (private) | [`interpolate_mesh_height`] | |
//! | `triangleInterpolateHeight` (private) | [`triangle_interpolate_height`] | |
//! | `getHeight` (private) | [`get_height`] | |
//! | `setHeight` (private) | [`set_height`] | |
//!
//! DEVIATION: `createMesh` schedules `createVerticesFromHeightmap` through a
//! web worker in JS. Rust computes the mesh synchronously with a simplified
//! tessellator (no skirt vertices, no webMercatorT, no exaggeration vertex
//! transform, occludee point left at zero) and wraps the result in a ready
//! future; throttling semantics (`undefined` when too many tasks are in
//! flight) are preserved. Full `HeightmapTessellator.computeVertices`
//! materialization belongs to the Globe terrain batch (Track B4-3/4/5).
//!
//! DEVIATION: JS typed arrays (`Uint8Array`, ...) are modeled by the
//! [`HeightmapBuffer`] enum; element writes truncate toward zero like the
//! JS `| 0` / typed-array conversion.

use std::sync::Mutex;

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::check;
use crate::developer_error::throw_developer_error;
use crate::heightmap_encoding::HeightmapEncoding;
use crate::heightmap_tessellator::{HeightmapStructure, HeightmapTessellator};
use crate::math::CesiumMath;
use crate::rectangle::Rectangle;
use crate::terrain_data::TerrainData;
use crate::terrain_encoding::TerrainEncoding;
use crate::terrain_mesh::TerrainMesh;
use crate::terrain_provider;
use crate::tiling_scheme::TilingScheme;

/// The element type of a heightmap buffer.
///
/// Mirrors the JS typed-array constructors (`Uint16Array`, `Float32Array`, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeightmapBufferType {
    Float32,
    Float64,
    Uint8,
    Int8,
    Uint16,
    Int16,
    Uint32,
    Int32,
}

/// A heightmap buffer holding height samples.
///
/// DEVIATION: models the JS typed arrays that `options.buffer` may be.
#[derive(Debug, Clone, PartialEq)]
pub enum HeightmapBuffer {
    F32(Vec<f32>),
    F64(Vec<f64>),
    U8(Vec<u8>),
    I8(Vec<i8>),
    U16(Vec<u16>),
    I16(Vec<i16>),
    U32(Vec<u32>),
    I32(Vec<i32>),
}

impl HeightmapBuffer {
    /// Creates a zero-filled buffer of the given element type and length
    /// (mirrors `new bufferType(length)`).
    pub fn zeroed(buffer_type: HeightmapBufferType, length: usize) -> Self {
        match buffer_type {
            HeightmapBufferType::Float32 => Self::F32(vec![0.0; length]),
            HeightmapBufferType::Float64 => Self::F64(vec![0.0; length]),
            HeightmapBufferType::Uint8 => Self::U8(vec![0; length]),
            HeightmapBufferType::Int8 => Self::I8(vec![0; length]),
            HeightmapBufferType::Uint16 => Self::U16(vec![0; length]),
            HeightmapBufferType::Int16 => Self::I16(vec![0; length]),
            HeightmapBufferType::Uint32 => Self::U32(vec![0; length]),
            HeightmapBufferType::Int32 => Self::I32(vec![0; length]),
        }
    }

    /// The element type of this buffer.
    pub fn buffer_type(&self) -> HeightmapBufferType {
        match self {
            Self::F32(_) => HeightmapBufferType::Float32,
            Self::F64(_) => HeightmapBufferType::Float64,
            Self::U8(_) => HeightmapBufferType::Uint8,
            Self::I8(_) => HeightmapBufferType::Int8,
            Self::U16(_) => HeightmapBufferType::Uint16,
            Self::I16(_) => HeightmapBufferType::Int16,
            Self::U32(_) => HeightmapBufferType::Uint32,
            Self::I32(_) => HeightmapBufferType::Int32,
        }
    }

    /// Number of elements.
    pub fn len(&self) -> usize {
        match self {
            Self::F32(v) => v.len(),
            Self::F64(v) => v.len(),
            Self::U8(v) => v.len(),
            Self::I8(v) => v.len(),
            Self::U16(v) => v.len(),
            Self::I16(v) => v.len(),
            Self::U32(v) => v.len(),
            Self::I32(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Reads element `index` as a number.
    pub fn get(&self, index: usize) -> f64 {
        match self {
            Self::F32(v) => v[index] as f64,
            Self::F64(v) => v[index],
            Self::U8(v) => v[index] as f64,
            Self::I8(v) => v[index] as f64,
            Self::U16(v) => v[index] as f64,
            Self::I16(v) => v[index] as f64,
            Self::U32(v) => v[index] as f64,
            Self::I32(v) => v[index] as f64,
        }
    }

    /// Writes element `index`, converting like a JS typed-array element
    /// assignment (truncate toward zero, wrap for integer types).
    pub fn set(&mut self, index: usize, value: f64) {
        let truncated = value.trunc() as i64;
        match self {
            Self::F32(v) => v[index] = value as f32,
            Self::F64(v) => v[index] = value,
            Self::U8(v) => v[index] = truncated as u8,
            Self::I8(v) => v[index] = truncated as i8,
            Self::U16(v) => v[index] = truncated as u16,
            Self::I16(v) => v[index] = truncated as i16,
            Self::U32(v) => v[index] = truncated as u32,
            Self::I32(v) => v[index] = truncated as i32,
        }
    }

    /// Flattens the buffer into `f32` values (for spec assertions).
    pub fn to_f32_vec(&self) -> Vec<f32> {
        match self {
            Self::F32(v) => v.clone(),
            Self::F64(v) => v.iter().map(|x| *x as f32).collect(),
            Self::U8(v) => v.iter().map(|x| *x as f32).collect(),
            Self::I8(v) => v.iter().map(|x| *x as f32).collect(),
            Self::U16(v) => v.iter().map(|x| *x as f32).collect(),
            Self::I16(v) => v.iter().map(|x| *x as f32).collect(),
            Self::U32(v) => v.iter().map(|x| *x as f32).collect(),
            Self::I32(v) => v.iter().map(|x| *x as f32).collect(),
        }
    }
}

/// Partial heightmap structure options; unspecified fields fall back to
/// [`HeightmapTessellator::DEFAULT_STRUCTURE`].
///
/// Mirrors `options.structure` (each field is merged with `??`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HeightmapStructureOptions {
    pub height_scale: Option<f64>,
    pub height_offset: Option<f64>,
    pub elements_per_height: Option<usize>,
    pub stride: Option<usize>,
    pub element_multiplier: Option<f64>,
    pub is_big_endian: Option<bool>,
    pub lowest_encoded_height: Option<f64>,
    pub highest_encoded_height: Option<f64>,
}

impl HeightmapStructureOptions {
    /// Merges each unspecified field with the default structure.
    ///
    /// Mirrors the constructor's `structure.X ?? defaultStructure.X` block.
    pub fn merged_with_defaults(&self) -> HeightmapStructure {
        let default = HeightmapTessellator::DEFAULT_STRUCTURE;
        HeightmapStructure {
            height_scale: self.height_scale.unwrap_or(default.height_scale),
            height_offset: self.height_offset.unwrap_or(default.height_offset),
            elements_per_height: self
                .elements_per_height
                .unwrap_or(default.elements_per_height),
            stride: self.stride.unwrap_or(default.stride),
            element_multiplier: self
                .element_multiplier
                .unwrap_or(default.element_multiplier),
            is_big_endian: self.is_big_endian.unwrap_or(default.is_big_endian),
            lowest_encoded_height: self.lowest_encoded_height,
            highest_encoded_height: self.highest_encoded_height,
        }
    }
}

/// Constructor options for [`HeightmapTerrainData`].
///
/// Mirrors `HeightmapTerrainData` `options`.
#[derive(Debug, Clone, Default)]
pub struct HeightmapTerrainDataOptions {
    /// The buffer containing height data.
    pub buffer: Option<HeightmapBuffer>,
    /// The width (longitude direction) of the heightmap, in samples.
    pub width: Option<usize>,
    /// The height (latitude direction) of the heightmap, in samples.
    pub height: Option<usize>,
    /// A bit mask indicating which of this tile's four children exist.
    pub child_tile_mask: Option<i32>,
    /// The water mask included in this terrain data, if any.
    pub water_mask: Option<Vec<u8>>,
    /// An object describing the structure of the height data.
    pub structure: Option<HeightmapStructureOptions>,
    /// The encoding that is used on the buffer.
    pub encoding: Option<HeightmapEncoding>,
    /// True if this instance was created by upsampling another instance.
    pub created_by_upsampling: Option<bool>,
}

/// Options for [`HeightmapTerrainData::create_mesh`].
///
/// Mirrors the `createMesh` options object.
pub struct CreateMeshOptions<'a> {
    /// The tiling scheme to which this tile belongs.
    pub tiling_scheme: &'a dyn TilingScheme,
    /// The X coordinate of the tile.
    pub x: i32,
    /// The Y coordinate of the tile.
    pub y: i32,
    /// The level of the tile.
    pub level: i32,
    /// The scale used to exaggerate the terrain (default 1.0).
    pub exaggeration: Option<f64>,
    /// The height relative to which terrain is exaggerated (default 0.0).
    pub exaggeration_relative_height: Option<f64>,
    /// If true (default), the operation may be postponed when too many
    /// asynchronous mesh creations are already in progress.
    pub throttle: Option<bool>,
}

/// Terrain data for a single tile, represented as a heightmap.
pub struct HeightmapTerrainData {
    buffer: Option<HeightmapBuffer>,
    width: usize,
    height: usize,
    child_tile_mask: i32,
    encoding: HeightmapEncoding,
    structure: HeightmapStructure,
    created_by_upsampling: bool,
    water_mask: Option<Vec<u8>>,
    skirt_height: Option<f64>,
    buffer_type: HeightmapBufferType,
    mesh: Option<TerrainMesh>,
}

impl HeightmapTerrainData {
    /// Creates a new `HeightmapTerrainData`.
    ///
    /// Mirrors the JS constructor, including the debug checks and the
    /// per-field `structure ?? defaultStructure` merge.
    pub fn new(options: HeightmapTerrainDataOptions) -> Self {
        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            check::type_of::object("options.buffer", options.buffer.as_ref());
            check::type_of::number("options.width", options.width.map(|w| w as f64));
            check::type_of::number("options.height", options.height.map(|h| h as f64));
        }
        //>>includeEnd('debug');

        let encoding = options.encoding.unwrap_or(HeightmapEncoding::None);
        let structure = match options.structure {
            Some(partial) => partial.merged_with_defaults(),
            None => *HeightmapTessellator::DEFAULT_STRUCTURE,
        };

        // this._bufferType = encoding === LERC ? Float32Array : buffer.constructor
        let buffer_type = if encoding == HeightmapEncoding::Lerc {
            HeightmapBufferType::Float32
        } else {
            options
                .buffer
                .as_ref()
                .map(|b| b.buffer_type())
                .unwrap_or(HeightmapBufferType::Float32)
        };

        Self {
            width: options.width.unwrap_or(0),
            height: options.height.unwrap_or(0),
            child_tile_mask: options.child_tile_mask.unwrap_or(15),
            encoding,
            structure,
            created_by_upsampling: options.created_by_upsampling.unwrap_or(false),
            water_mask: options.water_mask,
            skirt_height: None,
            buffer_type,
            buffer: options.buffer,
            mesh: None,
        }
    }

    /// The water mask included in this terrain data, if any.
    pub fn water_mask(&self) -> Option<&Vec<u8>> {
        self.water_mask.as_ref()
    }

    /// A bit mask indicating which of this tile's four children exist.
    pub fn child_tile_mask(&self) -> i32 {
        self.child_tile_mask
    }

    /// The width (longitude direction) of the heightmap, in samples.
    pub fn width(&self) -> usize {
        self.width
    }

    /// The height (latitude direction) of the heightmap, in samples.
    pub fn height(&self) -> usize {
        self.height
    }

    /// The height buffer (`undefined` after `create_mesh` frees it).
    pub fn buffer(&self) -> Option<&HeightmapBuffer> {
        self.buffer.as_ref()
    }

    /// The encoding used on the buffer.
    pub fn encoding(&self) -> HeightmapEncoding {
        self.encoding
    }

    /// The (merged) heightmap structure.
    pub fn structure(&self) -> &HeightmapStructure {
        &self.structure
    }

    /// The buffer element type (mirrors `_bufferType`).
    pub fn buffer_type(&self) -> HeightmapBufferType {
        self.buffer_type
    }

    /// The skirt height computed by the last `create_mesh` call, if any.
    pub fn skirt_height(&self) -> Option<f64> {
        self.skirt_height
    }

    /// The mesh created by `create_mesh`, if any.
    pub fn mesh(&self) -> Option<&TerrainMesh> {
        self.mesh.as_ref()
    }

    /// Creates a [`TerrainMesh`] from this terrain data.
    ///
    /// Mirrors `createMesh`. Returns `None` when throttling is enabled and
    /// [`TerrainData::MAXIMUM_ASYNCHRONOUS_TASKS`] creations are already in
    /// progress (JS returns `undefined`, "Postponed"). The returned future is
    /// ready: the Rust port computes the mesh synchronously (see module-level
    /// DEVIATION). Awaiting it stores the mesh in `self` and frees the buffer.
    pub fn create_mesh(
        &mut self,
        options: CreateMeshOptions<'_>,
    ) -> Option<impl std::future::Future<Output = ()> + '_> {
        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            check::type_of::object("options.tilingScheme", Some(&options.tiling_scheme));
            // x / y / level are required i32 fields in Rust; JS checks them
            // for definedness only.
        }
        //>>includeEnd('debug');

        let tiling_scheme = options.tiling_scheme;
        let x = options.x;
        let y = options.y;
        let level = options.level;
        let exaggeration = options.exaggeration.unwrap_or(1.0);
        let exaggeration_relative_height = options.exaggeration_relative_height.unwrap_or(0.0);
        let throttle = options.throttle.unwrap_or(true);

        let permit = if throttle {
            let mut active = active_mesh_tasks().lock().unwrap_or_else(|e| e.into_inner());
            if *active >= <HeightmapTerrainData as TerrainData>::MAXIMUM_ASYNCHRONOUS_TASKS {
                // Postponed
                return None;
            }
            *active += 1;
            Some(MeshTaskPermit)
        } else {
            None
        };

        let ellipsoid = tiling_scheme.ellipsoid();
        let mut rectangle = Rectangle::default();
        tiling_scheme.tile_xy_to_rectangle(x, y, level, &mut rectangle);

        // Compute the center of the tile for RTC rendering.
        let center_cartographic = Rectangle::center(&rectangle);
        let mut center = Cartesian3::default();
        ellipsoid.cartographic_to_cartesian(&center_cartographic, &mut center);

        let level_zero_max_error =
            terrain_provider::get_estimated_level_zero_geometric_error_for_a_heightmap(
                ellipsoid,
                self.width as f64,
                tiling_scheme.get_number_of_x_tiles_at_level(0),
            );
        let this_level_max_error = level_zero_max_error / (1 << level) as f64;
        self.skirt_height = Some((this_level_max_error * 4.0).min(1000.0));

        // Simplified tessellation (see module-level DEVIATION): a
        // width x height grid of [X, Y, Z, H, U, V] vertices relative to
        // `center`, rows running north to south.
        let structure = self.structure;
        let buffer = self
            .buffer
            .as_ref()
            .expect("createMesh requires the height buffer to still be defined");

        let width = self.width;
        let height = self.height;
        let vertex_count = width * height;
        let mut vertices: Vec<f32> = Vec::with_capacity(vertex_count * 6);
        let mut positions_for_sphere: Vec<f64> = Vec::with_capacity(vertex_count * 6);
        let mut minimum_height = f64::INFINITY;
        let mut maximum_height = f64::NEG_INFINITY;

        let mut cartographic = Cartographic::default();
        let mut cartesian = Cartesian3::default();

        for row in 0..height {
            cartographic.latitude = CesiumMath::lerp(
                rectangle.north,
                rectangle.south,
                row as f64 / (height - 1) as f64,
            );
            for col in 0..width {
                cartographic.longitude = CesiumMath::lerp(
                    rectangle.west,
                    rectangle.east,
                    col as f64 / (width - 1) as f64,
                );

                let encoded_height = get_height(
                    buffer,
                    structure.elements_per_height,
                    structure.element_multiplier,
                    structure.stride,
                    structure.is_big_endian,
                    row * width + col,
                );
                let terrain_height =
                    encoded_height * structure.height_scale + structure.height_offset;

                minimum_height = minimum_height.min(terrain_height);
                maximum_height = maximum_height.max(terrain_height);

                cartographic.height = terrain_height;
                ellipsoid.cartographic_to_cartesian(&cartographic, &mut cartesian);

                let px = cartesian.x - center.x;
                let py = cartesian.y - center.y;
                let pz = cartesian.z - center.z;
                let u = col as f64 / (width - 1) as f64;
                let v = row as f64 / (height - 1) as f64;

                vertices.extend_from_slice(&[
                    px as f32,
                    py as f32,
                    pz as f32,
                    terrain_height as f32,
                    u as f32,
                    v as f32,
                ]);
                positions_for_sphere.extend_from_slice(&[px, py, pz, terrain_height, u, v]);
            }
        }

        let encoding = TerrainEncoding::new(
            false,
            false,
            exaggeration,
            exaggeration_relative_height,
        );

        let bounding_sphere = BoundingSphere::from_vertices(
            &positions_for_sphere,
            Some(&center),
            Some(6),
            None,
        );

        let indices_and_edges =
            terrain_provider::get_regular_grid_indices_and_edge_indices(
                width as i32,
                height as i32,
            );

        let mesh = TerrainMesh {
            center,
            vertices,
            stride: encoding.stride,
            indices: indices_and_edges.indices,
            index_count_without_skirts: (width - 1) * (height - 1) * 6,
            vertex_count_without_skirts: vertex_count,
            minimum_height,
            maximum_height,
            rectangle,
            bounding_sphere_3d: bounding_sphere,
            // DEVIATION: JS computes the horizon occlusion point via
            // EllipsoidTangentPlane.computeHorizonCullingPoint; left at ZERO.
            occludee_point_in_scaled_space: Cartesian3::default(),
            encoding,
            // DEVIATION: JS computes an OrientedBoundingBox; left as None.
            oriented_bounding_box: None,
            west_indices_south_to_north: indices_and_edges
                .west_indices_south_to_north
                .iter()
                .map(|i| *i as u32)
                .collect(),
            south_indices_east_to_west: indices_and_edges
                .south_indices_east_to_west
                .iter()
                .map(|i| *i as u32)
                .collect(),
            east_indices_north_to_south: indices_and_edges
                .east_indices_north_to_south
                .iter()
                .map(|i| *i as u32)
                .collect(),
            north_indices_west_to_east: indices_and_edges
                .north_indices_west_to_east
                .iter()
                .map(|i| *i as u32)
                .collect(),
        };

        // Free memory received from server after mesh is created.
        self.buffer = None;
        self.mesh = Some(mesh);

        Some(async move {
            // Ready future; the permit is released when the future resolves
            // (or is dropped), mirroring the JS worker slot lifecycle.
            let _permit = permit;
        })
    }

    /// Computes the terrain height at a specified longitude and latitude.
    ///
    /// Mirrors `interpolateHeight`. Returns `None` when interpolation is
    /// impossible (LERC-encoded buffer without a mesh; JS returns
    /// `undefined`).
    pub fn interpolate_height(
        &self,
        rectangle: &Rectangle,
        longitude: f64,
        latitude: f64,
    ) -> Option<f64> {
        let width = self.width;
        let height = self.height;

        let structure = &self.structure;
        let stride = structure.stride;
        let elements_per_height = structure.elements_per_height;
        let element_multiplier = structure.element_multiplier;
        let is_big_endian = structure.is_big_endian;
        let height_offset = structure.height_offset;
        let height_scale = structure.height_scale;

        let is_mesh_created = self.mesh.is_some();
        let is_lerc_encoding = self.encoding == HeightmapEncoding::Lerc;
        let is_interpolation_impossible = !is_mesh_created && is_lerc_encoding;
        if is_interpolation_impossible {
            // We can't interpolate using the buffer because it's LERC encoded
            // so please call createMesh() first and interpolate using the
            // mesh; as mesh creation will decode the LERC buffer.
            return None;
        }

        if is_mesh_created {
            let mesh = self.mesh.as_ref().unwrap();
            Some(interpolate_mesh_height(
                &mesh.vertices,
                &mesh.encoding,
                height_offset,
                height_scale,
                rectangle,
                width,
                height,
                longitude,
                latitude,
            ))
        } else {
            let buffer = self.buffer.as_ref().unwrap();
            let height_sample = interpolate_buffer_height(
                buffer,
                elements_per_height,
                element_multiplier,
                stride,
                is_big_endian,
                rectangle,
                width,
                height,
                longitude,
                latitude,
            );
            Some(height_sample * height_scale + height_offset)
        }
    }

    /// Upsamples this terrain data for use by a descendant tile. The
    /// resulting instance will contain a subset of the height samples in
    /// this instance, interpolated if necessary.
    ///
    /// Mirrors `upsample`. Returns `None` when the mesh is unavailable
    /// (JS returns `undefined`); the JS promise is unwrapped because the
    /// computation is synchronous in Rust.
    #[allow(clippy::too_many_arguments)]
    pub fn upsample(
        &self,
        tiling_scheme: Option<&dyn TilingScheme>,
        this_x: Option<i32>,
        this_y: Option<i32>,
        this_level: Option<i32>,
        descendant_x: Option<i32>,
        descendant_y: Option<i32>,
        descendant_level: Option<i32>,
    ) -> Option<HeightmapTerrainData> {
        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            check::defined("tilingScheme", tiling_scheme);
            check::defined("thisX", this_x.as_ref());
            check::defined("thisY", this_y.as_ref());
            check::defined("thisLevel", this_level.as_ref());
            check::defined("descendantX", descendant_x.as_ref());
            check::defined("descendantY", descendant_y.as_ref());
            check::defined("descendantLevel", descendant_level.as_ref());
            let level_difference = descendant_level.unwrap() - this_level.unwrap();
            if level_difference > 1 {
                throw_developer_error(
                    "Upsampling through more than one level at a time is not currently supported.",
                );
            }
        }
        //>>includeEnd('debug');

        let tiling_scheme = tiling_scheme?;
        let this_x = this_x?;
        let this_y = this_y?;
        let this_level = this_level?;
        let descendant_x = descendant_x?;
        let descendant_y = descendant_y?;
        let descendant_level = descendant_level?;

        let mesh_data = self.mesh.as_ref()?;

        let width = self.width;
        let height = self.height;
        let structure = self.structure;
        let stride = structure.stride;

        let mut heights = HeightmapBuffer::zeroed(self.buffer_type, width * height * stride);

        let buffer = &mesh_data.vertices;
        let encoding = &mesh_data.encoding;

        // PERFORMANCE_IDEA: don't recompute these rectangles - the caller
        // already knows them.
        let mut source_rectangle = Rectangle::default();
        tiling_scheme.tile_xy_to_rectangle(this_x, this_y, this_level, &mut source_rectangle);
        let mut destination_rectangle = Rectangle::default();
        tiling_scheme.tile_xy_to_rectangle(
            descendant_x,
            descendant_y,
            descendant_level,
            &mut destination_rectangle,
        );

        let height_offset = structure.height_offset;
        let height_scale = structure.height_scale;

        let elements_per_height = structure.elements_per_height;
        let element_multiplier = structure.element_multiplier;
        let is_big_endian = structure.is_big_endian;

        let divisor = element_multiplier.powi(elements_per_height as i32 - 1);

        for j in 0..height {
            let latitude = CesiumMath::lerp(
                destination_rectangle.north,
                destination_rectangle.south,
                j as f64 / (height - 1) as f64,
            );
            for i in 0..width {
                let longitude = CesiumMath::lerp(
                    destination_rectangle.west,
                    destination_rectangle.east,
                    i as f64 / (width - 1) as f64,
                );
                let mut height_sample = interpolate_mesh_height(
                    buffer,
                    encoding,
                    height_offset,
                    height_scale,
                    &source_rectangle,
                    width,
                    height,
                    longitude,
                    latitude,
                );

                // Use conditionals here instead of min/max so that an absent
                // lowestEncodedHeight or highestEncodedHeight has no effect.
                if let Some(lowest) = structure.lowest_encoded_height {
                    if height_sample < lowest {
                        height_sample = lowest;
                    }
                }
                if let Some(highest) = structure.highest_encoded_height {
                    if height_sample > highest {
                        height_sample = highest;
                    }
                }

                set_height(
                    &mut heights,
                    elements_per_height,
                    element_multiplier,
                    divisor,
                    stride,
                    is_big_endian,
                    j * width + i,
                    height_sample,
                );
            }
        }

        Some(HeightmapTerrainData::new(HeightmapTerrainDataOptions {
            buffer: Some(heights),
            width: Some(width),
            height: Some(height),
            child_tile_mask: Some(0),
            structure: Some(HeightmapStructureOptions {
                height_scale: Some(structure.height_scale),
                height_offset: Some(structure.height_offset),
                elements_per_height: Some(structure.elements_per_height),
                stride: Some(structure.stride),
                element_multiplier: Some(structure.element_multiplier),
                is_big_endian: Some(structure.is_big_endian),
                lowest_encoded_height: structure.lowest_encoded_height,
                highest_encoded_height: structure.highest_encoded_height,
            }),
            created_by_upsampling: Some(true),
            ..Default::default()
        }))
    }

    /// Determines if a given child tile is available, based on the
    /// `childTileMask`.
    ///
    /// Mirrors `isChildAvailable`.
    pub fn is_child_available(
        &self,
        this_x: Option<i32>,
        this_y: Option<i32>,
        child_x: Option<i32>,
        child_y: Option<i32>,
    ) -> bool {
        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            check::type_of::number("thisX", this_x.map(|v| v as f64));
            check::type_of::number("thisY", this_y.map(|v| v as f64));
            check::type_of::number("childX", child_x.map(|v| v as f64));
            check::type_of::number("childY", child_y.map(|v| v as f64));
        }
        //>>includeEnd('debug');

        let (this_x, this_y, child_x, child_y) =
            (this_x.unwrap(), this_y.unwrap(), child_x.unwrap(), child_y.unwrap());

        let mut bit_number = 2; // northwest child
        if child_x != this_x * 2 {
            bit_number += 1; // east child
        }
        if child_y != this_y * 2 {
            bit_number -= 2; // south child
        }

        (self.child_tile_mask & (1 << bit_number)) != 0
    }

    /// Gets a value indicating whether this terrain data was created by
    /// upsampling lower resolution terrain data.
    ///
    /// Mirrors `wasCreatedByUpsampling`.
    pub fn was_created_by_upsampling(&self) -> bool {
        self.created_by_upsampling
    }
}

impl TerrainData for HeightmapTerrainData {
    fn interpolate_height(&self, rectangle: &Rectangle, longitude: f64, latitude: f64) -> f64 {
        // DEVIATION: the trait cannot express JS `undefined`; LERC buffers
        // without a mesh yield NaN.
        Self::interpolate_height(self, rectangle, longitude, latitude).unwrap_or(f64::NAN)
    }

    fn is_child_available(&self, this_x: i32, this_y: i32, child_x: i32, child_y: i32) -> bool {
        Self::is_child_available(self, Some(this_x), Some(this_y), Some(child_x), Some(child_y))
    }

    fn was_created_by_upsampling(&self) -> bool {
        self.created_by_upsampling
    }
}

// ── Throttling ─────────────────────────────────────────────────────────

fn active_mesh_tasks() -> &'static Mutex<usize> {
    static ACTIVE: Mutex<usize> = Mutex::new(0);
    &ACTIVE
}

/// RAII decrement of the active mesh-task counter, mirroring the release of
/// a throttled `TaskProcessor` slot when the worker promise settles.
struct MeshTaskPermit;

impl Drop for MeshTaskPermit {
    fn drop(&mut self) {
        let mut active = active_mesh_tasks().lock().unwrap_or_else(|e| e.into_inner());
        *active = active.saturating_sub(1);
    }
}

// ── Private interpolation helpers ──────────────────────────────────────

/// Mirrors the private `interpolateHeight` function: bilinear location of
/// the sample in the source rectangle, then triangle interpolation of the
/// four surrounding decoded heights.
#[allow(clippy::too_many_arguments)]
fn interpolate_buffer_height(
    source_heights: &HeightmapBuffer,
    elements_per_height: usize,
    element_multiplier: f64,
    stride: usize,
    is_big_endian: bool,
    source_rectangle: &Rectangle,
    width: usize,
    height: usize,
    longitude: f64,
    latitude: f64,
) -> f64 {
    let from_west = ((longitude - source_rectangle.west) * (width - 1) as f64)
        / (source_rectangle.east - source_rectangle.west);
    let from_south = ((latitude - source_rectangle.south) * (height - 1) as f64)
        / (source_rectangle.north - source_rectangle.south);

    let mut west_integer = from_west as i64; // JS `| 0` truncation
    let mut east_integer = west_integer + 1;
    if east_integer >= width as i64 {
        east_integer = width as i64 - 1;
        west_integer = width as i64 - 2;
    }

    let mut south_integer = from_south as i64;
    let mut north_integer = south_integer + 1;
    if north_integer >= height as i64 {
        north_integer = height as i64 - 1;
        south_integer = height as i64 - 2;
    }

    let dx = from_west - west_integer as f64;
    let dy = from_south - south_integer as f64;

    south_integer = height as i64 - 1 - south_integer;
    north_integer = height as i64 - 1 - north_integer;

    let southwest_height = get_height(
        source_heights,
        elements_per_height,
        element_multiplier,
        stride,
        is_big_endian,
        (south_integer * width as i64 + west_integer) as usize,
    );
    let southeast_height = get_height(
        source_heights,
        elements_per_height,
        element_multiplier,
        stride,
        is_big_endian,
        (south_integer * width as i64 + east_integer) as usize,
    );
    let northwest_height = get_height(
        source_heights,
        elements_per_height,
        element_multiplier,
        stride,
        is_big_endian,
        (north_integer * width as i64 + west_integer) as usize,
    );
    let northeast_height = get_height(
        source_heights,
        elements_per_height,
        element_multiplier,
        stride,
        is_big_endian,
        (north_integer * width as i64 + east_integer) as usize,
    );

    triangle_interpolate_height(
        dx,
        dy,
        southwest_height,
        southeast_height,
        northwest_height,
        northeast_height,
    )
}

/// Mirrors the private `interpolateMeshHeight` function: same bilinear
/// location logic, but heights are decoded from mesh vertices via the mesh
/// encoding and normalized by `heightScale` / `heightOffset`.
#[allow(clippy::too_many_arguments)]
fn interpolate_mesh_height(
    buffer: &[f32],
    encoding: &TerrainEncoding,
    height_offset: f64,
    height_scale: f64,
    source_rectangle: &Rectangle,
    width: usize,
    height: usize,
    longitude: f64,
    latitude: f64,
) -> f64 {
    // Returns a height encoded according to the structure's heightScale and
    // heightOffset.
    let from_west = ((longitude - source_rectangle.west) * (width - 1) as f64)
        / (source_rectangle.east - source_rectangle.west);
    let from_south = ((latitude - source_rectangle.south) * (height - 1) as f64)
        / (source_rectangle.north - source_rectangle.south);

    let mut west_integer = from_west as i64;
    let mut east_integer = west_integer + 1;
    if east_integer >= width as i64 {
        east_integer = width as i64 - 1;
        west_integer = width as i64 - 2;
    }

    let mut south_integer = from_south as i64;
    let mut north_integer = south_integer + 1;
    if north_integer >= height as i64 {
        north_integer = height as i64 - 1;
        south_integer = height as i64 - 2;
    }

    let dx = from_west - west_integer as f64;
    let dy = from_south - south_integer as f64;

    south_integer = height as i64 - 1 - south_integer;
    north_integer = height as i64 - 1 - north_integer;

    let southwest_height = (encoding.decode_height(buffer, (south_integer * width as i64 + west_integer) as usize)
        - height_offset)
        / height_scale;
    let southeast_height = (encoding.decode_height(buffer, (south_integer * width as i64 + east_integer) as usize)
        - height_offset)
        / height_scale;
    let northwest_height = (encoding.decode_height(buffer, (north_integer * width as i64 + west_integer) as usize)
        - height_offset)
        / height_scale;
    let northeast_height = (encoding.decode_height(buffer, (north_integer * width as i64 + east_integer) as usize)
        - height_offset)
        / height_scale;

    triangle_interpolate_height(
        dx,
        dy,
        southwest_height,
        southeast_height,
        northwest_height,
        northeast_height,
    )
}

/// Mirrors the private `triangleInterpolateHeight` function. The
/// HeightmapTessellator bisects the quad from southwest to northeast.
fn triangle_interpolate_height(
    dx: f64,
    dy: f64,
    southwest_height: f64,
    southeast_height: f64,
    northwest_height: f64,
    northeast_height: f64,
) -> f64 {
    if dy < dx {
        // Lower right triangle
        return southwest_height
            + dx * (southeast_height - southwest_height)
            + dy * (northeast_height - southeast_height);
    }

    // Upper left triangle
    southwest_height
        + dx * (northeast_height - northwest_height)
        + dy * (northwest_height - southwest_height)
}

/// Mirrors the private `getHeight` function: decodes a multi-element height
/// according to `elementMultiplier` and endianness.
fn get_height(
    heights: &HeightmapBuffer,
    elements_per_height: usize,
    element_multiplier: f64,
    stride: usize,
    is_big_endian: bool,
    index: usize,
) -> f64 {
    let mut index = index * stride;

    let mut height = 0.0;

    if is_big_endian {
        for _ in 0..elements_per_height {
            height = height * element_multiplier + heights.get(index);
            index += 1;
        }
    } else {
        // JS: `index += elementsPerHeight - 1;` then reads `index--` in a
        // descending loop (the final post-decrement is a JS no-op; with a
        // usize it would underflow, so iterate the range directly).
        let start = index + elements_per_height - 1;
        for i in 0..elements_per_height {
            height = height * element_multiplier + heights.get(start - i);
        }
    }

    height
}

/// Mirrors the private `setHeight` function: encodes a height into
/// `elementsPerHeight` elements according to `elementMultiplier` and
/// endianness.
#[allow(clippy::too_many_arguments)]
fn set_height(
    heights: &mut HeightmapBuffer,
    elements_per_height: usize,
    element_multiplier: f64,
    divisor: f64,
    stride: usize,
    is_big_endian: bool,
    index: usize,
    mut height: f64,
) {
    let index = index * stride;
    let mut divisor = divisor;

    if is_big_endian {
        for i in 0..elements_per_height - 1 {
            heights.set(index + i, (height / divisor).trunc());
            height -= heights.get(index + i) * divisor;
            divisor /= element_multiplier;
        }
        heights.set(index + elements_per_height - 1, height);
    } else {
        let mut i = elements_per_height - 1;
        while i > 0 {
            heights.set(index + i, (height / divisor).trunc());
            height -= heights.get(index + i) * divisor;
            divisor /= element_multiplier;
            i -= 1;
        }
        heights.set(index, height);
    }
}
