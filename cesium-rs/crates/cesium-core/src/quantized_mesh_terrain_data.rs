//! Ported from `packages/engine/Source/Core/QuantizedMeshTerrainData.js` (736 lines).
//!
//! Terrain data for a single tile where the terrain data is represented as a
//! quantized mesh. A quantized mesh consists of three vertex attributes,
//! longitude, latitude, and height. All attributes are expressed as 16-bit
//! values in the range 0 to 32767. Longitude and latitude are zero at the
//! southwest corner of the tile and 32767 at the northeast corner. Height is
//! zero at the minimum height in the tile and 32767 at the maximum height.
//!
//! ## Method-level alignment table
//!
//! | JS | Rust | Notes |
//! |---|---|---|
//! | `constructor` | [`QuantizedMeshTerrainData::new`] | edge indices sorted like `sortIndicesIfNecessary` |
//! | `credits` | [`QuantizedMeshTerrainData::credits`] | DEVIATION: `Credit[]` modeled as `Vec<String>` |
//! | `waterMask` | [`QuantizedMeshTerrainData::water_mask`] | |
//! | `childTileMask` | [`QuantizedMeshTerrainData::child_tile_mask`] | |
//! | `canUpsample` | [`QuantizedMeshTerrainData::can_upsample`] | |
//! | `createMesh` | [`QuantizedMeshTerrainData::create_mesh`] | the `createVerticesFromQuantizedTerrainMesh` worker is inlined (see that module) |
//! | `upsample` | [`QuantizedMeshTerrainData::upsample`] | the `upsampleQuantizedTerrainMesh` worker is inlined (see that module) |
//! | `interpolateHeight` | [`QuantizedMeshTerrainData::interpolate_height`] | |
//! | `pointInBoundingBox` (private) | [`point_in_bounding_box`] | |
//! | `interpolateMeshHeight` (private) | [`interpolate_mesh_height`] | |
//! | `interpolateHeight` (private) | [`interpolate_quantized_height`] | |
//! | `isChildAvailable` | [`QuantizedMeshTerrainData::is_child_available`] | |
//! | `wasCreatedByUpsampling` | [`QuantizedMeshTerrainData::was_created_by_upsampling`] | |
//!
//! DEVIATION: JS typed arrays (`Uint16Array` / `Uint32Array`) are modeled as
//! `Vec<u16>` / `Vec<u32>`; `IndexDatatype.createTypedArray` width selection
//! is therefore implicit (indices are always 32-bit). Edge-index views
//! (`_uValues`, ...) are copies.
//!
//! DEVIATION: the JS web workers (`createVerticesFromQuantizedTerrainMesh`,
//! `upsampleQuantizedTerrainMesh`) run in-process; the throttled
//! `TaskProcessor` slot accounting is mirrored with module-level counters
//! and RAII permits released when the returned future completes.

use std::sync::{LazyLock, Mutex};

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::check;
use crate::create_vertices_from_quantized_terrain_mesh::{
    create_vertices_from_quantized_terrain_mesh, CreateVerticesParams,
};
use crate::developer_error::throw_developer_error;
use crate::intersections2d::Intersections2D;
use crate::math::CesiumMath;
use crate::oriented_bounding_box::OrientedBoundingBox;
use crate::rectangle::Rectangle;
use crate::terrain_data::TerrainData;
use crate::terrain_mesh::TerrainMesh;
use crate::tiling_scheme::TilingScheme;
use crate::upsample_quantized_terrain_mesh::{
    upsample_quantized_terrain_mesh, UpsampleQuantizedMeshParams,
};

const MAX_SHORT: f64 = 32767.0;

/// Constructor options for [`QuantizedMeshTerrainData`].
///
/// Mirrors `QuantizedMeshTerrainData` `options`.
#[derive(Default)]
pub struct QuantizedMeshTerrainDataOptions {
    /// The buffer containing the quantized mesh (u, then v, then heights).
    pub quantized_vertices: Option<Vec<u16>>,
    /// The indices specifying how the quantized vertices are linked into
    /// triangles.
    pub indices: Option<Vec<u32>>,
    /// The minimum terrain height within the tile, in meters.
    pub minimum_height: Option<f64>,
    /// The maximum terrain height within the tile, in meters.
    pub maximum_height: Option<f64>,
    /// A sphere bounding all of the vertices in the mesh.
    pub bounding_sphere: Option<BoundingSphere>,
    /// An OrientedBoundingBox bounding all of the vertices in the mesh.
    pub oriented_bounding_box: Option<OrientedBoundingBox>,
    /// The horizon occlusion point of the mesh, in ellipsoid-scaled
    /// coordinates.
    pub horizon_occlusion_point: Option<Cartesian3>,
    /// The indices of the vertices on the western edge of the tile.
    pub west_indices: Option<Vec<u32>>,
    /// The indices of the vertices on the southern edge of the tile.
    pub south_indices: Option<Vec<u32>>,
    /// The indices of the vertices on the eastern edge of the tile.
    pub east_indices: Option<Vec<u32>>,
    /// The indices of the vertices on the northern edge of the tile.
    pub north_indices: Option<Vec<u32>>,
    /// The height of the skirt on the western edge of the tile.
    pub west_skirt_height: Option<f64>,
    /// The height of the skirt on the southern edge of the tile.
    pub south_skirt_height: Option<f64>,
    /// The height of the skirt on the eastern edge of the tile.
    pub east_skirt_height: Option<f64>,
    /// The height of the skirt on the northern edge of the tile.
    pub north_skirt_height: Option<f64>,
    /// A bit mask indicating which of this tile's four children exist.
    pub child_tile_mask: Option<i32>,
    /// True if this instance was created by upsampling another instance.
    pub created_by_upsampling: Option<bool>,
    /// Per-vertex normals, oct-encoded.
    pub encoded_normals: Option<Vec<u8>>,
    /// The water mask included in this terrain data, if any.
    pub water_mask: Option<Vec<u8>>,
    /// Credits for this tile.
    pub credits: Option<Vec<String>>,
}

/// Options for [`QuantizedMeshTerrainData::create_mesh`].
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

/// Terrain data for a single tile, represented as a quantized mesh.
// DEVIATION: oriented_bounding_box / horizon_occlusion_point are consumed
// by the inlined createVerticesFromQuantizedTerrainMesh worker equivalent
// only as pass-through values into the resulting TerrainMesh.
pub struct QuantizedMeshTerrainData {
    quantized_vertices: Option<Vec<u16>>,
    encoded_normals: Option<Vec<u8>>,
    indices: Option<Vec<u32>>,
    minimum_height: f64,
    maximum_height: f64,
    bounding_sphere: BoundingSphere,
    oriented_bounding_box: Option<OrientedBoundingBox>,
    horizon_occlusion_point: Cartesian3,
    credits: Option<Vec<String>>,
    u_values: Option<Vec<u16>>,
    v_values: Option<Vec<u16>>,
    height_values: Option<Vec<u16>>,
    west_indices: Vec<u32>,
    south_indices: Vec<u32>,
    east_indices: Vec<u32>,
    north_indices: Vec<u32>,
    west_skirt_height: f64,
    south_skirt_height: f64,
    east_skirt_height: f64,
    north_skirt_height: f64,
    child_tile_mask: i32,
    created_by_upsampling: bool,
    water_mask: Option<Vec<u8>>,
    mesh: Option<TerrainMesh>,
}

impl QuantizedMeshTerrainData {
    /// Creates a new `QuantizedMeshTerrainData`.
    ///
    /// Mirrors the JS constructor, including the debug checks and the
    /// `sortIndicesIfNecessary` pass over the edge index arrays.
    pub fn new(options: QuantizedMeshTerrainDataOptions) -> Self {
        //>>includeStart('debug', pragmas.debug)
        if cfg!(debug_assertions) {
            check::object("options.quantizedVertices", options.quantized_vertices.as_ref());
            check::object("options.indices", options.indices.as_ref());
            check::number("options.minimumHeight", options.minimum_height);
            check::number("options.maximumHeight", options.maximum_height);
            check::object("options.boundingSphere", options.bounding_sphere.as_ref());
            check::object(
                "options.horizonOcclusionPoint",
                options.horizon_occlusion_point.as_ref(),
            );
            check::object("options.westIndices", options.west_indices.as_ref());
            check::object("options.southIndices", options.south_indices.as_ref());
            check::object("options.eastIndices", options.east_indices.as_ref());
            check::object("options.northIndices", options.north_indices.as_ref());
            check::number("options.westSkirtHeight", options.west_skirt_height);
            check::number("options.southSkirtHeight", options.south_skirt_height);
            check::number("options.eastSkirtHeight", options.east_skirt_height);
            check::number("options.northSkirtHeight", options.north_skirt_height);
        }
        //>>includeEnd('debug');

        let quantized_vertices = options.quantized_vertices.unwrap_or_default();
        let vertex_count = quantized_vertices.len() / 3;
        let u_values = quantized_vertices[0..vertex_count].to_vec();
        let v_values = quantized_vertices[vertex_count..2 * vertex_count].to_vec();
        let height_values = quantized_vertices[2 * vertex_count..3 * vertex_count].to_vec();

        // We don't assume that we can count on the edge vertices being
        // sorted by u or v.
        let west_indices = sort_indices_if_necessary(
            &options.west_indices.unwrap_or_default(),
            &v_values,
        );
        let south_indices = sort_indices_if_necessary(
            &options.south_indices.unwrap_or_default(),
            &u_values,
        );
        let east_indices = sort_indices_if_necessary(
            &options.east_indices.unwrap_or_default(),
            &v_values,
        );
        let north_indices = sort_indices_if_necessary(
            &options.north_indices.unwrap_or_default(),
            &u_values,
        );

        Self {
            quantized_vertices: Some(quantized_vertices),
            encoded_normals: options.encoded_normals,
            indices: Some(options.indices.unwrap_or_default()),
            minimum_height: options.minimum_height.unwrap_or(0.0),
            maximum_height: options.maximum_height.unwrap_or(0.0),
            bounding_sphere: options.bounding_sphere.unwrap_or_else(|| {
                BoundingSphere::new(Cartesian3::ZERO, 0.0)
            }),
            oriented_bounding_box: options.oriented_bounding_box,
            horizon_occlusion_point: options
                .horizon_occlusion_point
                .unwrap_or(Cartesian3::ZERO),
            credits: options.credits,
            u_values: Some(u_values),
            v_values: Some(v_values),
            height_values: Some(height_values),
            west_indices,
            south_indices,
            east_indices,
            north_indices,
            west_skirt_height: options.west_skirt_height.unwrap_or(0.0),
            south_skirt_height: options.south_skirt_height.unwrap_or(0.0),
            east_skirt_height: options.east_skirt_height.unwrap_or(0.0),
            north_skirt_height: options.north_skirt_height.unwrap_or(0.0),
            child_tile_mask: options.child_tile_mask.unwrap_or(15),
            created_by_upsampling: options.created_by_upsampling.unwrap_or(false),
            water_mask: options.water_mask,
            mesh: None,
        }
    }

    /// An array of credits for this tile.
    pub fn credits(&self) -> Option<&Vec<String>> {
        self.credits.as_ref()
    }

    /// The water mask included in this terrain data, if any.
    pub fn water_mask(&self) -> Option<&Vec<u8>> {
        self.water_mask.as_ref()
    }

    /// A bit mask indicating which of this tile's four children exist.
    pub fn child_tile_mask(&self) -> i32 {
        self.child_tile_mask
    }

    /// True once a mesh has been created and upsampling is possible.
    pub fn can_upsample(&self) -> bool {
        self.mesh.is_some()
    }

    /// The minimum terrain height within the tile, in meters.
    pub fn minimum_height(&self) -> f64 {
        self.minimum_height
    }

    /// The maximum terrain height within the tile, in meters.
    pub fn maximum_height(&self) -> f64 {
        self.maximum_height
    }

    /// The quantized vertex buffer (`u`, then `v`, then heights).
    pub fn quantized_vertices(&self) -> Option<&Vec<u16>> {
        self.quantized_vertices.as_ref()
    }

    /// The per-vertex oct-encoded normals, if any.
    pub fn encoded_normals(&self) -> Option<&Vec<u8>> {
        self.encoded_normals.as_ref()
    }

    /// The triangle indices.
    pub fn indices(&self) -> Option<&Vec<u32>> {
        self.indices.as_ref()
    }

    /// The bounding sphere of the mesh.
    pub fn bounding_sphere(&self) -> &BoundingSphere {
        &self.bounding_sphere
    }

    /// The u components of the quantized vertices.
    pub fn u_values(&self) -> Option<&Vec<u16>> {
        self.u_values.as_ref()
    }

    /// The v components of the quantized vertices.
    pub fn v_values(&self) -> Option<&Vec<u16>> {
        self.v_values.as_ref()
    }

    /// The quantized height components of the vertices.
    pub fn height_values(&self) -> Option<&Vec<u16>> {
        self.height_values.as_ref()
    }

    /// The indices of the vertices on the western edge, sorted by v.
    pub fn west_indices(&self) -> &Vec<u32> {
        &self.west_indices
    }

    /// The indices of the vertices on the southern edge, sorted by u.
    pub fn south_indices(&self) -> &Vec<u32> {
        &self.south_indices
    }

    /// The indices of the vertices on the eastern edge, sorted by v.
    pub fn east_indices(&self) -> &Vec<u32> {
        &self.east_indices
    }

    /// The indices of the vertices on the northern edge, sorted by u.
    pub fn north_indices(&self) -> &Vec<u32> {
        &self.north_indices
    }

    /// The mesh created by `create_mesh`, if any.
    pub fn mesh(&self) -> Option<&TerrainMesh> {
        self.mesh.as_ref()
    }

    /// Creates a [`TerrainMesh`] from this terrain data.
    ///
    /// Mirrors `createMesh`. Returns `None` when throttling is enabled and
    /// [`TerrainData::MAXIMUM_ASYNCHRONOUS_TASKS`] creations are already in
    /// progress (JS returns `undefined`, "Postponed"). Awaiting the returned
    /// future stores the mesh in `self` and frees the server-side buffers.
    pub fn create_mesh(
        &mut self,
        options: CreateMeshOptions<'_>,
    ) -> Option<impl std::future::Future<Output = ()> + '_> {
        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            check::object("options.tilingScheme", Some(&options.tiling_scheme));
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
            if *active >= <Self as TerrainData>::MAXIMUM_ASYNCHRONOUS_TASKS {
                // Postponed
                return None;
            }
            *active += 1;
            Some(MeshTaskPermit)
        } else {
            None
        };

        let ellipsoid = *tiling_scheme.ellipsoid();
        let mut rectangle = Rectangle::default();
        tiling_scheme.tile_xy_to_rectangle(x, y, level, &mut rectangle);

        // Mirrors the scheduled worker task parameters.
        let params = CreateVerticesParams {
            minimum_height: self.minimum_height,
            maximum_height: self.maximum_height,
            quantized_vertices: self
                .quantized_vertices
                .clone()
                .expect("createMesh requires the quantized vertices to still be defined"),
            oct_encoded_normals: self.encoded_normals.clone(),
            indices: self.indices.clone().unwrap_or_default(),
            west_indices: self.west_indices.clone(),
            south_indices: self.south_indices.clone(),
            east_indices: self.east_indices.clone(),
            north_indices: self.north_indices.clone(),
            west_skirt_height: self.west_skirt_height,
            south_skirt_height: self.south_skirt_height,
            east_skirt_height: self.east_skirt_height,
            north_skirt_height: self.north_skirt_height,
            rectangle,
            center: self.bounding_sphere.center,
            ellipsoid,
            exaggeration,
            exaggeration_relative_height,
        };

        let mut mesh = create_vertices_from_quantized_terrain_mesh(&params);

        // Mirrors the promise continuation: clone complex result objects and
        // fill the mesh metadata from `this`.
        mesh.bounding_sphere_3d = self.bounding_sphere.clone();
        mesh.oriented_bounding_box = self.oriented_bounding_box.clone();
        // JS: `occludeePointInScaledSpace ?? this._horizonOcclusionPoint`;
        // the worker only recomputes the point when `minimumHeight < 0`.
        if self.minimum_height >= 0.0 {
            mesh.occludee_point_in_scaled_space = self.horizon_occlusion_point;
        }

        let ready = std::future::ready(mesh);
        Some(async move {
            let mesh = ready.await;
            self.mesh = Some(mesh);

            // Free memory received from server after mesh is created.
            self.quantized_vertices = None;
            self.encoded_normals = None;
            self.indices = None;
            self.u_values = None;
            self.v_values = None;
            self.height_values = None;
            self.west_indices = Vec::new();
            self.south_indices = Vec::new();
            self.east_indices = Vec::new();
            self.north_indices = Vec::new();

            drop(permit);
        })
    }

    /// Upsamples this terrain data for use by a descendant tile.
    ///
    /// Mirrors `upsample`. Returns `None` when no mesh exists yet or when
    /// [`TerrainData::MAXIMUM_ASYNCHRONOUS_TASKS`] upsample operations are
    /// already in progress (JS returns `undefined` in both cases). Awaiting
    /// the returned future yields the upsampled
    /// [`QuantizedMeshTerrainData`].
    #[allow(clippy::too_many_arguments)]
    pub fn upsample(
        &self,
        tiling_scheme: &dyn TilingScheme,
        this_x: i32,
        this_y: i32,
        this_level: i32,
        descendant_x: i32,
        descendant_y: i32,
        descendant_level: i32,
    ) -> Option<impl std::future::Future<Output = QuantizedMeshTerrainData> + '_> {
        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            // All parameters are required by the Rust signature; JS checks
            // them for definedness only.
            let level_difference = descendant_level - this_level;
            if level_difference > 1 {
                throw_developer_error(
                    "Upsampling through more than one level at a time is not currently supported.",
                );
            }
        }
        //>>includeEnd('debug');

        let mesh = self.mesh.as_ref()?;

        // The upsample TaskProcessor is always throttled in JS.
        let permit = {
            let mut active = active_upsample_tasks().lock().unwrap_or_else(|e| e.into_inner());
            if *active >= <Self as TerrainData>::MAXIMUM_ASYNCHRONOUS_TASKS {
                // Postponed
                return None;
            }
            *active += 1;
            UpsampleTaskPermit
        };

        let is_east_child = this_x * 2 != descendant_x;
        let is_north_child = this_y * 2 == descendant_y;

        let ellipsoid = *tiling_scheme.ellipsoid();
        let mut child_rectangle = Rectangle::default();
        tiling_scheme.tile_xy_to_rectangle(
            descendant_x,
            descendant_y,
            descendant_level,
            &mut child_rectangle,
        );

        let params = UpsampleQuantizedMeshParams {
            vertices: mesh.vertices.clone(),
            vertex_count_without_skirts: mesh.vertex_count_without_skirts,
            indices: mesh.indices.clone(),
            index_count_without_skirts: mesh.index_count_without_skirts,
            stride: mesh.stride,
            has_vertex_normals: mesh.encoding.has_vertex_normals,
            minimum_height: self.minimum_height,
            maximum_height: self.maximum_height,
            is_east_child,
            is_north_child,
            child_rectangle,
            ellipsoid,
        };

        let mut shortest_skirt = self.west_skirt_height.min(self.east_skirt_height);
        shortest_skirt = shortest_skirt.min(self.south_skirt_height);
        shortest_skirt = shortest_skirt.min(self.north_skirt_height);

        let west_skirt_height = if is_east_child {
            shortest_skirt * 0.5
        } else {
            self.west_skirt_height
        };
        let south_skirt_height = if is_north_child {
            shortest_skirt * 0.5
        } else {
            self.south_skirt_height
        };
        let east_skirt_height = if is_east_child {
            self.east_skirt_height
        } else {
            shortest_skirt * 0.5
        };
        let north_skirt_height = if is_north_child {
            self.north_skirt_height
        } else {
            shortest_skirt * 0.5
        };
        let credits = self.credits.clone();

        let result = upsample_quantized_terrain_mesh(&params);

        let ready = std::future::ready(result);
        Some(async move {
            let result = ready.await;
            let data = QuantizedMeshTerrainData::new(QuantizedMeshTerrainDataOptions {
                quantized_vertices: Some(result.quantized_vertices),
                indices: Some(result.indices),
                encoded_normals: result.encoded_normals,
                minimum_height: Some(result.minimum_height),
                maximum_height: Some(result.maximum_height),
                bounding_sphere: Some(result.bounding_sphere),
                oriented_bounding_box: Some(result.oriented_bounding_box),
                horizon_occlusion_point: Some(result.horizon_occlusion_point),
                west_indices: Some(result.west_indices),
                south_indices: Some(result.south_indices),
                east_indices: Some(result.east_indices),
                north_indices: Some(result.north_indices),
                west_skirt_height: Some(west_skirt_height),
                south_skirt_height: Some(south_skirt_height),
                east_skirt_height: Some(east_skirt_height),
                north_skirt_height: Some(north_skirt_height),
                child_tile_mask: None,
                created_by_upsampling: Some(true),
                water_mask: None,
                credits,
            });
            drop(permit);
            data
        })
    }

    /// Computes the terrain height at a specified longitude and latitude.
    /// The position is clamped to the rectangle, so expect incorrect results
    /// for positions far outside the rectangle.
    ///
    /// Mirrors `interpolateHeight`; `undefined` (position not inside any
    /// triangle) maps to `None`.
    pub fn interpolate_height(
        &self,
        rectangle: &Rectangle,
        longitude: f64,
        latitude: f64,
    ) -> Option<f64> {
        let rectangle_width = rectangle.east - rectangle.west;
        let rectangle_height = rectangle.north - rectangle.south;

        let mut u = CesiumMath::clamp((longitude - rectangle.west) / rectangle_width, 0.0, 1.0);
        u *= MAX_SHORT;
        let mut v = CesiumMath::clamp((latitude - rectangle.south) / rectangle_height, 0.0, 1.0);
        v *= MAX_SHORT;

        if self.mesh.is_none() {
            return interpolate_quantized_height(self, u, v);
        }

        interpolate_mesh_height(self, u, v)
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
            check::number("thisX", this_x.map(|v| v as f64));
            check::number("thisY", this_y.map(|v| v as f64));
            check::number("childX", child_x.map(|v| v as f64));
            check::number("childY", child_y.map(|v| v as f64));
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

impl TerrainData for QuantizedMeshTerrainData {
    fn interpolate_height(&self, rectangle: &Rectangle, longitude: f64, latitude: f64) -> f64 {
        // DEVIATION: the trait cannot express JS `undefined`; positions that
        // do not lie in any triangle yield NaN.
        Self::interpolate_height(self, rectangle, longitude, latitude).unwrap_or(f64::NAN)
    }

    fn is_child_available(&self, this_x: i32, this_y: i32, child_x: i32, child_y: i32) -> bool {
        Self::is_child_available(self, Some(this_x), Some(this_y), Some(child_x), Some(child_y))
    }

    fn was_created_by_upsampling(&self) -> bool {
        self.created_by_upsampling
    }
}

// ── Private helpers ────────────────────────────────────────────────────

// ── Throttling (mirrors the module-level TaskProcessor slots) ──────────

static ACTIVE_MESH_TASKS: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));

fn active_mesh_tasks() -> &'static Mutex<usize> {
    &ACTIVE_MESH_TASKS
}

static ACTIVE_UPSAMPLE_TASKS: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));

fn active_upsample_tasks() -> &'static Mutex<usize> {
    &ACTIVE_UPSAMPLE_TASKS
}

/// RAII decrement of the active create-mesh task counter, mirroring the
/// release of a throttled `TaskProcessor` slot when the promise settles.
struct MeshTaskPermit;

impl Drop for MeshTaskPermit {
    fn drop(&mut self) {
        let mut active = ACTIVE_MESH_TASKS.lock().unwrap_or_else(|e| e.into_inner());
        *active = active.saturating_sub(1);
    }
}

/// RAII decrement of the active upsample task counter.
struct UpsampleTaskPermit;

impl Drop for UpsampleTaskPermit {
    fn drop(&mut self) {
        let mut active = ACTIVE_UPSAMPLE_TASKS.lock().unwrap_or_else(|e| e.into_inner());
        *active = active.saturating_sub(1);
    }
}

/// Mirrors `sortIndicesIfNecessary`: sorts edge indices by their u (or v)
/// value when they are not already sorted.
fn sort_indices_if_necessary(indices: &[u32], sort_values: &[u16]) -> Vec<u32> {
    let mut needs_sort = false;
    for i in 1..indices.len() {
        if sort_values[indices[i - 1] as usize] > sort_values[indices[i] as usize] {
            needs_sort = true;
            break;
        }
    }

    if needs_sort {
        let mut sorted = indices.to_vec();
        sorted.sort_by_key(|index| sort_values[*index as usize]);
        return sorted;
    }
    indices.to_vec()
}

/// Mirrors the private `pointInBoundingBox` function.
fn point_in_bounding_box(u: f64, v: f64, u0: f64, v0: f64, u1: f64, v1: f64, u2: f64, v2: f64) -> bool {
    let min_u = u0.min(u1).min(u2);
    let max_u = u0.max(u1).max(u2);
    let min_v = v0.min(v1).min(v2);
    let max_v = v0.max(v1).max(v2);
    u >= min_u && u <= max_u && v >= min_v && v <= max_v
}

/// Mirrors the private `interpolateMeshHeight` function: finds the triangle
/// containing (u, v) in the mesh and barycentrically interpolates its
/// decoded heights.
fn interpolate_mesh_height(terrain_data: &QuantizedMeshTerrainData, u: f64, v: f64) -> Option<f64> {
    let mesh = terrain_data.mesh.as_ref()?;
    let vertices = &mesh.vertices;
    let encoding = &mesh.encoding;
    let indices = &mesh.indices;

    let mut uv0 = Cartesian2::default();
    let mut uv1 = Cartesian2::default();
    let mut uv2 = Cartesian2::default();

    let mut i = 0;
    while i < indices.len() {
        let i0 = indices[i] as usize;
        let i1 = indices[i + 1] as usize;
        let i2 = indices[i + 2] as usize;

        encoding.decode_texture_coordinates(vertices, i0, &mut uv0);
        encoding.decode_texture_coordinates(vertices, i1, &mut uv1);
        encoding.decode_texture_coordinates(vertices, i2, &mut uv2);

        if point_in_bounding_box(u, v, uv0.x, uv0.y, uv1.x, uv1.y, uv2.x, uv2.y) {
            let barycentric = Intersections2D::compute_barycentric_coordinates(
                u, v, uv0.x, uv0.y, uv1.x, uv1.y, uv2.x, uv2.y,
            );
            if barycentric.x >= -1e-15 && barycentric.y >= -1e-15 && barycentric.z >= -1e-15 {
                let h0 = encoding.decode_height(vertices, i0);
                let h1 = encoding.decode_height(vertices, i1);
                let h2 = encoding.decode_height(vertices, i2);
                return Some(barycentric.x * h0 + barycentric.y * h1 + barycentric.z * h2);
            }
        }

        i += 3;
    }

    // Position does not lie in any triangle in this mesh.
    None
}

/// Mirrors the private `interpolateHeight` function: finds the triangle
/// containing (u, v) among the quantized vertices and barycentrically
/// interpolates the quantized heights, then dequantizes.
fn interpolate_quantized_height(
    terrain_data: &QuantizedMeshTerrainData,
    u: f64,
    v: f64,
) -> Option<f64> {
    let u_buffer = terrain_data.u_values.as_ref()?;
    let v_buffer = terrain_data.v_values.as_ref()?;
    let height_buffer = terrain_data.height_values.as_ref()?;

    let indices = terrain_data.indices.as_ref()?;

    let mut i = 0;
    while i < indices.len() {
        let i0 = indices[i] as usize;
        let i1 = indices[i + 1] as usize;
        let i2 = indices[i + 2] as usize;

        let u0 = u_buffer[i0] as f64;
        let u1 = u_buffer[i1] as f64;
        let u2 = u_buffer[i2] as f64;

        let v0 = v_buffer[i0] as f64;
        let v1 = v_buffer[i1] as f64;
        let v2 = v_buffer[i2] as f64;

        if point_in_bounding_box(u, v, u0, v0, u1, v1, u2, v2) {
            let barycentric = Intersections2D::compute_barycentric_coordinates(
                u, v, u0, v0, u1, v1, u2, v2,
            );
            if barycentric.x >= -1e-15 && barycentric.y >= -1e-15 && barycentric.z >= -1e-15 {
                let quantized_height = barycentric.x * height_buffer[i0] as f64
                    + barycentric.y * height_buffer[i1] as f64
                    + barycentric.z * height_buffer[i2] as f64;
                return Some(CesiumMath::lerp(
                    terrain_data.minimum_height,
                    terrain_data.maximum_height,
                    quantized_height / MAX_SHORT,
                ));
            }
        }

        i += 3;
    }

    // Position does not lie in any triangle in this mesh.
    None
}
