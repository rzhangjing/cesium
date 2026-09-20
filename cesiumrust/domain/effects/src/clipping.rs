//! Clipping planes for selective rendering disable.
//!
//! Maps to CesiumJS:
//! - `Scene/ClippingPlane.js` — a single clipping plane
//! - `Scene/ClippingPlaneCollection.js` — collection with union/intersection modes
//!
//! Domain layer — pure Rust, f64 precision.

use glam::{DMat3, DMat4, DVec3};

/// A single clipping plane defined by a normal and distance.
///
/// Maps to CesiumJS `ClippingPlane`.
///
/// The plane equation is: dot(normal, point) + distance = 0
/// Points on the positive side (dot + distance > 0) are kept.
/// Points on the negative side are clipped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClippingPlane {
    /// The plane normal (normalized).
    pub normal: DVec3,
    /// The distance from the origin along the normal.
    /// Positive distance means the plane is offset in the normal direction.
    pub distance: f64,
}

impl ClippingPlane {
    /// Creates a new clipping plane.
    ///
    /// # Arguments
    /// * `normal` - The plane normal (will be normalized)
    /// * `distance` - The signed distance from origin
    pub fn new(normal: DVec3, distance: f64) -> Self {
        Self {
            normal: normal.normalize(),
            distance,
        }
    }

    /// Computes the signed distance from a point to this plane.
    ///
    /// Positive = point is on the kept side.
    /// Negative = point is on the clipped side.
    pub fn signed_distance(&self, point: DVec3) -> f64 {
        self.normal.dot(point) + self.distance
    }

    /// Returns whether a point is inside (kept) by this plane.
    pub fn is_inside(&self, point: DVec3) -> bool {
        self.signed_distance(point) >= 0.0
    }

    /// Transforms this plane by a 4x4 matrix.
    ///
    /// Uses the inverse transpose of the matrix for correct normal transformation.
    pub fn transform(&self, matrix: &DMat4) -> Self {
        // Transform a point on the plane (points transform by the full affine M).
        let point_on_plane = self.normal * (-self.distance);
        let transformed_point = matrix.transform_point3(point_on_plane);

        // Normals are covectors: the correct transform is the *inverse transpose* of
        // the upper-left 3×3, so the plane stays perpendicular to the surface under
        // non-uniform scale. The previous `Mᵀ`-only form was valid solely for rigid /
        // uniformly-scaled matrices, contradicting this docstring.
        // FIX-CLIP-TRANSFORM. Guard the inverse against a singular / non-finite 3×3.
        let upper = DMat3::from_cols(
            matrix.x_axis.truncate(),
            matrix.y_axis.truncate(),
            matrix.z_axis.truncate(),
        );
        let det = upper.determinant();
        let normal_matrix = if det.is_finite() && det.abs() > 1.0e-12 {
            upper.inverse().transpose()
        } else {
            // Degenerate scale: fall back to the rigid-assumption `Mᵀ` form.
            upper.transpose()
        };
        let transformed_normal = normal_matrix.mul_vec3(self.normal).normalize();

        let new_distance = -transformed_normal.dot(transformed_point);

        Self {
            normal: transformed_normal,
            distance: new_distance,
        }
    }

    /// Packs the plane into a vec4 (normal.xyz, distance) for GPU upload.
    pub fn to_vec4(&self) -> [f64; 4] {
        [self.normal.x, self.normal.y, self.normal.z, self.distance]
    }

    /// Creates a plane from a packed vec4.
    pub fn from_vec4(v: [f64; 4]) -> Self {
        Self {
            normal: DVec3::new(v[0], v[1], v[2]),
            distance: v[3],
        }
    }
}

/// Intersection test result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intersect {
    /// The object is completely inside (kept).
    Inside,
    /// The object intersects the plane.
    Intersecting,
    /// The object is completely outside (clipped).
    Outside,
}

/// A collection of clipping planes.
///
/// Maps to CesiumJS `ClippingPlaneCollection`.
#[derive(Debug, Clone)]
pub struct ClippingPlaneCollection {
    /// The clipping planes.
    planes: Vec<ClippingPlane>,
    /// Whether clipping is enabled.
    pub enabled: bool,
    /// Additional transform applied to all planes.
    pub model_matrix: DMat4,
    /// If true, clip if outside ANY plane (union).
    /// If false, clip only if outside ALL planes (intersection).
    pub union_clipping_regions: bool,
    /// Edge highlight color [R, G, B, A].
    pub edge_color: [f64; 4],
    /// Edge highlight width in pixels.
    pub edge_width: f64,
}

impl Default for ClippingPlaneCollection {
    fn default() -> Self {
        Self {
            planes: Vec::new(),
            enabled: true,
            model_matrix: DMat4::IDENTITY,
            union_clipping_regions: false,
            edge_color: [1.0, 1.0, 1.0, 1.0],
            edge_width: 0.0,
        }
    }
}

impl ClippingPlaneCollection {
    /// Creates a new empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a collection with initial planes.
    pub fn with_planes(planes: Vec<ClippingPlane>) -> Self {
        Self {
            planes,
            ..Default::default()
        }
    }

    /// Adds a clipping plane.
    pub fn add(&mut self, plane: ClippingPlane) {
        self.planes.push(plane);
    }

    /// Removes a plane by index.
    ///
    /// Returns the removed plane, or None if index is out of bounds.
    pub fn remove(&mut self, index: usize) -> Option<ClippingPlane> {
        if index < self.planes.len() {
            Some(self.planes.remove(index))
        } else {
            None
        }
    }

    /// Removes all planes.
    pub fn remove_all(&mut self) {
        self.planes.clear();
    }

    /// Returns the number of planes.
    pub fn len(&self) -> usize {
        self.planes.len()
    }

    /// Returns whether the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.planes.is_empty()
    }

    /// Gets a plane by index.
    pub fn get(&self, index: usize) -> Option<&ClippingPlane> {
        self.planes.get(index)
    }

    /// Gets a mutable plane by index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut ClippingPlane> {
        self.planes.get_mut(index)
    }

    /// Returns the clipping planes state value.
    ///
    /// The sign encodes the clipping mode:
    /// - Positive = union mode
    /// - Negative = intersection mode
    ///
    /// Maps to CesiumJS `clippingPlanesState`.
    pub fn clipping_planes_state(&self) -> i32 {
        let count = self.planes.len() as i32;
        if self.union_clipping_regions {
            count
        } else {
            -count
        }
    }

    /// Tests whether a point is clipped by this collection.
    ///
    /// # Arguments
    /// * `point` - The world-space point to test
    ///
    /// # Returns
    /// `true` if the point should be clipped (not rendered).
    pub fn is_clipped(&self, point: DVec3) -> bool {
        if !self.enabled || self.planes.is_empty() {
            return false;
        }

        // Transform point to clipping plane space
        let inverse = self.model_matrix.inverse();
        let local_point = inverse.transform_point3(point);

        if self.union_clipping_regions {
            // Union: clip if outside ANY plane
            self.planes.iter().any(|p| !p.is_inside(local_point))
        } else {
            // Intersection: clip only if outside ALL planes
            self.planes.iter().all(|p| !p.is_inside(local_point))
        }
    }

    /// f64 CPU reference for the GPU `apply_clipping_planes` accumulation
    /// (`adapters/bevy-render/shaders/clipping.wgsl`), entry-for-entry, used to
    /// cross-validate the f32 shader. Unlike [`Self::is_clipped`] it also returns
    /// the *signed* clipAmount and uses the blueprint's `<= 0.0` outside test
    /// (FIX-CLIP-LTE): a point exactly on a plane counts as clipped.
    ///
    /// Returns `(clipped, clip_amount)` where `clip_amount` is the signed min over
    /// planes in union mode (blueprint L59) or the signed max seeded at `0.0` in
    /// intersection mode (blueprint L64) — the value that drives the edge band.
    pub fn clip_signed(&self, point: DVec3) -> (bool, f64) {
        if !self.enabled || self.planes.is_empty() {
            return (false, 0.0);
        }
        let local_point = self.model_matrix.inverse().transform_point3(point);

        let mut any_outside = false;
        let mut all_outside = true;
        let mut clip_amount = 0.0;
        for (i, p) in self.planes.iter().enumerate() {
            let d = p.signed_distance(local_point);
            if d <= 0.0 {
                any_outside = true;
            } else {
                all_outside = false;
            }
            clip_amount = if self.union_clipping_regions {
                if i == 0 {
                    d
                } else {
                    d.min(clip_amount)
                }
            } else {
                d.max(clip_amount)
            };
        }
        let clipped = if self.union_clipping_regions {
            any_outside
        } else {
            all_outside
        };
        (clipped, clip_amount)
    }

    /// Tests the intersection of a bounding sphere with the clipping planes.
    ///
    /// Maps to CesiumJS `ClippingPlaneCollection.prototype.computeIntersectionWithBoundingVolume`.
    ///
    /// # Arguments
    /// * `center` - Sphere center (world space)
    /// * `radius` - Sphere radius
    ///
    /// # Returns
    /// The intersection result.
    pub fn intersect_bounding_sphere(&self, center: DVec3, radius: f64) -> Intersect {
        if !self.enabled || self.planes.is_empty() {
            return Intersect::Inside;
        }

        let inverse = self.model_matrix.inverse();
        let local_center = inverse.transform_point3(center);

        // Initialize based on clipping mode (matching CesiumJS):
        // - Union mode: start INSIDE; if any plane contains the sphere on
        //   its negative side, the entire sphere is clipped → OUTSIDE.
        // - Intersection mode: start OUTSIDE; if any plane contains the
        //   sphere on its positive side, no point can be outside ALL
        //   planes → INSIDE.
        let mut intersection = if self.union_clipping_regions {
            Intersect::Inside
        } else {
            Intersect::Outside
        };

        for plane in &self.planes {
            let dist = plane.signed_distance(local_center);

            let value = if dist < -radius {
                Intersect::Outside
            } else if dist > radius {
                Intersect::Inside
            } else {
                Intersect::Intersecting
            };

            if value == Intersect::Intersecting {
                intersection = Intersect::Intersecting;
            } else if self.union_clipping_regions {
                // Union mode: if any plane is OUTSIDE, the whole sphere is clipped
                if value == Intersect::Outside {
                    return Intersect::Outside;
                }
            } else {
                // Intersection mode: if any plane is INSIDE, no point can be
                // outside ALL planes, so the sphere is kept
                if value == Intersect::Inside {
                    return Intersect::Inside;
                }
            }
        }

        intersection
    }

    /// Packs all planes into a flat array for GPU upload.
    ///
    /// Each plane is 4 floats: [normal.x, normal.y, normal.z, distance].
    pub fn pack_planes(&self) -> Vec<f64> {
        let mut packed = Vec::with_capacity(self.planes.len() * 4);
        for plane in &self.planes {
            packed.extend_from_slice(&plane.to_vec4());
        }
        packed
    }

    /// Computes the edge highlight factor for a fragment.
    ///
    /// Returns a value in [0, 1] indicating how close the fragment is to a clip edge.
    ///
    /// # Arguments
    /// * `point` - The fragment position (world space)
    /// * `pixel_size` - The world-space size of a pixel at this depth
    pub fn edge_factor(&self, point: DVec3, pixel_size: f64) -> f64 {
        if self.edge_width <= 0.0 || !self.enabled || self.planes.is_empty() {
            return 0.0;
        }

        let inverse = self.model_matrix.inverse();
        let local_point = inverse.transform_point3(point);

        let edge_threshold = self.edge_width * pixel_size;

        for plane in &self.planes {
            let dist = plane.signed_distance(local_point).abs();
            if dist < edge_threshold {
                return 1.0 - dist / edge_threshold;
            }
        }

        0.0
    }

    /// Returns the planes transformed into world space by [`Self::model_matrix`].
    ///
    /// This is the CPU-side equivalent of CesiumJS's per-fragment
    /// `czm_transformPlane(plane, clippingPlanesMatrix)`
    /// (`ModelClippingPlanesStageFS.glsl` L14/L35): instead of transforming each
    /// plane on the GPU every fragment, the collection's `model_matrix` is baked
    /// into the planes once on the CPU. The two are sign-equivalent — a world
    /// point `p` is inside `world_planes()[i]` iff `model_matrix.inverse() * p`
    /// is inside `planes[i]` — which is exactly the invariant [`Self::is_clipped`]
    /// relies on (see `test_world_planes_transform_equivalence`).
    ///
    /// The returned planes stay in **domain metric f64** space (no
    /// `METERS_PER_RENDER_UNIT` rescaling here — that conversion is applied at the
    /// adapter/GPU boundary, red line).
    pub fn world_planes(&self) -> Vec<ClippingPlane> {
        self.planes
            .iter()
            .map(|p| p.transform(&self.model_matrix))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── ClippingPlane tests ────────────────────────────────────────────

    #[test]
    fn test_plane_creation() {
        let plane = ClippingPlane::new(DVec3::new(0.0, 1.0, 0.0), 5.0);
        assert!((plane.normal - DVec3::Y).length() < 1e-10);
        assert!((plane.distance - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_plane_normalizes() {
        let plane = ClippingPlane::new(DVec3::new(0.0, 2.0, 0.0), 5.0);
        assert!((plane.normal.length() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_plane_signed_distance() {
        let plane = ClippingPlane::new(DVec3::Y, 0.0);

        // Point above plane
        assert!((plane.signed_distance(DVec3::new(0.0, 5.0, 0.0)) - 5.0).abs() < 1e-10);
        // Point below plane
        assert!((plane.signed_distance(DVec3::new(0.0, -3.0, 0.0)) - (-3.0)).abs() < 1e-10);
        // Point on plane
        assert!(plane.signed_distance(DVec3::new(1.0, 0.0, 1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_plane_is_inside() {
        let plane = ClippingPlane::new(DVec3::Y, 0.0);

        assert!(plane.is_inside(DVec3::new(0.0, 1.0, 0.0)));
        assert!(!plane.is_inside(DVec3::new(0.0, -1.0, 0.0)));
        assert!(plane.is_inside(DVec3::ZERO)); // On plane = inside
    }

    #[test]
    fn test_plane_with_offset() {
        // Plane at y = 5 (normal pointing up, distance = -5)
        let plane = ClippingPlane::new(DVec3::Y, -5.0);

        assert!(plane.is_inside(DVec3::new(0.0, 10.0, 0.0))); // Above
        assert!(!plane.is_inside(DVec3::new(0.0, 3.0, 0.0))); // Below
    }

    #[test]
    fn test_plane_vec4_roundtrip() {
        let plane = ClippingPlane::new(DVec3::new(1.0, 2.0, 3.0), 4.0);
        let packed = plane.to_vec4();
        let unpacked = ClippingPlane::from_vec4(packed);

        assert!((plane.normal - unpacked.normal).length() < 1e-10);
        assert!((plane.distance - unpacked.distance).abs() < 1e-10);
    }

    #[test]
    fn test_plane_transform_translation() {
        let plane = ClippingPlane::new(DVec3::Y, 0.0);
        let translation = DMat4::from_translation(DVec3::new(0.0, 10.0, 0.0));

        let transformed = plane.transform(&translation);

        // After translating the plane's coordinate system up by 10,
        // the plane (originally at y=0) is now effectively at y=10 in world space.
        // A point at y=15 should be inside (above the plane).
        assert!(transformed.is_inside(DVec3::new(0.0, 15.0, 0.0)));
        // A point at y=5 should be outside (below the plane).
        assert!(!transformed.is_inside(DVec3::new(0.0, 5.0, 0.0)));
    }

    #[test]
    fn test_plane_transform_inverse_transpose_under_non_uniform_scale() {
        // FIX-CLIP-TRANSFORM: normals must transform by the inverse-transpose, not
        // Mᵀ. Plane normal (1,1,0)/√2 under scale(2,1,1):
        //   correct  (S⁻ᵀ n) ∝ (1/2, 1, 0) -> (0.4472, 0.8944, 0)
        //   buggy    (Sᵀ  n) ∝ (2,   1, 0) -> (0.8944, 0.4472, 0)
        let plane = ClippingPlane::new(DVec3::new(1.0, 1.0, 0.0), 0.0);
        let scaled = DMat4::from_scale(DVec3::new(2.0, 1.0, 1.0));
        let transformed = plane.transform(&scaled);

        let expected = DVec3::new(0.5, 1.0, 0.0).normalize();
        assert!(
            (transformed.normal - expected).length() < 1e-9,
            "inverse-transpose expected, got {:?}",
            transformed.normal
        );
    }

    #[test]
    fn test_clip_signed_boundary_on_plane_is_clipped() {
        // FIX-CLIP-LTE: a point exactly ON a plane (signed distance == 0) counts as
        // clipped, matching the blueprint `<= 0.0` GPU test.
        let collection = ClippingPlaneCollection::with_planes(vec![ClippingPlane::new(DVec3::Y, 0.0)]);
        let (clipped, amount) = collection.clip_signed(DVec3::new(3.0, 0.0, -2.0));
        assert!(clipped, "point on plane must be clipped");
        assert!(amount.abs() < 1e-12, "on-plane clip_amount is 0, got {amount}");
    }

    #[test]
    fn test_clip_signed_union_vs_intersection() {
        // Two opposing planes: x >= 0 kept (normal +X, distance 0) and y >= 0 kept.
        let planes = vec![ClippingPlane::new(DVec3::X, 0.0), ClippingPlane::new(DVec3::Y, 0.0)];

        let mut union = ClippingPlaneCollection::with_planes(planes.clone());
        union.union_clipping_regions = true;
        // Outside +X plane (x<0) but inside +Y plane ⇒ union clips (outside ANY).
        let (u_clipped, u_amt) = union.clip_signed(DVec3::new(-1.0, 2.0, 0.0));
        assert!(u_clipped, "union clips when outside any plane");
        assert!(u_amt < 0.0, "union signed min is the most-negative, got {u_amt}");

        let inter = ClippingPlaneCollection::with_planes(planes);
        // Inside +Y plane ⇒ intersection does NOT clip (needs outside ALL).
        let (i_clipped, _) = inter.clip_signed(DVec3::new(-1.0, 2.0, 0.0));
        assert!(!i_clipped, "intersection keeps a point inside any plane");
        // Outside both ⇒ intersection clips; signed max seeded at 0 ⇒ 0 here.
        let (i2_clipped, i2_amt) = inter.clip_signed(DVec3::new(-1.0, -1.0, 0.0));
        assert!(i2_clipped, "intersection clips when outside all planes");
        assert!(i2_amt >= 0.0, "intersection signed max is floored at 0, got {i2_amt}");
    }

    // ─── ClippingPlaneCollection tests ──────────────────────────────────

    #[test]
    fn test_collection_default() {
        let collection = ClippingPlaneCollection::default();
        assert!(collection.enabled);
        assert!(!collection.union_clipping_regions);
        assert!(collection.is_empty());
        assert!((collection.edge_width).abs() < 1e-10);
    }

    #[test]
    fn test_collection_add_remove() {
        let mut collection = ClippingPlaneCollection::new();
        collection.add(ClippingPlane::new(DVec3::Y, 0.0));
        collection.add(ClippingPlane::new(DVec3::X, 0.0));

        assert_eq!(collection.len(), 2);

        let removed = collection.remove(0);
        assert!(removed.is_some());
        assert_eq!(collection.len(), 1);

        assert!(collection.remove(5).is_none());
    }

    #[test]
    fn test_collection_remove_all() {
        let mut collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
            ClippingPlane::new(DVec3::X, 0.0),
        ]);

        collection.remove_all();
        assert!(collection.is_empty());
    }

    #[test]
    fn test_clipping_planes_state() {
        let mut collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
            ClippingPlane::new(DVec3::X, 0.0),
        ]);

        // Intersection mode (default): negative
        assert_eq!(collection.clipping_planes_state(), -2);

        // Union mode: positive
        collection.union_clipping_regions = true;
        assert_eq!(collection.clipping_planes_state(), 2);
    }

    #[test]
    fn test_is_clipped_disabled() {
        let mut collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
        ]);
        collection.enabled = false;

        assert!(!collection.is_clipped(DVec3::new(0.0, -100.0, 0.0)));
    }

    #[test]
    fn test_is_clipped_intersection_mode() {
        // Two planes forming a corner (intersection mode)
        let collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0), // Keep y >= 0
            ClippingPlane::new(DVec3::X, 0.0), // Keep x >= 0
        ]);

        // Inside both planes
        assert!(!collection.is_clipped(DVec3::new(1.0, 1.0, 0.0)));

        // Outside one plane but inside other → NOT clipped (intersection mode)
        assert!(!collection.is_clipped(DVec3::new(-1.0, 1.0, 0.0)));

        // Outside both planes → clipped
        assert!(collection.is_clipped(DVec3::new(-1.0, -1.0, 0.0)));
    }

    #[test]
    fn test_is_clipped_union_mode() {
        let mut collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
            ClippingPlane::new(DVec3::X, 0.0),
        ]);
        collection.union_clipping_regions = true;

        // Inside both planes
        assert!(!collection.is_clipped(DVec3::new(1.0, 1.0, 0.0)));

        // Outside one plane → clipped (union mode)
        assert!(collection.is_clipped(DVec3::new(-1.0, 1.0, 0.0)));
    }

    #[test]
    fn test_intersect_bounding_sphere_inside() {
        let collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
        ]);

        // Sphere completely above plane
        let result = collection.intersect_bounding_sphere(DVec3::new(0.0, 10.0, 0.0), 1.0);
        assert_eq!(result, Intersect::Inside);
    }

    #[test]
    fn test_intersect_bounding_sphere_outside() {
        let collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
        ]);

        // Sphere completely below plane
        let result = collection.intersect_bounding_sphere(DVec3::new(0.0, -10.0, 0.0), 1.0);
        assert_eq!(result, Intersect::Outside);
    }

    #[test]
    fn test_intersect_bounding_sphere_intersecting() {
        let collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
        ]);

        // Sphere straddling the plane
        let result = collection.intersect_bounding_sphere(DVec3::new(0.0, 0.5, 0.0), 1.0);
        assert_eq!(result, Intersect::Intersecting);
    }

    #[test]
    fn test_intersect_bounding_sphere_union_outside_any_plane() {
        // Union mode: clip if outside ANY plane
        let mut collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0), // y >= 0
            ClippingPlane::new(DVec3::X, 0.0), // x >= 0
        ]);
        collection.union_clipping_regions = true;

        // Sphere fully inside plane Y but fully outside plane X → Outside
        let result = collection.intersect_bounding_sphere(DVec3::new(-10.0, 10.0, 0.0), 1.0);
        assert_eq!(result, Intersect::Outside);

        // Sphere fully inside plane X but fully outside plane Y → Outside
        let result = collection.intersect_bounding_sphere(DVec3::new(10.0, -10.0, 0.0), 1.0);
        assert_eq!(result, Intersect::Outside);

        // Sphere fully inside both → Inside
        let result = collection.intersect_bounding_sphere(DVec3::new(10.0, 10.0, 0.0), 1.0);
        assert_eq!(result, Intersect::Inside);

        // Sphere fully outside both → Outside
        let result = collection.intersect_bounding_sphere(DVec3::new(-10.0, -10.0, 0.0), 1.0);
        assert_eq!(result, Intersect::Outside);
    }

    #[test]
    fn test_intersect_bounding_sphere_intersection_inside_any_plane() {
        // Intersection mode: clip only if outside ALL planes.
        // If the sphere is fully inside ANY plane, it is kept.
        let collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0), // y >= 0
            ClippingPlane::new(DVec3::X, 0.0), // x >= 0
        ]);

        // Sphere fully inside Y, fully outside X → Inside (every point is inside Y)
        let result = collection.intersect_bounding_sphere(DVec3::new(-10.0, 10.0, 0.0), 1.0);
        assert_eq!(result, Intersect::Inside);

        // Sphere fully inside X, fully outside Y → Inside (every point is inside X)
        let result = collection.intersect_bounding_sphere(DVec3::new(10.0, -10.0, 0.0), 1.0);
        assert_eq!(result, Intersect::Inside);

        // Sphere fully outside both → Outside
        let result = collection.intersect_bounding_sphere(DVec3::new(-10.0, -10.0, 0.0), 1.0);
        assert_eq!(result, Intersect::Outside);

        // Sphere fully inside both → Inside
        let result = collection.intersect_bounding_sphere(DVec3::new(10.0, 10.0, 0.0), 1.0);
        assert_eq!(result, Intersect::Inside);
    }

    #[test]
    fn test_intersect_bounding_sphere_intersecting_multi_plane() {
        // Sphere straddles all planes → Intersecting
        let mut collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
            ClippingPlane::new(DVec3::X, 0.0),
        ]);

        // Intersection mode: sphere straddles both planes
        let result = collection.intersect_bounding_sphere(DVec3::new(0.5, 0.5, 0.0), 1.0);
        assert_eq!(result, Intersect::Intersecting);

        // Union mode: sphere straddles both planes
        collection.union_clipping_regions = true;
        let result = collection.intersect_bounding_sphere(DVec3::new(0.5, 0.5, 0.0), 1.0);
        assert_eq!(result, Intersect::Intersecting);
    }

    #[test]
    fn test_pack_planes() {
        let collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 5.0),
            ClippingPlane::new(DVec3::X, -3.0),
        ]);

        let packed = collection.pack_planes();
        assert_eq!(packed.len(), 8); // 2 planes * 4 values

        // First plane: normal Y, distance 5
        assert!((packed[1] - 1.0).abs() < 1e-10);
        assert!((packed[3] - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_edge_factor_no_edge() {
        let collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
        ]);

        // Point far from edge
        let factor = collection.edge_factor(DVec3::new(0.0, 10.0, 0.0), 0.1);
        assert!((factor).abs() < 1e-10);
    }

    #[test]
    fn test_edge_factor_near_edge() {
        let mut collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
        ]);
        collection.edge_width = 2.0;

        // Point very close to the clipping plane
        let factor = collection.edge_factor(DVec3::new(0.0, 0.05, 0.0), 0.1);
        assert!(factor > 0.0);
        assert!(factor <= 1.0);
    }

    #[test]
    fn test_edge_factor_zero_width() {
        let collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
        ]);

        // edge_width = 0 → no edge highlight
        let factor = collection.edge_factor(DVec3::new(0.0, 0.01, 0.0), 0.1);
        assert!((factor).abs() < 1e-10);
    }

    // ─── M6.2 CPU-reference cross-checks ────────────────────────────────
    //
    // Independent brute-force reference implementations of the union /
    // intersection clip decision, written directly from the upstream CesiumJS
    // semantics (`ClippingPlaneCollection.js` `unionIntersectFunction` =
    // `v === OUTSIDE`, `defaultIntersectFunction` = `v === INSIDE`) rather than
    // reusing the production code paths. These guard `is_clipped` /
    // `intersect_bounding_sphere` against silent semantic drift, and pin the
    // CPU-side `world_planes()` baking invariant that the M6.2 bevy-render
    // adapter relies on when it pre-transforms planes instead of doing a
    // per-fragment `czm_transformPlane`.

    /// Brute-force reference: transform the point into plane-local space, then
    /// apply the union (any-outside) / intersection (all-outside) rule directly.
    fn reference_is_clipped(collection: &ClippingPlaneCollection, point: DVec3) -> bool {
        if !collection.enabled || collection.is_empty() {
            return false;
        }
        let local = collection.model_matrix.inverse().transform_point3(point);
        // A point is OUTSIDE a plane when its signed distance is negative.
        let outside: Vec<bool> = (0..collection.len())
            .map(|i| {
                let p = collection.get(i).unwrap();
                (p.normal.dot(local) + p.distance) < 0.0
            })
            .collect();
        if collection.union_clipping_regions {
            outside.iter().any(|&o| o)
        } else {
            outside.iter().all(|&o| o)
        }
    }

    #[test]
    fn test_is_clipped_matches_cpu_reference() {
        // A deterministic sweep of points across both modes and a non-identity
        // model matrix, cross-checked against the brute-force reference.
        let mut collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
            ClippingPlane::new(DVec3::X, -2.0),
            ClippingPlane::new(DVec3::new(1.0, 1.0, 0.0), 1.0),
        ]);
        collection.model_matrix = DMat4::from_translation(DVec3::new(3.0, -1.0, 2.0));

        let samples = [
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(5.0, 5.0, 5.0),
            DVec3::new(-4.0, 2.0, 0.0),
            DVec3::new(2.0, -3.0, 1.0),
            DVec3::new(-1.0, -1.0, -1.0),
            DVec3::new(10.0, -2.0, 4.0),
        ];

        for union in [false, true] {
            collection.union_clipping_regions = union;
            for &p in &samples {
                assert_eq!(
                    collection.is_clipped(p),
                    reference_is_clipped(&collection, p),
                    "is_clipped diverged from CPU reference (union={union}, point={p:?})"
                );
            }
        }
    }

    #[test]
    fn test_world_planes_transform_equivalence() {
        // The adapter bakes `model_matrix` into the planes on the CPU via
        // `world_planes()` instead of transforming per-fragment on the GPU. Prove
        // the baking is sign-equivalent to `is_clipped`'s local-space test: a
        // world point is clipped by the collection iff the union/intersection rule
        // over the *world-space* planes (tested directly against the world point,
        // no inverse transform) yields the same answer.
        let mut collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
            ClippingPlane::new(DVec3::X, -2.0),
        ]);
        collection.model_matrix = DMat4::from_translation(DVec3::new(1.0, 2.0, -3.0));

        let world = collection.world_planes();
        assert_eq!(world.len(), collection.len());

        let samples = [
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(3.0, 4.0, 5.0),
            DVec3::new(-2.0, 1.0, 0.0),
            DVec3::new(1.0, -5.0, 2.0),
        ];

        for union in [false, true] {
            collection.union_clipping_regions = union;
            let world = collection.world_planes();
            for &p in &samples {
                let outside: Vec<bool> = world.iter().map(|pl| pl.signed_distance(p) < 0.0).collect();
                let clipped_world = if union {
                    outside.iter().any(|&o| o)
                } else {
                    outside.iter().all(|&o| o)
                };
                assert_eq!(
                    collection.is_clipped(p),
                    clipped_world,
                    "world_planes() baking diverged from is_clipped (union={union}, point={p:?})"
                );
            }
        }
    }

    #[test]
    fn test_intersect_bounding_sphere_consistent_with_is_clipped() {
        // Centre-only sanity: when the sphere radius is ~0 the volume test must
        // agree with the point test on the centre (Inside ⇒ not clipped,
        // Outside ⇒ clipped).
        let collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
        ]);
        let r = 1e-9;

        let centre_inside = DVec3::new(0.0, 5.0, 0.0);
        assert_eq!(
            collection.intersect_bounding_sphere(centre_inside, r),
            Intersect::Inside
        );
        assert!(!collection.is_clipped(centre_inside));

        let centre_outside = DVec3::new(0.0, -5.0, 0.0);
        assert_eq!(
            collection.intersect_bounding_sphere(centre_outside, r),
            Intersect::Outside
        );
        assert!(collection.is_clipped(centre_outside));
    }
}
