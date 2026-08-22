//! Ported from packages/engine/Source/Core/createGuid.js

/// Creates a Globally unique identifier (GUID) string. A GUID is 128 bits
/// long, and can guarantee uniqueness across space and time.
///
/// Port of CesiumJS `createGuid()`. The JS implementation fills the RFC 4122
/// version-4 template `"xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx"` with random
/// hex digits; the Rust port produces an RFC 4122 v4 UUID directly, which
/// matches the same template (version nibble `4`, variant nibble `y` ∈
/// `8-b`).
///
/// # Example
/// ```ignore
/// let guid = cesium_core::create_guid::create_guid();
/// ```
#[must_use]
pub fn create_guid() -> String {
    // http://stackoverflow.com/questions/105034/how-to-create-a-guid-uuid-in-javascript
    uuid::Uuid::new_v4().to_string()
}
