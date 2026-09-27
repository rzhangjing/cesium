//! A single plot element: geometry + style + business attributes + per-element
//! flags and scale/time visibility windows (plan §5).
//!
//! The geographic bounds are cached on the element and refreshed whenever the
//! geometry changes, so culling / zoom-band checks stay O(1).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::geo::GeoBounds;
use super::geometry::Geometry;
use super::ids::ElementId;
use super::style::Style;

/// Manual / interaction flags (independent of the visibility dimensions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementFlags {
    /// The element's own visibility toggle (AND-ed with layer/group chain).
    pub visible_manual: bool,
    /// Can it be picked / selected?
    pub selectable: bool,
    /// Can editing move / deform it?
    pub editable: bool,
}

impl Default for ElementFlags {
    fn default() -> Self {
        Self {
            visible_manual: true,
            selectable: true,
            editable: true,
        }
    }
}

/// Screen-/ground-scale visibility band (§10.7). Each bound is optional; the
/// element is shown only when the current view falls inside every present
/// bound. Units: `*_px` are pixels-per-world-unit (bigger == zoomed in),
/// `*_meters` are ground metres-per-pixel (bigger == zoomed out).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ScaleVisibility {
    pub min_pixels_per_world: Option<f64>,
    pub max_pixels_per_world: Option<f64>,
    pub min_meters_per_pixel: Option<f64>,
    pub max_meters_per_pixel: Option<f64>,
}

impl ScaleVisibility {
    /// True when there is no band, or both known metrics satisfy it. A metric
    /// of `0.0` (unknown) never *violates* a bound it can't be compared to
    /// conservatively — the bridge is expected to fill the metric for its mode.
    pub fn allows(&self, pixels_per_world: f64, meters_per_pixel: f64) -> bool {
        if let Some(m) = self.min_pixels_per_world {
            if pixels_per_world > 0.0 && pixels_per_world < m {
                return false;
            }
        }
        if let Some(m) = self.max_pixels_per_world {
            if pixels_per_world > 0.0 && pixels_per_world > m {
                return false;
            }
        }
        if let Some(m) = self.min_meters_per_pixel {
            if meters_per_pixel > 0.0 && meters_per_pixel < m {
                return false;
            }
        }
        if let Some(m) = self.max_meters_per_pixel {
            if meters_per_pixel > 0.0 && meters_per_pixel > m {
                return false;
            }
        }
        true
    }
}

/// A plotted element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Element {
    pub id: ElementId,
    pub name: String,
    pub geometry: Geometry,
    pub style: Style,
    /// Free-form business attributes (敌我/番号/状态 …), GeoJSON `properties`.
    pub attributes: Map<String, Value>,
    pub flags: ElementFlags,
    pub scale_visibility: ScaleVisibility,
    /// Reserved time window `[start, end)` in seconds-since-epoch (§10.10).
    /// `None` == always. M1 does not gate on it (see `visibility`).
    pub time_window: Option<(f64, f64)>,
    /// Cached geographic bounds, kept in sync with `geometry`.
    pub bounds: GeoBounds,
}

impl Element {
    /// Build an element, minting no id (caller supplies the id the document
    /// allocates) and computing its bounds.
    pub fn new(id: ElementId, name: impl Into<String>, geometry: Geometry) -> Self {
        let bounds = geometry.bounds();
        Self {
            id,
            name: name.into(),
            geometry,
            style: Style::default(),
            attributes: Map::new(),
            flags: ElementFlags::default(),
            scale_visibility: ScaleVisibility::default(),
            time_window: None,
            bounds,
        }
    }

    /// Replace the geometry and refresh the cached bounds.
    pub fn set_geometry(&mut self, geometry: Geometry) {
        self.bounds = geometry.bounds();
        self.geometry = geometry;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::GeoPoint;

    fn pt(lon: f64, lat: f64) -> Geometry {
        Geometry::Point(GeoPoint::surface(lon, lat))
    }

    #[test]
    fn new_caches_bounds() {
        let e = Element::new(ElementId(1), "p", pt(12.0, -3.0));
        assert!(e.flags.visible_manual && e.flags.selectable && e.flags.editable);
        assert_eq!((e.bounds.west_deg, e.bounds.north_deg), (12.0, -3.0));
    }

    #[test]
    fn set_geometry_refreshes_bounds() {
        let mut e = Element::new(ElementId(2), "l", pt(0.0, 0.0));
        e.set_geometry(Geometry::Polyline(super::super::geometry::Polyline {
            positions: vec![GeoPoint::surface(-5.0, -5.0), GeoPoint::surface(7.0, 9.0)],
        }));
        // The bounds are the conservative (great-circle densified) box: it must
        // contain both endpoints and stay within a tight margin of their span.
        let b = e.bounds;
        assert!(
            b.west_deg <= -5.0 && b.south_deg <= -5.0 && b.east_deg >= 7.0 && b.north_deg >= 9.0,
            "endpoints must be contained: {b:?}",
        );
        assert!(b.width_deg() < 13.0 && b.height_deg() < 15.0, "bulge margin too wide: {b:?}");
    }

    #[test]
    fn scale_band_respects_present_bounds() {
        let sv = ScaleVisibility {
            min_pixels_per_world: Some(100.0),
            max_pixels_per_world: Some(1000.0),
            ..Default::default()
        };
        assert!(sv.allows(500.0, 0.0));
        assert!(!sv.allows(50.0, 0.0), "too far out");
        assert!(!sv.allows(5000.0, 0.0), "too far in");
        // unknown metric (0) never violates.
        assert!(sv.allows(0.0, 0.0));
        // empty band always allows.
        assert!(ScaleVisibility::default().allows(1.0, 1.0));
    }

    #[test]
    fn element_serde_roundtrips() {
        let mut e = Element::new(ElementId(3), "n", pt(1.0, 2.0));
        e.attributes.insert("side".into(), Value::String("friend".into()));
        let j = serde_json::to_string(&e).unwrap();
        let back: Element = serde_json::from_str(&j).unwrap();
        assert_eq!(e, back);
    }
}
