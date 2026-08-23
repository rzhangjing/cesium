//! Ported from `packages/engine/Source/DataSources/DataSource.js`.
//!
//! An interface for data sources that provide entities.

use cesium_core::event::Event;
use crate::entity_collection::EntityCollection;

/// An interface for data sources that provide entities.
///
/// A data source provides a collection of entities that can be displayed
/// in a scene. Examples include CZML, GeoJSON, and KML data sources.
///
/// In CesiumJS, DataSource is an abstract class with:
/// - `name` (string)
/// - `clock` (DataSourceClock)
/// - `entities` (EntityCollection)
/// - `isLoading` (boolean)
/// - `loadingEvent` (Event)
/// - `changedEvent` (Event)
/// - `errorEvent` (Event)
/// - `show` (boolean)
/// - `load()` / `loadUrl()` / `loadJson()` methods
pub trait DataSource {
    /// Returns the name of this data source.
    fn name(&self) -> &str;

    /// Returns the entity collection for this data source.
    fn entities(&self) -> &EntityCollection;

    /// Returns whether this data source is currently loading.
    fn is_loading(&self) -> bool;

    /// Returns whether this data source has been destroyed.
    fn is_destroyed(&self) -> bool;

    /// Returns the `changed` event.
    fn changed_event(&self) -> &Event;

    /// Returns the `error` event.
    fn error_event(&self) -> &Event;

    /// Returns the `loading` event.
    fn loading_event(&self) -> &Event;

    /// Returns whether this data source is shown.
    fn show(&self) -> bool;

    /// Sets whether this data source is shown.
    fn set_show(&mut self, show: bool);

    /// Destroys this data source.
    fn destroy(&mut self);
}
