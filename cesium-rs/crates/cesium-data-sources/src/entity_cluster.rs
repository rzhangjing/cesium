//! Ported from `packages/engine/Source/DataSources/EntityCluster.js`.
//!
//! Defines how screen space objects (billboards, points, labels) are
//! clustered.
//!
//! ## Pure-logic / gpu-limited split
//!
//! CesiumJS' clustering pipeline mixes pure spatial logic with
//! rendering-context access. This port keeps the pure logic substantive:
//!
//! - option/property handling with dirty-flag tracking,
//! - the `_collectionIndicesByEntity` index bookkeeping
//!   (`getLabel`/`removeLabel`/... family),
//! - the declutter clustering algorithm itself (KDBush-backed grid
//!   clustering over screen-space coordinates, previous-cluster reuse on
//!   zoom-in, centroid averaging, `clusterEvent` raising).
//!
//! Everything that requires the render surface stays at a clearly marked
//! boundary and is injected by the caller:
//!
//! - screen-space coordinates (`computeScreenSpacePosition`) and
//!   screen-space bounding boxes (`Label/Billboard/PointPrimitive
//!   .getScreenSpaceBoundingBox`, glyph/image-atlas measurement) arrive as
//!   precomputed inputs on [`ScreenSpacePoint`] / [`ClusterCandidate`];
//! - occluder visibility (SCENE3D horizon culling) is precomputed by the
//!   caller into [`ClusterCandidate::visible`];
//! - the zoom-in pass' `Billboard._computeScreenSpacePosition` is replaced
//!   by the injected projection callback [`ClusterFrame::project`];
//! - the six primitive collections (entity/cluster x label/billboard/point)
//!   are modeled as virtual lengths; their per-frame `update(frameState)`
//!   is gpu-limited and skipped.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

use cesium_core::bounding_rectangle::BoundingRectangle;
use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::event::Event;

/// Options for [`EntityCluster::with_options`].
///
/// Port of the `options` object of the `EntityCluster` constructor.
#[derive(Clone, Debug, Default)]
pub struct EntityClusterOptions {
    /// Whether or not to enable clustering (default `false`).
    pub enabled: Option<bool>,
    /// The pixel range to extend the screen space bounding box (default 80).
    pub pixel_range: Option<f64>,
    /// The minimum number of screen space objects that can be clustered
    /// (default 2).
    pub minimum_cluster_size: Option<usize>,
    /// Whether or not to cluster the billboards of an entity (default true).
    pub cluster_billboards: Option<bool>,
    /// Whether or not to cluster the labels of an entity (default true).
    pub cluster_labels: Option<bool>,
    /// Whether or not to cluster the points of an entity (default true).
    pub cluster_points: Option<bool>,
    /// Determines if the entities in the cluster will be shown (default true).
    pub show: Option<bool>,
}

/// Which screen-space primitive an item was created from.
///
/// Rust stand-in for CesiumJS duck typing on `item._labelCollection` /
/// `item._billboardCollection` / `item._pointPrimitiveCollection`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClusterItemKind {
    /// Item belongs to a `LabelCollection`.
    Label,
    /// Item belongs to a `BillboardCollection`.
    Billboard,
    /// Item belongs to a `PointPrimitiveCollection`.
    Point,
}

/// A candidate screen-space item before clustering filters are applied.
///
/// Port of one iteration of `getScreenSpacePositions`: the caller supplies
/// the gpu-limited facts (visibility, projection, bounding boxes) and the
/// pure filter decides participation.
#[derive(Clone, Debug)]
pub struct ClusterCandidate {
    /// Id of the entity that owns this item (`item.id.id`).
    pub entity_id: String,
    /// Which collection the item belongs to.
    pub kind: ClusterItemKind,
    /// `item.show`.
    pub show: bool,
    /// Whether the item's world position is visible on the globe.
    /// gpu-limited: computed from `EllipsoidalOccluder.isPointVisible` in
    /// SCENE3D; pass `true` for SCENE2D/COLUMBIA_VIEW or when unknown.
    pub visible: bool,
    /// The entity has a billboard being displayed (`defined(item.id._billboard)`).
    pub has_billboard: bool,
    /// The entity has a label being displayed (`defined(item.id._label)`).
    pub has_label: bool,
    /// The entity has a point being displayed (`defined(item.id._point)`).
    pub has_point: bool,
    /// World position of the item (`item.position`).
    pub position: Cartesian3,
    /// Screen-space position; `None` when projection failed
    /// (`computeScreenSpacePosition` returned undefined). gpu-limited input.
    pub screen_position: Option<Cartesian2>,
    /// Unexpanded screen-space bounding box of the item. gpu-limited input:
    /// in CesiumJS computed by `Label/Billboard/PointPrimitive
    /// .getScreenSpaceBoundingBox` from glyph/image-atlas measurement.
    pub bbox: BoundingRectangle,
    /// Screen-space bounding box of the entity's associated label (when the
    /// item itself is not the label), used by the `hasLabelIndex` union
    /// branch of `getBoundingBox`. gpu-limited input.
    pub label_bbox: Option<BoundingRectangle>,
    /// Index of the item within its collection (JS `i`).
    pub index: usize,
}

/// A point participating in the declutter pass.
///
/// Port of the entries pushed into `points` by `getScreenSpacePositions`
/// (`{ index, collection, clustered, coord }`) merged with the data needed
/// by `getBoundingBox`/`addNonClusteredItem`.
#[derive(Clone, Debug)]
pub struct ScreenSpacePoint {
    /// Id of the owning entity.
    pub entity_id: String,
    /// Which collection the item belongs to.
    pub kind: ClusterItemKind,
    /// World position (`item.position`), used for the cluster centroid.
    pub position: Cartesian3,
    /// Screen-space coordinate (`coord`).
    pub coord: Cartesian2,
    /// Unexpanded screen-space bounding box. gpu-limited input, see
    /// [`ClusterCandidate::bbox`].
    pub bbox: BoundingRectangle,
    /// Optional screen-space bbox of the entity's label, see
    /// [`ClusterCandidate::label_bbox`].
    pub label_bbox: Option<BoundingRectangle>,
    /// Index of the item within its collection.
    pub index: usize,
    /// Whether this item has already been absorbed into a cluster during the
    /// current declutter pass.
    pub clustered: bool,
    /// Whether the item should be shown individually (unclustered). Set to
    /// `false` for all participants at pass start, restored to `true` by
    /// `addNonClusteredItem`. Output of the declutter pass.
    pub cluster_show: bool,
}

/// Port of `getScreenSpacePositions` (filtering logic; the screen-space
/// projection itself is supplied by the caller, gpu-limited in JS).
///
/// Applies the JS per-item rules: unconditionally resets `clusterShow`,
/// skips hidden or occluded items, skips label items whose entity is also
/// shown as billboard/point, and skips items whose projection failed.
pub fn get_screen_space_positions(
    cluster_labels: bool,
    cluster_billboards: bool,
    cluster_points: bool,
    candidates: &[ClusterCandidate],
    points: &mut Vec<ScreenSpacePoint>,
) {
    for candidate in candidates {
        // item.clusterShow = false — modeled by the flag below.
        if !candidate.show || !candidate.visible {
            continue;
        }

        let can_cluster_labels = cluster_labels && candidate.kind == ClusterItemKind::Label;
        let can_cluster_billboards = cluster_billboards && candidate.has_billboard;
        let can_cluster_points = cluster_points && candidate.has_point;
        if can_cluster_labels && (can_cluster_points || can_cluster_billboards) {
            continue;
        }

        let Some(coord) = candidate.screen_position else {
            continue;
        };

        points.push(ScreenSpacePoint {
            entity_id: candidate.entity_id.clone(),
            kind: candidate.kind,
            position: candidate.position,
            coord,
            bbox: candidate.bbox,
            label_bbox: candidate.label_bbox,
            index: candidate.index,
            clustered: false,
            cluster_show: false,
        });
    }
}

/// Port of `expandBoundingBox`.
pub fn expand_bounding_box(bbox: &mut BoundingRectangle, pixel_range: f64) {
    bbox.x -= pixel_range;
    bbox.y -= pixel_range;
    bbox.width += pixel_range * 2.0;
    bbox.height += pixel_range * 2.0;
}

/// Minimal faithful port of the `kdbush` dependency used by CesiumJS'
/// `EntityCluster.js` (static kd-tree spatial index, node size 64,
/// Floyd–Rivest selection during build).
///
/// Not mirrored as its own file because kdbush is an external npm
/// dependency, not a CesiumJS source file (file-mirroring rule applies to
/// `packages/engine/Source` only).
#[derive(Debug)]
struct KdIndex {
    node_size: usize,
    ids: Vec<usize>,
    coords: Vec<f64>,
    pos: usize,
    built: bool,
}

impl KdIndex {
    fn new(num_items: usize) -> Self {
        Self {
            node_size: 64,
            ids: vec![0; num_items],
            coords: vec![0.0; num_items * 2],
            pos: 0,
            built: false,
        }
    }

    fn add(&mut self, x: f64, y: f64) {
        debug_assert!(!self.built, "KdIndex::add called after finish.");
        debug_assert!(self.pos < self.ids.len(), "KdIndex overflow.");
        self.ids[self.pos] = self.pos;
        self.coords[2 * self.pos] = x;
        self.coords[2 * self.pos + 1] = y;
        self.pos += 1;
    }

    fn finish(&mut self) {
        debug_assert_eq!(self.pos, self.ids.len(), "KdIndex finished before full.");
        self.built = true;
        if self.ids.len() > 1 {
            let last = self.ids.len() - 1;
            Self::sort_rec(
                &mut self.ids,
                &mut self.coords,
                self.node_size,
                0,
                last,
                0,
            );
        }
    }

    /// Returns the indices (into the original insertion order) of all points
    /// inside the axis-aligned range `[min_x, max_x] x [min_y, max_y]`.
    fn range(&self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Vec<usize> {
        let mut result = Vec::new();
        if self.ids.is_empty() {
            return result;
        }

        let mut stack: Vec<(usize, usize, usize)> = vec![(0, self.ids.len() - 1, 0)];
        while let Some((left, right, axis)) = stack.pop() {
            if right - left <= self.node_size {
                for i in left..=right {
                    let x = self.coords[2 * i];
                    let y = self.coords[2 * i + 1];
                    if x >= min_x && x <= max_x && y >= min_y && y <= max_y {
                        result.push(self.ids[i]);
                    }
                }
                continue;
            }

            let m = (left + right) >> 1;
            let x = self.coords[2 * m];
            let y = self.coords[2 * m + 1];
            if x >= min_x && x <= max_x && y >= min_y && y <= max_y {
                result.push(self.ids[m]);
            }

            if (axis == 0 && min_x <= x) || (axis == 1 && min_y <= y) {
                stack.push((left, m - 1, 1 - axis));
            }
            if (axis == 0 && max_x >= x) || (axis == 1 && max_y >= y) {
                stack.push((m + 1, right, 1 - axis));
            }
        }

        result
    }

    fn sort_rec(
        ids: &mut [usize],
        coords: &mut [f64],
        node_size: usize,
        left: usize,
        right: usize,
        axis: usize,
    ) {
        if right <= left || right - left <= node_size {
            return;
        }

        let m = (left + right) >> 1;
        Self::select(ids, coords, m, left, right, axis);
        Self::sort_rec(ids, coords, node_size, left, m - 1, 1 - axis);
        Self::sort_rec(ids, coords, node_size, m + 1, right, 1 - axis);
    }

    fn select(
        ids: &mut [usize],
        coords: &mut [f64],
        k: usize,
        mut left: usize,
        mut right: usize,
        axis: usize,
    ) {
        while right > left {
            if right - left > 600 {
                let n = (right - left + 1) as f64;
                let m = n.ln();
                let s = 0.5 * (2.0 * m / 3.0).exp();
                let sd = 0.5
                    * (m * s * (n - s) / n).sqrt()
                    * if (k as f64) - n / 2.0 < 0.0 { -1.0 } else { 1.0 };
                let new_left = (left as isize)
                    .max((k as f64 - (k as f64) * s / n + sd).floor() as isize)
                    as usize;
                let new_right = (right as isize)
                    .min((k as f64 + (n - k as f64) * s / n + sd).floor() as isize)
                    as usize;
                Self::select(ids, coords, k, new_left, new_right, axis);
            }

            let value = coords[2 * k + axis];
            let mut i = left;
            let mut j = right;

            Self::swap_item(ids, coords, left, k);
            if coords[2 * right + axis] > value {
                Self::swap_item(ids, coords, left, right);
            }

            while i < j {
                Self::swap_item(ids, coords, i, j);
                i += 1;
                j -= 1;
                while coords[2 * i + axis] < value {
                    i += 1;
                }
                while coords[2 * j + axis] > value {
                    j -= 1;
                }
            }

            if coords[2 * left + axis] == value {
                Self::swap_item(ids, coords, left, j);
            } else {
                j += 1;
                Self::swap_item(ids, coords, j, right);
            }

            if j <= k {
                left = j + 1;
            }
            if k <= j {
                // JS would set right = j - 1 = -1 here; left was bumped past
                // right in that case, so clamping to 0 terminates the loop.
                right = if j == 0 { 0 } else { j - 1 };
            }
        }
    }

    fn swap_item(ids: &mut [usize], coords: &mut [f64], i: usize, j: usize) {
        ids.swap(i, j);
        coords.swap(2 * i, 2 * j);
        coords.swap(2 * i + 1, 2 * j + 1);
    }
}

/// Bookkeeping record for one previous cluster, used by the zoom-in pass.
///
/// Port of the objects pushed into `_previousClusters`
/// (`{ position, width, height, minimumWidth, minimumHeight }`).
#[derive(Clone, Copy, Debug)]
pub struct ClusterGeometry {
    /// World position of the cluster centroid.
    pub position: Cartesian3,
    /// Total screen-space width of the merged bounding boxes.
    pub width: f64,
    /// Total screen-space height of the merged bounding boxes.
    pub height: f64,
    /// Width of the first (minimum) bounding box.
    pub minimum_width: f64,
    /// Height of the first (minimum) bounding box.
    pub minimum_height: f64,
}

/// Mutable styling state of one cluster primitive.
#[derive(Clone, Debug, Default)]
pub struct ClusterPrimitiveState {
    /// Whether the primitive is shown.
    pub show: bool,
    /// Label text (labels only).
    pub text: String,
    /// World position of the primitive.
    pub position: Cartesian3,
}

/// A single cluster primitive (billboard, label or point).
///
/// gpu-limited: in CesiumJS this is a `Billboard`/`Label`/`PointPrimitive`
/// owned by the cluster collections; the port keeps only the style state
/// that `clusterEvent` listeners may read/write, with interior mutability
/// mirroring the JS object-reference semantics.
#[derive(Debug, Default)]
pub struct ClusterPrimitive {
    state: RefCell<ClusterPrimitiveState>,
}

impl ClusterPrimitive {
    fn new(state: ClusterPrimitiveState) -> Self {
        Self {
            state: RefCell::new(state),
        }
    }

    /// Whether the primitive is shown.
    #[must_use]
    pub fn show(&self) -> bool {
        self.state.borrow().show
    }

    /// Sets whether the primitive is shown.
    pub fn set_show(&self, value: bool) {
        self.state.borrow_mut().show = value;
    }

    /// The label text.
    #[must_use]
    pub fn text(&self) -> String {
        self.state.borrow().text.clone()
    }

    /// Sets the label text.
    pub fn set_text(&self, value: &str) {
        self.state.borrow_mut().text = value.to_string();
    }

    /// The world position.
    #[must_use]
    pub fn position(&self) -> Cartesian3 {
        self.state.borrow().position
    }

    /// Sets the world position.
    pub fn set_position(&self, value: Cartesian3) {
        self.state.borrow_mut().position = value;
    }
}

/// The three primitives representing one cluster.
///
/// Port of the `cluster` object created by `addCluster`
/// (`{ billboard, label, point }`).
#[derive(Debug)]
pub struct ClusterPrimitives {
    /// The billboard primitive (hidden by default).
    pub billboard: ClusterPrimitive,
    /// The label primitive (shown by default, text = point count).
    pub label: ClusterPrimitive,
    /// The point primitive (hidden by default).
    pub point: ClusterPrimitive,
}

/// Payload raised on [`EntityCluster::cluster_event`].
///
/// Port of the `newClusterCallback` arguments
/// (`clusteredEntities`, `cluster`).
#[derive(Debug)]
pub struct ClusterEventPayload {
    /// The entities contained in the cluster (entity ids).
    pub clustered_entities: Vec<String>,
    /// The billboard/label/point primitives representing the cluster.
    pub cluster: ClusterPrimitives,
}

/// Per-entity indices into the virtual label/billboard/point collections.
///
/// Port of the `_collectionIndicesByEntity[entityId]` entries.
#[derive(Clone, Debug, Default)]
pub struct CollectionIndices {
    /// Index into the billboard collection, if allocated.
    pub billboard_index: Option<usize>,
    /// Index into the label collection, if allocated.
    pub label_index: Option<usize>,
    /// Index into the point collection, if allocated.
    pub point_index: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CollectionKind {
    Label,
    Billboard,
    Point,
}

/// Projection callback replacing `Billboard._computeScreenSpacePosition` in
/// the zoom-in reuse pass (gpu-limited in JS). Returns `None` when the
/// position is not projectable (occluded / behind camera), mirroring the JS
/// `occluder.isPointVisible` + `!defined(coord)` checks.
pub type ProjectionFn = dyn Fn(&Cartesian3) -> Option<Cartesian2>;

/// Per-frame inputs for [`EntityCluster::update`].
///
/// Rust stand-in for `frameState` plus the data gathered by
/// `getScreenSpacePositions`.
pub struct ClusterFrame<'a> {
    /// Screen-space points participating this frame (caller pre-filters via
    /// [`get_screen_space_positions`]).
    pub points: Vec<ScreenSpacePoint>,
    /// Camera height this frame (`scene.camera.positionCartographic.height`).
    pub current_height: f64,
    /// Projection callback for previous-cluster positions; `None` disables
    /// the zoom-in reuse pass' position projection (clusters are then
    /// rebuilt from scratch by the main pass). gpu-limited injection point.
    pub project: Option<&'a ProjectionFn>,
}

/// Defines how screen space objects (billboards, points, labels) are
/// clustered.
///
/// Port of `EntityCluster`. The pure clustering logic (dirty-flag property
/// model, index bookkeeping, KDBush-based declutter algorithm, cluster
/// event) is substantive; rendering-context interactions are injected or
/// marked gpu-limited, see the module docs.
pub struct EntityCluster {
    enabled: bool,
    pixel_range: f64,
    minimum_cluster_size: usize,
    cluster_billboards: bool,
    cluster_labels: bool,
    cluster_points: bool,

    /// Virtual collection lengths; `Some` mirrors `defined(this._xCollection)`
    /// (the collection is created lazily on first `getLabel`/... call).
    /// gpu-limited: real collections are Scene primitives.
    label_collection: Option<usize>,
    billboard_collection: Option<usize>,
    point_collection: Option<usize>,

    cluster_label_collection: Option<usize>,
    cluster_billboard_collection: Option<usize>,
    cluster_point_collection: Option<usize>,

    collection_indices_by_entity: HashMap<String, CollectionIndices>,

    unused_label_indices: VecDeque<usize>,
    unused_billboard_indices: VecDeque<usize>,
    unused_point_indices: VecDeque<usize>,

    previous_clusters: Vec<ClusterGeometry>,
    previous_height: Option<f64>,

    enabled_dirty: bool,
    cluster_dirty: bool,

    initialized: bool,
    is_destroyed: bool,

    cluster_event: Event<ClusterEventPayload>,

    /// Determines if entities in this collection will be shown.
    /// Plain field, mirroring the JS `this.show` property.
    pub show: bool,
}

impl EntityCluster {
    /// Creates a new entity cluster with default options.
    ///
    /// Port of `new EntityCluster()`.
    #[must_use]
    pub fn new() -> Self {
        Self::with_options(&EntityClusterOptions::default())
    }

    /// Creates a new entity cluster from the given options.
    ///
    /// Port of `new EntityCluster(options)`.
    #[must_use]
    pub fn with_options(options: &EntityClusterOptions) -> Self {
        Self {
            enabled: options.enabled.unwrap_or(false),
            pixel_range: options.pixel_range.unwrap_or(80.0),
            minimum_cluster_size: options.minimum_cluster_size.unwrap_or(2),
            cluster_billboards: options.cluster_billboards.unwrap_or(true),
            cluster_labels: options.cluster_labels.unwrap_or(true),
            cluster_points: options.cluster_points.unwrap_or(true),
            label_collection: None,
            billboard_collection: None,
            point_collection: None,
            cluster_label_collection: None,
            cluster_billboard_collection: None,
            cluster_point_collection: None,
            collection_indices_by_entity: HashMap::new(),
            unused_label_indices: VecDeque::new(),
            unused_billboard_indices: VecDeque::new(),
            unused_point_indices: VecDeque::new(),
            previous_clusters: Vec::new(),
            previous_height: None,
            enabled_dirty: false,
            cluster_dirty: false,
            initialized: false,
            is_destroyed: false,
            cluster_event: Event::new(),
            show: options.show.unwrap_or(true),
        }
    }

    /// Port of `_initialize`.
    ///
    /// gpu-limited: CesiumJS stores the scene and subscribes the declutter
    /// callback to `scene.camera.changed`; the port marks the cluster as
    /// initialized and expects the caller to drive [`Self::update`] each
    /// frame (equivalent to the camera-changed callback firing).
    pub fn initialize(&mut self) {
        self.initialized = true;
    }

    /// Returns whether this cluster has been initialized.
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Returns whether this cluster has been destroyed.
    ///
    /// Note: per CesiumJS semantics the instance remains reusable after
    /// destroy (e.g. when a data source moves between displays).
    #[must_use]
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Whether clustering is enabled.
    ///
    /// Port of the `enabled` getter.
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Sets whether clustering is enabled.
    ///
    /// Port of the `enabled` setter (tracks `_enabledDirty`).
    pub fn set_enabled(&mut self, value: bool) {
        self.enabled_dirty = value != self.enabled;
        self.enabled = value;
    }

    /// The pixel range to extend the screen space bounding box.
    ///
    /// Port of the `pixelRange` getter.
    #[must_use]
    pub fn pixel_range(&self) -> f64 {
        self.pixel_range
    }

    /// Sets the pixel range.
    ///
    /// Port of the `pixelRange` setter (tracks `_clusterDirty`).
    pub fn set_pixel_range(&mut self, value: f64) {
        #[cfg(debug_assertions)]
        debug_assert!(value >= 0.0, "pixelRange must be non-negative.");
        self.cluster_dirty = self.cluster_dirty || value != self.pixel_range;
        self.pixel_range = value;
    }

    /// The minimum number of screen space objects that can be clustered.
    ///
    /// Port of the `minimumClusterSize` getter.
    #[must_use]
    pub fn minimum_cluster_size(&self) -> usize {
        self.minimum_cluster_size
    }

    /// Sets the minimum cluster size.
    ///
    /// Port of the `minimumClusterSize` setter (tracks `_clusterDirty`).
    pub fn set_minimum_cluster_size(&mut self, value: usize) {
        #[cfg(debug_assertions)]
        debug_assert!(value >= 1, "minimumClusterSize must be at least 1.");
        self.cluster_dirty = self.cluster_dirty || value != self.minimum_cluster_size;
        self.minimum_cluster_size = value;
    }

    /// Whether clustering billboard entities is enabled.
    ///
    /// Port of the `clusterBillboards` getter.
    #[must_use]
    pub fn cluster_billboards(&self) -> bool {
        self.cluster_billboards
    }

    /// Sets whether clustering billboard entities is enabled.
    ///
    /// Port of the `clusterBillboards` setter (tracks `_clusterDirty`).
    pub fn set_cluster_billboards(&mut self, value: bool) {
        self.cluster_dirty = self.cluster_dirty || value != self.cluster_billboards;
        self.cluster_billboards = value;
    }

    /// Whether clustering label entities is enabled.
    ///
    /// Port of the `clusterLabels` getter.
    #[must_use]
    pub fn cluster_labels(&self) -> bool {
        self.cluster_labels
    }

    /// Sets whether clustering label entities is enabled.
    ///
    /// Port of the `clusterLabels` setter (tracks `_clusterDirty`).
    pub fn set_cluster_labels(&mut self, value: bool) {
        self.cluster_dirty = self.cluster_dirty || value != self.cluster_labels;
        self.cluster_labels = value;
    }

    /// Whether clustering point entities is enabled.
    ///
    /// Port of the `clusterPoints` getter.
    #[must_use]
    pub fn cluster_points(&self) -> bool {
        self.cluster_points
    }

    /// Sets whether clustering point entities is enabled.
    ///
    /// Port of the `clusterPoints` setter (tracks `_clusterDirty`).
    pub fn set_cluster_points(&mut self, value: bool) {
        self.cluster_dirty = self.cluster_dirty || value != self.cluster_points;
        self.cluster_points = value;
    }

    /// The event raised when a new cluster will be displayed.
    ///
    /// Port of the `clusterEvent` getter; listeners receive a
    /// [`ClusterEventPayload`] (`newClusterCallback` signature).
    #[must_use]
    pub fn cluster_event(&self) -> &Event<ClusterEventPayload> {
        &self.cluster_event
    }

    /// Returns true when all clustered data has been rendered.
    ///
    /// Port of the `ready` getter.
    ///
    /// DEVIATION (gpu-limited): CesiumJS additionally requires the billboard
    /// and label collections to be `ready` (image atlas / glyph creation);
    /// the port treats collections as immediately ready.
    #[must_use]
    pub fn ready(&self) -> bool {
        !self.enabled_dirty && !self.cluster_dirty
    }

    /// Returns whether `_enabledDirty` is currently set (test/inspection aid).
    #[must_use]
    pub fn enabled_dirty(&self) -> bool {
        self.enabled_dirty
    }

    /// Returns whether `_clusterDirty` is currently set (test/inspection aid).
    #[must_use]
    pub fn cluster_dirty(&self) -> bool {
        self.cluster_dirty
    }

    /// Number of clusters produced by the last declutter pass.
    ///
    /// Mirrors `_clusterLabelCollection.length` in CesiumJS (each cluster
    /// adds exactly one label/billboard/point).
    #[must_use]
    pub fn cluster_count(&self) -> usize {
        self.cluster_label_collection.unwrap_or(0)
    }

    /// Whether the cluster collections currently exist.
    ///
    /// Mirrors `defined(this._clusterLabelCollection)` in CesiumJS.
    #[must_use]
    pub fn has_cluster_collections(&self) -> bool {
        self.cluster_label_collection.is_some()
    }

    /// The cluster geometries recorded by the last declutter pass
    /// (`_previousClusters`, inspection aid for the zoom-in reuse pass).
    #[must_use]
    pub fn previous_clusters(&self) -> &[ClusterGeometry] {
        &self.previous_clusters
    }

    /// The camera height recorded by the last declutter pass.
    #[must_use]
    pub fn previous_height(&self) -> Option<f64> {
        self.previous_height
    }

    /// Returns the collection indices recorded for the given entity id
    /// (`_collectionIndicesByEntity[entityId]`, inspection aid).
    #[must_use]
    pub fn collection_indices(&self, entity_id: &str) -> Option<&CollectionIndices> {
        self.collection_indices_by_entity.get(entity_id)
    }

    /// Port of `hasLabelIndex`.
    #[must_use]
    pub fn has_label_index(&self, entity_id: &str) -> bool {
        self.collection_indices_by_entity
            .get(entity_id)
            .and_then(|indices| indices.label_index)
            .is_some()
    }

    /// Returns a new label for the entity.
    ///
    /// Port of `getLabel` (a `createGetEntity` instantiation). Returns the
    /// index of the label within the (virtual) label collection.
    pub fn get_label(&mut self, entity_id: &str) -> usize {
        self.get_entity_item(entity_id, CollectionKind::Label)
    }

    /// Returns a new billboard for the entity.
    ///
    /// Port of `getBillboard`.
    pub fn get_billboard(&mut self, entity_id: &str) -> usize {
        self.get_entity_item(entity_id, CollectionKind::Billboard)
    }

    /// Returns a new point for the entity.
    ///
    /// Port of `getPoint`.
    pub fn get_point(&mut self, entity_id: &str) -> usize {
        self.get_entity_item(entity_id, CollectionKind::Point)
    }

    /// Port of `createGetEntity` (shared by getLabel/getBillboard/getPoint).
    fn get_entity_item(&mut self, entity_id: &str, kind: CollectionKind) -> usize {
        // Create the (virtual) collection on first use.
        let length_slot = match kind {
            CollectionKind::Label => &mut self.label_collection,
            CollectionKind::Billboard => &mut self.billboard_collection,
            CollectionKind::Point => &mut self.point_collection,
        };
        if length_slot.is_none() {
            *length_slot = Some(0);
        }

        let indices = self
            .collection_indices_by_entity
            .entry(entity_id.to_string())
            .or_default();

        let existing = match kind {
            CollectionKind::Label => indices.label_index,
            CollectionKind::Billboard => indices.billboard_index,
            CollectionKind::Point => indices.point_index,
        };
        if let Some(index) = existing {
            return index;
        }

        let unused_indices = match kind {
            CollectionKind::Label => &mut self.unused_label_indices,
            CollectionKind::Billboard => &mut self.unused_billboard_indices,
            CollectionKind::Point => &mut self.unused_point_indices,
        };
        let length_slot = match kind {
            CollectionKind::Label => &mut self.label_collection,
            CollectionKind::Billboard => &mut self.billboard_collection,
            CollectionKind::Point => &mut self.point_collection,
        };

        let index = if let Some(index) = unused_indices.pop_front() {
            index
        } else {
            let index = length_slot.unwrap_or(0);
            *length_slot = Some(index + 1);
            index
        };

        let indices = self
            .collection_indices_by_entity
            .get_mut(entity_id)
            .expect("indices entry created above");
        match kind {
            CollectionKind::Label => indices.label_index = Some(index),
            CollectionKind::Billboard => indices.billboard_index = Some(index),
            CollectionKind::Point => indices.point_index = Some(index),
        }

        // DEVIATION: CesiumJS defers `_clusterDirty = true` to a microtask
        // (Promise.resolve().then); the port sets it immediately, which only
        // ever triggers one additional (idempotent) declutter pass.
        self.cluster_dirty = true;

        index
    }

    /// Removes the label associated with an entity so it can be reused.
    ///
    /// Port of `removeLabel`.
    pub fn remove_label(&mut self, entity_id: &str) {
        if self.label_collection.is_none() {
            return;
        }
        let Some(index) = self
            .collection_indices_by_entity
            .get_mut(entity_id)
            .and_then(|indices| indices.label_index.take())
        else {
            return;
        };

        self.remove_entity_indices_if_unused(entity_id);

        // gpu-limited: CesiumJS resets label.show/text/id on the primitive.

        self.unused_label_indices.push_back(index);
        self.cluster_dirty = true;
    }

    /// Removes the billboard associated with an entity so it can be reused.
    ///
    /// Port of `removeBillboard`.
    pub fn remove_billboard(&mut self, entity_id: &str) {
        if self.billboard_collection.is_none() {
            return;
        }
        let Some(index) = self
            .collection_indices_by_entity
            .get_mut(entity_id)
            .and_then(|indices| indices.billboard_index.take())
        else {
            return;
        };

        self.remove_entity_indices_if_unused(entity_id);

        // gpu-limited: CesiumJS resets billboard.id/show/image on the primitive.

        self.unused_billboard_indices.push_back(index);
        self.cluster_dirty = true;
    }

    /// Removes the point associated with an entity so it can be reused.
    ///
    /// Port of `removePoint`.
    pub fn remove_point(&mut self, entity_id: &str) {
        if self.point_collection.is_none() {
            return;
        }
        let Some(index) = self
            .collection_indices_by_entity
            .get_mut(entity_id)
            .and_then(|indices| indices.point_index.take())
        else {
            return;
        };

        self.remove_entity_indices_if_unused(entity_id);

        // gpu-limited: CesiumJS resets point.show/id on the primitive.

        self.unused_point_indices.push_back(index);
        self.cluster_dirty = true;
    }

    /// Port of `removeEntityIndicesIfUnused`.
    fn remove_entity_indices_if_unused(&mut self, entity_id: &str) {
        if let Some(indices) = self.collection_indices_by_entity.get(entity_id) {
            if indices.billboard_index.is_none()
                && indices.label_index.is_none()
                && indices.point_index.is_none()
            {
                self.collection_indices_by_entity.remove(entity_id);
            }
        }
    }

    /// Gets the draw commands for the clustered billboards/points/labels if
    /// enabled, otherwise queues the draw commands for billboards/points/
    /// labels created for entities.
    ///
    /// Port of `EntityCluster.prototype.update`, driven with the pure
    /// [`ClusterFrame`] inputs instead of `frameState`.
    pub fn update(&mut self, frame: &mut ClusterFrame<'_>) {
        if !self.show {
            return;
        }

        // gpu-limited: CesiumJS pre-updates the label collection (glyph
        // creation) and billboard collection (image atlas) here while they
        // are not ready, swallowing the queued commands.

        if self.enabled_dirty {
            self.enabled_dirty = false;
            self.update_enable(frame);
            self.cluster_dirty = true;
        }

        if self.cluster_dirty {
            self.declutter(frame);

            // DEVIATION (gpu-limited): CesiumJS keeps `_clusterDirty` while
            // the label/billboard collections are not ready; the port clears
            // it since collection readiness is a render-surface concern.
            self.cluster_dirty = false;
        }

        // gpu-limited: per-frame `update(frameState)` of the cluster and
        // entity label/billboard/point collections.
    }

    /// Port of `updateEnable`.
    fn update_enable(&mut self, frame: &mut ClusterFrame<'_>) {
        if self.enabled {
            return;
        }

        // gpu-limited: CesiumJS destroys the three cluster collections here;
        // the port drops the virtual lengths.
        self.cluster_label_collection = None;
        self.cluster_billboard_collection = None;
        self.cluster_point_collection = None;

        self.disable_collection_clustering(frame);
    }

    /// Port of `disableCollectionClustering`.
    ///
    /// DEVIATION: CesiumJS iterates the three entity collections restoring
    /// `clusterShow` on every item; the port restores it on this frame's
    /// participants, which carry the same flag.
    fn disable_collection_clustering(&self, frame: &mut ClusterFrame<'_>) {
        for point in frame.points.iter_mut() {
            point.cluster_show = true;
        }
    }

    /// The declutter pass: recomputes clusters from the current
    /// screen-space points.
    ///
    /// Port of the callback created by `createDeclutterCallback` (stored as
    /// `_cluster` in CesiumJS, wired to `scene.camera.changed`).
    fn declutter(&mut self, frame: &mut ClusterFrame<'_>) {
        // JS callback early-return (createDeclutterCallback):
        // `if ((defined(amount) && amount < 0.05) || !entityCluster.enabled)
        //   return;`
        // DEVIATION: the camera-change `amount` threshold is a scene
        // concern; the port keeps the enabled guard only.
        if !self.enabled {
            return;
        }

        let pixel_range = self.pixel_range;
        let minimum_cluster_size = self.minimum_cluster_size;

        let mut clusters = std::mem::take(&mut self.previous_clusters);
        let mut new_clusters: Vec<ClusterGeometry> = Vec::new();

        let previous_height = self.previous_height;
        let current_height = frame.current_height;

        // gpu-limited: CesiumJS creates (or removeAll's) the cluster
        // label/billboard/point collections here.
        if let Some(length) = self.cluster_label_collection.as_mut() {
            *length = 0;
        }
        if let Some(length) = self.cluster_billboard_collection.as_mut() {
            *length = 0;
        }
        if let Some(length) = self.cluster_point_collection.as_mut() {
            *length = 0;
        }

        let project = frame.project;
        let points = &mut frame.points;

        // JS gates each collection by its cluster flag before scanning it
        // (`if (entityCluster._clusterLabels) getScreenSpacePositions(...)`
        // etc.); the port filters participants by item kind and mirrors
        // getScreenSpacePositions' `item.clusterShow = false` reset.
        let mut participants: Vec<usize> = Vec::new();
        for i in 0..points.len() {
            let participates = match points[i].kind {
                ClusterItemKind::Label => self.cluster_labels,
                ClusterItemKind::Billboard => self.cluster_billboards,
                ClusterItemKind::Point => self.cluster_points,
            };
            if participates {
                points[i].clustered = false;
                points[i].cluster_show = false;
                participants.push(i);
            } else {
                points[i].cluster_show = true;
            }
        }

        if !participants.is_empty() {
            let mut index = KdIndex::new(participants.len());
            for &i in &participants {
                index.add(points[i].coord.x, points[i].coord.y);
            }
            index.finish();

            // Zoom-in pass: re-anchor previous clusters while the camera
            // descends so cluster positions stay stable.
            if let Some(prev_height) = previous_height {
                if current_height < prev_height {
                    for cluster in clusters.iter_mut() {
                        // gpu-limited in JS: occluder.isPointVisible +
                        // Billboard._computeScreenSpacePosition; both fold
                        // into the injected projection callback.
                        let Some(coord) = project.and_then(|f| f(&cluster.position)) else {
                            continue;
                        };

                        let factor = 1.0 - current_height / prev_height;
                        cluster.width *= factor;
                        cluster.height *= factor;

                        let width = cluster.width.max(cluster.minimum_width);
                        let height = cluster.height.max(cluster.minimum_height);
                        cluster.width = width;
                        cluster.height = height;

                        let min_x = coord.x - width * 0.5;
                        let min_y = coord.y - height * 0.5;
                        let max_x = coord.x + width;
                        let max_y = coord.y + height;

                        let neighbors = index.range(min_x, min_y, max_x, max_y);

                        let mut num_points = 0usize;
                        let mut ids: Vec<String> = Vec::new();
                        for &neighbor_slot in &neighbors {
                            let neighbor_point = &points[participants[neighbor_slot]];
                            if !neighbor_point.clustered {
                                num_points += 1;
                                ids.push(neighbor_point.entity_id.clone());
                            }
                        }

                        if num_points >= minimum_cluster_size {
                            let position = cluster.position;
                            new_clusters.push(*cluster);
                            self.add_cluster(position, num_points, ids);

                            for &neighbor_slot in &neighbors {
                                points[participants[neighbor_slot]].clustered = true;
                            }
                        }
                    }
                }
            }

            let length = participants.len();
            for slot in 0..length {
                let i = participants[slot];
                if points[i].clustered {
                    continue;
                }

                points[i].clustered = true;

                let bbox = self.get_bounding_box(points, i, pixel_range);
                let mut total_bbox = bbox;

                let neighbors =
                    index.range(bbox.x, bbox.y, bbox.x + bbox.width, bbox.y + bbox.height);

                let mut cluster_position = points[i].position;
                let mut num_points = 1usize;
                let mut ids = vec![points[i].entity_id.clone()];

                for &neighbor_slot in &neighbors {
                    let neighbor_index = participants[neighbor_slot];
                    if !points[neighbor_index].clustered {
                        let neighbor_bbox =
                            self.get_bounding_box(points, neighbor_index, pixel_range);

                        cluster_position = Cartesian3::add_new(
                            &points[neighbor_index].position,
                            &cluster_position,
                        );

                        total_bbox = BoundingRectangle::union_new(&total_bbox, &neighbor_bbox);
                        num_points += 1;

                        ids.push(points[neighbor_index].entity_id.clone());
                    }
                }

                if num_points >= minimum_cluster_size {
                    let position = Cartesian3::multiply_by_scalar_new(
                        &cluster_position,
                        1.0 / num_points as f64,
                    );
                    self.add_cluster(position, num_points, ids);
                    new_clusters.push(ClusterGeometry {
                        position,
                        width: total_bbox.width,
                        height: total_bbox.height,
                        minimum_width: bbox.width,
                        minimum_height: bbox.height,
                    });

                    for &neighbor_slot in &neighbors {
                        points[participants[neighbor_slot]].clustered = true;
                    }
                } else {
                    Self::add_non_clustered_item(points, i);
                }
            }
        }

        // gpu-limited in JS: empty cluster collections are destroyed
        // (set back to undefined).
        if self.cluster_label_collection == Some(0) {
            self.cluster_label_collection = None;
        }
        if self.cluster_billboard_collection == Some(0) {
            self.cluster_billboard_collection = None;
        }
        if self.cluster_point_collection == Some(0) {
            self.cluster_point_collection = None;
        }

        self.previous_clusters = new_clusters;
        self.previous_height = Some(current_height);
    }

    /// Port of `getBoundingBox`.
    ///
    /// DEVIATION (gpu-limited): CesiumJS derives the box from
    /// `Label/Billboard/PointPrimitive.getScreenSpaceBoundingBox` (glyph and
    /// image-atlas measurement) and unions the entity's label box looked up
    /// through the collection indices; the port consumes the precomputed
    /// boxes from [`ScreenSpacePoint`] and performs the same pixelRange
    /// expansion and label-union logic.
    fn get_bounding_box(
        &self,
        points: &[ScreenSpacePoint],
        index: usize,
        pixel_range: f64,
    ) -> BoundingRectangle {
        let item = &points[index];

        let mut result = item.bbox;
        expand_bounding_box(&mut result, pixel_range);

        if self.cluster_labels
            && item.kind != ClusterItemKind::Label
            && self.has_label_index(&item.entity_id)
        {
            if let Some(label_bbox) = item.label_bbox {
                let mut label_bbox = label_bbox;
                expand_bounding_box(&mut label_bbox, pixel_range);
                result = BoundingRectangle::union_new(&result, &label_bbox);
            }
        }

        result
    }

    /// Port of `addNonClusteredItem`.
    ///
    /// DEVIATION (gpu-limited): CesiumJS additionally restores `clusterShow`
    /// on the entity's associated label primitive (looked up via the
    /// collection indices); the port models associated-label visibility
    /// through the owning point's flag.
    fn add_non_clustered_item(points: &mut [ScreenSpacePoint], index: usize) {
        points[index].cluster_show = true;
    }

    /// Port of `addCluster`: registers one entry in each cluster collection
    /// and raises `clusterEvent` with the default styling (billboard and
    /// point hidden, label shown with the point count).
    fn add_cluster(&mut self, position: Cartesian3, num_points: usize, ids: Vec<String>) {
        for slot in [
            &mut self.cluster_billboard_collection,
            &mut self.cluster_label_collection,
            &mut self.cluster_point_collection,
        ] {
            *slot = Some(slot.unwrap_or(0) + 1);
        }

        // DEVIATION: CesiumJS uses `numPoints.toLocaleString()` (locale
        // dependent); the port uses the plain decimal representation.
        let label_text = num_points.to_string();

        let payload = ClusterEventPayload {
            clustered_entities: ids,
            cluster: ClusterPrimitives {
                billboard: ClusterPrimitive::new(ClusterPrimitiveState {
                    show: false,
                    text: String::new(),
                    position,
                }),
                label: ClusterPrimitive::new(ClusterPrimitiveState {
                    show: true,
                    text: label_text,
                    position,
                }),
                point: ClusterPrimitive::new(ClusterPrimitiveState {
                    show: false,
                    text: String::new(),
                    position,
                }),
            },
        };

        self.cluster_event.raise_event(&payload);
    }

    /// Destroys the WebGL resources held by this object.
    ///
    /// Port of `EntityCluster.prototype.destroy`. Per CesiumJS the instance
    /// remains reusable afterwards (e.g. when a data source is removed from
    /// one display and added to another).
    pub fn destroy(&mut self) {
        // gpu-limited: CesiumJS removes the camera.changed listener and
        // destroys all six primitive collections here.

        self.label_collection = None;
        self.billboard_collection = None;
        self.point_collection = None;

        self.cluster_label_collection = None;
        self.cluster_billboard_collection = None;
        self.cluster_point_collection = None;

        // JS sets `_collectionIndicesByEntity = undefined`; `get_entity_item`
        // recreates it lazily (mirrored by clearing the map).
        self.collection_indices_by_entity.clear();

        self.unused_label_indices.clear();
        self.unused_billboard_indices.clear();
        self.unused_point_indices.clear();

        self.previous_clusters.clear();
        self.previous_height = None;

        self.enabled_dirty = false;
        // JS also resets `_pixelRangeDirty`/`_minimumClusterSizeDirty`,
        // which are dead fields never read elsewhere in CesiumJS.
        self.cluster_dirty = false;

        self.is_destroyed = true;
    }
}

impl Default for EntityCluster {
    fn default() -> Self {
        Self::new()
    }
}
