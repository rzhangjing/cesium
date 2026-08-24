//! Ported from `packages/engine/Source/Core/GoogleEarthEnterpriseTerrainData.js`
//! (plus the tessellation from
//! `packages/engine/Source/Workers/createVerticesFromGoogleEarthEnterpriseBuffer.js`).
//!
//! # Alignment table
//!
//! | JS | Rust | Notes |
//! |---|---|---|
//! | `GoogleEarthEnterpriseTerrainData` constructor | [`GoogleEarthEnterpriseTerrainData::new`] | identical child-mask conversion |
//! | `credits` / `waterMask` properties | [`GoogleEarthEnterpriseTerrainData::credits`] / [`GoogleEarthEnterpriseTerrainData::water_mask`] | |
//! | `createMesh` | [`GoogleEarthEnterpriseTerrainData::create_mesh`] | worker inlined (DEVIATION 1) |
//! | `processBuffer` (worker) | [`process_buffer`] | identical parsing/tessellation |
//! | `addSkirt` (worker) | [`add_skirt`] | identical |
//! | `interpolateHeight` | [`GoogleEarthEnterpriseTerrainData::interpolate_height`] | `undefined` → `None` |
//! | `interpolateHeight` (buffer path, private) | [`interpolate_buffer_height`] | identical |
//! | `interpolateMeshHeight` (private) | [`interpolate_mesh_height`] | identical |
//! | `upsample` | [`GoogleEarthEnterpriseTerrainData::upsample`] | DEVIATION 5 |
//! | `isChildAvailable` | [`GoogleEarthEnterpriseTerrainData::is_child_available`] | identical |
//! | `wasCreatedByUpsampling` | [`GoogleEarthEnterpriseTerrainData::was_created_by_upsampling`] | identical |
//!
//! # DEVIATIONS
//!
//! 1. `createMesh` schedules `createVerticesFromGoogleEarthEnterpriseBuffer`
//!    through a web worker in JS. Rust computes the mesh synchronously
//!    in-process (the tessellator is ported below); throttling mirrors the
//!    JS `maximumAsynchronousTasks` via a global active-task counter. The
//!    returned future is ready immediately; awaiting it stores the mesh in
//!    `self` and frees the buffer.
//! 2. The simplified [`TerrainEncoding`] has no webMercatorT / geodetic
//!    surface normal slots (JS stride 6+1+3 with those); vertex slots are
//!    `[X, Y, Z, H, U, V]`.
//! 3. Vertex positions are stored as absolute ECEF (JS encodes
//!    RTC-relative and `decodePosition` adds the center back; the decoded
//!    behavior is equivalent). The horizon occlusion point is left at ZERO
//!    (`EllipsoidalOccluder.computeHorizonCullingPointPossiblyUnderEllipsoid`
//!    is not ported yet).
//! 4. `upsample` requires the `upsampleQuantizedTerrainMesh` worker; it is
//!    materialized alongside `QuantizedMeshTerrainData` (see that module) and
//!    wired here.
//! 5. JS typed arrays are modeled with `Vec<u8>` / `Vec<u32>`; the buffer
//!    (`ArrayBuffer`) is a `Vec<u8>`.

use std::sync::{LazyLock, Mutex};

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::check;
use crate::credit::Credit;
use crate::ellipsoid::Ellipsoid;
use crate::intersections2d::Intersections2D;
use crate::math::CesiumMath;
use crate::matrix4::Matrix4;
use crate::oriented_bounding_box::OrientedBoundingBox;
use crate::rectangle::Rectangle;
use crate::runtime_error::RuntimeError;
use crate::terrain_data::TerrainData;
use crate::terrain_encoding::TerrainEncoding;
use crate::terrain_mesh::TerrainMesh;
use crate::tiling_scheme::TilingScheme;
use crate::transforms;

/// Mirrors the `options` parameter of the JS constructor.
#[derive(Default)]
pub struct GoogleEarthEnterpriseTerrainDataOptions {
    /// The buffer containing terrain data.
    pub buffer: Option<Vec<u8>>,
    /// Multiplier for negative terrain heights that are encoded as very small
    /// positive values.
    pub negative_altitude_exponent_bias: Option<f64>,
    /// Threshold for negative values.
    pub negative_elevation_threshold: Option<f64>,
    /// A bit mask indicating which of this tile's four children exist
    /// (Google child layout). Default 15.
    pub child_tile_mask: Option<u32>,
    /// True if this instance was created by upsampling another instance.
    pub created_by_upsampling: Option<bool>,
    /// Array of credits for this tile.
    pub credits: Option<Vec<Credit>>,
}

/// Terrain data for a single tile from a Google Earth Enterprise server.
pub struct GoogleEarthEnterpriseTerrainData {
    buffer: Option<Vec<u8>>,
    credits: Option<Vec<Credit>>,
    negative_altitude_exponent_bias: f64,
    negative_elevation_threshold: f64,
    child_tile_mask: u32,
    created_by_upsampling: bool,
    skirt_height: Option<f64>,
    mesh: Option<TerrainMesh>,
    minimum_height: Option<f64>,
    maximum_height: Option<f64>,
}

impl GoogleEarthEnterpriseTerrainData {
    /// Mirrors the JS constructor.
    pub fn new(options: GoogleEarthEnterpriseTerrainDataOptions) -> Self {
        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            check::type_of::object("options.buffer", options.buffer.as_ref());
            check::type_of::number(
                "options.negativeAltitudeExponentBias",
                options.negative_altitude_exponent_bias,
            );
            check::type_of::number(
                "options.negativeElevationThreshold",
                options.negative_elevation_threshold,
            );
        }
        //>>includeEnd('debug');

        // Convert from google layout to layout of other providers
        // 3 2 -> 2 3
        // 0 1 -> 0 1
        let google_child_tile_mask = options.child_tile_mask.unwrap_or(15);
        let mut child_tile_mask = google_child_tile_mask & 3; // Bottom row is identical
        child_tile_mask |= if google_child_tile_mask & 4 != 0 { 8 } else { 0 }; // NE
        child_tile_mask |= if google_child_tile_mask & 8 != 0 { 4 } else { 0 }; // NW

        Self {
            buffer: options.buffer,
            credits: options.credits,
            negative_altitude_exponent_bias: options.negative_altitude_exponent_bias.unwrap_or(0.0),
            negative_elevation_threshold: options.negative_elevation_threshold.unwrap_or(0.0),
            child_tile_mask,
            created_by_upsampling: options.created_by_upsampling.unwrap_or(false),
            skirt_height: None,
            mesh: None,
            minimum_height: None,
            maximum_height: None,
        }
    }

    /// An array of credits for this tile.
    pub fn credits(&self) -> Option<&Vec<Credit>> {
        self.credits.as_ref()
    }

    /// The water mask included in this terrain data, if any. Always `None`
    /// (JS returns `undefined`).
    pub fn water_mask(&self) -> Option<()> {
        None
    }

    /// The child tile mask (converted to the standard layout).
    pub fn child_tile_mask(&self) -> u32 {
        self.child_tile_mask
    }

    /// The mesh created by `create_mesh`, if any.
    pub fn mesh(&self) -> Option<&TerrainMesh> {
        self.mesh.as_ref()
    }

    /// The raw terrain buffer (`None` after `create_mesh` frees it).
    pub fn buffer(&self) -> Option<&Vec<u8>> {
        self.buffer.as_ref()
    }

    /// The skirt height computed by the last `create_mesh` call, if any.
    pub fn skirt_height(&self) -> Option<f64> {
        self.skirt_height
    }

    /// The minimum height recorded by the last `create_mesh` call, if any.
    pub fn minimum_height(&self) -> Option<f64> {
        self.minimum_height
    }

    /// The maximum height recorded by the last `create_mesh` call, if any.
    pub fn maximum_height(&self) -> Option<f64> {
        self.maximum_height
    }

    /// Mirrors `createMesh` (DEVIATION 1). Returns `None` when throttling is
    /// enabled and [`TerrainData::MAXIMUM_ASYNCHRONOUS_TASKS`] creations are
    /// already in progress (JS returns `undefined`, "Postponed"). Awaiting
    /// the returned future stores the mesh in `self` and frees the buffer.
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

        // Compute the center of the tile for RTC rendering.
        let center_cartographic = Rectangle::center(&rectangle);
        let mut center = Cartesian3::default();
        ellipsoid.cartographic_to_cartesian(&center_cartographic, &mut center);

        let level_zero_max_error = 40075.16; // From Google's Doc
        let this_level_max_error = level_zero_max_error / (1 << level) as f64;
        self.skirt_height = Some((this_level_max_error * 8.0).min(1000.0));

        let buffer = self
            .buffer
            .clone()
            .expect("createMesh requires the terrain buffer to still be defined");
        let negative_altitude_exponent_bias = self.negative_altitude_exponent_bias;
        let negative_elevation_threshold = self.negative_elevation_threshold;

        let mesh = process_buffer(
            &buffer,
            &center,
            &ellipsoid,
            &rectangle,
            exaggeration,
            exaggeration_relative_height,
            self.skirt_height.unwrap(),
            negative_altitude_exponent_bias,
            negative_elevation_threshold,
        )
        .expect("terrain buffer must be well formed");

        let ready = std::future::ready(mesh);
        Some(async move {
            let mesh = ready.await;
            self.minimum_height = Some(mesh.minimum_height);
            self.maximum_height = Some(mesh.maximum_height);
            self.mesh = Some(mesh);
            // Free memory received from server after mesh is created.
            self.buffer = None;
            drop(permit);
        })
    }

    /// Computes the terrain height at a specified longitude and latitude.
    ///
    /// Mirrors `interpolateHeight`. Returns `None` when the position does not
    /// lie in any triangle (JS `undefined`).
    pub fn interpolate_height(
        &self,
        rectangle: &Rectangle,
        longitude: f64,
        latitude: f64,
    ) -> Option<f64> {
        let u = CesiumMath::clamp((longitude - rectangle.west) / rectangle.width(), 0.0, 1.0);
        let v = CesiumMath::clamp((latitude - rectangle.south) / rectangle.height(), 0.0, 1.0);

        if self.mesh.is_none() {
            return interpolate_buffer_height(self, u, v, rectangle);
        }

        interpolate_mesh_height(self, u, v)
    }

    /// Upsamples this terrain data for use by a descendant tile (DEVIATION 4:
    /// requires the `upsampleQuantizedTerrainMesh` worker port; wired once
    /// that lands).
    pub fn upsample(
        &self,
        _tiling_scheme: &dyn TilingScheme,
        _this_x: i32,
        _this_y: i32,
        _this_level: i32,
        _descendant_x: i32,
        _descendant_y: i32,
        _descendant_level: i32,
    ) -> Option<()> {
        // DEVIATION 4: stub until the upsampleQuantizedTerrainMesh worker
        // port lands; JS also returns `undefined` when no mesh exists yet.
        None
    }

    /// Determines if a given child tile is available.
    ///
    /// Mirrors `isChildAvailable`.
    pub fn is_child_available(&self, this_x: i32, this_y: i32, child_x: i32, child_y: i32) -> bool {
        let mut bit_number = 2i32; // northwest child
        if child_x != this_x * 2 {
            bit_number += 1; // east child
        }
        if child_y != this_y * 2 {
            bit_number -= 2; // south child
        }

        (self.child_tile_mask & (1 << bit_number)) != 0
    }

    /// Gets a value indicating whether or not this terrain data was created
    /// by upsampling lower resolution terrain data.
    pub fn was_created_by_upsampling(&self) -> bool {
        self.created_by_upsampling
    }
}

impl TerrainData for GoogleEarthEnterpriseTerrainData {
    fn interpolate_height(&self, rectangle: &Rectangle, longitude: f64, latitude: f64) -> f64 {
        // DEVIATION: the trait cannot express JS `undefined`; positions that
        // lie in no triangle yield NaN.
        Self::interpolate_height(self, rectangle, longitude, latitude).unwrap_or(f64::NAN)
    }

    fn is_child_available(&self, this_x: i32, this_y: i32, child_x: i32, child_y: i32) -> bool {
        Self::is_child_available(self, this_x, this_y, child_x, child_y)
    }

    fn was_created_by_upsampling(&self) -> bool {
        self.created_by_upsampling
    }
}

/// Mirrors the `options` of `createMesh`.
pub struct CreateMeshOptions<'a> {
    /// The tiling scheme to which this tile belongs.
    pub tiling_scheme: &'a dyn TilingScheme,
    /// The X coordinate of the tile.
    pub x: i32,
    /// The Y coordinate of the tile.
    pub y: i32,
    /// The level of the tile.
    pub level: i32,
    /// The scale used to exaggerate the terrain.
    pub exaggeration: Option<f64>,
    /// The height from which terrain is exaggerated.
    pub exaggeration_relative_height: Option<f64>,
    /// If true, indicates that this operation will need to be retried if too
    /// many asynchronous mesh creations are already in progress.
    pub throttle: Option<bool>,
}

// ── Throttling (mirrors the module-level TaskProcessor slots) ──────────

static ACTIVE_MESH_TASKS: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));

fn active_mesh_tasks() -> &'static Mutex<usize> {
    &ACTIVE_MESH_TASKS
}

/// RAII decrement of the active mesh-task counter, mirroring the release of
/// a throttled `TaskProcessor` slot when the worker promise settles.
struct MeshTaskPermit;

impl Drop for MeshTaskPermit {
    fn drop(&mut self) {
        let mut active = ACTIVE_MESH_TASKS.lock().unwrap_or_else(|e| e.into_inner());
        *active = active.saturating_sub(1);
    }
}

// ── Buffer readers (DataView little-endian equivalents) ────────────────

fn dv_u8(buffer: &[u8], offset: usize) -> Result<u8, RuntimeError> {
    buffer
        .get(offset)
        .copied()
        .ok_or_else(|| RuntimeError::new(Some("Invalid terrain tile.")))
}

fn dv_u16(buffer: &[u8], offset: usize) -> Result<u16, RuntimeError> {
    let bytes: [u8; 2] = buffer
        .get(offset..offset + 2)
        .ok_or_else(|| RuntimeError::new(Some("Invalid terrain tile.")))?
        .try_into()
        .unwrap();
    Ok(u16::from_le_bytes(bytes))
}

fn dv_i32(buffer: &[u8], offset: usize) -> Result<i32, RuntimeError> {
    let bytes: [u8; 4] = buffer
        .get(offset..offset + 4)
        .ok_or_else(|| RuntimeError::new(Some("Invalid terrain tile.")))?
        .try_into()
        .unwrap();
    Ok(i32::from_le_bytes(bytes))
}

fn dv_u32(buffer: &[u8], offset: usize) -> Result<u32, RuntimeError> {
    let bytes: [u8; 4] = buffer
        .get(offset..offset + 4)
        .ok_or_else(|| RuntimeError::new(Some("Invalid terrain tile.")))?
        .try_into()
        .unwrap();
    Ok(u32::from_le_bytes(bytes))
}

fn dv_f32(buffer: &[u8], offset: usize) -> Result<f32, RuntimeError> {
    let bytes: [u8; 4] = buffer
        .get(offset..offset + 4)
        .ok_or_else(|| RuntimeError::new(Some("Invalid terrain tile.")))?
        .try_into()
        .unwrap();
    Ok(f32::from_le_bytes(bytes))
}

fn dv_f64(buffer: &[u8], offset: usize) -> Result<f64, RuntimeError> {
    let bytes: [u8; 8] = buffer
        .get(offset..offset + 8)
        .ok_or_else(|| RuntimeError::new(Some("Invalid terrain tile.")))?
        .try_into()
        .unwrap();
    Ok(f64::from_le_bytes(bytes))
}

// ── Tessellator (port of the worker's processBuffer) ───────────────────

fn index_of_epsilon(values: &[f64], value: f64) -> Option<usize> {
    values
        .iter()
        .position(|v| CesiumMath::equals_epsilon(*v, value, Some(CesiumMath::EPSILON12), None))
}

/// A border point tracked while tessellating (mirrors the worker's
/// `{ index, cartographic }` entries).
#[derive(Clone)]
struct BorderPoint {
    index: usize,
    cartographic: Cartographic,
}

/// Mirrors the worker's `processBuffer`. Builds the full terrain mesh
/// (regular vertices + skirt vertices) for one GEE terrain quad buffer.
#[allow(clippy::too_many_arguments)]
fn process_buffer(
    buffer: &[u8],
    relative_to_center: &Cartesian3,
    ellipsoid: &Ellipsoid,
    rectangle: &Rectangle,
    exaggeration: f64,
    exaggeration_relative_height: f64,
    skirt_height: f64,
    negative_altitude_exponent_bias: f64,
    negative_elevation_threshold: f64,
) -> Result<TerrainMesh, RuntimeError> {
    let geographic_west = rectangle.west;
    let geographic_south = rectangle.south;
    let geographic_east = rectangle.east;
    let geographic_north = rectangle.north;
    let rectangle_width = rectangle.width();
    let rectangle_height = rectangle.height();

    // Keep track of quad borders so we can remove duplicates around the
    // borders
    let mut quad_border_latitudes = vec![geographic_south, geographic_north];
    let mut quad_border_longitudes = vec![geographic_west, geographic_east];

    let from_enu = transforms::east_north_up_to_fixed_frame_new(relative_to_center, Some(ellipsoid));
    let mut to_enu = Matrix4::default();
    Matrix4::inverse_transformation(&from_enu, &mut to_enu);

    let mut min_height = f64::INFINITY;
    let mut max_height = f64::NEG_INFINITY;

    // Compute sizes
    let mut offset = 0usize;
    let mut size = 0usize;
    let mut indices_size = 0usize;
    for _quad in 0..4 {
        let mut o = offset;
        let quad_size = dv_u32(buffer, o)? as usize;
        o += 4;

        let x = CesiumMath::to_radians(dv_f64(buffer, o)? * 180.0);
        o += 8;
        if index_of_epsilon(&quad_border_longitudes, x).is_none() {
            quad_border_longitudes.push(x);
        }

        let y = CesiumMath::to_radians(dv_f64(buffer, o)? * 180.0);
        o += 8;
        if index_of_epsilon(&quad_border_latitudes, y).is_none() {
            quad_border_latitudes.push(y);
        }

        o += 2 * 8; // stepX + stepY

        let mut c = dv_i32(buffer, o)?; // Read point count
        o += 4;
        size += c.max(0) as usize;

        c = dv_i32(buffer, o)?; // Read index count
        indices_size += (c.max(0) as usize) * 3;

        offset += quad_size + 4; // Jump to next quad
    }

    // Quad Border points to remove duplicates
    let mut quad_border_points: Vec<Cartographic> = Vec::new();
    let mut quad_border_indices: Vec<usize> = Vec::new();

    // Create arrays
    let mut positions: Vec<Cartesian3> = Vec::with_capacity(size);
    let mut uvs: Vec<Cartesian2> = Vec::with_capacity(size);
    let mut heights: Vec<f64> = Vec::with_capacity(size);
    let mut indices: Vec<u32> = Vec::with_capacity(indices_size);

    // Points are laid out in rows starting at SW, so storing border points
    // as we come across them all points will be adjacent.
    let mut west_border: Vec<BorderPoint> = Vec::new();
    let mut south_border: Vec<BorderPoint> = Vec::new();
    let mut east_border: Vec<BorderPoint> = Vec::new();
    let mut north_border: Vec<BorderPoint> = Vec::new();

    // Each tile is split into 4 parts
    let mut point_offset = 0usize;
    let mut indices_offset = 0usize;
    offset = 0;
    for _quad in 0..4 {
        let quad_size = dv_u32(buffer, offset)? as usize;
        offset += 4;
        let start_quad = offset;

        let origin_x = CesiumMath::to_radians(dv_f64(buffer, offset)? * 180.0);
        offset += 8;

        let origin_y = CesiumMath::to_radians(dv_f64(buffer, offset)? * 180.0);
        offset += 8;

        let step_x = CesiumMath::to_radians(dv_f64(buffer, offset)? * 180.0);
        let half_step_x = step_x * 0.5;
        offset += 8;

        let step_y = CesiumMath::to_radians(dv_f64(buffer, offset)? * 180.0);
        let half_step_y = step_y * 0.5;
        offset += 8;

        let num_points = dv_i32(buffer, offset)? as usize;
        offset += 4;

        let num_faces = dv_i32(buffer, offset)? as usize;
        offset += 4;

        //const level = dv.getInt32(offset, true);
        offset += 4;

        // Keep track of quad indices to overall tile indices
        let mut indices_mapping: Vec<Option<usize>> = vec![None; num_points];
        for i in 0..num_points {
            let longitude = origin_x + dv_u8(buffer, offset)? as f64 * step_x;
            offset += 1;
            let latitude = origin_y + dv_u8(buffer, offset)? as f64 * step_y;
            offset += 1;

            let mut height = dv_f32(buffer, offset)? as f64;
            offset += 4;

            // In order to support old clients, negative altitude values are
            // stored as height/-2^32. Old clients see the value as really
            // close to 0 but new clients multiply by -2^32 to get the real
            // negative altitude value.
            if height != 0.0 && height < negative_elevation_threshold {
                height *= -(2f64.powf(negative_altitude_exponent_bias));
            }

            // Height is stored in units of (1/EarthRadius) or (1/6371010.0)
            height *= 6371010.0;

            let scratch_cartographic = Cartographic::from_radians_new(longitude, latitude, Some(height));

            // Is it along a quad border - if so check if already exists and
            // use that index
            if index_of_epsilon(&quad_border_longitudes, longitude).is_some()
                || index_of_epsilon(&quad_border_latitudes, latitude).is_some()
            {
                let index = quad_border_points.iter().position(|p| {
                    Cartographic::equals_epsilon(
                        Some(p),
                        Some(&scratch_cartographic),
                        Some(CesiumMath::EPSILON12),
                    )
                });
                if let Some(index) = index {
                    indices_mapping[i] = Some(quad_border_indices[index]);
                    continue;
                }
                quad_border_points.push(scratch_cartographic.clone());
                quad_border_indices.push(point_offset);
            }
            indices_mapping[i] = Some(point_offset);

            if (longitude - geographic_west).abs() < half_step_x {
                west_border.push(BorderPoint {
                    index: point_offset,
                    cartographic: scratch_cartographic.clone(),
                });
            } else if (longitude - geographic_east).abs() < half_step_x {
                east_border.push(BorderPoint {
                    index: point_offset,
                    cartographic: scratch_cartographic.clone(),
                });
            } else if (latitude - geographic_south).abs() < half_step_y {
                south_border.push(BorderPoint {
                    index: point_offset,
                    cartographic: scratch_cartographic.clone(),
                });
            } else if (latitude - geographic_north).abs() < half_step_y {
                north_border.push(BorderPoint {
                    index: point_offset,
                    cartographic: scratch_cartographic.clone(),
                });
            }

            min_height = height.min(min_height);
            max_height = height.max(max_height);
            heights.push(height);

            let mut pos = Cartesian3::default();
            ellipsoid.cartographic_to_cartesian(&scratch_cartographic, &mut pos);
            positions.push(pos);

            let mut u = (longitude - geographic_west) / (geographic_east - geographic_west);
            u = CesiumMath::clamp(u, 0.0, 1.0);
            let mut v = (latitude - geographic_south) / (geographic_north - geographic_south);
            v = CesiumMath::clamp(v, 0.0, 1.0);

            uvs.push(Cartesian2::new(u, v));
            point_offset += 1;
        }

        let faces_element_count = num_faces * 3;
        for _j in 0..faces_element_count {
            let mapped = indices_mapping[dv_u16(buffer, offset)? as usize]
                .expect("quad point indices must be mapped");
            indices.push(mapped as u32);
            indices_offset += 1;
            offset += 2;
        }

        if quad_size != offset - start_quad {
            return Err(RuntimeError::new(Some("Invalid terrain tile.")));
        }
    }

    let vertex_count_without_skirts = point_offset;
    let index_count_without_skirts = indices_offset;

    // Add skirt points
    let mut h_min = min_height;

    // Sort counter clockwise from NW corner
    // Corner points are in the east/west arrays
    west_border.sort_by(|a, b| {
        b.cartographic
            .latitude
            .partial_cmp(&a.cartographic.latitude)
            .unwrap()
    });
    south_border.sort_by(|a, b| {
        a.cartographic
            .longitude
            .partial_cmp(&b.cartographic.longitude)
            .unwrap()
    });
    east_border.sort_by(|a, b| {
        a.cartographic
            .latitude
            .partial_cmp(&b.cartographic.latitude)
            .unwrap()
    });
    north_border.sort_by(|a, b| {
        b.cartographic
            .longitude
            .partial_cmp(&a.cartographic.longitude)
            .unwrap()
    });

    let percentage = 0.00001;
    let mut last_border_point: Option<BorderPoint> = None;
    add_skirt(
        &mut positions,
        &mut heights,
        &mut uvs,
        &mut indices,
        &mut h_min,
        &mut last_border_point,
        skirt_height,
        ellipsoid,
        &west_border,
        -percentage * rectangle_width,
        true,
        Some(-percentage * rectangle_height),
    );
    add_skirt(
        &mut positions,
        &mut heights,
        &mut uvs,
        &mut indices,
        &mut h_min,
        &mut last_border_point,
        skirt_height,
        ellipsoid,
        &south_border,
        -percentage * rectangle_height,
        false,
        None,
    );
    add_skirt(
        &mut positions,
        &mut heights,
        &mut uvs,
        &mut indices,
        &mut h_min,
        &mut last_border_point,
        skirt_height,
        ellipsoid,
        &east_border,
        percentage * rectangle_width,
        true,
        Some(percentage * rectangle_height),
    );
    add_skirt(
        &mut positions,
        &mut heights,
        &mut uvs,
        &mut indices,
        &mut h_min,
        &mut last_border_point,
        skirt_height,
        ellipsoid,
        &north_border,
        percentage * rectangle_height,
        false,
        None,
    );

    // Since the corner between the north and west sides is in the west
    // array, generate the last two triangles between the last north vertex
    // and the first west vertex
    if !west_border.is_empty() && !north_border.is_empty() {
        let first_border_index = west_border[0].index;
        let first_skirt_index = vertex_count_without_skirts;
        let last_border_index = north_border[north_border.len() - 1].index;
        let last_skirt_index = positions.len() - 1;

        indices.extend_from_slice(&[
            last_border_index as u32,
            last_skirt_index as u32,
            first_skirt_index as u32,
            first_skirt_index as u32,
            first_border_index as u32,
            last_border_index as u32,
        ]);
    }

    let bounding_sphere_3d = BoundingSphere::from_points(&positions, None);
    let oriented_bounding_box = Some(OrientedBoundingBox::from_rectangle(
        Some(rectangle),
        Some(min_height),
        Some(max_height),
        Some(*ellipsoid),
        None,
    ));

    // DEVIATION 3: the JS horizon occlusion point comes from
    // EllipsoidalOccluder.computeHorizonCullingPointPossiblyUnderEllipsoid;
    // left at ZERO.
    let occludee_point_in_scaled_space = Cartesian3::default();

    let _ = &to_enu; // ENU extents feed the JS aaBox only (DEVIATION 2/3)

    let encoding = TerrainEncoding::new(
        false,
        false,
        exaggeration,
        exaggeration_relative_height,
    );

    // DEVIATION 3: positions are stored as absolute ECEF.
    let mut vertices: Vec<f32> = Vec::with_capacity(positions.len() * encoding.stride);
    for (k, pos) in positions.iter().enumerate() {
        vertices.push(pos.x as f32);
        vertices.push(pos.y as f32);
        vertices.push(pos.z as f32);
        vertices.push(heights[k] as f32);
        vertices.push(uvs[k].x as f32);
        vertices.push(uvs[k].y as f32);
    }

    let west_indices_south_to_north: Vec<u32> =
        west_border.iter().map(|v| v.index as u32).rev().collect();
    let mut south_indices_east_to_west: Vec<u32> =
        south_border.iter().map(|v| v.index as u32).rev().collect();
    let east_indices_north_to_south: Vec<u32> =
        east_border.iter().map(|v| v.index as u32).rev().collect();
    let mut north_indices_west_to_east: Vec<u32> =
        north_border.iter().map(|v| v.index as u32).rev().collect();

    if let Some(last) = east_indices_north_to_south.last() {
        south_indices_east_to_west.insert(0, *last);
    }
    if let Some(first) = west_indices_south_to_north.first() {
        south_indices_east_to_west.push(*first);
    }
    if let Some(last) = west_indices_south_to_north.last() {
        north_indices_west_to_east.insert(0, *last);
    }
    if let Some(first) = east_indices_north_to_south.first() {
        north_indices_west_to_east.push(*first);
    }

    Ok(TerrainMesh {
        center: *relative_to_center,
        vertices,
        stride: encoding.stride,
        indices,
        index_count_without_skirts,
        vertex_count_without_skirts,
        minimum_height: min_height,
        maximum_height: max_height,
        rectangle: *rectangle,
        bounding_sphere_3d,
        occludee_point_in_scaled_space,
        encoding,
        oriented_bounding_box,
        west_indices_south_to_north,
        south_indices_east_to_west,
        east_indices_north_to_south,
        north_indices_west_to_east,
    })
}

/// Mirrors the worker's `addSkirt`.
#[allow(clippy::too_many_arguments)]
fn add_skirt(
    positions: &mut Vec<Cartesian3>,
    heights: &mut Vec<f64>,
    uvs: &mut Vec<Cartesian2>,
    indices: &mut Vec<u32>,
    h_min: &mut f64,
    last_border_point: &mut Option<BorderPoint>,
    skirt_height: f64,
    ellipsoid: &Ellipsoid,
    border_points: &[BorderPoint],
    fudge_factor: f64,
    east_or_west: bool,
    corner_fudge: Option<f64>,
) {
    let count = border_points.len();
    for j in 0..count {
        let border_point = &border_points[j];
        let border_cartographic = &border_point.cartographic;
        let border_index = border_point.index;
        let current_index = positions.len();

        let longitude = border_cartographic.longitude;
        let mut latitude = border_cartographic.latitude;
        // Don't go over the poles
        latitude = CesiumMath::clamp(latitude, -CesiumMath::PI_OVER_TWO, CesiumMath::PI_OVER_TWO);
        let height = border_cartographic.height - skirt_height;
        *h_min = (*h_min).min(height);

        let mut scratch_cartographic =
            Cartographic::from_radians_new(longitude, latitude, Some(height));

        // Adjust sides to angle out
        if east_or_west {
            scratch_cartographic.longitude += fudge_factor;
        }

        // Adjust top or bottom to angle out
        // Since corners are in the east/west arrays angle the first and last
        // points as well
        if !east_or_west {
            scratch_cartographic.latitude += fudge_factor;
        } else if j == count - 1 {
            scratch_cartographic.latitude += corner_fudge.unwrap_or(0.0);
        } else if j == 0 {
            scratch_cartographic.latitude -= corner_fudge.unwrap_or(0.0);
        }

        let mut pos = Cartesian3::default();
        ellipsoid.cartographic_to_cartesian(&scratch_cartographic, &mut pos);
        positions.push(pos);
        heights.push(height);
        // Copy UVs from border point
        let uv = uvs[border_index];
        uvs.push(uv);

        if let Some(last_border_point) = last_border_point {
            let last_border_index = last_border_point.index;
            indices.extend_from_slice(&[
                last_border_index as u32,
                (current_index - 1) as u32,
                current_index as u32,
                current_index as u32,
                border_index as u32,
                last_border_index as u32,
            ]);
        }

        *last_border_point = Some(border_point.clone());
    }
}

// ── Height interpolation ───────────────────────────────────────────────

fn interpolate_mesh_height(terrain_data: &GoogleEarthEnterpriseTerrainData, u: f64, v: f64) -> Option<f64> {
    let mesh = terrain_data.mesh.as_ref()?;
    let vertices = &mesh.vertices;
    let encoding = &mesh.encoding;
    let indices = &mesh.indices;

    let mut tex0 = Cartesian2::default();
    let mut tex1 = Cartesian2::default();
    let mut tex2 = Cartesian2::default();

    let mut i = 0usize;
    while i < indices.len() {
        let i0 = indices[i] as usize;
        let i1 = indices[i + 1] as usize;
        let i2 = indices[i + 2] as usize;

        encoding.decode_texture_coordinates(vertices, i0, &mut tex0);
        encoding.decode_texture_coordinates(vertices, i1, &mut tex1);
        encoding.decode_texture_coordinates(vertices, i2, &mut tex2);

        let barycentric = Intersections2D::compute_barycentric_coordinates(
            u, v, tex0.x, tex0.y, tex1.x, tex1.y, tex2.x, tex2.y,
        );
        if barycentric.x >= -1e-15 && barycentric.y >= -1e-15 && barycentric.z >= -1e-15 {
            let h0 = encoding.decode_height(vertices, i0);
            let h1 = encoding.decode_height(vertices, i1);
            let h2 = encoding.decode_height(vertices, i2);
            return Some(barycentric.x * h0 + barycentric.y * h1 + barycentric.z * h2);
        }
        i += 3;
    }

    // Position does not lie in any triangle in this mesh.
    None
}

fn interpolate_buffer_height(
    terrain_data: &GoogleEarthEnterpriseTerrainData,
    u: f64,
    v: f64,
    rectangle: &Rectangle,
) -> Option<f64> {
    let buffer = terrain_data.buffer.as_ref()?;
    let mut quad = 0usize; // SW
    let mut u_start = 0.0;
    let mut v_start = 0.0;
    if v > 0.5 {
        // Upper row
        if u > 0.5 {
            // NE
            quad = 2;
            u_start = 0.5;
        } else {
            // NW
            quad = 3;
        }
        v_start = 0.5;
    } else if u > 0.5 {
        // SE
        quad = 1;
        u_start = 0.5;
    }

    let mut offset = 0usize;
    for _q in 0..quad {
        offset += dv_u32(buffer, offset).ok()? as usize;
        offset += 4;
    }
    offset += 4; // Skip length of quad
    offset += 2 * 8; // Skip origin

    // Read sizes
    let x_size = CesiumMath::to_radians(dv_f64(buffer, offset).ok()? * 180.0);
    offset += 8;
    let y_size = CesiumMath::to_radians(dv_f64(buffer, offset).ok()? * 180.0);
    offset += 8;

    // Samples per quad
    let x_scale = rectangle.width() / x_size / 2.0;
    let y_scale = rectangle.height() / y_size / 2.0;

    // Number of points
    let num_points = dv_i32(buffer, offset).ok()? as usize;
    offset += 4;

    // Number of faces
    let num_indices = (dv_i32(buffer, offset).ok()? as usize) * 3;
    offset += 4;

    offset += 4; // Skip Level

    let mut u_buffer = vec![0.0f64; num_points];
    let mut v_buffer = vec![0.0f64; num_points];
    let mut heights = vec![0.0f64; num_points];
    for i in 0..num_points {
        u_buffer[i] = u_start + dv_u8(buffer, offset).ok()? as f64 * x_scale;
        offset += 1;
        v_buffer[i] = v_start + dv_u8(buffer, offset).ok()? as f64 * y_scale;
        offset += 1;

        // Height is stored in units of (1/EarthRadius) or (1/6371010.0)
        heights[i] = dv_f32(buffer, offset).ok()? as f64 * 6371010.0;
        offset += 4;
    }

    let mut indices = vec![0usize; num_indices];
    for i in 0..num_indices {
        indices[i] = dv_u16(buffer, offset).ok()? as usize;
        offset += 2;
    }

    let mut i = 0usize;
    while i < num_indices {
        let i0 = indices[i];
        let i1 = indices[i + 1];
        let i2 = indices[i + 2];

        let u0 = u_buffer[i0];
        let u1 = u_buffer[i1];
        let u2 = u_buffer[i2];

        let v0 = v_buffer[i0];
        let v1 = v_buffer[i1];
        let v2 = v_buffer[i2];

        let barycentric =
            Intersections2D::compute_barycentric_coordinates(u, v, u0, v0, u1, v1, u2, v2);
        if barycentric.x >= -1e-15 && barycentric.y >= -1e-15 && barycentric.z >= -1e-15 {
            return Some(
                barycentric.x * heights[i0]
                    + barycentric.y * heights[i1]
                    + barycentric.z * heights[i2],
            );
        }
        i += 3;
    }

    // Position does not lie in any triangle in this mesh.
    None
}
