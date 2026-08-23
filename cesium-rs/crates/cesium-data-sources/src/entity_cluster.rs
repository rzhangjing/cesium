//! Ported from `packages/engine/Source/DataSources/EntityCluster.js`.
//!
//! Controls clustering of entities that are close together.

/// Controls clustering of entities that are close together.
///
/// When clustering is enabled, entities within a pixel radius are
/// grouped into a single cluster billboard/label/point.
///
/// In CesiumJS, EntityCluster is a Primitive that manages:
/// - BillboardCollection for clustered billboards
/// - LabelCollection for clustered labels
/// - PointPrimitiveCollection for clustered points
/// - Event listeners for entity collection changes
///
/// DEVIATION: The actual cluster algorithm and primitive management
/// require BillboardCollection/LabelCollection/PointPrimitiveCollection
/// integration with the scene.
pub struct EntityCluster {
    /// Whether clustering is enabled.
    pub enabled: bool,
    /// The pixel radius for clustering.
    pub pixel_range: f64,
    /// The minimum number of entities to form a cluster.
    pub minimum_cluster_size: u32,
    /// Whether the cluster has been initialized.
    initialized: bool,
    is_destroyed: bool,
}

impl EntityCluster {
    /// Creates a new entity cluster.
    pub fn new() -> Self {
        Self {
            enabled: false,
            pixel_range: 80.0,
            minimum_cluster_size: 2,
            initialized: false,
            is_destroyed: false,
        }
    }

    /// Initializes the entity cluster.
    ///
    /// In CesiumJS, this creates the internal BillboardCollection,
    /// LabelCollection, and PointPrimitiveCollection.
    pub fn initialize(&mut self) {
        if self.initialized {
            return;
        }
        // DEVIATION: Requires scene to create primitive collections
        self.initialized = true;
    }

    /// Returns whether this cluster has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Returns whether this cluster has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys this cluster.
    pub fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}

impl Default for EntityCluster {
    fn default() -> Self {
        Self::new()
    }
}
