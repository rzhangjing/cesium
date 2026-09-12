//! Ported from `packages/engine/Source/Core/TerrainMesh.js`.
//!
//! | CesiumJS | Port |
//! | --- | --- |
//! | `function TerrainMesh(...)` | struct literal (see the field list) |
//! | `prototype.getTransform` | [`TerrainMesh::get_transform`] |
//! | `computeTransform` | [`compute_transform`] |
//! | `computeTransform2D` | [`compute_transform2d`] |
//! | `prototype.pick` | [`TerrainMesh::pick`] |
//! | `prototype.updateExaggeration` | [`TerrainMesh::update_exaggeration`] |
//! | `prototype.updateSceneMode` | [`TerrainMesh::update_scene_mode`] |
//! | private `_transform` | [`TerrainMesh::transform`] |
//! | private `_lastPickSceneMode` | [`TerrainMesh::last_pick_scene_mode`] |
//! | private `_terrainPicker` | [`TerrainMesh::terrain_picker`] |
//!
//! # Deviations
//!
//! **`Ellipsoid.default`.** `computeTransform` reads the process-global
//! `Ellipsoid.default`. This port threads an `Option<&Ellipsoid>` through
//! `get_transform` / `pick` instead and falls back to [`Ellipsoid::WGS84`],
//! matching the convention already used across `cesium-core` (`transforms.rs`,
//! `oriented_bounding_box.rs`, …).
//!
//! **`getTransform` returns a copy.** The JS returns `this._transform` itself,
//! so a caller holding the reference would observe the next `computeTransform`
//! mutating it. No caller does — `pick` consumes the matrix immediately — and
//! returning by value keeps the borrow of `self` from colliding with the
//! `&mut self.terrain_picker` the pick needs. The cache in
//! [`TerrainMesh::transform`] is still written, so the
//! `_lastPickSceneMode === mode` short circuit behaves as in the JS.

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::map_projection::MapProjection;
use crate::math::CesiumMath;
use crate::matrix4::Matrix4;
use crate::oriented_bounding_box::OrientedBoundingBox;
use crate::ray::Ray;
use crate::rectangle::Rectangle;
use crate::scene_mode::SceneMode;
use crate::terrain_encoding::TerrainEncoding;
use crate::terrain_picker::TerrainPicker;
use crate::transforms;
use crate::vertical_exaggeration::VerticalExaggeration;

/// A mesh plus related metadata for a single tile of terrain.
pub struct TerrainMesh {
    /// The center of the tile.
    pub center: Cartesian3,
    /// The vertex data: [X, Y, Z, H, U, V, ...].
    pub vertices: Vec<f32>,
    /// The number of components in each vertex.
    pub stride: usize,
    /// The indices describing how vertices form triangles.
    pub indices: Vec<u32>,
    /// Index count not including skirts.
    pub index_count_without_skirts: usize,
    /// Vertex count not including skirts.
    pub vertex_count_without_skirts: usize,
    /// The lowest height in the tile, in meters.
    pub minimum_height: f64,
    /// The highest height in the tile, in meters.
    pub maximum_height: f64,
    /// The rectangle, in radians, covered by this tile.
    pub rectangle: Rectangle,
    /// A bounding sphere that completely contains the tile.
    pub bounding_sphere_3d: BoundingSphere,
    /// The occludee point for horizon culling.
    pub occludee_point_in_scaled_space: Cartesian3,
    /// Information about how the vertices are encoded.
    ///
    /// Mirrors `TerrainMesh.encoding` (the JS constructor takes the encoding
    /// object; `stride` above mirrors the `vertexStride` result field).
    pub encoding: TerrainEncoding,
    /// A bounding box that completely contains the tile.
    pub oriented_bounding_box: Option<OrientedBoundingBox>,
    /// Edge indices: west (S→N), south (E→W), east (N→S), north (W→E).
    pub west_indices_south_to_north: Vec<u32>,
    pub south_indices_east_to_west: Vec<u32>,
    pub east_indices_north_to_south: Vec<u32>,
    pub north_indices_west_to_east: Vec<u32>,
    /// The transform from model to world coordinates based on the terrain
    /// mesh's oriented bounding box. In 3D mode this is computed from the
    /// oriented bounding box; in 2D and Columbus View modes, from the tile's
    /// rectangle's projected coordinates.
    ///
    /// Mirrors the JS's private `_transform`, initialised to `new Matrix4()` —
    /// the identity, not the zero matrix that `Matrix4::default()` gives here.
    pub transform: Matrix4,
    /// The scene mode used the last time a pick was performed on this terrain
    /// mesh.
    ///
    /// Mirrors the JS's private `_lastPickSceneMode`; `None` is JS `undefined`,
    /// which is the constructor's initial value and what both
    /// `updateExaggeration` and `updateSceneMode` reset it to.
    pub last_pick_scene_mode: Option<SceneMode>,
    /// The terrain picker for this mesh, used for ray intersection tests.
    ///
    /// Mirrors the JS's private `_terrainPicker`. The JS constructor binds the
    /// mesh's `vertices`/`indices`/`encoding` into it by reference; this port
    /// passes them per call — see the `terrain_picker` module's **Buffer
    /// ownership** deviation.
    pub terrain_picker: TerrainPicker,
}

impl TerrainMesh {
    /// Gets the terrain tile's model-to-world transform matrix for the given
    /// scene mode and projection.
    ///
    /// Mirrors `TerrainMesh.prototype.getTransform`. `ellipsoid` replaces the
    /// JS's `Ellipsoid.default` — see the module-level deviation.
    pub fn get_transform(
        &mut self,
        mode: Option<SceneMode>,
        projection: Option<&dyn MapProjection>,
        ellipsoid: Option<&Ellipsoid>,
    ) -> Matrix4 {
        // Note this treats an undefined mode as 3D, whereas
        // `TerrainPicker`'s `getVertexPosition` does not. The JS is
        // inconsistent here and the port keeps it that way.
        if self.last_pick_scene_mode == mode {
            return self.transform;
        }
        self.terrain_picker.needs_rebuild = true;

        let ellipsoid = ellipsoid.unwrap_or(&Ellipsoid::WGS84);
        if mode.is_none() || mode == Some(SceneMode::Scene3D) {
            compute_transform(self, ellipsoid);
        } else {
            let projection = projection
                .expect("projection is required to compute a 2D/Columbus View terrain transform");
            compute_transform2d(self, projection);
        }
        self.transform
    }

    /// Gives the point on this terrain tile where the given ray intersects.
    ///
    /// Mirrors `TerrainMesh.prototype.pick`. Returns the point on the mesh
    /// where the ray intersects, or `None` if there is no intersection.
    pub fn pick(
        &mut self,
        ray: &Ray,
        cull_back_faces: bool,
        mode: Option<SceneMode>,
        projection: Option<&dyn MapProjection>,
        ellipsoid: Option<&Ellipsoid>,
    ) -> Option<Cartesian3> {
        let transform = self.get_transform(mode, projection, ellipsoid);

        // The three buffer arguments are this mesh's own fields, reproducing
        // the JS's constructor-time buffer sharing. They are disjoint from
        // `terrain_picker`, so the simultaneous `&mut` is legal.
        let intersection = self.terrain_picker.ray_intersect(
            &self.vertices,
            &self.indices,
            &self.encoding,
            ray,
            &transform,
            cull_back_faces,
            mode,
            projection,
        );

        self.last_pick_scene_mode = mode;
        intersection
    }

    /// Updates the terrain mesh to account for changes in vertical
    /// exaggeration.
    ///
    /// Mirrors `TerrainMesh.prototype.updateExaggeration`. The JS also
    /// re-binds `this._terrainPicker._vertices = this.vertices`; that is a no-op
    /// there (the same reference) and has no counterpart here, since the
    /// buffers are passed per call.
    pub fn update_exaggeration(&mut self, _exaggeration: f64, _exaggeration_relative_height: f64) {
        // The encoding stored on the TerrainMesh references the updated
        // exaggeration values already. This is just used to trigger a rebuild
        // on the terrain picker.
        self.terrain_picker.needs_rebuild = true;
        self.last_pick_scene_mode = None;
    }

    /// Updates the terrain mesh to account for changes in scene mode.
    ///
    /// Mirrors `TerrainMesh.prototype.updateSceneMode`.
    pub fn update_scene_mode(&mut self, _mode: Option<SceneMode>) {
        self.terrain_picker.needs_rebuild = true;
        self.last_pick_scene_mode = None;
    }
}

/// Gets the terrain tile's model-to-world transform matrix for 3D mode.
///
/// Mirrors `TerrainMesh.js#computeTransform`, writing into `mesh.transform`.
fn compute_transform(mesh: &mut TerrainMesh, ellipsoid: &Ellipsoid) {
    let exaggeration = mesh.encoding.exaggeration;
    let exaggeration_relative_height = mesh.encoding.exaggeration_relative_height;

    let exaggerated_min_height = VerticalExaggeration::get_height(
        mesh.minimum_height,
        exaggeration,
        exaggeration_relative_height,
    );

    let exaggerated_max_height = VerticalExaggeration::get_height(
        mesh.maximum_height,
        exaggeration,
        exaggeration_relative_height,
    );

    // The JS passes `mesh.orientedBoundingBox` as the `result` out-parameter,
    // which keeps the mesh's cached box current when there is one and leaves it
    // `undefined` when there is not. `Option::as_mut` reproduces both.
    let obb = OrientedBoundingBox::from_rectangle(
        Some(&mesh.rectangle),
        Some(exaggerated_min_height),
        Some(exaggerated_max_height),
        Some(*ellipsoid),
        mesh.oriented_bounding_box.as_mut(),
    );

    OrientedBoundingBox::compute_transformation(&obb, &mut mesh.transform);

    let mut scratch_scale = Cartesian3::default();
    Matrix4::get_scale(&mesh.transform, &mut scratch_scale);
    let z_scale = scratch_scale.z;
    if z_scale <= CesiumMath::EPSILON16 {
        scratch_scale.z = 1.0;
        // JS: `Matrix4.setScale(result, scratchScale, result)`. The aliasing is
        // legal there; here it goes through a local.
        let mut scaled = Matrix4::IDENTITY;
        Matrix4::set_scale(&mesh.transform, &scratch_scale, &mut scaled);
        mesh.transform = scaled;
    }
}

/// Gets the terrain tile's model-to-world transform matrix for 2D or Columbus
/// View modes. Assumes tiles in 2D are axis-aligned and still rectangular.
/// (This is true for Web Mercator and Geographic projections.)
///
/// Mirrors `TerrainMesh.js#computeTransform2D`, writing into `mesh.transform`.
fn compute_transform2d(mesh: &mut TerrainMesh, projection: &dyn MapProjection) {
    let exaggeration = mesh.encoding.exaggeration;
    let exaggeration_relative_height = mesh.encoding.exaggeration_relative_height;

    let exaggerated_min_height = VerticalExaggeration::get_height(
        mesh.minimum_height,
        exaggeration,
        exaggeration_relative_height,
    );

    let exaggerated_max_height = VerticalExaggeration::get_height(
        mesh.maximum_height,
        exaggeration,
        exaggeration_relative_height,
    );

    let southwest_cartographic = Cartographic::from_radians_new(
        mesh.rectangle.west,
        mesh.rectangle.south,
        Some(0.0),
    );
    let northeast_cartographic = Cartographic::from_radians_new(
        mesh.rectangle.east,
        mesh.rectangle.north,
        Some(0.0),
    );

    let southwest = projection.project(&southwest_cartographic);
    let northeast = projection.project(&northeast_cartographic);

    let height_range = exaggerated_max_height - exaggerated_min_height;
    let scale = Cartesian3::new(
        northeast.x - southwest.x,
        northeast.y - southwest.y,
        // Avoid zero scale
        if height_range > 0.0 { height_range } else { 1.0 },
    );

    let center = Cartesian3::new(
        southwest.x + scale.x * 0.5,
        southwest.y + scale.y * 0.5,
        exaggerated_min_height + scale.z * 0.5,
    );

    Matrix4::from_translation(&center, &mut mesh.transform);

    // JS: `Matrix4.setScale(result, scale, result)` — aliased, so via a local.
    let mut scaled = Matrix4::IDENTITY;
    Matrix4::set_scale(&mesh.transform, &scale, &mut scaled);
    mesh.transform = scaled;

    // JS: `Matrix4.multiply(Transforms.SWIZZLE_3D_TO_2D_MATRIX, result, result)`
    let multiplied = Matrix4::multiply_new(&transforms::SWIZZLE_3D_TO_2D_MATRIX, &mesh.transform);
    mesh.transform = multiplied;
}
