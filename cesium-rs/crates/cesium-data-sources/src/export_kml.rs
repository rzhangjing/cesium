//! Ported from `packages/engine/Source/DataSources/exportKml.js`.

/// Exports entities to KML format.
///
/// Returns the KML string representation of the given entities.
pub fn export_kml(_entity_ids: &[String]) -> String {
    // DEVIATION: Requires full KML generation logic
    String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<kml></kml>")
}
