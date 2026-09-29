//! 面向 agent 的**意图协议**（计划 P0）。
//!
//! 一个外部 agent（LLM 工具调用者、脚本、服务）从不触及渲染器、
//! 顶点或 GPU —— 它产生一个小巧、稳定、可 JSON 序列化的
//! [`AgentAction`]。[`compile`] 通过填入服务器拥有的、agent 必须
//! *不能* 猜测的事实，将一个动作编译为可逆的 [`PlotCommand`]：
//! 它从实时 [`Document`] 读到的 `before` 状态、新铸造的元素 id，
//! 以及解析后的目标图层。[`apply_action`] 随后作为单个 undo 步骤
//! 应用 + 记录该命令。
//!
//! 为何用一个外障而非直接暴露 [`PlotCommand`]：命令携带
//! `before` / `id` / `layer`，是一个内部的记录单元，会将实现
//! 细节冻结进公共契约。意图层使外部保持稳定而内部演进，且它是
//! 每一次写入在触及文档前被*校验*（未知 id、点太少、无活动图层）的地方。

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::geo::GeoPoint;
use crate::model::geometry::Geometry;
use crate::model::ids::{ElementId, LayerId};
use crate::model::{Document, HeightReference, Rgba, Style};
use crate::ops::transform;
use crate::ops::{commit_draft, DrawKind, HistoryStack, PlotCommand};

/// 一个最小的、渐进式的样式编辑 —— 每个字段都可选，只有存在的那些
/// 会被 [`StylePatch::apply_to`] 复制到元素当前的 [`Style`] 上。
/// 镜像 `Style` 中与 agent 相关的子集。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StylePatch {
    pub color: Option<Rgba>,
    pub opacity: Option<f32>,
    pub width_px: Option<f32>,
    /// 多边形填充色；`Some(_)` 设置它，`None` 保持不变。
    pub fill: Option<Rgba>,
    pub point_size_px: Option<f32>,
    pub z_order: Option<i32>,
    pub height_reference: Option<HeightReference>,
    pub depth_test: Option<bool>,
    pub show_in_flat: Option<bool>,
    pub show_in_globe: Option<bool>,
}

impl StylePatch {
    /// 将本补丁折叠到 `base` 上，返回新样式。
    pub fn apply_to(&self, base: &Style) -> Style {
        let mut s = base.clone();
        if let Some(v) = self.color {
            s.color = v;
        }
        if let Some(v) = self.opacity {
            s.opacity = v;
        }
        if let Some(v) = self.width_px {
            s.width_px = v;
        }
        if let Some(v) = self.fill {
            s.fill = Some(v);
        }
        if let Some(v) = self.point_size_px {
            s.point_size_px = v;
        }
        if let Some(v) = self.z_order {
            s.z_order = v;
        }
        if let Some(v) = self.height_reference {
            s.height_reference = v;
        }
        if let Some(v) = self.depth_test {
            s.depth_test = v;
        }
        if let Some(v) = self.show_in_flat {
            s.show_in_flat = v;
        }
        if let Some(v) = self.show_in_globe {
            s.show_in_globe = v;
        }
        s
    }
}

/// 单个高层编辑意图。它有意**不**携带任何服务器拥有的事实
/// （无 `before`、无铸造的 id、无解析的图层）：[`compile`] 提供
/// 这些并校验请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentAction {
    /// 从一个绘制类型 + 其控制点创建一个图元，折叠经过交互绘制
    /// 工具所用的同一 [`commit_draft`] 规则。更丰富的几何用后续
    /// [`AgentAction::SetGeometry`] 放置。
    Create {
        kind: DrawKind,
        positions: Vec<GeoPoint>,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        style: Option<StylePatch>,
        #[serde(default)]
        attributes: Option<Map<String, Value>>,
        /// 目标图层；默认为文档的活动图层。
        #[serde(default)]
        layer: Option<LayerId>,
    },
    /// 按 id 删除一个元素（在命令中保留完整以便 undo 恢复它）。
    Delete { target: ElementId },
    /// 将元素的所有顶点平移 `(dlon, dlat)` 度。
    Move {
        target: ElementId,
        delta_lonlat: [f64; 2],
    },
    /// 替换一个元素的整个几何（任意 [`Geometry`] 变体）。
    SetGeometry {
        target: ElementId,
        geometry: Geometry,
    },
    /// 修补一个元素的样式（只有存在的字段会改变）。
    Style {
        target: ElementId,
        patch: StylePatch,
    },
    /// 合并自由形式的业务属性（敌我 / 番号 / 状态 …）；同名的现有键
    /// 被覆盖，其他保留。
    SetAttributes {
        target: ElementId,
        merge: Map<String, Value>,
    },
    /// 翻转手动可见性标志。
    SetVisible {
        target: ElementId,
        visible: bool,
    },
    /// 多个动作编译成一个 [`PlotCommand::Composite`] —— 单个
    /// undo 步骤。子动作必须引用已存在的元素（一个批次不会
    /// 解析指向同批次中早前子动作所创建元素的引用）；要那样做请用
    /// 顺序的 [`apply_action`] 调用。
    Batch { actions: Vec<AgentAction> },
}

/// 一个动作无法编译的原因。每个失败都在 [`compile`] 中、*在*
/// 文档被变更之前检测到，因此一个被拒绝的动作会使场景保持
/// 不变（除了一个隐式消耗的 id 计数器）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ActionError {
    #[error("no element with id {0:?}")]
    UnknownTarget(ElementId),
    #[error("{kind:?} needs at least {need} points, got {got}")]
    TooFewPoints {
        kind: DrawKind,
        need: usize,
        got: usize,
    },
    #[error("no active layer to add into — set one or create a layer first")]
    NoActiveLayer,
    #[error("geometry rejected by validation: {0}")]
    InvalidGeometry(String),
    #[error("value out of range for field `{field}`")]
    OutOfRange { field: &'static str },
}

/// 一个已应用动作做了什么 —— 足够让宿主调和它的视图。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Applied {
    /// 结果命令触及的每个叶子元素 id。
    pub touched_ids: Vec<ElementId>,
    /// 当一个动作创建了新元素时，那个新建元素的 id。
    pub new_id: Option<ElementId>,
}

/// 针对 `doc` 校验 `action` 并产出它映射到的可逆命令 —— 但**不**应用
/// 它。这会读取 `before` 状态、解析目标图层，并（对于创建）通过
/// [`Document::make_element`] 铸造一个新 id。
pub fn compile(doc: &mut Document, action: &AgentAction) -> Result<PlotCommand, ActionError> {
    match action {
        AgentAction::Create {
            kind,
            positions,
            name,
            style,
            attributes,
            layer,
        } => {
            let target = (*layer)
                .or_else(|| doc.active_layer())
                .ok_or(ActionError::NoActiveLayer)?;
            let geometry = commit_draft(*kind, positions).ok_or(ActionError::TooFewPoints {
                kind: *kind,
                need: kind.min_points(),
                got: positions.len(),
            })?;
            let mut ne = doc.make_element(name.clone().unwrap_or_default(), geometry);
            if let Some(patch) = style {
                ne.element.style = patch.apply_to(&ne.element.style);
            }
            if let Some(attrs) = attributes {
                for (k, v) in attrs {
                    ne.element.attributes.insert(k.clone(), v.clone());
                }
            }
            Ok(PlotCommand::AddElement {
                layer: target,
                element: Box::new(ne.element),
            })
        }
        AgentAction::Delete { target } => {
            let el = doc
                .element(*target)
                .ok_or(ActionError::UnknownTarget(*target))?
                .clone();
            let layer = doc
                .element_context(*target)
                .map(|(l, _)| l)
                .ok_or(ActionError::UnknownTarget(*target))?;
            Ok(PlotCommand::RemoveElement {
                element: Box::new(el),
                layer,
            })
        }
        AgentAction::Move {
            target,
            delta_lonlat,
        } => {
            let before = doc
                .element(*target)
                .ok_or(ActionError::UnknownTarget(*target))?
                .geometry
                .clone();
            let after = transform::translate(&before, delta_lonlat[0], delta_lonlat[1]);
            Ok(PlotCommand::UpdateGeometry {
                id: *target,
                before: Box::new(before),
                after: Box::new(after),
            })
        }
        AgentAction::SetGeometry { target, geometry } => {
            let before = doc
                .element(*target)
                .ok_or(ActionError::UnknownTarget(*target))?
                .geometry
                .clone();
            Ok(PlotCommand::UpdateGeometry {
                id: *target,
                before: Box::new(before),
                after: Box::new(geometry.clone()),
            })
        }
        AgentAction::Style { target, patch } => {
            let before = doc
                .element(*target)
                .ok_or(ActionError::UnknownTarget(*target))?
                .style
                .clone();
            let after = patch.apply_to(&before);
            Ok(PlotCommand::SetStyle {
                id: *target,
                before: Box::new(before),
                after: Box::new(after),
            })
        }
        AgentAction::SetAttributes { target, merge } => {
            let before = doc
                .element(*target)
                .ok_or(ActionError::UnknownTarget(*target))?
                .attributes
                .clone();
            let mut after = before.clone();
            for (k, v) in merge {
                after.insert(k.clone(), v.clone());
            }
            Ok(PlotCommand::SetAttributes {
                id: *target,
                before,
                after,
            })
        }
        AgentAction::SetVisible { target, visible } => {
            let before = doc
                .element(*target)
                .ok_or(ActionError::UnknownTarget(*target))?
                .flags
                .visible_manual;
            Ok(PlotCommand::SetVisibilityFlag {
                id: *target,
                before,
                after: *visible,
            })
        }
        AgentAction::Batch { actions } => {
            let mut steps = Vec::with_capacity(actions.len());
            for a in actions {
                steps.push(compile(doc, a)?);
            }
            Ok(PlotCommand::Composite { steps })
        }
    }
}

/// 在 `doc` 上编译、应用并记录 `action` 作为单个 undo 步骤。校验
/// 失败会在任何变更之前短路，且**不会**被记录。
pub fn apply_action(
    doc: &mut Document,
    history: &mut HistoryStack,
    action: &AgentAction,
) -> Result<Applied, ActionError> {
    let command = compile(doc, action)?;
    let new_id = match &command {
        PlotCommand::AddElement { element, .. } => Some(element.id),
        _ => None,
    };
    command.apply(doc);
    let touched_ids = command.targets();
    history.record(command);
    Ok(Applied {
        touched_ids,
        new_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ids::ElementId;

    fn pt(lon: f64, lat: f64) -> GeoPoint {
        GeoPoint::surface(lon, lat)
    }

    fn doc() -> Document {
        Document::with_default_layer()
    }

    #[test]
    fn create_adds_and_undo_removes() {
        let mut doc = doc();
        let mut h = HistoryStack::new();
        let applied = apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Create {
                kind: DrawKind::Point,
                positions: vec![pt(3.0, 4.0)],
                name: Some("watch".into()),
                style: None,
                attributes: None,
                layer: None,
            },
        )
        .unwrap();
        let id = applied.new_id.expect("created id reported");
        assert_eq!(doc.element_count(), 1);
        assert_eq!(doc.element(id).unwrap().name, "watch");
        // 单次 undo 会再次移除它。
        h.undo(&mut doc);
        assert_eq!(doc.element_count(), 0);
    }

    #[test]
    fn create_applies_style_and_attributes() {
        let mut doc = doc();
        let mut h = HistoryStack::new();
        let mut attrs = Map::new();
        attrs.insert("side".into(), Value::String("hostile".into()));
        let applied = apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Create {
                kind: DrawKind::Point,
                positions: vec![pt(0.0, 0.0)],
                name: None,
                style: Some(StylePatch {
                    color: Some([1.0, 0.0, 0.0, 1.0]),
                    ..Default::default()
                }),
                attributes: Some(attrs),
                layer: None,
            },
        )
        .unwrap();
        let e = doc.element(applied.new_id.unwrap()).unwrap();
        assert_eq!(e.style.color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(e.attributes["side"], Value::String("hostile".into()));
        assert_eq!(e.name, "");
    }

    #[test]
    fn move_then_undo_restores_geometry() {
        let mut doc = doc();
        let mut h = HistoryStack::new();
        let id = apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Create {
                kind: DrawKind::Point,
                positions: vec![pt(1.0, 1.0)],
                name: None,
                style: None,
                attributes: None,
                layer: None,
            },
        )
        .unwrap()
        .new_id
        .unwrap();
        apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Move {
                target: id,
                delta_lonlat: [2.0, -3.0],
            },
        )
        .unwrap();
        match &doc.element(id).unwrap().geometry {
            Geometry::Point(p) => assert_eq!((p.lon_deg, p.lat_deg), (3.0, -2.0)),
            g => panic!("{g:?}"),
        }
        h.undo(&mut doc);
        match &doc.element(id).unwrap().geometry {
            Geometry::Point(p) => assert_eq!((p.lon_deg, p.lat_deg), (1.0, 1.0)),
            g => panic!("{g:?}"),
        }
    }

    #[test]
    fn set_geometry_and_style_are_undoable() {
        let mut doc = doc();
        let mut h = HistoryStack::new();
        let id = apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Create {
                kind: DrawKind::Point,
                positions: vec![pt(0.0, 0.0)],
                name: None,
                style: None,
                attributes: None,
                layer: None,
            },
        )
        .unwrap()
        .new_id
        .unwrap();
        // 将几何设为一条折线。
        apply_action(
            &mut doc,
            &mut h,
            &AgentAction::SetGeometry {
                target: id,
                geometry: Geometry::Polyline(crate::model::geometry::Polyline {
                    positions: vec![pt(0.0, 0.0), pt(1.0, 1.0)],
                }),
            },
        )
        .unwrap();
        assert_eq!(doc.element(id).unwrap().geometry.kind(), crate::model::GeometryKind::Line);
        h.undo(&mut doc);
        assert_eq!(doc.element(id).unwrap().geometry.kind(), crate::model::GeometryKind::Point);
        // 样式补丁颜色，然后 undo。
        apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Style {
                target: id,
                patch: StylePatch {
                    width_px: Some(9.0),
                    ..Default::default()
                },
            },
        )
        .unwrap();
        assert_eq!(doc.element(id).unwrap().style.width_px, 9.0);
        h.undo(&mut doc);
        assert_ne!(doc.element(id).unwrap().style.width_px, 9.0);
    }

    #[test]
    fn set_attributes_merges_and_undo_restores() {
        let mut doc = doc();
        let mut h = HistoryStack::new();
        let id = apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Create {
                kind: DrawKind::Point,
                positions: vec![pt(0.0, 0.0)],
                name: None,
                style: None,
                attributes: Some({
                    let mut m = Map::new();
                    m.insert("a".into(), Value::from(1));
                    m
                }),
                layer: None,
            },
        )
        .unwrap()
        .new_id
        .unwrap();
        apply_action(
            &mut doc,
            &mut h,
            &AgentAction::SetAttributes {
                target: id,
                merge: {
                    let mut m = Map::new();
                    m.insert("b".into(), Value::from(2));
                    m
                },
            },
        )
        .unwrap();
        let e = doc.element(id).unwrap();
        assert_eq!(e.attributes["a"], Value::from(1));
        assert_eq!(e.attributes["b"], Value::from(2));
        h.undo(&mut doc);
        // Undo 丢弃合并的键但保留原始的。
        let e = doc.element(id).unwrap();
        assert!(e.attributes.contains_key("a"));
        assert!(!e.attributes.contains_key("b"));
    }

    #[test]
    fn set_visible_roundtrips() {
        let mut doc = doc();
        let mut h = HistoryStack::new();
        let id = apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Create {
                kind: DrawKind::Point,
                positions: vec![pt(0.0, 0.0)],
                name: None,
                style: None,
                attributes: None,
                layer: None,
            },
        )
        .unwrap()
        .new_id
        .unwrap();
        apply_action(&mut doc, &mut h, &AgentAction::SetVisible { target: id, visible: false })
            .unwrap();
        assert!(!doc.element(id).unwrap().flags.visible_manual);
        h.undo(&mut doc);
        assert!(doc.element(id).unwrap().flags.visible_manual);
    }

    #[test]
    fn delete_then_undo_restores_the_element() {
        let mut doc = doc();
        let mut h = HistoryStack::new();
        let id = apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Create {
                kind: DrawKind::Point,
                positions: vec![pt(5.0, 6.0)],
                name: None,
                style: None,
                attributes: None,
                layer: None,
            },
        )
        .unwrap()
        .new_id
        .unwrap();
        apply_action(&mut doc, &mut h, &AgentAction::Delete { target: id }).unwrap();
        assert!(doc.element(id).is_none());
        h.undo(&mut doc);
        let back = doc.element(id).expect("restored by undo");
        match &back.geometry {
            Geometry::Point(p) => assert_eq!((p.lon_deg, p.lat_deg), (5.0, 6.0)),
            g => panic!("{g:?}"),
        }
    }

    #[test]
    fn batch_is_a_single_undo_step() {
        let mut doc = doc();
        let mut h = HistoryStack::new();
        let mk = |lon: f64| AgentAction::Create {
            kind: DrawKind::Point,
            positions: vec![pt(lon, 0.0)],
            name: None,
            style: None,
            attributes: None,
            layer: None,
        };
        apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Batch {
                actions: vec![mk(1.0), mk(2.0), mk(3.0)],
            },
        )
        .unwrap();
        assert_eq!(doc.element_count(), 3);
        assert_eq!(h.undo_len(), 1, "batch is one undo step");
        h.undo(&mut doc);
        assert_eq!(doc.element_count(), 0);
    }

    #[test]
    fn unknown_target_errors_without_mutation() {
        let mut doc = doc();
        let mut h = HistoryStack::new();
        let err = apply_action(&mut doc, &mut h, &AgentAction::Delete { target: ElementId(999) })
            .unwrap_err();
        assert_eq!(err, ActionError::UnknownTarget(ElementId(999)));
        assert_eq!(doc.element_count(), 0);
        assert!(!h.can_undo(), "a failed action is not recorded");
    }

    #[test]
    fn too_few_points_errors() {
        let mut doc = doc();
        let mut h = HistoryStack::new();
        let err = apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Create {
                kind: DrawKind::Polygon,
                positions: vec![pt(0.0, 0.0), pt(1.0, 1.0)],
                name: None,
                style: None,
                attributes: None,
                layer: None,
            },
        )
        .unwrap_err();
        assert_eq!(
            err,
            ActionError::TooFewPoints {
                kind: DrawKind::Polygon,
                need: 3,
                got: 2
            }
        );
        assert_eq!(doc.element_count(), 0);
        assert!(!h.can_undo());
    }

    #[test]
    fn no_active_layer_errors() {
        // 一个完全没有图层的文档。
        let mut doc = Document::default();
        let mut h = HistoryStack::new();
        let err = apply_action(
            &mut doc,
            &mut h,
            &AgentAction::Create {
                kind: DrawKind::Point,
                positions: vec![pt(0.0, 0.0)],
                name: None,
                style: None,
                attributes: None,
                layer: None,
            },
        )
        .unwrap_err();
        assert_eq!(err, ActionError::NoActiveLayer);
    }

    #[test]
    fn actions_roundtrip_through_json() {
        let a = AgentAction::Create {
            kind: DrawKind::Polyline,
            positions: vec![pt(0.0, 0.0), pt(1.0, 2.0)],
            name: Some("route".into()),
            style: Some(StylePatch {
                color: Some([0.0, 1.0, 0.0, 1.0]),
                ..Default::default()
            }),
            attributes: None,
            layer: None,
        };
        let s = serde_json::to_string(&a).unwrap();
        let back: AgentAction = serde_json::from_str(&s).unwrap();
        assert_eq!(a, back);
    }
}
