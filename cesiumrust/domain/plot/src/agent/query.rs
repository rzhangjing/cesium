//! 面向 agent 的**只读态势查询**（计划 P2）。
//!
//! 如果说 [`action`](super::action) 是写路径，这里就是 agent 需要的
//! “理解态势”那一半：一个纯 [`query`]，按包围盒 / 属性 / 几何类型 /
//! 图层 / 名称过滤文档并返回可序列化的 [`ElementSummary`] 行，
//! 加上 [`measure_length`] / [`measure_area`] 读数。无变更、无引擎类型 ——
//! 可确定性地无头单元测试。
//!
//! `bbox` 过滤器针对每个元素的*保守* 地理包围盒测试（见
//! `Geometry::bounds`），它已涵盖大圆外鼓与参数化图形的范围，
//! 因此一个宽相位重叠绝不会丢弃一个实际与窗口相交的元素。

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::geo::GeoBounds;
use crate::model::geometry::GeometryKind;
use crate::model::ids::{ElementId, LayerId};
use crate::model::{Document, ViewContext, ViewMode};
use crate::ops::measure::{measure_area_m2, measure_length_m};

/// 一个合取过滤器：一个元素仅当满足*每一个* 存在的条件时才匹配。
/// 一个全 `None` / 空的过滤器返回所有元素。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct QueryFilter {
    /// 保留其包围盒与此窗口重叠（含边界）的元素。
    pub bbox: Option<GeoBounds>,
    /// 保留其属性包含所有这些精确键/值的元素。
    pub attributes: Vec<(String, Value)>,
    /// 只保留这一粗粒度几何类别。
    pub kind: Option<GeometryKind>,
    /// 只保留属于此图层的元素。
    pub layer: Option<LayerId>,
    /// 保留名称包含此子串（区分大小写）的元素。
    pub name_contains: Option<String>,
    /// 限定为可拾取 / 可选的元素。
    pub selectable_only: bool,
}

/// 一个匹配元素的可序列化摘要行。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementSummary {
    pub id: ElementId,
    pub name: String,
    pub kind: GeometryKind,
    /// 元素最终归属的图层（其祖先链的图层根）。
    pub layer: Option<LayerId>,
    pub bounds: GeoBounds,
    pub attributes: Map<String, Value>,
    /// 有效可见性：手动标志与图层开关取 AND，且当提供了
    /// `view` 时，再加上比例尺带与每模式显示标志。
    pub visible: bool,
}

/// 两个轴对齐经纬框的宽相位重叠（含边界）。一个空框
/// 与任何东西都不重叠。
fn overlaps(a: &GeoBounds, b: &GeoBounds) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    a.west_deg <= b.east_deg
        && a.east_deg >= b.west_deg
        && a.south_deg <= b.north_deg
        && a.north_deg >= b.south_deg
}

/// 每个请求的 `(key, value)` 对都存在于 `attrs` 中且值相等。
fn attr_matches(attrs: &Map<String, Value>, want: &[(String, Value)]) -> bool {
    want.iter().all(|(k, v)| attrs.get(k) == Some(v))
}

/// 以稳定绘制顺序为每个匹配 `filter` 的元素返回一个摘要。
/// `view` 可选：当提供时，计算出的 `visible` 标志还会兼顾元素的
/// 比例尺带及其每模式显示标志。
pub fn query(
    doc: &Document,
    filter: &QueryFilter,
    view: Option<&ViewContext>,
) -> Vec<ElementSummary> {
    let mut out = Vec::new();
    for id in doc.flatten_draw_order() {
        let Some(el) = doc.element(id) else {
            continue;
        };
        if filter.selectable_only && !el.flags.selectable {
            continue;
        }
        if let Some(kind) = filter.kind {
            if el.geometry.kind() != kind {
                continue;
            }
        }
        if let Some(needle) = &filter.name_contains {
            if !el.name.contains(needle.as_str()) {
                continue;
            }
        }
        let layer = doc.element_context(id).map(|(l, _)| l);
        if let Some(ly) = filter.layer {
            if layer != Some(ly) {
                continue;
            }
        }
        if let Some(window) = &filter.bbox {
            if !overlaps(window, &el.bounds) {
                continue;
            }
        }
        if !attr_matches(&el.attributes, &filter.attributes) {
            continue;
        }
        let mut visible = el.flags.visible_manual
            && layer
                .and_then(|l| doc.layer(l))
                .map(|l| l.visible)
                .unwrap_or(false);
        if let Some(v) = view {
            visible &= el
                .scale_visibility
                .allows(v.pixels_per_world, v.meters_per_pixel);
            visible &= match v.mode {
                ViewMode::Flat => el.style.show_in_flat,
                ViewMode::Globe => el.style.show_in_globe,
            };
        }
        out.push(ElementSummary {
            id,
            name: el.name.clone(),
            kind: el.geometry.kind(),
            layer,
            bounds: el.bounds,
            attributes: el.attributes.clone(),
            visible,
        });
    }
    out
}

/// 一个元素的大圆 / 周长长度（米），未知 id 返回 `None`。点类
/// 类型量得 `0.0`。
pub fn measure_length(doc: &Document, id: ElementId) -> Option<f64> {
    doc.element(id).map(|e| measure_length_m(&e.geometry))
}

/// 一个元素的包围面积（m²），未知 id 返回 `None`。开放 /
/// 点类类型量得 `0.0`。
pub fn measure_area(doc: &Document, id: ElementId) -> Option<f64> {
    doc.element(id).map(|e| measure_area_m2(&e.geometry))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::GeoPoint;
    use crate::model::geometry::{Geometry, Polyline};
    use crate::model::ids::ElementId;

    fn pt(lon: f64, lat: f64) -> GeoPoint {
        GeoPoint::surface(lon, lat)
    }

    fn line(doc: &mut Document, name: &str, a: GeoPoint, b: GeoPoint) -> ElementId {
        let layer = doc.active_layer().unwrap();
        let ne = doc.make_element(
            name,
            Geometry::Polyline(Polyline {
                positions: vec![a, b],
            }),
        );
        let id = ne.id;
        doc.add_element_to_layer(layer, ne);
        id
    }

    fn box_filter(w: f64, s: f64, e: f64, n: f64) -> QueryFilter {
        QueryFilter {
            bbox: Some(GeoBounds {
                west_deg: w,
                south_deg: s,
                east_deg: e,
                north_deg: n,
            }),
            ..Default::default()
        }
    }

    #[test]
    fn empty_filter_returns_all_in_order() {
        let mut doc = Document::with_default_layer();
        line(&mut doc, "a", pt(0.0, 0.0), pt(1.0, 1.0));
        line(&mut doc, "b", pt(5.0, 5.0), pt(6.0, 6.0));
        let got = query(&doc, &QueryFilter::default(), None);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].name, "a");
        assert_eq!(got[1].name, "b");
    }

    #[test]
    fn kind_and_name_and_layer_filters() {
        let mut doc = Document::with_default_layer();
        let id = line(&mut doc, "front-line", pt(0.0, 0.0), pt(1.0, 0.0));
        let pid = {
            let layer = doc.active_layer().unwrap();
            let ne = doc.make_element("depot", Geometry::Point(pt(2.0, 2.0)));
            let id = ne.id;
            doc.add_element_to_layer(layer, ne);
            id
        };
        let only_lines = query(
            &doc,
            &QueryFilter {
                kind: Some(GeometryKind::Line),
                ..Default::default()
            },
            None,
        );
        assert_eq!(only_lines.len(), 1);
        assert_eq!(only_lines[0].id, id);

        let by_name = query(
            &doc,
            &QueryFilter {
                name_contains: Some("depot".into()),
                ..Default::default()
            },
            None,
        );
        assert_eq!(by_name.len(), 1);
        assert_eq!(by_name[0].id, pid);

        let layer = doc.active_layer().unwrap();
        let by_layer = query(
            &doc,
            &QueryFilter {
                layer: Some(layer),
                ..Default::default()
            },
            None,
        );
        assert_eq!(by_layer.len(), 2);
        // 一个无人归属的图层 → 空。
        let none = query(
            &doc,
            &QueryFilter {
                layer: Some(LayerId(999)),
                ..Default::default()
            },
            None,
        );
        assert!(none.is_empty());
    }

    #[test]
    fn attribute_filter_is_exact_subset() {
        let mut doc = Document::with_default_layer();
        let layer = doc.active_layer().unwrap();
        let mut ne = doc.make_element("hostile", Geometry::Point(pt(0.0, 0.0)));
        ne.element
            .attributes
            .insert("side".into(), Value::String("hostile".into()));
        let hid = ne.id;
        doc.add_element_to_layer(layer, ne);
        let _friend = {
            let mut ne2 = doc.make_element("friend", Geometry::Point(pt(1.0, 1.0)));
            ne2.element
                .attributes
                .insert("side".into(), Value::String("friend".into()));
            let id = ne2.id;
            doc.add_element_to_layer(layer, ne2);
            id
        };
        let got = query(
            &doc,
            &QueryFilter {
                attributes: vec![("side".into(), Value::String("hostile".into()))],
                ..Default::default()
            },
            None,
        );
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id, hid);
    }

    #[test]
    fn bbox_overlap_hits_and_misses() {
        let mut doc = Document::with_default_layer();
        let id = line(&mut doc, "square-edge", pt(0.0, 0.0), pt(2.0, 2.0));
        let hit = query(&doc, &box_filter(1.0, 1.0, 3.0, 3.0), None);
        assert_eq!(hit.len(), 1);
        assert_eq!(hit[0].id, id);
        let miss = query(&doc, &box_filter(40.0, 40.0, 50.0, 50.0), None);
        assert!(miss.is_empty());
    }

    #[test]
    fn bbox_does_not_miss_great_circle_bulge() {
        // 一条 60°N 纬线弦跟随一个大圆向极点方向外鼓，因此它的
        // 真实范围超出其原始控制点的 60.0°。一个朴素的控制点盒
        // 会恰好止于 60.0 而错误地拒绝一个位于其紧邻北方的窗口（经典的
        // 静默宽相位遗漏）；元素的*保守* 包围盒涵盖了外鼓。
        let mut doc = Document::with_default_layer();
        let id = line(&mut doc, "arctic", pt(0.0, 60.0), pt(10.0, 60.0));
        let bounds = doc.element(id).unwrap().bounds;
        assert!(
            bounds.north_deg > 60.05,
            "bounds must cover the bulge, got {bounds:?}"
        );
        // 一个严格高于原始 60.0 纬度、但位于外鼓内部的窗口，
        // 仍会匹配 —— 一个仅含控制点的盒会遗漏它。
        let got = query(&doc, &box_filter(4.0, 60.02, 6.0, 60.04), None);
        assert_eq!(got.len(), 1, "bulge window must not miss the line");
        assert_eq!(got[0].id, id);
        // 一个高于真实包围盒顶部的窗口被正确拒绝（而非虚报）。
        let miss = query(&doc, &box_filter(4.0, 60.5, 6.0, 60.9), None);
        assert!(miss.is_empty(), "window above the true bulge must miss");
    }

    #[test]
    fn visibility_reflects_manual_flag_and_view_scale() {
        let mut doc = Document::with_default_layer();
        let layer = doc.active_layer().unwrap();
        let mut ne = doc.make_element("band", Geometry::Point(pt(0.0, 0.0)));
        ne.element.scale_visibility.min_pixels_per_world = Some(100.0);
        ne.element.scale_visibility.max_pixels_per_world = Some(1000.0);
        let id = ne.id;
        doc.add_element_to_layer(layer, ne);

        // 无 view → 仅手动 + 图层支配可见性（忽略比例尺带）。
        let plain = query(&doc, &QueryFilter::default(), None).remove(0);
        assert!(plain.id == id && plain.visible);

        // 一个在带外的 view 隐藏它；一个在带内的 view 保持可见。
        let out_of_band = ViewContext {
            pixels_per_world: 5000.0,
            ..Default::default()
        };
        assert!(!query(&doc, &QueryFilter::default(), Some(&out_of_band))[0].visible);
        let in_band = ViewContext {
            pixels_per_world: 500.0,
            ..Default::default()
        };
        assert!(query(&doc, &QueryFilter::default(), Some(&in_band))[0].visible);
    }

    #[test]
    fn measure_wrappers_match_geometry() {
        let mut doc = Document::with_default_layer();
        // 赤道上经度一度 ≈ 111.32 km。
        let id = line(&mut doc, "deg", pt(0.0, 0.0), pt(1.0, 0.0));
        let len = measure_length(&doc, id).unwrap();
        assert!((len - 111_319.0).abs() < 200.0, "got {len}");
        assert_eq!(measure_area(&doc, id).unwrap(), 0.0, "open line has no area");
        assert!(measure_length(&doc, ElementId(9999)).is_none());
    }
}
