//! Ported from `packages/engine/Source/Scene/GetFeatureInfoFormat.js`.
//!
//! Describes the format in which to request GetFeatureInfo from a WMS or
//! WMTS server.

use serde_json::Value;

use cesium_core::cartographic::Cartographic;

use crate::imagery_layer_feature_info::ImageryLayerFeatureInfo;

/// The type of GetFeatureInfo response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeatureInfoType {
    /// JSON / GeoJSON response.
    Json,
    /// XML response.
    Xml,
    /// HTML response.
    Html,
    /// Plain text response.
    Text,
}

/// Describes the format in which to request GetFeatureInfo from a Web Map
/// Service (WMS) or Web Map Tile Service (WMTS) server.
///
/// Mirrors the CesiumJS `GetFeatureInfoFormat` constructor.
#[derive(Debug, Clone)]
pub struct GetFeatureInfoFormat {
    /// The type of response (json, xml, html, text).
    pub type_: FeatureInfoType,
    /// The MIME format string sent to the server.
    pub format: String,
}

impl GetFeatureInfoFormat {
    /// Creates a new format for the given response type.
    ///
    /// If `format` is `None`, a default MIME type is chosen based on
    /// `type_`:
    /// - `Json` → `application/json`
    /// - `Xml` → `text/xml`
    /// - `Html` → `text/html`
    /// - `Text` → `text/plain`
    ///
    /// Mirrors `new GetFeatureInfoFormat(type, format, callback)`.
    pub fn new(type_: FeatureInfoType, format: Option<&str>) -> Self {
        let format = match format {
            Some(f) => f.to_string(),
            None => match type_ {
                FeatureInfoType::Json => "application/json",
                FeatureInfoType::Xml => "text/xml",
                FeatureInfoType::Html => "text/html",
                FeatureInfoType::Text => "text/plain",
            }
            .to_string(),
        };
        Self { type_, format }
    }

    /// Converts a GeoJSON response into an array of feature info
    /// objects.
    ///
    /// Mirrors `geoJsonToFeatureInfo(json)`.
    pub fn geo_json_to_feature_info(json: &Value) -> Vec<ImageryLayerFeatureInfo> {
        let mut result = Vec::new();

        let features = match json.get("features").and_then(|f| f.as_array()) {
            Some(arr) => arr,
            None => return result,
        };

        for feature in features {
            let mut feature_info = ImageryLayerFeatureInfo::new();
            feature_info.data = Some(feature.clone());

            if let Some(properties) = feature.get("properties") {
                feature_info.properties = Some(properties.clone());
                feature_info.configure_name_from_properties(properties);
                feature_info.configure_description_from_properties(properties);
            }

            // If this is a point feature, use the coordinates.
            if let Some(geometry) = feature.get("geometry") {
                if geometry.get("type").and_then(|t| t.as_str()) == Some("Point") {
                    if let Some(coords) = geometry
                        .get("coordinates")
                        .and_then(|c| c.as_array())
                    {
                        if coords.len() >= 2 {
                            let lon = coords[0].as_f64().unwrap_or(0.0);
                            let lat = coords[1].as_f64().unwrap_or(0.0);
                            feature_info.position =
                                Some(Cartographic::from_degrees_new(lon, lat, None));
                        }
                    }
                }
            }

            result.push(feature_info);
        }

        result
    }

    /// Converts a plain text response into an array of feature info
    /// objects.
    ///
    /// Mirrors `textToFeatureInfo(text)`. Returns `None` when the text
    /// indicates an empty body or a WMS ServiceExceptionReport.
    pub fn text_to_feature_info(text: &str) -> Option<Vec<ImageryLayerFeatureInfo>> {
        // Empty body tag → no features.
        let empty_body_regex = regex::Regex::new(r"(?i)<body>\s*</body>").unwrap();
        if empty_body_regex.is_match(text) {
            return None;
        }

        // WMS ServiceExceptionReport → no features.
        let exception_regex =
            regex::Regex::new(r"(?i)<ServiceExceptionReport[\s\S]*</ServiceExceptionReport>")
                .unwrap();
        if exception_regex.is_match(text) {
            return None;
        }

        // Extract <title> if present.
        let title_regex = regex::Regex::new(r"(?i)<title>([\s\S]*)</title>").unwrap();
        let name = title_regex
            .captures(text)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string());

        let mut feature_info = ImageryLayerFeatureInfo::new();
        feature_info.name = name;
        feature_info.description = Some(text.to_string());
        feature_info.data = Some(Value::String(text.to_string()));

        Some(vec![feature_info])
    }
}

impl Default for GetFeatureInfoFormat {
    fn default() -> Self {
        Self::new(FeatureInfoType::Json, None)
    }
}
