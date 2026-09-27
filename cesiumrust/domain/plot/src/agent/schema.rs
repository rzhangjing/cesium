//! Agent-facing **contract export** (plan P1).
//!
//! Two jobs an external host needs before it can drive the plot core:
//!
//!  1. a machine-readable description of the [`AgentAction`] wire format, handed
//!     out as a plain JSON Schema [`Value`] so a tool-caller can feed it straight
//!     into function-calling / structured-output without any codegen dependency;
//!  2. a whole-document read / write façade (`export_document_json` /
//!     [`import_document_json`]) that reuses the lossless GeoJSON path so the
//!     agent's batch import / export entry point is the same one the rest of the
//!     system already trusts.
//!
//! The schema is written by hand — there is deliberately **no** `schemars` (or
//! any new) dependency. [`EXAMPLE_ACTION`] is a concrete payload that the tests
//! parse back into an [`AgentAction`], pinning the schema to the real type so
//! they cannot silently drift apart.

use serde_json::{json, Value};

use crate::io::{from_geojson, to_geojson, PlotIoError};
use crate::model::Document;

/// A stable, hand-written JSON Schema (draft 2020-12 subset) describing the
/// [`AgentAction`] envelope. Externally-tagged serde enums are modelled as
/// `oneOf` over single-key objects, which is exactly how they serialise.
pub fn action_json_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "AgentAction",
        "description": "A single high-level plotting intent. The server fills in \
            server-owned facts (before-state, minted ids, resolved layer) and \
            validates the request before anything mutates the document.",
        "$ref": "#/$defs/AgentAction",
        "$defs": {
            "GeoPoint": {
                "type": "object",
                "description": "Longitude / latitude degrees + height metres.",
                "properties": {
                    "lon_deg": { "type": "number" },
                    "lat_deg": { "type": "number" },
                    "height_m": { "type": "number" }
                },
                "required": ["lon_deg", "lat_deg", "height_m"],
                "additionalProperties": false
            },
            "Rgba": {
                "type": "array",
                "description": "RGBA colour, components in 0..=1.",
                "items": { "type": "number" },
                "minItems": 4,
                "maxItems": 4
            },
            "DrawKind": {
                "type": "string",
                "description": "Primitive a Create is drawn as; folded through \
                    the same draft-commit rules as the interactive tool.",
                "enum": ["Point", "Polyline", "Polygon", "Rectangle", "Circle"]
            },
            "HeightReference": {
                "type": "string",
                "enum": ["None", "ClampToGround", "RelativeToGround", "Absolute"]
            },
            "GeometryKind": {
                "type": "string",
                "enum": [
                    "Point", "Icon", "Label", "Line", "Polygon", "Rectangle",
                    "Circle", "Ellipse", "Arc", "Path", "Composite"
                ]
            },
            "StylePatch": {
                "type": "object",
                "description": "Incremental style edit; only present fields apply.",
                "properties": {
                    "color": { "$ref": "#/$defs/Rgba" },
                    "opacity": { "type": "number" },
                    "width_px": { "type": "number" },
                    "fill": { "$ref": "#/$defs/Rgba" },
                    "point_size_px": { "type": "number" },
                    "z_order": { "type": "integer" },
                    "height_reference": { "$ref": "#/$defs/HeightReference" },
                    "depth_test": { "type": "boolean" },
                    "show_in_flat": { "type": "boolean" },
                    "show_in_globe": { "type": "boolean" }
                },
                "additionalProperties": false
            },
            "Geometry": {
                "description": "Full geometry union (externally-tagged). Basic \
                    shapes are the common case; richer variants are the reserved \
                    symbol-composition slot.",
                "oneOf": [
                    { "type": "object", "properties": { "Point": { "$ref": "#/$defs/GeoPoint" } }, "required": ["Point"], "additionalProperties": false },
                    { "type": "object", "properties": { "Polyline": { "type": "object", "properties": { "positions": { "type": "array", "items": { "$ref": "#/$defs/GeoPoint" } } }, "required": ["positions"], "additionalProperties": false } }, "required": ["Polyline"], "additionalProperties": false },
                    { "type": "object", "properties": { "Polygon": { "type": "object", "properties": { "outer": { "type": "array", "items": { "$ref": "#/$defs/GeoPoint" } }, "holes": { "type": "array", "items": { "type": "array", "items": { "$ref": "#/$defs/GeoPoint" } } } }, "required": ["outer", "holes"], "additionalProperties": false } }, "required": ["Polygon"], "additionalProperties": false },
                    { "type": "object", "properties": { "Rectangle": { "type": "object", "properties": { "west": { "type": "number" }, "south": { "type": "number" }, "east": { "type": "number" }, "north": { "type": "number" } }, "required": ["west", "south", "east", "north"], "additionalProperties": false } }, "required": ["Rectangle"], "additionalProperties": false },
                    { "type": "object", "properties": { "Circle": true } },
                    { "type": "object", "properties": { "Icon": true } },
                    { "type": "object", "properties": { "Label": true } },
                    { "type": "object", "properties": { "Ellipse": true } },
                    { "type": "object", "properties": { "Arc": true } },
                    { "type": "object", "properties": { "Path": true } },
                    { "type": "object", "properties": { "Composite": true } }
                ]
            },
            "ElementId": { "type": "integer", "minimum": 0 },
            "LayerId": { "type": "integer", "minimum": 0 },
            "Attributes": {
                "type": "object",
                "description": "Free-form business attributes (敌我 / 番号 / 状态 …).",
                "additionalProperties": true
            },
            "AgentAction": {
                "oneOf": [
                    {
                        "type": "object",
                        "properties": {
                            "Create": {
                                "type": "object",
                                "properties": {
                                    "kind": { "$ref": "#/$defs/DrawKind" },
                                    "positions": { "type": "array", "items": { "$ref": "#/$defs/GeoPoint" } },
                                    "name": { "type": ["string", "null"] },
                                    "style": {
                                        "anyOf": [
                                            { "$ref": "#/$defs/StylePatch" },
                                            { "type": "null" }
                                        ]
                                    },
                                    "attributes": {
                                        "anyOf": [
                                            { "$ref": "#/$defs/Attributes" },
                                            { "type": "null" }
                                        ]
                                    },
                                    "layer": {
                                        "anyOf": [
                                            { "$ref": "#/$defs/LayerId" },
                                            { "type": "null" }
                                        ]
                                    }
                                },
                                "required": ["kind", "positions"],
                                "additionalProperties": false
                            }
                        },
                        "required": ["Create"],
                        "additionalProperties": false
                    },
                    {
                        "type": "object",
                        "properties": {
                            "Delete": {
                                "type": "object",
                                "properties": { "target": { "$ref": "#/$defs/ElementId" } },
                                "required": ["target"],
                                "additionalProperties": false
                            }
                        },
                        "required": ["Delete"],
                        "additionalProperties": false
                    },
                    {
                        "type": "object",
                        "properties": {
                            "Move": {
                                "type": "object",
                                "properties": {
                                    "target": { "$ref": "#/$defs/ElementId" },
                                    "delta_lonlat": {
                                        "type": "array",
                                        "items": { "type": "number" },
                                        "minItems": 2,
                                        "maxItems": 2
                                    }
                                },
                                "required": ["target", "delta_lonlat"],
                                "additionalProperties": false
                            }
                        },
                        "required": ["Move"],
                        "additionalProperties": false
                    },
                    {
                        "type": "object",
                        "properties": {
                            "SetGeometry": {
                                "type": "object",
                                "properties": {
                                    "target": { "$ref": "#/$defs/ElementId" },
                                    "geometry": { "$ref": "#/$defs/Geometry" }
                                },
                                "required": ["target", "geometry"],
                                "additionalProperties": false
                            }
                        },
                        "required": ["SetGeometry"],
                        "additionalProperties": false
                    },
                    {
                        "type": "object",
                        "properties": {
                            "Style": {
                                "type": "object",
                                "properties": {
                                    "target": { "$ref": "#/$defs/ElementId" },
                                    "patch": { "$ref": "#/$defs/StylePatch" }
                                },
                                "required": ["target", "patch"],
                                "additionalProperties": false
                            }
                        },
                        "required": ["Style"],
                        "additionalProperties": false
                    },
                    {
                        "type": "object",
                        "properties": {
                            "SetAttributes": {
                                "type": "object",
                                "properties": {
                                    "target": { "$ref": "#/$defs/ElementId" },
                                    "merge": { "$ref": "#/$defs/Attributes" }
                                },
                                "required": ["target", "merge"],
                                "additionalProperties": false
                            }
                        },
                        "required": ["SetAttributes"],
                        "additionalProperties": false
                    },
                    {
                        "type": "object",
                        "properties": {
                            "SetVisible": {
                                "type": "object",
                                "properties": {
                                    "target": { "$ref": "#/$defs/ElementId" },
                                    "visible": { "type": "boolean" }
                                },
                                "required": ["target", "visible"],
                                "additionalProperties": false
                            }
                        },
                        "required": ["SetVisible"],
                        "additionalProperties": false
                    },
                    {
                        "type": "object",
                        "properties": {
                            "Batch": {
                                "type": "object",
                                "properties": {
                                    "actions": {
                                        "type": "array",
                                        "items": { "$ref": "#/$defs/AgentAction" }
                                    }
                                },
                                "required": ["actions"],
                                "additionalProperties": false
                            }
                        },
                        "required": ["Batch"],
                        "additionalProperties": false
                    }
                ]
            }
        }
    })
}

/// A canonical [`AgentAction`] payload. Doubles as documentation and as the
/// regression anchor: the test parses this back into the real type, so the
/// hand-written [`action_json_schema`] cannot silently drift from the enum.
pub const EXAMPLE_ACTION: &str = r#"{
    "Create": {
        "kind": "Polyline",
        "positions": [
            { "lon_deg": 116.39, "lat_deg": 39.91, "height_m": 0.0 },
            { "lon_deg": 116.45, "lat_deg": 39.94, "height_m": 0.0 }
        ],
        "name": "route-a",
        "style": { "color": [1.0, 0.2, 0.2, 1.0], "width_px": 3.0 },
        "attributes": { "side": "friendly", "priority": 1 }
    }
}"#;

/// Serialise the whole document through the lossless GeoJSON façade.
///
/// In-memory document serialisation is infallible (every field is plain
/// `Serialize` data and the encoder is total), so this returns a `String`
/// directly rather than leaking the [`PlotIoError`] the reader can produce.
pub fn export_document_json(doc: &Document) -> String {
    to_geojson(doc).expect("document JSON serialisation is infallible")
}

/// Rebuild a [`Document`] from an [`export_document_json`] payload (or any
/// GeoJSON `FeatureCollection`, imported best-effort).
pub fn import_document_json(text: &str) -> Result<Document, PlotIoError> {
    from_geojson(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::action::AgentAction;
    use crate::geo::GeoPoint;
    use crate::model::{Document, HeightReference};
    use serde_json::Value;

    #[test]
    fn schema_is_valid_json_with_expected_shape() {
        let schema = action_json_schema();
        // A `Value` is by construction valid; assert the contract landmarks so a
        // careless edit that drops them fails loudly.
        assert!(schema["$schema"].as_str().unwrap().contains("2020-12"));
        assert_eq!(schema["title"], Value::String("AgentAction".into()));
        let defs = schema["$defs"].as_object().unwrap();
        for key in [
            "GeoPoint",
            "Geometry",
            "StylePatch",
            "DrawKind",
            "AgentAction",
        ] {
            assert!(defs.contains_key(key), "missing $defs.{key}");
        }
        let variants = defs["AgentAction"]["oneOf"].as_array().unwrap();
        assert_eq!(variants.len(), 8, "eight action variants");

        // Round-trips through the text JSON encoder without error.
        let text = serde_json::to_string(&schema).unwrap();
        let reparsed: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(schema, reparsed);
    }

    #[test]
    fn example_action_parses_into_the_real_type() {
        let action: AgentAction = serde_json::from_str(EXAMPLE_ACTION).unwrap();
        match &action {
            AgentAction::Create {
                positions,
                name,
                style,
                attributes,
                ..
            } => {
                assert_eq!(positions.len(), 2);
                assert_eq!(name.as_deref(), Some("route-a"));
                let patch = style.as_ref().unwrap();
                assert_eq!(patch.width_px, Some(3.0));
                assert_eq!(patch.color, Some([1.0, 0.2, 0.2, 1.0]));
                assert_eq!(
                    attributes.as_ref().unwrap()["side"],
                    Value::String("friendly".into())
                );
            }
            other => panic!("example did not parse as Create: {other:?}"),
        }
        // Re-serialising then re-parsing is stable (schema ↔ type do not drift).
        let again: AgentAction =
            serde_json::from_str(&serde_json::to_string(&action).unwrap()).unwrap();
        assert_eq!(action, again);
    }

    #[test]
    fn document_roundtrips_losslessly_through_the_facade() {
        use crate::agent::action::{apply_action, StylePatch};
        use crate::ops::{DrawKind, HistoryStack};
        let mut doc = Document::with_default_layer();
        let mut history = HistoryStack::new();
        apply_action(
            &mut doc,
            &mut history,
            &AgentAction::Create {
                kind: DrawKind::Rectangle,
                positions: vec![
                    GeoPoint::surface(0.0, 0.0),
                    GeoPoint::surface(2.0, 2.0),
                ],
                name: Some("zone".into()),
                style: Some(StylePatch {
                    height_reference: Some(HeightReference::ClampToGround),
                    ..Default::default()
                }),
                attributes: None,
                layer: None,
            },
        )
        .unwrap();

        let text = export_document_json(&doc);
        let back = import_document_json(&text).unwrap();
        assert_eq!(back.element_count(), 1);
        // The lossless `x-plot` payload restores the whole document exactly.
        assert_eq!(back, doc);
    }

    #[test]
    fn import_rejects_non_json() {
        assert!(import_document_json("not json at all").is_err());
    }
}
