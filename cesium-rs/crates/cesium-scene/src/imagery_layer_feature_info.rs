//! Ported from `packages/engine/Source/Scene/ImageryLayerFeatureInfo.js`.

use serde_json::Value;

use cesium_core::cartographic::Cartographic;

/// Describes a feature identified by a GetFeatureInfo request from an
/// imagery layer.
///
/// Mirrors the CesiumJS `ImageryLayerFeatureInfo` class.
#[derive(Debug, Clone, Default)]
pub struct ImageryLayerFeatureInfo {
    /// The name / title of the feature.
    pub name: Option<String>,
    /// The description of the feature (may be HTML).
    pub description: Option<String>,
    /// The raw data associated with the feature.
    pub data: Option<Value>,
    /// The properties of the feature as key-value pairs.
    pub properties: Option<Value>,
    /// The position of the feature, if it is a point feature.
    pub position: Option<Cartographic>,
}

impl ImageryLayerFeatureInfo {
    /// Creates a new empty feature info.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configures the `name` from the properties object.
    ///
    /// Mirrors `configureNameFromProperties(properties)`. Looks for
    /// common name fields (`name`, `Name`, `title`, `id`, `ID`).
    pub fn configure_name_from_properties(&mut self, properties: &Value) {
        for key in &["name", "Name", "title", "id", "ID"] {
            if let Some(val) = properties.get(*key).and_then(|v| v.as_str()) {
                if !val.is_empty() {
                    self.name = Some(val.to_string());
                    return;
                }
            }
        }
    }

    /// Configures the `description` from the properties object.
    ///
    /// Mirrors `configureDescriptionFromProperties(properties)`. Builds
    /// an HTML table of all properties.
    pub fn configure_description_from_properties(&mut self, properties: &Value) {
        if let Value::Object(map) = properties {
            if map.is_empty() {
                return;
            }
            let mut html = String::from("<table class=\"cesium-infoBox-defaultTable\">");
            for (key, value) in map {
                let val_str = match value {
                    Value::String(s) => s.clone(),
                    Value::Null => String::new(),
                    other => other.to_string(),
                };
                html.push_str(&format!(
                    "<tbody><tr><th>{key}</th><td>{val_str}</td></tr></tbody>"
                ));
            }
            html.push_str("</table>");
            self.description = Some(html);
        }
    }
}
