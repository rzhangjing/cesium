//! cesium-ports-driving: Driving ports (External → Domain)
//! Trait contracts for user/application interactions with the domain.
//!
//! In hexagonal architecture, driving ports define how external code
//! (UI, CLI, tests) interacts with the domain (adapters call these traits).

use cesium_camera::Camera;
use cesium_geospatial::{Cartographic, Ellipsoid, Rectangle};
use cesium_time::JulianDate;

// ============================================================================
// Viewer/Scene Control
// ============================================================================

/// The main viewer interface for controlling the 3D globe.
/// Maps to CesiumJS `Viewer` / `CesiumWidget` (API surface only)
pub trait ViewerApi {
    /// Gets a reference to the camera.
    fn camera(&self) -> &Camera;

    /// Gets a mutable reference to the camera.
    fn camera_mut(&mut self) -> &mut Camera;

    /// Gets the current simulation time.
    fn current_time(&self) -> JulianDate;

    /// Sets the current simulation time.
    fn set_current_time(&mut self, time: JulianDate);

    /// Gets the ellipsoid used by this viewer.
    fn ellipsoid(&self) -> &Ellipsoid;

    /// Renders a frame.
    fn render(&mut self);

    /// Resizes the viewport.
    fn resize(&mut self, width: u32, height: u32);
}

// ============================================================================
// Camera Control
// ============================================================================

/// Camera manipulation interface.
/// Maps to CesiumJS `Camera` methods exposed to users
pub trait CameraControl {
    /// Sets the camera view from position and orientation.
    fn set_view(
        &mut self,
        position: Cartographic,
        heading: f64,
        pitch: f64,
        roll: f64,
    );

    /// Flies the camera to a destination.
    fn fly_to(
        &mut self,
        destination: Cartographic,
        heading: Option<f64>,
        pitch: Option<f64>,
        roll: Option<f64>,
        duration_secs: f64,
    );

    /// Looks at a target position from a given range.
    fn look_at(
        &mut self,
        target: Cartographic,
        heading: f64,
        pitch: f64,
        range: f64,
    );

    /// Zooms in by the given amount.
    fn zoom_in(&mut self, amount: Option<f64>);

    /// Zooms out by the given amount.
    fn zoom_out(&mut self, amount: Option<f64>);

    /// Resets the camera to the home view.
    fn home(&mut self);
}

// ============================================================================
// Data Source Management
// ============================================================================

/// A unique identifier for a data source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DataSourceId(pub u64);

/// Manages data sources (GeoJSON, CZML, 3D Tiles, etc.)
pub trait DataSourceManager {
    /// Adds a data source from a URL.
    fn add_from_url(&mut self, url: &str) -> DataSourceId;

    /// Removes a data source.
    fn remove(&mut self, id: DataSourceId) -> bool;

    /// Shows/hides a data source.
    fn set_visible(&mut self, id: DataSourceId, visible: bool);

    /// Gets the bounding rectangle of a data source.
    fn bounds(&self, id: DataSourceId) -> Option<Rectangle>;
}

// ============================================================================
// Imagery Layer Management
// ============================================================================

/// A unique identifier for an imagery layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageryLayerId(pub u64);

/// Manages imagery layers.
pub trait ImageryLayerManager {
    /// Adds an imagery layer from a provider URL.
    fn add_layer(&mut self, url: &str) -> ImageryLayerId;

    /// Removes an imagery layer.
    fn remove_layer(&mut self, id: ImageryLayerId) -> bool;

    /// Sets the opacity of a layer (0.0 - 1.0).
    fn set_opacity(&mut self, id: ImageryLayerId, opacity: f64);

    /// Sets the visibility of a layer.
    fn set_visible(&mut self, id: ImageryLayerId, visible: bool);

    /// Raises a layer (increases its z-order).
    fn raise(&mut self, id: ImageryLayerId);

    /// Lowers a layer (decreases its z-order).
    fn lower(&mut self, id: ImageryLayerId);
}

// ============================================================================
// Terrain Management
// ============================================================================

/// Manages terrain providers.
pub trait TerrainManager {
    /// Sets the terrain provider from a URL.
    fn set_terrain(&mut self, url: &str);

    /// Disables terrain (uses ellipsoid surface).
    fn disable_terrain(&mut self);

    /// Gets whether terrain is enabled.
    fn is_terrain_enabled(&self) -> bool;

    /// Gets the height at a cartographic position.
    fn sample_height(&self, position: &Cartographic) -> Option<f64>;
}

// ============================================================================
// Entity/Primitive Management
// ============================================================================

/// A unique identifier for an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub u64);

/// Manages entities (points, polylines, polygons, models, etc.)
pub trait EntityManager {
    /// Adds a point entity.
    fn add_point(
        &mut self,
        position: Cartographic,
        color: [f32; 4],
        pixel_size: f64,
    ) -> EntityId;

    /// Adds a polyline entity.
    fn add_polyline(
        &mut self,
        positions: &[Cartographic],
        color: [f32; 4],
        width: f64,
    ) -> EntityId;

    /// Adds a polygon entity.
    fn add_polygon(
        &mut self,
        positions: &[Cartographic],
        color: [f32; 4],
    ) -> EntityId;

    /// Adds a 3D model entity.
    fn add_model(
        &mut self,
        position: Cartographic,
        uri: &str,
        scale: f64,
    ) -> EntityId;

    /// Removes an entity.
    fn remove(&mut self, id: EntityId) -> bool;

    /// Sets the position of an entity.
    fn set_position(&mut self, id: EntityId, position: Cartographic);

    /// Shows/hides an entity.
    fn set_visible(&mut self, id: EntityId, visible: bool);
}

// ============================================================================
// Picking/Selection
// ============================================================================

/// Result of a pick operation.
#[derive(Debug, Clone)]
pub enum PickResult {
    /// Picked an entity.
    Entity(EntityId),
    /// Picked a tile feature (3D Tiles).
    TileFeature { tileset_id: u64, feature_id: u64 },
    /// Picked the globe surface.
    GlobeSurface(Cartographic),
    /// Nothing was picked.
    None,
}

/// Provides picking/selection functionality.
pub trait Picking {
    /// Picks at screen coordinates (x, y).
    fn pick(&self, x: f64, y: f64) -> PickResult;

    /// Drills pick at screen coordinates (returns all objects at that position).
    fn drill_pick(&self, x: f64, y: f64) -> Vec<PickResult>;

    /// Gets the cartographic position at screen coordinates.
    fn pick_position(&self, x: f64, y: f64) -> Option<Cartographic>;
}
