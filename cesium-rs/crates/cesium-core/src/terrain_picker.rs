//! Ported from `packages/engine/Source/Core/TerrainPicker.js`.
//!
//! Handles arbitrary ray intersections with a terrain mesh using a spatial
//! acceleration structure: a lazily-refined quadtree of triangle indices, in
//! the tile's normalised local space.
//!
//! | CesiumJS | Port |
//! | --- | --- |
//! | `MAXIMUM_TERRAIN_PICKER_LEVEL` | [`MAXIMUM_TERRAIN_PICKER_LEVEL`] |
//! | `class TerrainPicker` | [`TerrainPicker`] |
//! | `rayIntersect` | [`TerrainPicker::ray_intersect`] |
//! | `get/set needsRebuild` | [`TerrainPicker::needs_rebuild`] field |
//! | `class TerrainPickerNode` | [`TerrainPickerNode`] |
//! | `TerrainPickerNode#addChild` | [`TerrainPickerNode::add_child`] |
//! | `reset` | [`TerrainPicker::reset`] |
//! | `createAABBForNode` | [`create_aabb_for_node`] |
//! | `getNodesIntersectingRay` | [`get_nodes_intersecting_ray`] |
//! | `findClosestPointInClosestNode` | [`TerrainPicker::find_closest_point_in_closest_node`] |
//! | `getClosestTriangleInNode` | [`TerrainPicker::get_closest_triangle_in_node`] |
//! | `getVertexPosition` | [`get_vertex_position`] |
//! | `packTriangleBuffers` | [`pack_triangle_buffers`] |
//! | `addTrianglesToChildrenNodes` | [`add_triangles_to_children_nodes`] |
//! | `Workers/incrementallyBuildTerrainPicker.js` | [`incrementally_build_terrain_picker`] |
//! | `createAABBFromTriangle` (worker) | [`create_aabb_from_triangle`] |
//!
//! # Deviations
//!
//! **Buffer ownership.** The JS constructor binds the mesh's buffers into the
//! picker once — `new TerrainPicker(this.vertices, this.indices,
//! this.encoding)` — relying on JS reference sharing. A Rust picker owned by
//! the same `TerrainMesh` cannot hold those borrows without becoming
//! self-referential, so [`TerrainPicker::ray_intersect`] takes the three
//! buffers as parameters. Only their *location* moves: the picker's own
//! persistent state (`_inverseTransform`, `_needsRebuild`, the `_rootNode`
//! quadtree) is stored exactly as the JS does, and `TerrainMesh::pick` passes
//! its own fields, reproducing the shared-buffer semantics.
//!
//! **Node identity.** `IntersectingNode.node` is an object reference in the JS.
//! `getClosestTriangleInNode` then *mutates* the node it is handed (it grows
//! the next quadtree layer), so the gathered list cannot hold `&` into the tree
//! while `&mut self` is live. The reference is replaced by [`NodePath`], the
//! chain of child indices leading to the same node.
//!
//! **Synchronous worker.** `addTrianglesToChildrenNodes` schedules
//! `incrementallyBuildTerrainPicker` on a `TaskProcessor` and awaits it. This
//! port inlines the worker's algorithm
//! ([`incrementally_build_terrain_picker`]) and runs it synchronously, because
//! `TaskProcessor` is not ported yet. That is behaviourally equivalent for a
//! sequential stream of picks: in the JS the refinement is only observable on
//! the *next* `rayIntersect` (the quadtree is walked in
//! `getNodesIntersectingRay`, before any `getClosestTriangleInNode` runs), and
//! the intermediate `buildingChildren = true` window — during which the parent
//! node is treated as a leaf holding its full triangle list — cannot be
//! observed from a single-threaded caller. The pack/unpack round trip through
//! `Float64Array`s is likewise dropped where it is lossless; see
//! [`add_triangles_to_children_nodes`].

use crate::axis_aligned_bounding_box::AxisAlignedBoundingBox;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::developer_error::throw_developer_error;
use crate::intersection_tests::IntersectionTests;
use crate::interval::Interval;
use crate::map_projection::MapProjection;
use crate::math::{js_min, js_round, CesiumMath};
use crate::matrix4::Matrix4;
use crate::ray::Ray;
use crate::scene_mode::SceneMode;
use crate::terrain_encoding::TerrainEncoding;
use std::cmp::Ordering;

/// Terrain picker can be 4 levels deep (0-3)
///
/// Mirrors the module-private `MAXIMUM_TERRAIN_PICKER_LEVEL`.
const MAXIMUM_TERRAIN_PICKER_LEVEL: u32 = 3;

/// The tile's local-space AABB, used to clamp a triangle AABB whose height
/// extent escaped the tile because of a degenerate 2D height scale.
///
/// Mirrors the worker's `TILE_AABB_MIN` / `TILE_AABB_MAX`.
const TILE_AABB_MIN: Cartesian3 = Cartesian3::new(-0.5, -0.5, -0.5);
const TILE_AABB_MAX: Cartesian3 = Cartesian3::new(0.5, 0.5, 0.5);

/// A node in the terrain picker quadtree.
///
/// Mirrors `class TerrainPickerNode`.
#[derive(Debug, Clone)]
pub struct TerrainPickerNode {
    /// The tree-space x-coordinate of this node.
    pub x: u32,
    /// The tree-space y-coordinate of this node.
    pub y: u32,
    /// The level of this node in the quadtree.
    pub level: u32,
    /// The axis-aligned bounding box of this node (in the tree's local space).
    pub aabb: AxisAlignedBoundingBox,
    /// The indices of the triangles that intersect this node.
    ///
    /// The JS uses a `Uint32Array`; a `Vec<u32>` is the same value type and
    /// lets the child arrays be grown by the worker port without a second
    /// allocation pass.
    pub intersecting_triangles: Vec<u32>,
    /// The child terrain picker nodes of this node.
    pub children: Vec<TerrainPickerNode>,
    /// Whether or not this node is currently building its children on a worker.
    pub building_children: bool,
}

impl Default for TerrainPickerNode {
    fn default() -> Self {
        Self::new()
    }
}

impl TerrainPickerNode {
    /// Creates a node at the tree origin (the JS constructor takes no
    /// arguments; every node starts as `(x, y, level) = (0, 0, 0)` and
    /// [`Self::add_child`] relocates it).
    pub fn new() -> Self {
        Self {
            x: 0,
            y: 0,
            level: 0,
            aabb: create_aabb_for_node(0, 0, 0),
            intersecting_triangles: Vec::new(),
            children: Vec::new(),
            building_children: false,
        }
    }

    /// Adds a child node to this node.
    ///
    /// @param child_idx The index of the child to add (0-3).
    pub fn add_child(&mut self, child_idx: usize) {
        if child_idx > 3 {
            throw_developer_error(
                "TerrainPickerNode child index must be between 0 and 3, inclusive.",
            );
        }

        let mut child_node = TerrainPickerNode::new();
        // Use bitwise operations to get child x,y from child index and parent x,y
        child_node.x = self.x * 2 + (child_idx & 1) as u32;
        child_node.y = self.y * 2 + ((child_idx >> 1) & 1) as u32;
        child_node.level = self.level + 1;
        child_node.aabb = create_aabb_for_node(child_node.x, child_node.y, child_node.level);

        // JS: `this.children[childIdx] = childNode`. The sole caller fills
        // 0..4 in order, so appending reproduces the assignment; the
        // replace-arm keeps a repeated index idempotent instead of growing.
        if child_idx < self.children.len() {
            self.children[child_idx] = child_node;
        } else {
            self.children.push(child_node);
        }
    }
}

/// Locates a node inside the picker's quadtree, as the chain of child indices
/// that leads to it. An empty path is the root.
///
/// Stands in for the JS `IntersectingNode.node` object reference — see the
/// module-level **Node identity** deviation.
type NodePath = Vec<usize>;

/// A quadtree node that the ray passed through, plus the interval along the ray
/// where it does.
///
/// Mirrors the `IntersectingNode` typedef.
#[derive(Debug, Clone)]
struct IntersectingNode {
    path: NodePath,
    interval: Interval,
}

/// Creates an object that handles arbitrary ray intersections with a terrain
/// mesh using a spatial acceleration structure.
///
/// Mirrors `class TerrainPicker`. The JS constructor's three buffer parameters
/// are passed to [`Self::ray_intersect`] instead; see the module-level
/// **Buffer ownership** deviation.
#[derive(Debug, Clone)]
pub struct TerrainPicker {
    /// The inverse of the terrain mesh tile's transform from world space to
    /// local space. Computed as-needed on rebuild.
    ///
    /// The JS initialises this to `new Matrix4()`, i.e. the identity — note
    /// that `Matrix4::default()` in this port is the *zero* matrix, so
    /// [`Matrix4::IDENTITY`] is spelled out.
    inverse_transform: Matrix4,
    /// Whether the internal data structures need rebuilding.
    ///
    /// Mirrors the `needsRebuild` accessor pair; a plain field, as the port had
    /// before, so `TerrainMesh::get_transform` can set it directly.
    pub needs_rebuild: bool,
    /// The root node of the terrain picker's quadtree.
    root_node: TerrainPickerNode,
}

impl Default for TerrainPicker {
    fn default() -> Self {
        Self::new()
    }
}

impl TerrainPicker {
    /// Creates a new TerrainPicker.
    pub fn new() -> Self {
        Self {
            inverse_transform: Matrix4::IDENTITY,
            needs_rebuild: true,
            root_node: TerrainPickerNode::new(),
        }
    }

    /// Determines the point on the mesh where the given ray intersects.
    ///
    /// Mirrors `TerrainPicker.prototype.rayIntersect`. `tile_transform` is the
    /// terrain mesh tile's transform from local space to world space;
    /// `cull_back_faces` says whether to ignore back-facing triangles; `mode`
    /// and `projection` drive the 2D/Columbus View vertex reprojection in
    /// [`get_vertex_position`].
    ///
    /// Returns the intersection point, or `None` if there is no intersection.
    #[allow(clippy::too_many_arguments)]
    pub fn ray_intersect(
        &mut self,
        vertices: &[f32],
        indices: &[u32],
        encoding: &TerrainEncoding,
        ray: &Ray,
        tile_transform: &Matrix4,
        cull_back_faces: bool,
        mode: Option<SceneMode>,
        projection: Option<&dyn MapProjection>,
    ) -> Option<Cartesian3> {
        // Lazily (re)create the terrain picker
        if self.needs_rebuild {
            self.reset(indices, tile_transform);
        }

        // Mirrors `scratchTransformedRay`. Only the node AABB tests use this
        // ray: it lives in the tile's normalised local space, where the root
        // node is the unit cube. The triangle tests below use the original
        // world-space `ray` against world-space vertices, because
        // `TerrainEncoding::decode_position` already adds the RTC centre back.
        let mut transformed_ray = Ray::default();
        Matrix4::multiply_by_point(
            &self.inverse_transform,
            &ray.origin,
            &mut transformed_ray.origin,
        );
        Matrix4::multiply_by_point_as_vector(
            &self.inverse_transform,
            &ray.direction,
            &mut transformed_ray.direction,
        );

        let mut intersections: Vec<IntersectingNode> = Vec::new();
        get_nodes_intersecting_ray(
            &self.root_node,
            Vec::new(),
            &transformed_ray,
            &mut intersections,
        );

        self.find_closest_point_in_closest_node(
            vertices,
            indices,
            encoding,
            &intersections,
            ray,
            cull_back_faces,
            mode,
            projection,
        )
    }

    /// Resets the terrain picker's quadtree structure to just the root node.
    /// Done whenever the underlying terrain mesh changes.
    ///
    /// Mirrors the module-private `reset`. The root node receives *every*
    /// triangle; the quadtree is refined lazily, on the first pick that reaches
    /// a node.
    ///
    /// PERFORMANCE_IDEA (carried over from the JS): warm-start the terrain
    /// picker by building a level on a worker.
    fn reset(&mut self, indices: &[u32], tile_transform: &Matrix4) {
        if !Matrix4::inverse(tile_transform, &mut self.inverse_transform) {
            // JS `Matrix4.inverse` clones `Matrix4.ZERO` into the result when
            // the determinant is zero; this port's returns `false` and leaves
            // the result untouched. Zero it here so the observable state
            // matches.
            self.inverse_transform = Matrix4::ZERO;
        }

        self.needs_rebuild = false;
        let triangle_count = indices.len() / 3;
        self.root_node.intersecting_triangles = (0..triangle_count as u32).collect();
        self.root_node.children.clear();
    }

    /// Resolves a [`NodePath`] against the quadtree.
    fn node(&self, path: &[usize]) -> &TerrainPickerNode {
        let mut node = &self.root_node;
        for &index in path {
            node = &node.children[index];
        }
        node
    }

    /// Mutable form of [`Self::node`].
    fn node_mut(&mut self, path: &[usize]) -> &mut TerrainPickerNode {
        let mut node = &mut self.root_node;
        for &index in path {
            node = &mut node.children[index];
        }
        node
    }

    /// Finds the closest intersecting node along the ray, in world space, and
    /// the closest point in that node, by testing all triangles in the closest
    /// node against the ray.
    ///
    /// Mirrors `findClosestPointInClosestNode`.
    ///
    /// Note the early `break`: the nodes are sorted by the *start* of their ray
    /// interval, and the first node that produces any hit wins. A triangle's
    /// own `t` is always at least its node AABB's `start`, but a *later* node's
    /// hit may be closer than an earlier node's — so the refined tree can
    /// return a farther hit than a brute-force scan would. That is CesiumJS's
    /// behaviour and is reproduced here.
    #[allow(clippy::too_many_arguments)]
    fn find_closest_point_in_closest_node(
        &mut self,
        vertices: &[f32],
        indices: &[u32],
        encoding: &TerrainEncoding,
        intersections: &[IntersectingNode],
        ray: &Ray,
        cull_back_faces: bool,
        mode: Option<SceneMode>,
        projection: Option<&dyn MapProjection>,
    ) -> Option<Cartesian3> {
        let mut sorted_intersections: Vec<&IntersectingNode> = intersections.iter().collect();
        // JS: `intersections.sort((a, b) => a.interval.start - b.interval.start)`.
        // Both sorts are stable. `partial_cmp` yields `None` only for a NaN
        // bound, where the JS comparator is implementation-defined; treating
        // those as equal is the neutral choice.
        sorted_intersections.sort_by(|a, b| {
            a.interval
                .start
                .partial_cmp(&b.interval.start)
                .unwrap_or(Ordering::Equal)
        });

        let mut min_t = f64::MAX;
        for intersection in sorted_intersections {
            let intersection_result = self.get_closest_triangle_in_node(
                vertices,
                indices,
                encoding,
                ray,
                &intersection.path,
                cull_back_faces,
                mode,
                projection,
            );
            // JS: `Math.min`, so `js_min` rather than `f64::min`.
            min_t = js_min(intersection_result, min_t);
            if min_t != f64::MAX {
                break;
            }
        }

        if min_t != f64::MAX {
            return Some(Ray::get_point_new(ray, Some(min_t)));
        }

        None
    }

    /// Tests all triangles in the given node against the ray, returning the
    /// closest intersection `t` value along the ray. Additionally, collects the
    /// triangles' positions and indices along the way, to build out the child
    /// nodes.
    ///
    /// Mirrors `getClosestTriangleInNode`. Returns `f64::MAX`
    /// (`Number.MAX_VALUE`) when the node yields no hit.
    #[allow(clippy::too_many_arguments)]
    fn get_closest_triangle_in_node(
        &mut self,
        vertices: &[f32],
        indices: &[u32],
        encoding: &TerrainEncoding,
        ray: &Ray,
        path: &[usize],
        cull_back_faces: bool,
        mode: Option<SceneMode>,
        projection: Option<&dyn MapProjection>,
    ) -> f64 {
        let mut result = f64::MAX;
        let triangle_count = self.node(path).intersecting_triangles.len();
        let is_max_level = self.node(path).level >= MAXIMUM_TERRAIN_PICKER_LEVEL;
        let should_build_children = !is_max_level && !self.node(path).building_children;

        // If the tree can be built deeper, prepare buffers to store triangle
        // data for the child nodes.
        let mut triangle_positions: Vec<f64> = Vec::new();
        let mut triangle_indices: Vec<u32> = Vec::new();
        if should_build_children {
            // 3 vertices per triangle * 3 floats per vertex
            triangle_positions = vec![0.0; triangle_count * 9];
            triangle_indices = vec![0; triangle_count];
        }

        // Mirrors `scratchTrianglePoints`.
        let mut scratch_triangle_points = [Cartesian3::default(); 3];

        for i in 0..triangle_count {
            let tri_index = self.node(path).intersecting_triangles[i] as usize;
            get_vertex_position(
                encoding,
                mode,
                projection,
                ray,
                vertices,
                indices[3 * tri_index] as usize,
                &mut scratch_triangle_points[0],
            );
            get_vertex_position(
                encoding,
                mode,
                projection,
                ray,
                vertices,
                indices[3 * tri_index + 1] as usize,
                &mut scratch_triangle_points[1],
            );
            get_vertex_position(
                encoding,
                mode,
                projection,
                ray,
                vertices,
                indices[3 * tri_index + 2] as usize,
                &mut scratch_triangle_points[2],
            );

            let tri_t = IntersectionTests::ray_triangle_parametric(
                ray,
                &scratch_triangle_points[0],
                &scratch_triangle_points[1],
                &scratch_triangle_points[2],
                cull_back_faces,
            );

            if let Some(tri_t) = tri_t {
                if tri_t < result && tri_t >= 0.0 {
                    result = tri_t;
                }
            }

            if should_build_children {
                pack_triangle_buffers(
                    &mut triangle_positions,
                    &mut triangle_indices,
                    &scratch_triangle_points,
                    tri_index as u32,
                    i,
                );
            }
        }

        if should_build_children {
            let node = self.node_mut(path);
            for child_idx in 0..4 {
                node.add_child(child_idx);
            }

            // `self.inverse_transform` is read out before the `&mut` borrow of
            // the node; the JS passes `terrainPicker._inverseTransform`
            // straight in.
            let inverse_transform = self.inverse_transform;
            let node = self.node_mut(path);
            add_triangles_to_children_nodes(
                &inverse_transform,
                node,
                &triangle_indices,
                &triangle_positions,
            );
        }

        result
    }
}

/// Creates an axis-aligned bounding box for a quadtree node at the given
/// tree-space coordinates and level. This AABB is in the tree's local space
/// (where the root node of the tree is a unit cube in its own local space).
///
/// Mirrors `createAABBForNode`.
fn create_aabb_for_node(x: u32, y: u32, level: u32) -> AxisAlignedBoundingBox {
    // JS: `1.0 / Math.pow(2, level)` — exact for integer levels either way.
    let size_at_level = 1.0 / 2.0f64.powi(level as i32);

    let aabb_min = Cartesian3::new(
        x as f64 * size_at_level - 0.5,
        y as f64 * size_at_level - 0.5,
        -0.5,
    );

    let aabb_max = Cartesian3::new(
        (x + 1) as f64 * size_at_level - 0.5,
        (y + 1) as f64 * size_at_level - 0.5,
        0.5,
    );

    AxisAlignedBoundingBox::from_corners(&aabb_min, &aabb_max)
}

/// Recursively gathers all nodes in the quadtree that intersect the ray.
///
/// Mirrors `getNodesIntersectingRay`. A node is a leaf when it has no children
/// *or* is currently building them — in the latter case the parent still holds
/// the full triangle list, so testing it is correct.
fn get_nodes_intersecting_ray(
    current_node: &TerrainPickerNode,
    path: NodePath,
    ray: &Ray,
    intersecting_nodes: &mut Vec<IntersectingNode>,
) {
    let Some(interval) =
        IntersectionTests::ray_axis_aligned_bounding_box(ray, &current_node.aabb)
    else {
        return;
    };

    let is_leaf = current_node.children.is_empty() || current_node.building_children;
    if is_leaf {
        // JS: `new Interval(interval.start, interval.stop)` — a copy out of the
        // module-level scratch. `Interval` is `Copy` here.
        intersecting_nodes.push(IntersectingNode { path, interval });
        return;
    }

    for (i, child) in current_node.children.iter().enumerate() {
        let mut child_path = path.clone();
        child_path.push(i);
        get_nodes_intersecting_ray(child, child_path, ray, intersecting_nodes);
    }
}

/// Gets a vertex position from the buffer, taking into account the exaggeration
/// and scene mode of the terrain.
///
/// Mirrors `getVertexPosition`. The `ray` is used only as a reference for
/// resolving antimeridian wrapping in 2D/Columbus View.
fn get_vertex_position(
    encoding: &TerrainEncoding,
    mode: Option<SceneMode>,
    projection: Option<&dyn MapProjection>,
    ray: &Ray,
    vertices: &[f32],
    index: usize,
    result: &mut Cartesian3,
) {
    encoding.get_exaggerated_position(vertices, index, result);
    // Note this is a *stricter* test than `TerrainMesh.getTransform`'s
    // `!defined(mode) || mode === SceneMode.SCENE3D`: the JS treats an
    // undefined mode as 3D when computing the transform but not here. Kept as
    // is, quirk included.
    if mode == Some(SceneMode::Scene3D) {
        return;
    }

    // The JS dereferences `projection.ellipsoid` unconditionally on this path;
    // a missing projection is the same programming error.
    let projection =
        projection.expect("projection is required to pick terrain in 2D or Columbus View");
    let ellipsoid = projection.ellipsoid();
    let mut position_cartographic = Cartographic::default();
    ellipsoid.cartesian_to_cartographic(result, &mut position_cartographic);
    let projected = projection.project(&position_cartographic);

    // Swizzle because coordinate basis are different in 2D/Columbus View.
    // The JS reads all three components out of `result` before `fromElements`
    // writes back into it; a separate `projected` value is the same thing.
    Cartesian3::from_elements(projected.z, projected.x, projected.y, result);

    // Due to wrapping in 2D/CV modes, near the antimeridian, the vertex
    // position may correspond to the other side of the world from the ray
    // origin. Compare the vertex position to the ray origin and adjust it
    // accordingly. A spherical approximation is sufficient for cylindrical
    // projections, like mercator and geographic.
    let world_width = CesiumMath::TWO_PI * ellipsoid.maximum_radius();
    // JS `Math.round`, which rounds half towards +infinity — `f64::round` does
    // not, and `k` here is routinely negative.
    let k = js_round((ray.origin.y - result.y) / world_width);
    result.y += k * world_width;
}

/// Packs triangle vertex positions and index into the provided buffers, for the
/// child-node build to process.
///
/// Mirrors `packTriangleBuffers`.
fn pack_triangle_buffers(
    triangle_positions_buffer: &mut [f64],
    triangle_indices_buffer: &mut [u32],
    triangle_positions: &[Cartesian3; 3],
    triangle_index: u32,
    buffer_index: usize,
) {
    Cartesian3::pack(
        &triangle_positions[0],
        triangle_positions_buffer,
        Some(9 * buffer_index),
    );
    Cartesian3::pack(
        &triangle_positions[1],
        triangle_positions_buffer,
        Some(9 * buffer_index + 3),
    );
    Cartesian3::pack(
        &triangle_positions[2],
        triangle_positions_buffer,
        Some(9 * buffer_index + 6),
    );
    triangle_indices_buffer[buffer_index] = triangle_index;
}

/// Adds triangles to the child nodes of the given node.
///
/// Mirrors `addTrianglesToChildrenNodes`.
///
/// The JS packs the four child AABBs into a `Float64Array` and ships it, plus
/// the inverse transform and the triangle buffers, to a worker. Two of those
/// round trips are dropped here:
///
/// * the AABB pack/unpack — `Cartesian3.unpack` of a packed `f64` triple is the
///   identity, and `AxisAlignedBoundingBox.fromCorners` is then recomputed from
///   the same `minimum`/`maximum`, so handing the child AABBs over directly is
///   bit-identical;
/// * the `inverseTransform` pack/unpack, for the same reason.
///
/// The *triangle* buffers stay packed, because
/// [`incrementally_build_terrain_picker`] is a faithful port of the worker and
/// takes the same `Float64Array`-shaped input.
fn add_triangles_to_children_nodes(
    inverse_transform: &Matrix4,
    node: &mut TerrainPickerNode,
    triangle_indices: &[u32],
    triangle_positions: &[f64],
) {
    node.building_children = true;

    let mut node_aabbs = [
        AxisAlignedBoundingBox::default(),
        AxisAlignedBoundingBox::default(),
        AxisAlignedBoundingBox::default(),
        AxisAlignedBoundingBox::default(),
    ];
    for (index, aabb) in node_aabbs.iter_mut().enumerate() {
        *aabb = node.children[index].aabb.clone();
    }

    let intersecting_triangles_arrays = incrementally_build_terrain_picker(
        &node_aabbs,
        inverse_transform,
        triangle_indices,
        triangle_positions,
    );

    for (index, buffer) in intersecting_triangles_arrays.into_iter().enumerate() {
        // JS: `if (defined(node.children[index]))`
        if index < node.children.len() {
            node.children[index].intersecting_triangles = buffer;
        }
    }

    node.intersecting_triangles = Vec::new();
    node.building_children = false;
}

/// Builds the next layer of the terrain picker's quadtree by determining which
/// triangles intersect each of the four child nodes. (Essentially distributing
/// the parent's triangles to its children.)
///
/// Port of `Workers/incrementallyBuildTerrainPicker.js`. Takes the AABBs of the
/// four child nodes in the tree's local space, an inverse transform to convert
/// triangle positions to the tree's local space, and the parent node's triangle
/// indices and positions; returns four arrays — one per child node — of the
/// indices of the triangles that intersect it.
///
/// The JS reaches this code through a `TaskProcessor` worker; see the
/// module-level **Synchronous worker** deviation.
fn incrementally_build_terrain_picker(
    node_aabbs: &[AxisAlignedBoundingBox; 4],
    inverse_transform: &Matrix4,
    triangle_indices: &[u32],
    triangle_positions: &[f64],
) -> [Vec<u32>; 4] {
    let mut intersecting_triangles_arrays: [Vec<u32>; 4] = Default::default();

    // Mirrors the worker's `scratchTrianglePoints`.
    let mut scratch_triangle_points = [Cartesian3::default(); 3];

    for j in 0..triangle_indices.len() {
        Cartesian3::unpack(
            triangle_positions,
            Some(j * 9),
            &mut scratch_triangle_points[0],
        );
        Cartesian3::unpack(
            triangle_positions,
            Some(j * 9 + 3),
            &mut scratch_triangle_points[1],
        );
        Cartesian3::unpack(
            triangle_positions,
            Some(j * 9 + 6),
            &mut scratch_triangle_points[2],
        );

        let triangle_aabb = create_aabb_from_triangle(inverse_transform, &mut scratch_triangle_points);

        for i in 0..4 {
            let aabbs_intersect =
                AxisAlignedBoundingBox::intersect_axis_aligned_bounding_box(&node_aabbs[i], &triangle_aabb);
            if !aabbs_intersect {
                continue;
            }

            intersecting_triangles_arrays[i].push(triangle_indices[j]);
        }
    }

    intersecting_triangles_arrays
}

/// Creates a tree-space axis-aligned bounding box from the given triangle
/// points and inverse transform (from world to tree space).
///
/// Port of the worker's `createAABBFromTriangle`. Transforms the three points
/// in place, as the JS does.
fn create_aabb_from_triangle(
    inverse_transform: &Matrix4,
    triangle_points: &mut [Cartesian3; 3],
) -> AxisAlignedBoundingBox {
    for point in triangle_points.iter_mut() {
        // JS aliases the point as both the input and the output
        // (`multiplyByPoint(m, p, p)`). Rust will not hand out `&*point` and
        // `&mut *point` at once, so go through a temporary; the arithmetic is
        // identical.
        let mut transformed = Cartesian3::ZERO;
        Matrix4::multiply_by_point(inverse_transform, point, &mut transformed);
        *point = transformed;
    }

    let mut aabb = AxisAlignedBoundingBox::from_points(Some(triangle_points));

    // In 2D mode, sometimes the height-scale of a tile is 0. See
    // `TerrainMesh#computeTransform2D`. This makes the inverseTransform
    // degenerate, so we set the height-scale to 1 to prevent that. However,
    // this is artificial and can lead to the triangle's AABB extending beyond
    // the (height) bounds of the tile's AABB. Thus, we clamp the triangle's
    // AABB to the tile's local-space AABB.
    //
    // The JS clamps `minimum`/`maximum` only and leaves `center` at the
    // pre-clamp midpoint; reproduced literally, since
    // `intersectAxisAlignedBoundingBox` never reads `center`. Both calls alias
    // their input and output, hence the temporary.
    let mut clamped = Cartesian3::ZERO;
    Cartesian3::clamp(&aabb.minimum, &TILE_AABB_MIN, &TILE_AABB_MAX, &mut clamped);
    aabb.minimum = clamped;
    Cartesian3::clamp(&aabb.maximum, &TILE_AABB_MIN, &TILE_AABB_MAX, &mut clamped);
    aabb.maximum = clamped;
    aabb
}
