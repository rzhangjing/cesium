//! Pure geographic → world projection maths shared by every bridge view system.
//!
//! This is the concrete half of the plan's `GeoSurface` abstraction (§3): given
//! the metrics of the *currently active* camera it turns a [`GeoPoint`] into a
//! render-space position, and answers the per-depth scale a constant-pixel-size
//! primitive needs. Everything here is a free function of plain numbers — no ECS,
//! no `Query` — so the whole 2D/3D projection + sizing contract is unit-testable
//! headlessly (plan §15) before a single entity is spawned.
//!
//! Two coordinate systems (plan §3):
//!  * [`ViewMode::Globe`] — WGS84 ECEF in render units (unit oblate ellipsoid,
//!    Z-up) via [`geo_to_globe`]; a perspective camera orbits it, so a fixed
//!    screen size needs a *depth-dependent* world size (`dist / focal`).
//!  * [`ViewMode::Flat`] — equirectangular world units (`x = lon_rad`,
//!    `y = lat_rad`) via [`geo_to_flat`]; an orthographic top-down camera makes
//!    world-per-pixel a single constant (`1 / pixels_per_world`).

use bevy::math::Vec3;
use cesium_plot::geo::{geo_to_flat, geo_to_globe, GeoPoint, METERS_PER_RENDER_UNIT};
use cesium_plot::model::ViewMode;

/// Everything the projection needs to know about the active camera, gathered
/// once per frame by the sync system. Pure data.
#[derive(Clone, Copy, Debug)]
pub struct ViewMetrics {
    /// Active projection mode.
    pub mode: ViewMode,
    /// Flat-map pixels per world unit (`Map2dCam.zoom`); ignored for the globe.
    pub pixels_per_world: f64,
    /// Perspective focal length in pixels, `(screen_h / 2) / tan(fov_y / 2)`;
    /// ignored for the flat map.
    pub focal_px: f64,
    /// Active camera position in render-unit world space (for depth sizing).
    pub cam_pos: Vec3,
}

impl ViewMetrics {
    /// Project one geographic coordinate into the active render space. The z
    /// component is left at `0.0` for the flat map (the caller stacks overlay
    /// height) and set to the ellipsoid surface for the globe.
    #[inline]
    pub fn project(&self, geo: GeoPoint) -> Vec3 {
        match self.mode {
            ViewMode::Flat => {
                let p = geo_to_flat(geo);
                Vec3::new(p.x as f32, p.y as f32, 0.0)
            }
            ViewMode::Globe => {
                let v = geo_to_globe(geo);
                Vec3::new(v.x as f32, v.y as f32, v.z as f32)
            }
        }
    }

    /// Camera-to-point distance in render units (globe). The flat map is
    /// orthographic so its "distance" is meaningless; returns `1.0` there.
    #[inline]
    pub fn depth(&self, world: Vec3) -> f64 {
        match self.mode {
            ViewMode::Flat => 1.0,
            ViewMode::Globe => (self.cam_pos - world).length() as f64,
        }
    }

    /// Pixels that span one world unit at the given point's depth. This is the
    /// master scale metric: its reciprocal gives world-per-pixel, and it feeds
    /// the §10.7 scale band.
    #[inline]
    pub fn pixels_per_world_at(&self, world: Vec3) -> f64 {
        match self.mode {
            ViewMode::Flat => self.pixels_per_world,
            ViewMode::Globe => {
                let d = self.depth(world).max(1e-6);
                self.focal_px / d
            }
        }
    }

    /// World units covered by one screen pixel at the given point's depth — the
    /// factor a `size_px`-wide primitive must be scaled by to stay a constant
    /// size on screen at any zoom (plan §2 / §15, "屏幕恒定尺寸").
    #[inline]
    pub fn world_per_px_at(&self, world: Vec3) -> f64 {
        let ppw = self.pixels_per_world_at(world);
        if ppw > 1e-9 {
            1.0 / ppw
        } else {
            0.0
        }
    }

    /// Ground metres per screen pixel at the orbit target — the §10.7 metres
    /// band metric. Flat: `1 world unit == one radian ≈ EARTH_RADIUS` metres.
    #[inline]
    pub fn meters_per_pixel(&self) -> f64 {
        match self.mode {
            ViewMode::Flat => {
                if self.pixels_per_world > 1e-9 {
                    METERS_PER_RENDER_UNIT / self.pixels_per_world
                } else {
                    0.0
                }
            }
            ViewMode::Globe => {
                if self.focal_px > 1e-9 {
                    // Depth to the globe centre is the representative surface.
                    let d = self.cam_pos.length() as f64;
                    (d * METERS_PER_RENDER_UNIT) / self.focal_px
                } else {
                    0.0
                }
            }
        }
    }

    /// Project a run of coordinates (polyline / ring vertices).
    pub fn project_all(&self, pts: &[GeoPoint]) -> Vec<Vec3> {
        pts.iter().map(|p| self.project(*p)).collect()
    }
}

/// Build a camera-facing billboard [`Vec3`] scale so a unit quad in the XY plane
/// (extent `[-0.5, 0.5]`, i.e. 1 world unit wide) measures `size_px` on screen at
/// `world`'s depth.
#[inline]
pub fn billboard_scale(metrics: &ViewMetrics, world: Vec3, size_px: f64) -> Vec3 {
    let s = metrics.world_per_px_at(world) * size_px;
    Vec3::new(s as f32, s as f32, 1.0)
}

/// Half-thickness in world units for a line of `width_px` at `world`'s depth.
#[inline]
pub fn line_half_width(metrics: &ViewMetrics, world: Vec3, width_px: f64) -> f64 {
    metrics.world_per_px_at(world) * width_px * 0.5
}

/// A screen-constant-width ribbon (triangle strip) through `positions`. Each
/// edge is offset perpendicular to `(edge direction, `normal`) by the half-width
/// evaluated at that vertex, so a `width_px` line stays `width_px` wide on screen
/// whatever the zoom. `normal` is the quad-plane normal (`+Z` for the flat map,
/// the view direction for the globe). Returns interleaved `[left, right]` vertices
/// plus CCW triangle indices.
pub fn ribbon(
    positions: &[Vec3],
    width_px_fn: &dyn Fn(usize) -> f64,
    normal: Vec3,
) -> (Vec<[f32; 3]>, Vec<u32>) {
    let mut out_pos: Vec<[f32; 3]> = Vec::with_capacity(positions.len() * 2);
    let mut idx: Vec<u32> = Vec::new();
    if positions.len() < 2 {
        return (out_pos, idx);
    }
    for i in 0..positions.len() {
        let cur = positions[i];
        // Tangent: average of the adjacent segments, clamped at the endpoints.
        let mut tangent = Vec3::ZERO;
        if i > 0 {
            tangent += (cur - positions[i - 1]).normalize_or_zero();
        }
        if i + 1 < positions.len() {
            tangent += (positions[i + 1] - cur).normalize_or_zero();
        }
        let tangent = tangent.normalize_or_zero();
        // Perpendicular in the quad plane: normal × tangent.
        let side = normal.cross(tangent).normalize_or_zero();
        let half = width_px_fn(i) as f32;
        let l = cur + side * half;
        let r = cur - side * half;
        out_pos.push(l.to_array());
        out_pos.push(r.to_array());
        if i > 0 {
            let b = (i * 2) as u32;
            // quad (b-2,b-1,b,b+1) split into two CCW triangles.
            idx.extend_from_slice(&[b - 2, b - 1, b, b, b - 1, b + 1]);
        }
    }
    (out_pos, idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_plot::geo::globe_to_geo;

    fn flat(zoom: f64) -> ViewMetrics {
        ViewMetrics {
            mode: ViewMode::Flat,
            pixels_per_world: zoom,
            focal_px: 0.0,
            cam_pos: Vec3::ZERO,
        }
    }

    fn globe(cam: Vec3, focal: f64) -> ViewMetrics {
        ViewMetrics {
            mode: ViewMode::Globe,
            pixels_per_world: 0.0,
            focal_px: focal,
            cam_pos: cam,
        }
    }

    #[test]
    fn flat_project_is_radians_in_xy() {
        let m = flat(1.0);
        let p = m.project(GeoPoint::surface(180.0, 45.0));
        assert!((p.x - std::f32::consts::PI).abs() < 1e-5);
        assert!((p.y - std::f32::consts::FRAC_PI_4).abs() < 1e-5);
        assert_eq!(p.z, 0.0);
    }

    #[test]
    fn flat_world_per_px_is_inverse_zoom() {
        let m = flat(200.0);
        let w = m.world_per_px_at(Vec3::new(0.1, 0.1, 0.0));
        assert!((w - 1.0 / 200.0).abs() < 1e-9, "{w}");
        // constant regardless of position (orthographic)
        let w2 = m.world_per_px_at(Vec3::new(3.0, -2.0, 0.0));
        assert!((w - w2).abs() < 1e-12);
    }

    #[test]
    fn globe_project_lands_on_unit_ellipsoid() {
        let m = globe(Vec3::new(3.0, 0.0, 0.0), 600.0);
        let world = m.project(GeoPoint::surface(0.0, 0.0));
        // (0°,0°) → render (1,0,0); back-round-trips to the same geo.
        let back = globe_to_geo(world.as_dvec3()).unwrap();
        assert!((back.lon_deg - 0.0).abs() < 1e-6);
        assert!((back.lat_deg - 0.0).abs() < 1e-6);
        assert!((world.length() - 1.0).abs() < 1e-3);
    }

    #[test]
    fn globe_world_per_px_grows_with_depth() {
        // Same focal, camera further away → bigger world-per-pixel at a fixed
        // surface point (things shrink on screen), and pixels_per_world shrinks.
        let near = globe(Vec3::new(2.0, 0.0, 0.0), 600.0);
        let far = globe(Vec3::new(6.0, 0.0, 0.0), 600.0);
        let pt = Vec3::new(1.0, 0.0, 0.0);
        let dpp_near = near.world_per_px_at(pt); // dist 1 → 1/600
        let dpp_far = far.world_per_px_at(pt); // dist 5 → 5/600
        assert!((dpp_near - 1.0 / 600.0).abs() < 1e-9, "{dpp_near}");
        assert!((dpp_far - 5.0 / 600.0).abs() < 1e-9, "{dpp_far}");
        assert!(dpp_far > dpp_near);
        assert!(near.pixels_per_world_at(pt) > far.pixels_per_world_at(pt));
    }

    #[test]
    fn meters_per_pixel_flat_and_globe() {
        let f = flat(6378137.0); // 1 px == 1 metre at the equatorial radius scale
        assert!((f.meters_per_pixel() - 1.0).abs() < 1e-6);
        // Globe: camera at 2 render units, focal 1000 → d=2 → 2*6378137/1000.
        let g = globe(Vec3::new(2.0, 0.0, 0.0), 1000.0);
        let mpp = g.meters_per_pixel();
        assert!((mpp - 2.0 * 6378137.0 / 1000.0).abs() < 1.0, "{mpp}");
    }

    #[test]
    fn billboard_scale_matches_requested_px() {
        let m = flat(250.0);
        let s = billboard_scale(&m, Vec3::new(0.0, 0.0, 0.0), 10.0);
        // 10 px / 250 ppw == 0.04 world units
        assert!((s.x - 0.04).abs() < 1e-6);
        assert!((s.y - 0.04).abs() < 1e-6);
    }

    #[test]
    fn ribbon_has_two_vertices_per_point_and_two_triangles_per_quad() {
        let pts = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
        ];
        let (pos, idx) = ribbon(&pts, &|_| 0.1, Vec3::Z);
        assert_eq!(pos.len(), 6); // 2 per vertex
        assert_eq!(idx.len(), 12); // 2 quads × 6 indices... wait, 2 quads → 12
    }

    #[test]
    fn ribbon_offsets_perpendicular_to_travel() {
        // Travelling +X with normal +Z → side = Z×X = +Y, so the left vertex is
        // above the centreline and the right below.
        let pts = vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)];
        let (pos, _idx) = ribbon(&pts, &|_| 0.2, Vec3::Z);
        // first pair = vertex 0: left(0) then right(1)
        assert!(pos[0][1] > 0.0, "left vertex is +Y: {pos:?}");
        assert!(pos[1][1] < 0.0, "right vertex is -Y: {pos:?}");
    }
}
