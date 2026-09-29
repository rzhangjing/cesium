//! 面向 agent 的**契约导出**（计划 P1）。
//!
//! 外部宿主在驱动标绘内核前需要做的两件事：
//!
//!  1. 一份 [`AgentAction`] 线格式的机器可读描述，以纯 JSON Schema [`Value`]
//!     的形式交付，使工具调用者可以将其直接喂入 function-calling / 结构化
//!     输出，而无需任何代码生成依赖；
//!  2. 一个整文档读 / 写门面（`export_document_json` /
//!     [`import_document_json`]），复用无损 GeoJSON 路径，从而 agent 的批量
//!     导入 / 导出入口与系统其余部分已经信任的是同一个。
//!
//! 该 schema 是手写的 —— 有意**不**引入 `schemars`（或任何新的）依赖。
//! [`EXAMPLE_ACTION`] 是一个具体负载，测试会将其解析回一个 [`AgentAction`]，
//! 从而把 schema 钉死在真实类型上，使它们不会悄然偏离。

use serde_json::{json, Value};

use crate::io::{from_geojson, to_geojson, PlotIoError};
use crate::model::Document;

/// 一份稳定的、手写的 JSON Schema（draft 2020-12 子集），描述
/// [`AgentAction`] 信封。外部标签的 serde 枚举被建模为对单键对象的
/// `oneOf`，这正是它们序列化的方式。
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

/// 一份规范的 [`AgentAction`] 负载。既作文档，也作回归锚点：测试把它
/// 解析回真实类型，因此手写的 [`action_json_schema`] 不会悄然偏离该枚举。
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

/// 通过无损 GeoJSON 门面序列化整个文档。
///
/// 内存中文档的序列化是不会失败的（每个字段都是普通的 `Serialize` 数据，
/// 且编码器是全函数），所以这里直接返回一个 `String`，而不泄露读取器
/// 可能产生的 [`PlotIoError`]。
pub fn export_document_json(doc: &Document) -> String {
    to_geojson(doc).expect("document JSON serialisation is infallible")
}

/// 从 [`export_document_json`] 负载（或任何 GeoJSON `FeatureCollection`，
/// 尽最大努力导入）重建一个 [`Document`]。
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
        // `Value` 依构造即为有效；断言这些契约地标，使一个把它们删掉的
        // 粗心编辑会大声失败。
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

        // 经由文本 JSON 编码器往返而无错误。
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
        // 重新序列化再重新解析是稳定的（schema ↔ 类型不偏离）。
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
        // 无损的 `x-plot` 负载精确还原整个文档。
        assert_eq!(back, doc);
    }

    #[test]
    fn import_rejects_non_json() {
        assert!(import_document_json("not json at all").is_err());
    }
}
