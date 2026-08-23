//! Ported from `packages/engine/Source/DataSources/HeightReferenceOnEntityPropertyChanged.js`.

/// Callback invoked when an entity's height reference property changes.
///
/// This is used internally to update geometry when the height reference
/// (e.g., clamp to ground, relative to ground) changes.
pub fn on_height_reference_changed(_entity_id: &str, _old_value: u8, _new_value: u8) {
    // DEVIATION: Requires integration with geometry updaters to re-create geometry
}
