//! 纯多维显隐求值器（计划 §10）。
//!
//! [`eval_visibility`] 是 `(Document, ViewContext, Filters)` 的一个全函数，
//! 返回哪些元素**可见**，以及其子集中哪些**可拾取**
//! （可见*且*在其图层 / 组锁下可选）。十个维度中的每一个
//! 都是一道独立的关卡，以 AND 折叠；时间维度在 M1 中
//! 是一个预留的空操作（返回通过），待 M9 接入时钟。
//!
//! 让它保持为一个自由函数 —— 而非系统、非资源驱动 —— 正是这一点
//! 使得整张显隐规则表无需引擎即可做单元测试（计划 §15）。

use std::collections::BTreeSet;

use serde_json::Value;

use crate::model::document::Document;
use crate::model::element::Element;
use crate::model::filters::Filters;
use crate::model::group::Group;
use crate::model::ids::ElementId;
use crate::model::layer::Layer;
use crate::model::view::{ViewContext, ViewMode};

/// 一次显隐遍历的结果。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VisibilityResult {
    /// 要绘制的元素，无特定顺序（桥接层会重新排序）。
    pub visible: BTreeSet<ElementId>,
    /// 可见**且**可拾取 —— 用于命中测试的候选集（计划 §7）。
    pub pickable: BTreeSet<ElementId>,
}

/// 针对十个显隐维度评估每个元素。
pub fn eval_visibility(doc: &Document, view: &ViewContext, filters: &Filters) -> VisibilityResult {
    let mut out = VisibilityResult::default();

    // §10.1 总开关：关闭时无任何元素可见（也不可拾取）。
    if !filters.overlay_enabled {
        return out;
    }

    for element in doc.elements() {
        let Some((layer_id, group_chain)) = doc.element_context(element.id) else {
            continue; // 损坏树中的孤立节点 —— 防御性地跳过
        };
        let Some(layer) = doc.layer(layer_id) else {
            continue;
        };

        if !is_visible(doc, element, layer, &group_chain, view, filters) {
            continue;
        }
        out.visible.insert(element.id);

        if is_pickable(element, layer, &group_chain, doc, filters) {
            out.pickable.insert(element.id);
        }
    }
    out
}

/// 每个*可见性*维度的 AND（§10.2–§10.10，总开关已由
/// 调用方检查）。
fn is_visible(
    doc: &Document,
    element: &Element,
    layer: &Layer,
    group_chain: &[crate::model::ids::GroupId],
    view: &ViewContext,
    filters: &Filters,
) -> bool {
    // §10.2 图层开/关。
    if !layer.visible {
        return false;
    }
    // §10.3 链上每个组都可见。
    if !group_chain.iter().all(|g| {
        doc.group(*g).map(|grp| grp.visible).unwrap_or(false)
    }) {
        return false;
    }
    // §10.4 元素自身的手动开关。
    if !element.flags.visible_manual {
        return false;
    }
    // §10.5 类型维度。
    if let Some(allowed) = &filters.enabled_types {
        if !allowed.contains(&element.geometry.kind()) {
            return false;
        }
    }
    // §10.6 视图模式维度。
    let style = &element.style;
    match view.mode {
        ViewMode::Globe if !style.show_in_globe => return false,
        ViewMode::Flat if !style.show_in_flat => return false,
        _ => {}
    }
    // §10.7 比例尺带。
    if !element
        .scale_visibility
        .allows(view.pixels_per_world, view.meters_per_pixel)
    {
        return false;
    }
    // §10.8 选中聚焦。
    if filters.only_selected && !filters.selected.contains(&element.id) {
        return false;
    }
    // §10.9 属性维度。
    if !attributes_pass(element, &filters.attribute_predicates) {
        return false;
    }
    // §10.10 时间窗口 —— 预留；在 M1 中总是通过。
    true
}

/// 可拾取性（计划 §7/§9）：元素 `selectable`、其图层 `selectable`，且
/// 链上没有*锁定*的组（锁定的组会使成员各自不可抓取）。
/// 选中聚焦过滤器也会收窄可拾取集。
fn is_pickable(
    element: &Element,
    layer: &Layer,
    group_chain: &[crate::model::ids::GroupId],
    doc: &Document,
    filters: &Filters,
) -> bool {
    if !element.flags.selectable || !layer.selectable {
        return false;
    }
    let any_locked = group_chain.iter().any(|g| {
        doc.group(*g)
            .map(|grp: &Group| grp.locked)
            .unwrap_or(false)
    });
    if any_locked {
        return false;
    }
    // 聚焦于选中项时，只有选中的 id 才可拾取。
    if filters.only_selected && !filters.selected.contains(&element.id) {
        return false;
    }
    true
}

fn attributes_pass(element: &Element, predicates: &[(String, Value)]) -> bool {
    predicates.iter().all(|(k, v)| match element.attributes.get(k) {
        Some(actual) => actual == v,
        None => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::GeoPoint;
    use crate::model::geometry::{Geometry, Polyline};
    use crate::model::ids::LayerId;
    use crate::model::Document;
    use std::collections::HashSet;

    fn doc_with_layers() -> (Document, LayerId, LayerId) {
        let mut doc = Document::default();
        let a = doc.new_layer("A");
        let b = doc.new_layer("B");
        (doc, a, b)
    }

    fn add_point(doc: &mut Document, layer: LayerId, lon: f64, lat: f64) -> ElementId {
        let ne = doc.make_element("p", Geometry::Point(GeoPoint::surface(lon, lat)));
        let id = ne.id;
        doc.add_element_to_layer(layer, ne);
        id
    }

    fn view(mode: ViewMode, px: f64, mpp: f64) -> ViewContext {
        ViewContext {
            mode,
            pixels_per_world: px,
            meters_per_pixel: mpp,
            ..Default::default()
        }
    }

    #[test]
    fn master_switch_gates_everything() {
        let (mut doc, a, _b) = doc_with_layers();
        let e = add_point(&mut doc, a, 0.0, 0.0);
        let off = Filters::default(); // overlay_enabled 为 false
        let r = eval_visibility(&doc, &view(ViewMode::Globe, 0.0, 0.0), &off);
        assert!(r.visible.is_empty() && r.pickable.is_empty());

        let on = Filters::enabled();
        let r = eval_visibility(&doc, &view(ViewMode::Globe, 0.0, 0.0), &on);
        assert!(r.visible.contains(&e));
    }

    #[test]
    fn layer_visibility_hides_members() {
        let (mut doc, a, b) = doc_with_layers();
        let ea = add_point(&mut doc, a, 0.0, 0.0);
        let eb = add_point(&mut doc, b, 1.0, 1.0);
        doc.layer_mut(a).unwrap().visible = false;
        let r = eval_visibility(&doc, &view(ViewMode::Globe, 0.0, 0.0), &Filters::enabled());
        assert!(!r.visible.contains(&ea), "hidden layer hides its element");
        assert!(r.visible.contains(&eb));
    }

    #[test]
    fn manual_flag_and_group_chain_inherit() {
        let (mut doc, a, _b) = doc_with_layers();
        let g = doc.new_group_in_layer(a, "g");
        let ne = doc.make_element("e", Geometry::Point(GeoPoint::surface(0.0, 0.0)));
        let e = ne.id;
        doc.add_element_to_group(g, ne);
        let f = Filters::enabled();
        let v = view(ViewMode::Globe, 0.0, 0.0);

        assert!(eval_visibility(&doc, &v, &f).visible.contains(&e));
        // 隐藏该组 → 元素消失。
        doc.group_mut(g).unwrap().visible = false;
        assert!(!eval_visibility(&doc, &v, &f).visible.contains(&e));
        doc.group_mut(g).unwrap().visible = true;
        // 元素自身手动关闭。
        doc.element_mut(e).unwrap().flags.visible_manual = false;
        assert!(!eval_visibility(&doc, &v, &f).visible.contains(&e));
    }

    #[test]
    fn type_dimension_filters_kinds() {
        let (mut doc, a, _b) = doc_with_layers();
        let pt = add_point(&mut doc, a, 0.0, 0.0);
        let line = {
            let ne = doc.make_element(
                "line",
                Geometry::Polyline(Polyline {
                    positions: vec![GeoPoint::surface(0.0, 0.0), GeoPoint::surface(1.0, 1.0)],
                }),
            );
            let id = ne.id;
            doc.add_element_to_layer(a, ne);
            id
        };
        let mut only_points = HashSet::new();
        only_points.insert(crate::model::geometry::GeometryKind::Point);
        let f = Filters {
            enabled_types: Some(only_points),
            ..Filters::enabled()
        };
        let r = eval_visibility(&doc, &view(ViewMode::Globe, 0.0, 0.0), &f);
        assert!(r.visible.contains(&pt));
        assert!(!r.visible.contains(&line));
    }

    #[test]
    fn view_mode_dimension() {
        let (mut doc, a, _b) = doc_with_layers();
        let e = add_point(&mut doc, a, 0.0, 0.0);
        doc.element_mut(e).unwrap().style.show_in_globe = false; // 仅平面
        // 地球视图隐藏它，平面视图显示它。
        let rg = eval_visibility(&doc, &view(ViewMode::Globe, 0.0, 0.0), &Filters::enabled());
        assert!(!rg.visible.contains(&e));
        let rf = eval_visibility(&doc, &view(ViewMode::Flat, 0.0, 0.0), &Filters::enabled());
        assert!(rf.visible.contains(&e));
    }

    #[test]
    fn scale_band_dimension() {
        let (mut doc, a, _b) = doc_with_layers();
        let e = add_point(&mut doc, a, 0.0, 0.0);
        doc.element_mut(e).unwrap().scale_visibility.min_pixels_per_world = Some(100.0);
        let f = Filters::enabled();
        // 缩小 (px=50) → 隐藏；放大 (px=200) → 显示。
        assert!(!eval_visibility(&doc, &view(ViewMode::Flat, 50.0, 0.0), &f).visible.contains(&e));
        assert!(eval_visibility(&doc, &view(ViewMode::Flat, 200.0, 0.0), &f).visible.contains(&e));
    }

    #[test]
    fn selection_focus_dimension_narrows_both_sets() {
        let (mut doc, a, _b) = doc_with_layers();
        let e1 = add_point(&mut doc, a, 0.0, 0.0);
        let e2 = add_point(&mut doc, a, 1.0, 1.0);
        let f = Filters {
            only_selected: true,
            selected: [e1].into_iter().collect(),
            ..Filters::enabled()
        };
        let r = eval_visibility(&doc, &view(ViewMode::Globe, 0.0, 0.0), &f);
        assert_eq!(r.visible, [e1].into_iter().collect::<BTreeSet<_>>());
        assert!(r.pickable.contains(&e1));
        assert!(!r.pickable.contains(&e2));
    }

    #[test]
    fn attribute_dimension_and_semantics() {
        let (mut doc, a, _b) = doc_with_layers();
        let friend = add_point(&mut doc, a, 0.0, 0.0);
        doc.element_mut(friend)
            .unwrap()
            .attributes
            .insert("side".into(), Value::String("friend".into()));
        let hostile = add_point(&mut doc, a, 1.0, 1.0);
        doc.element_mut(hostile)
            .unwrap()
            .attributes
            .insert("side".into(), Value::String("hostile".into()));

        let f = Filters {
            attribute_predicates: vec![("side".into(), Value::String("friend".into()))],
            ..Filters::enabled()
        };
        let r = eval_visibility(&doc, &view(ViewMode::Globe, 0.0, 0.0), &f);
        assert!(r.visible.contains(&friend));
        assert!(!r.visible.contains(&hostile));
    }

    #[test]
    fn locked_group_and_unselectable_layer_block_pickable_only() {
        let (mut doc, a, _b) = doc_with_layers();
        let g = doc.new_group_in_layer(a, "g");
        let ne = doc.make_element("e", Geometry::Point(GeoPoint::surface(0.0, 0.0)));
        let e = ne.id;
        doc.add_element_to_group(g, ne);
        doc.group_mut(g).unwrap().locked = true;
        let r = eval_visibility(&doc, &view(ViewMode::Globe, 0.0, 0.0), &Filters::enabled());
        assert!(r.visible.contains(&e), "locked but still visible");
        assert!(!r.pickable.contains(&e), "locked group is not pickable");
    }

    #[test]
    fn unselectable_layer_blocks_pickable() {
        let (mut doc, a, _b) = doc_with_layers();
        let e = add_point(&mut doc, a, 0.0, 0.0);
        doc.layer_mut(a).unwrap().selectable = false;
        let r = eval_visibility(&doc, &view(ViewMode::Globe, 0.0, 0.0), &Filters::enabled());
        assert!(r.visible.contains(&e));
        assert!(!r.pickable.contains(&e));
    }
}
