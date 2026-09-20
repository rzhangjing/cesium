//! Panorama rendering (Equirectangular + CubeMap).
//!
//! Maps to CesiumJS:
//! - `Scene/EquirectangularPanorama.js` (266 lines)
//! - `Scene/CubeMapPanorama.js` (352 lines)
//! - `Scene/SkyBox.js` (164 lines) — delegates **completely** to `CubeMapPanorama`
//!   (L39-43 `this._panorama = new CubeMapPanorama({...})`, L100 comment
//!   "Delegate completely"), so the cube-map panorama *is* the skybox truth source.
//! - `Scene/PanoramaProvider.js`
//! - `Shaders/SkyBoxVS.glsl`, `Shaders/SkyBoxFS.glsl`,
//!   `Shaders/CubeMapPanoramaVS.glsl`
//!
//! # f64 discipline
//! Every geometric quantity in this module is `f64` and stays `f64`. The only place
//! it is narrowed to `f32` is the GPU uniform boundary
//! (`adapters/bevy-render/src/effects/panorama.rs::PanoramaUniforms`). The `*_render_units`
//! accessors below return `f64` render units — they convert **scale**, never
//! **precision** — so a caller still decides when to round.
//!
//! # Two placements, two texture layouts
//! Upstream ships exactly two pairings, and this module models both axes
//! independently ([`PanoramaPlacement`] × [`PanoramaSource`]):
//!
//! | upstream primitive           | placement | source        | transform type |
//! |------------------------------|-----------|---------------|----------------|
//! | `CubeMapPanorama` / `SkyBox` | `Skybox`  | `CubeMap`     | **`Matrix3`** |
//! | `EquirectangularPanorama`    | `Bubble`  | `Equirectangular` | `Matrix4` |
//!
//! ## DEVIATION — `CubeMapPanorama::transform` is `DMat4`, upstream is `Matrix3`
//! Upstream `CubeMapPanorama` stores a **`Matrix3`** (`CubeMapPanorama.js` L143-149,
//! bound as `uniform mat3 u_cubeMapPanoramaTransform` in `CubeMapPanoramaVS.glsl`
//! L1): a cube-map skybox is *always* centred on the camera, so it has orientation
//! but no position. This module stores `DMat4` for symmetry with
//! [`EquirectangularPanorama`]. [`CubeMapPanorama::orientation`] is therefore the
//! accessor that reproduces the upstream `Matrix3` — it discards the fourth row and
//! column, which upstream never had. The `DMat4` field is left untouched: changing
//! its type would be a breaking semantic change to an existing published field.

use glam::{DMat3, DMat4, DVec2, DVec3, DVec4};

/// Default panorama radius in meters.
///
/// Upstream `EquirectangularPanorama.js` L15 `const DEFAULT_RADIUS = 100000.0;`.
pub const DEFAULT_PANORAMA_RADIUS: f64 = 100000.0;

/// Meters per render unit — the project-wide scale constant.
///
/// Value-identical mirror of
/// `adapters/bevy-render/src/resources.rs::METERS_PER_RENDER_UNIT`, duplicated here
/// because the domain layer must not depend on an adapter (DDD). Asserted equal by
/// `adapters/bevy-render`'s `panorama_meters_per_render_unit_matches_the_domain`.
///
/// At this scale the upstream default radius is
/// `100_000 / 6_378_137 = 0.015678` render units — a **local bubble** roughly
/// 1.6 % of the globe radius, not an infinite sky. That is why
/// [`PanoramaPlacement`] has two members at all.
pub const PANORAMA_METERS_PER_RENDER_UNIT: f64 = 6_378_137.0;

/// Below this squared length a direction is treated as degenerate.
///
/// `normalize` of a shorter vector is `0/0 = NaN`, and NaN propagates into every
/// texture coordinate derived from it, silently poisoning a whole frame. The same
/// class of defect as the atmosphere first-frame `sun_direction == ZERO` bug
/// (M5 Ultra Review finding H1). Mirrored in f32 as
/// `DEGENERATE_DIRECTION_SQUARED_EPSILON` in `shaders/panorama.wgsl`; the two are
/// independent literals rather than a cast, so double rounding cannot separate them.
pub const DEGENERATE_DIRECTION_SQUARED_EPSILON: f64 = 1.0e-24;

/// Half extent of the upstream skybox box, in box-local units.
///
/// `CubeMapPanorama.js` L189-192:
/// `BoxGeometry.fromDimensions({ dimensions: new Cartesian3(2.0, 2.0, 2.0),
/// vertexFormat: VertexFormat.POSITION_ONLY })` — a 2×2×2 box centred on the
/// origin, so every corner coordinate is `±1`.
pub const SKYBOX_BOX_HALF_EXTENT: f64 = 1.0;

/// How a panorama is placed relative to the camera.
///
/// The `u32` discriminants are the wire format: they are written verbatim into
/// `PanoramaUniforms::mode` and compared against `MODE_SKYBOX` / `MODE_BUBBLE` in
/// `shaders/panorama.wgsl`. Reordering the variants is therefore a breaking change,
/// and is guarded by [`tests::placement_and_source_discriminants_match_the_shader`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum PanoramaPlacement {
    /// Infinite, camera-centred. Upstream `CubeMapPanorama` /
    /// `SkyBox`: `pass: Pass.ENVIRONMENT` (`CubeMapPanorama.js` L105-106, comment
    /// "render before everything else"), `depthTest: { enabled: false }`,
    /// `depthMask: false`. Placement carries no depth, so the GPU side writes the
    /// reversed-Z far plane instead.
    Skybox = 0,
    /// Finite sphere of [`EquirectangularPanorama::radius`] metres placed by
    /// [`EquirectangularPanorama::transform`]. Upstream renders this as an ordinary
    /// opaque `Primitive` (`EquirectangularPanorama.js` L123-138,
    /// `translucent: false`) so it depth-tests and depth-writes normally, and the
    /// camera can be inside it — the street-view case.
    Bubble = 1,
}

impl PanoramaPlacement {
    /// Wire value written into `PanoramaUniforms::mode`.
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self as u32
    }
}

/// How the panorama image is laid out in its texture.
///
/// Discriminants are the wire format for `PanoramaUniforms::source`; see
/// [`PanoramaPlacement`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum PanoramaSource {
    /// Six square faces addressed by direction: `[+X, -X, +Y, -Y, +Z, -Z]`
    /// (upstream `SkyBox.js` `createEarthSkyBox` uses `px/mx/py/my/pz/mz`).
    CubeMap = 0,
    /// One 2:1 image, longitude on x and latitude on y. Upstream comment at
    /// `EquirectangularPanorama.js` L116: "2:1 360 degrees equirectangular image path".
    Equirectangular = 1,
}

impl PanoramaSource {
    /// Wire value written into `PanoramaUniforms::source`.
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self as u32
    }
}

/// An equirectangular panorama rendered on a sphere.
///
/// Maps to CesiumJS `Scene/EquirectangularPanorama.js`.
#[derive(Debug, Clone, PartialEq)]
pub struct EquirectangularPanorama {
    /// 4x4 transformation matrix defining position and orientation.
    pub transform: DMat4,
    /// Image URL or resource identifier.
    pub image: String,
    /// Radius of the panorama sphere in meters.
    pub radius: f64,
    /// Number of times to repeat the texture horizontally.
    pub repeat_horizontal: f64,
    /// Number of times to repeat the texture vertically.
    pub repeat_vertical: f64,
    /// Credit/attribution string.
    pub credit: Option<String>,
    /// Whether the panorama is visible.
    pub show: bool,
}

impl Default for EquirectangularPanorama {
    fn default() -> Self {
        Self {
            transform: DMat4::IDENTITY,
            image: String::new(),
            radius: DEFAULT_PANORAMA_RADIUS,
            repeat_horizontal: 1.0,
            repeat_vertical: 1.0,
            credit: None,
            show: true,
        }
    }
}

impl EquirectangularPanorama {
    /// Create a new equirectangular panorama with an image.
    pub fn new(image: impl Into<String>) -> Self {
        Self {
            image: image.into(),
            ..Default::default()
        }
    }

    /// Create with transform and image.
    pub fn with_transform(transform: DMat4, image: impl Into<String>) -> Self {
        Self {
            transform,
            image: image.into(),
            ..Default::default()
        }
    }

    /// Set the radius.
    pub fn set_radius(&mut self, radius: f64) -> &mut Self {
        self.radius = radius;
        self
    }

    /// Set horizontal repeat.
    pub fn set_repeat_horizontal(&mut self, repeat: f64) -> &mut Self {
        self.repeat_horizontal = repeat;
        self
    }

    /// Set vertical repeat.
    pub fn set_repeat_vertical(&mut self, repeat: f64) -> &mut Self {
        self.repeat_vertical = repeat;
        self
    }

    /// Set the credit.
    pub fn set_credit(&mut self, credit: impl Into<String>) -> &mut Self {
        self.credit = Some(credit.into());
        self
    }

    /// Compute the texture coordinate for a given direction.
    ///
    /// Direction should be a unit vector in local space.
    /// Returns (u, v) in [0, 1] range (before repeat).
    pub fn direction_to_uv(&self, direction: glam::DVec3) -> [f64; 2] {
        // Totality guards mirroring `ray_sphere_entry` below and the GPU twin
        // (`panorama.wgsl` `direction_to_equirect_uv`): `normalize()` of a zero /
        // non-finite direction yields NaN, and even after f64 normalization
        // `|z|` can be `1.0 + ε` (rounding), so a bare `asin` returns NaN. Clamp
        // the latitude argument (the GPU already does `asin(clamp(z, -1, 1))`) and
        // fall back to a neutral UV on a degenerate direction so NaN never escapes.
        let squared_length = direction.length_squared();
        if !squared_length.is_finite() || squared_length <= DEGENERATE_DIRECTION_SQUARED_EPSILON {
            return [0.5 * self.repeat_horizontal, 0.5 * self.repeat_vertical];
        }
        let dir = direction / squared_length.sqrt();
        // Longitude: atan2(y, x) -> [-π, π] -> [0, 1]
        let lon = dir.y.atan2(dir.x);
        let u = (lon + std::f64::consts::PI) / std::f64::consts::TAU;

        // Latitude: asin(clamp(z, -1, 1)) -> [-π/2, π/2] -> [0, 1]
        let lat = dir.z.clamp(-1.0, 1.0).asin();
        let v = (lat + std::f64::consts::FRAC_PI_2) / std::f64::consts::PI;

        [u * self.repeat_horizontal, v * self.repeat_vertical]
    }

    /// Compute a direction vector from texture coordinates.
    ///
    /// UV should be in [0, 1] range (after repeat division).
    pub fn uv_to_direction(&self, u: f64, v: f64) -> glam::DVec3 {
        let u_norm = u / self.repeat_horizontal;
        let v_norm = v / self.repeat_vertical;

        let lon = u_norm * std::f64::consts::TAU - std::f64::consts::PI;
        let lat = v_norm * std::f64::consts::PI - std::f64::consts::FRAC_PI_2;

        let cos_lat = lat.cos();
        glam::DVec3::new(
            cos_lat * lon.cos(),
            cos_lat * lon.sin(),
            lat.sin(),
        )
    }

    // ─── M6.3 additions (upstream-faithful orientation / projection semantics) ──

    /// Texture-layout axis of [`PanoramaPlacement`] × [`PanoramaSource`].
    #[inline]
    pub const fn source(&self) -> PanoramaSource {
        PanoramaSource::Equirectangular
    }

    /// Placement axis: a finite sphere placed by [`Self::transform`].
    #[inline]
    pub const fn placement(&self) -> PanoramaPlacement {
        PanoramaPlacement::Bubble
    }

    /// The texture-repeat vector handed to the sampler, exactly as upstream builds it.
    ///
    /// `EquirectangularPanorama.js` L117:
    /// ```text
    /// repeat: new Cartesian2(-this._repeatHorizontal, this._repeatVertical),
    /// // flip horizontally by default to match expected orientation of images
    /// // inside a sphere, but allow user to override
    /// ```
    ///
    /// ## DEVIATION — [`Self::direction_to_uv`] does **not** apply this flip
    /// The pre-existing [`Self::direction_to_uv`] multiplies by the *positive*
    /// `repeat_horizontal`, so its u is mirror-imaged relative to upstream. Its
    /// sign is deliberately **not** corrected here: [`Self::uv_to_direction`] is
    /// its exact inverse (`tests::test_equirectangular_uv_roundtrip` asserts a
    /// 1e-10 round trip), so negating one without the other would break the pair,
    /// and negating both would be a semantic change to two published methods.
    /// [`Self::sample_uv`] below is the upstream-faithful accessor, and the GPU
    /// path (`shaders/panorama.wgsl::direction_to_equirect_uv`) uses this repeat
    /// vector — so the rendered image matches CesiumJS, while the legacy pair stays
    /// bit-for-bit as it was.
    #[inline]
    pub fn texture_repeat(&self) -> DVec2 {
        DVec2::new(-self.repeat_horizontal, self.repeat_vertical)
    }

    /// The GPU texture coordinate for `direction`, upstream-faithful.
    ///
    /// `direction` is a **unit** vector in the panorama's local frame (i.e. after
    /// [`Self::orientation`]'s inverse has been applied — the same frame in which
    /// [`Self::direction_to_uv`] works).
    ///
    /// Equals [`Self::direction_to_uv`] with x negated, which is algebraically the
    /// same as scaling the base uv by [`Self::texture_repeat`]:
    /// ```text
    ///   base = (u * repeat_h, v * repeat_v)
    ///   want = (u * (-repeat_h), v * repeat_v) = (-base.x, base.y)
    /// ```
    /// and IEEE-754 multiplication is exactly sign-symmetric (`(-a) * b == -(a * b)`
    /// bit for bit), so the two forms cannot drift. Asserted by
    /// [`tests::sample_uv_is_the_horizontal_mirror_of_direction_to_uv`].
    ///
    /// The result is generally **outside** `[0, 1]` whenever the repeat is not `1`,
    /// or always for x because of the flip; run it through [`wrap_uv`] to get the
    /// coordinate a `GL_REPEAT` / `AddressMode::Repeat` sampler would use.
    pub fn sample_uv(&self, direction: DVec3) -> DVec2 {
        let base = self.direction_to_uv(direction);
        DVec2::new(-base[0], base[1])
    }

    /// Rotation-only part of [`Self::transform`].
    ///
    /// Upstream composes this from a position plus heading/pitch/roll via
    /// `Transforms.headingPitchRollToFixedFrame` (`EquirectangularPanorama.js`
    /// L46-61), so the upper-left 3×3 is the orientation and the fourth column is
    /// the anchor position — see [`Self::center`].
    #[inline]
    pub fn orientation(&self) -> DMat3 {
        DMat3::from_cols(
            self.transform.x_axis.truncate(),
            self.transform.y_axis.truncate(),
            self.transform.z_axis.truncate(),
        )
    }

    /// Anchor position of the panorama sphere, in meters, in the same frame as
    /// [`Self::transform`]. This is the bubble's centre.
    #[inline]
    pub fn center(&self) -> DVec3 {
        self.transform.w_axis.truncate()
    }

    /// [`Self::radius`] expressed in render units (still `f64`).
    ///
    /// At the upstream default this is `100_000 / 6_378_137 = 0.015678…` — a local
    /// bubble, not an infinite sky. See [`PANORAMA_METERS_PER_RENDER_UNIT`].
    #[inline]
    pub fn radius_render_units(&self) -> f64 {
        self.radius / PANORAMA_METERS_PER_RENDER_UNIT
    }

    /// [`Self::center`] expressed in render units (still `f64`).
    #[inline]
    pub fn center_render_units(&self) -> DVec3 {
        self.center() / PANORAMA_METERS_PER_RENDER_UNIT
    }

    /// Nearest positive distance at which `origin + t * direction` enters this
    /// panorama's sphere, in meters; `None` when the ray never reaches it.
    ///
    /// This is the f64 CPU reference for `ray_sphere_entry` in
    /// `shaders/panorama.wgsl`, which drives the `MODE_BUBBLE` branch and its
    /// `frag_depth`. The geometric (project-the-centre) form is used rather than the
    /// quadratic `a t² + b t + c` because `direction` is normalised inside, so
    /// `a == 1` and the whole `2a` denominator — and its divide-by-zero NaN path —
    /// disappears.
    ///
    /// Returns `None` for a degenerate/non-finite `direction`, a non-finite or
    /// negative `radius`, a miss, or a sphere entirely behind the origin. When the
    /// origin is *inside* the sphere the far intersection is returned, which is the
    /// street-view case: the camera sits at the bubble centre and sees the inside of
    /// the far wall.
    pub fn ray_sphere_entry(&self, origin: DVec3, direction: DVec3) -> Option<f64> {
        ray_sphere_entry(origin, direction, self.center(), self.radius)
    }
}

/// Nearest positive ray/sphere entry distance, or `None`.
///
/// Free-function form of [`EquirectangularPanorama::ray_sphere_entry`] for callers
/// whose sphere is not a panorama (tests, CPU/GPU cross-checks). Every rejection is
/// a *totality* guard, not a semantic choice: NaN and `±inf` never escape.
pub fn ray_sphere_entry(
    origin: DVec3,
    direction: DVec3,
    center: DVec3,
    radius: f64,
) -> Option<f64> {
    let squared_length = direction.length_squared();
    if !squared_length.is_finite() || squared_length <= DEGENERATE_DIRECTION_SQUARED_EPSILON {
        return None;
    }
    if !radius.is_finite() || radius < 0.0 {
        return None;
    }

    let unit = direction / squared_length.sqrt();
    let to_center = center - origin;
    let projection = to_center.dot(unit);
    let center_distance_squared = to_center.dot(to_center);
    let half_chord_squared =
        radius * radius - (center_distance_squared - projection * projection);

    if !half_chord_squared.is_finite() || half_chord_squared < 0.0 {
        return None;
    }
    let half_chord = half_chord_squared.sqrt();

    let entry = projection - half_chord;
    if entry > 0.0 {
        return Some(entry);
    }
    let exit_distance = projection + half_chord;
    if exit_distance > 0.0 {
        return Some(exit_distance);
    }
    None
}

/// Wrap a texture coordinate the way `GL_REPEAT` / `AddressMode::Repeat` does:
/// `x - floor(x)`, giving a result in `[0, 1)` for finite input.
///
/// Needed because [`EquirectangularPanorama::sample_uv`] deliberately returns
/// out-of-range coordinates (the upstream horizontal flip makes x negative for
/// every direction). Non-finite input yields `0.0` rather than NaN, so a garbage
/// uniform degrades to "sample the first texel" instead of poisoning a frame.
pub fn wrap_uv(uv: DVec2) -> DVec2 {
    DVec2::new(wrap_repeat(uv.x), wrap_repeat(uv.y))
}

/// Scalar half of [`wrap_uv`].
pub fn wrap_repeat(value: f64) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    let wrapped = value - value.floor();
    // `floor` of an exact integer returns itself, so `wrapped` is `0.0`; the only
    // way to reach `1.0` is a rounding artefact for values just below an integer.
    if wrapped >= 1.0 {
        return 0.0;
    }
    wrapped
}

/// A cube map panorama rendered from 6 face images.
///
/// Maps to CesiumJS `Scene/CubeMapPanorama.js`.
#[derive(Debug, Clone, PartialEq)]
pub struct CubeMapPanorama {
    /// 4x4 transformation matrix.
    pub transform: DMat4,
    /// Image URLs for the 6 faces: [+X, -X, +Y, -Y, +Z, -Z].
    pub faces: [String; 6],
    /// Radius of the panorama sphere in meters.
    pub radius: f64,
    /// Credit/attribution string.
    pub credit: Option<String>,
    /// Whether the panorama is visible.
    pub show: bool,
}

impl Default for CubeMapPanorama {
    fn default() -> Self {
        Self {
            transform: DMat4::IDENTITY,
            faces: [
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
            radius: DEFAULT_PANORAMA_RADIUS,
            credit: None,
            show: true,
        }
    }
}

impl CubeMapPanorama {
    /// Create a new cube map panorama with 6 face images.
    pub fn new(faces: [String; 6]) -> Self {
        Self {
            faces,
            ..Default::default()
        }
    }

    /// Check if all faces have images.
    pub fn is_complete(&self) -> bool {
        self.faces.iter().all(|f| !f.is_empty())
    }

    /// Determine which face a direction vector maps to.
    ///
    /// Returns face index (0-5) and (u, v) coordinates on that face.
    pub fn direction_to_face_uv(&self, direction: glam::DVec3) -> (usize, [f64; 2]) {
        let dir = direction.normalize();
        let ax = dir.x.abs();
        let ay = dir.y.abs();
        let az = dir.z.abs();

        if ax >= ay && ax >= az {
            if dir.x > 0.0 {
                // +X face
                let u = (-dir.z / ax + 1.0) * 0.5;
                let v = (-dir.y / ax + 1.0) * 0.5;
                (0, [u, v])
            } else {
                // -X face
                let u = (dir.z / ax + 1.0) * 0.5;
                let v = (-dir.y / ax + 1.0) * 0.5;
                (1, [u, v])
            }
        } else if ay >= ax && ay >= az {
            if dir.y > 0.0 {
                // +Y face
                let u = (dir.x / ay + 1.0) * 0.5;
                let v = (dir.z / ay + 1.0) * 0.5;
                (2, [u, v])
            } else {
                // -Y face
                let u = (dir.x / ay + 1.0) * 0.5;
                let v = (-dir.z / ay + 1.0) * 0.5;
                (3, [u, v])
            }
        } else if dir.z > 0.0 {
            // +Z face
            let u = (dir.x / az + 1.0) * 0.5;
            let v = (dir.y / az + 1.0) * 0.5;
            (4, [u, v])
        } else {
            // -Z face
            let u = (dir.x / az + 1.0) * 0.5;
            let v = (-dir.y / az + 1.0) * 0.5;
            (5, [u, v])
        }
    }

    // ─── M6.3 additions (upstream-faithful orientation / projection semantics) ──

    /// Texture-layout axis of [`PanoramaPlacement`] × [`PanoramaSource`].
    #[inline]
    pub const fn source(&self) -> PanoramaSource {
        PanoramaSource::CubeMap
    }

    /// Placement axis: infinite and camera-centred.
    #[inline]
    pub const fn placement(&self) -> PanoramaPlacement {
        PanoramaPlacement::Skybox
    }

    /// Upstream face order, as `[+X, -X, +Y, -Y, +Z, -Z]`.
    ///
    /// Matches [`Self::faces`] and the `px/mx/py/my/pz/mz` suffixes of
    /// `SkyBox.js::getDefaultSkyBoxUrl` / `createEarthSkyBox`.
    pub const FACE_NAMES: [&'static str; 6] = ["+X", "-X", "+Y", "-Y", "+Z", "-Z"];

    /// The upstream `u_cubeMapPanoramaTransform` value: a **`Matrix3`**.
    ///
    /// `CubeMapPanorama.js` L143-149 stores a `Matrix3` and
    /// `CubeMapPanoramaVS.glsl` L1 declares `uniform mat3
    /// u_cubeMapPanoramaTransform`. A cube-map skybox is always centred on the
    /// camera, so it has orientation but no position — see the module-level
    /// DEVIATION note for why this struct still carries a `DMat4`.
    #[inline]
    pub fn orientation(&self) -> DMat3 {
        DMat3::from_cols(
            self.transform.x_axis.truncate(),
            self.transform.y_axis.truncate(),
            self.transform.z_axis.truncate(),
        )
    }

    /// [`Self::radius`] expressed in render units (still `f64`).
    #[inline]
    pub fn radius_render_units(&self) -> f64 {
        self.radius / PANORAMA_METERS_PER_RENDER_UNIT
    }

    /// Cube face addressing exactly as the OpenGL / WebGPU cube-map specification
    /// defines it, and therefore exactly as a wgpu `texture_cube` samples it.
    ///
    /// Returns the same face index as [`Self::direction_to_face_uv`] — the face
    /// **selection** rule is identical — but corrects the `(s, t)` of the two faces
    /// where the legacy method diverges from the spec:
    ///
    /// | face | major axis `ma` | `sc` | `tc` | legacy | spec |
    /// |------|-----------------|------|------|--------|------|
    /// | `+X` (0) | `+x` | `-z` | `-y` | same | same |
    /// | `-X` (1) | `-x` | `+z` | `-y` | same | same |
    /// | `+Y` (2) | `+y` | `+x` | `+z` | same | same |
    /// | `-Y` (3) | `-y` | `+x` | `-z` | same | same |
    /// | `+Z` (4) | `+z` | `+x` | **`-y`** | `tc = +y` ✗ | `tc = -y` ✓ |
    /// | `-Z` (5) | `-z` | **`-x`** | `-y` | `sc = +x` ✗ | `sc = -x` ✓ |
    ///
    /// with `s = (sc / |ma| + 1) / 2` and `t = (tc / |ma| + 1) / 2`.
    ///
    /// ## DEVIATION — [`Self::direction_to_face_uv`] is left as-is
    /// The legacy method is **not** corrected in place. Its published behaviour is
    /// covered by `tests::test_cubemap_direction_to_face` and by downstream callers;
    /// silently flipping two faces would change results for anyone already relying
    /// on them. This spec-faithful sibling is what the GPU path is validated
    /// against, and the divergence between the two is pinned by
    /// [`tests::cube_face_uv_legacy_and_spec_diverge_only_on_the_z_faces`].
    pub fn direction_to_face_uv_spec(&self, direction: DVec3) -> (usize, [f64; 2]) {
        let dir = direction.normalize();
        let ax = dir.x.abs();
        let ay = dir.y.abs();
        let az = dir.z.abs();

        // Face selection is copied verbatim from `direction_to_face_uv` so the two
        // methods can never disagree about *which* face, only about its (s, t).
        if ax >= ay && ax >= az {
            if dir.x > 0.0 {
                (0, face_uv(-dir.z, -dir.y, ax))
            } else {
                (1, face_uv(dir.z, -dir.y, ax))
            }
        } else if ay >= ax && ay >= az {
            if dir.y > 0.0 {
                (2, face_uv(dir.x, dir.z, ay))
            } else {
                (3, face_uv(dir.x, -dir.z, ay))
            }
        } else if dir.z > 0.0 {
            (4, face_uv(dir.x, -dir.y, az))
        } else {
            (5, face_uv(-dir.x, -dir.y, az))
        }
    }

    /// Inverse of [`Self::direction_to_face_uv_spec`]: rebuild the unit direction
    /// from a face index and its `(s, t)`.
    ///
    /// `face` is taken modulo 6 so an out-of-range uniform degrades instead of
    /// panicking. Round-tripped over a deterministic direction set by
    /// [`tests::cube_face_uv_spec_round_trips_in_both_directions`].
    pub fn face_uv_to_direction_spec(&self, face: usize, uv: [f64; 2]) -> DVec3 {
        // s, t in [0, 1] -> sc/|ma|, tc/|ma| in [-1, 1]
        let s = uv[0] * 2.0 - 1.0;
        let t = uv[1] * 2.0 - 1.0;
        let raw = match face % 6 {
            0 => DVec3::new(1.0, -t, -s),
            1 => DVec3::new(-1.0, -t, s),
            2 => DVec3::new(s, 1.0, t),
            3 => DVec3::new(s, -1.0, -t),
            4 => DVec3::new(s, -t, 1.0),
            _ => DVec3::new(-s, -t, -1.0),
        };
        raw.normalize()
    }
}

/// One cube-face `(s, t)` from the spec's `(sc, tc, ma)` triple:
/// `s = (sc / |ma| + 1) / 2`, `t = (tc / |ma| + 1) / 2`.
///
/// `ma` is passed as an already-absolute major component, so it is never zero for a
/// normalised direction; a zero still yields `0.5` rather than NaN because
/// `0.0 / 0.0` is guarded by the caller's face selection.
#[inline]
fn face_uv(sc: f64, tc: f64, ma: f64) -> [f64; 2] {
    if ma <= 0.0 {
        return [0.5, 0.5];
    }
    [(sc / ma + 1.0) * 0.5, (tc / ma + 1.0) * 0.5]
}

// ─── CPU reference for the upstream skybox vertex shader ─────────────────────

/// Result of [`skybox_vertex_transform`]: one transformed skybox vertex.
pub struct SkyBoxVertexOutput {
    /// `czm_projection * vec4(p, 1.0)` — the clip-space position to write.
    pub clip_position: DVec4,
    /// `position.xyz` — the **untransformed** box coordinate, used directly as the
    /// cube-map sampling direction (`v_texCoord = position.xyz`).
    pub texture_coordinate: DVec3,
}

/// f64 CPU reference for `Shaders/CubeMapPanoramaVS.glsl` L8-10 (and, with
/// `panorama_orientation` replaced by `czm_temeToPseudoFixed`, for
/// `Shaders/SkyBoxVS.glsl` L7-9):
/// ```glsl
/// vec3 p = czm_viewRotation * (u_cubeMapPanoramaTransform * (czm_entireFrustum.y * position));
/// gl_Position = czm_projection * vec4(p, 1.0);
/// v_texCoord = position.xyz;
/// ```
///
/// Types are pinned from `Renderer/AutomaticUniforms.js`:
/// * `czm_viewRotation` is a **`mat3`** (L329 `uniform mat3 czm_viewRotation;`,
///   `datatype: WebGLConstants.FLOAT_MAT3` at L341) — the rotation-only part of the
///   view matrix, which is what makes the skybox follow the camera without
///   translating with it.
/// * `czm_entireFrustum` is a **`vec2`** `(near, far)` (L1064 `uniform vec2
///   czm_entireFrustum;`), so `.y` is the far-plane distance. Scaling the unit box
///   by it pushes the skybox onto the far plane.
///
/// The multiplication order matters and is preserved exactly: scale, then orient,
/// then view-rotate. Because `f64` matrix–vector multiplication is not associative
/// under rounding, re-associating these three steps would produce a different
/// last-bit result — the same class of defect as the M5 Ultra Review finding M2.
///
/// ## Why this is a *reference* and not the GPU path
/// `shaders/panorama.wgsl` draws a fullscreen triangle and reconstructs the ray in
/// the fragment stage instead of rasterising a far-plane-scaled box, because Bevy
/// uses an infinite-reverse projection whose far plane is at infinity (see
/// DEVIATION 1 in that file's header, and `bevy_core_pipeline-0.15.3/src/skybox/skybox.wgsl`
/// L19-46 which does the same). This function exists so a test can prove the two
/// agree on the **sampling direction** — the only part of the upstream vertex stage
/// that survives into the fragment path, as `v_texCoord`.
pub fn skybox_vertex_transform(
    view_rotation: DMat3,
    panorama_orientation: DMat3,
    projection: DMat4,
    far_plane_distance: f64,
    box_position: DVec3,
) -> SkyBoxVertexOutput {
    // czm_entireFrustum.y * position
    let scaled = box_position * far_plane_distance;
    // u_cubeMapPanoramaTransform * (...)
    let oriented = panorama_orientation * scaled;
    // czm_viewRotation * (...)
    let eye = view_rotation * oriented;
    // czm_projection * vec4(p, 1.0)
    let clip_position = projection * DVec4::new(eye.x, eye.y, eye.z, 1.0);

    SkyBoxVertexOutput {
        clip_position,
        // v_texCoord = position.xyz — the RAW box coordinate, before any transform.
        texture_coordinate: box_position,
    }
}

/// The eight corners of the upstream skybox box.
///
/// `CubeMapPanorama.js` L189-192 builds a `2.0 × 2.0 × 2.0` `BoxGeometry` centred
/// on the origin, so every corner coordinate is `±SKYBOX_BOX_HALF_EXTENT`. Order is
/// `-x` fastest, then `-y`, then `-z`.
pub fn skybox_box_vertices() -> [DVec3; 8] {
    let h = SKYBOX_BOX_HALF_EXTENT;
    let mut out = [DVec3::ZERO; 8];
    for (index, slot) in out.iter_mut().enumerate() {
        let x = if index & 1 == 0 { -h } else { h };
        let y = if index & 2 == 0 { -h } else { h };
        let z = if index & 4 == 0 { -h } else { h };
        *slot = DVec3::new(x, y, z);
    }
    out
}

/// Panorama provider trait for loading panorama data.
pub trait PanoramaProvider {
    /// Get the panorama type name.
    fn provider_type(&self) -> &str;

    /// Check if the provider is ready.
    fn is_ready(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{DVec3, Vec4Swizzles};

    #[test]
    fn test_equirectangular_default() {
        let pano = EquirectangularPanorama::default();
        assert_eq!(pano.transform, DMat4::IDENTITY);
        assert_eq!(pano.radius, DEFAULT_PANORAMA_RADIUS);
        assert_eq!(pano.repeat_horizontal, 1.0);
        assert_eq!(pano.repeat_vertical, 1.0);
        assert!(pano.show);
        assert!(pano.credit.is_none());
    }

    #[test]
    fn test_equirectangular_new() {
        let pano = EquirectangularPanorama::new("panorama.jpg");
        assert_eq!(pano.image, "panorama.jpg");
    }

    #[test]
    fn test_equirectangular_with_transform() {
        let transform = DMat4::from_translation(DVec3::new(100.0, 200.0, 300.0));
        let pano = EquirectangularPanorama::with_transform(transform, "test.png");
        assert_eq!(pano.transform, transform);
        assert_eq!(pano.image, "test.png");
    }

    #[test]
    fn test_equirectangular_uv_roundtrip() {
        let pano = EquirectangularPanorama::new("test.jpg");

        // Test forward direction (lon=0, lat=0)
        let dir = DVec3::new(1.0, 0.0, 0.0);
        let uv = pano.direction_to_uv(dir);
        assert!((uv[0] - 0.5).abs() < 1e-10); // u = 0.5 at lon=0
        assert!((uv[1] - 0.5).abs() < 1e-10); // v = 0.5 at lat=0

        // Roundtrip
        let dir_back = pano.uv_to_direction(uv[0], uv[1]);
        assert!((dir_back - dir).length() < 1e-10);
    }

    #[test]
    fn test_direction_to_uv_clamps_and_guards_degenerate() {
        // FIX-PANO-UVCLAMP: `|z| == 1.0 + ε` after normalize must not yield NaN
        // (matches the GPU `asin(clamp(z, -1, 1))`); and a degenerate direction
        // falls back to a neutral finite UV instead of NaN.
        let pano = EquirectangularPanorama::new("test.jpg");

        let pole = pano.direction_to_uv(DVec3::new(0.0, 0.0, 1.0 + 1e-16));
        assert!(pole[0].is_finite() && pole[1].is_finite(), "pole uv must be finite");
        assert!((0.0..=1.0).contains(&pole[1]), "v in [0,1], got {}", pole[1]);

        let zero = pano.direction_to_uv(DVec3::ZERO);
        assert!(zero[0].is_finite() && zero[1].is_finite(), "degenerate uv must be finite");

        let nan = pano.direction_to_uv(DVec3::new(f64::NAN, 0.0, 0.0));
        assert!(nan[0].is_finite() && nan[1].is_finite(), "NaN input must not escape");
    }

    #[test]
    fn test_equirectangular_uv_poles() {
        let pano = EquirectangularPanorama::new("test.jpg");

        // North pole (lat = π/2)
        let north = DVec3::new(0.0, 0.0, 1.0);
        let uv_north = pano.direction_to_uv(north);
        assert!((uv_north[1] - 1.0).abs() < 1e-10);

        // South pole (lat = -π/2)
        let south = DVec3::new(0.0, 0.0, -1.0);
        let uv_south = pano.direction_to_uv(south);
        assert!(uv_south[1].abs() < 1e-10);
    }

    #[test]
    fn test_equirectangular_repeat() {
        let mut pano = EquirectangularPanorama::new("test.jpg");
        pano.set_repeat_horizontal(2.0);
        pano.set_repeat_vertical(3.0);

        let dir = DVec3::new(1.0, 0.0, 0.0);
        let uv = pano.direction_to_uv(dir);
        assert!((uv[0] - 1.0).abs() < 1e-10); // 0.5 * 2
        assert!((uv[1] - 1.5).abs() < 1e-10); // 0.5 * 3
    }

    #[test]
    fn test_cubemap_default() {
        let pano = CubeMapPanorama::default();
        assert!(!pano.is_complete());
        assert_eq!(pano.radius, DEFAULT_PANORAMA_RADIUS);
    }

    #[test]
    fn test_cubemap_new() {
        let faces = [
            "px.jpg".to_string(),
            "nx.jpg".to_string(),
            "py.jpg".to_string(),
            "ny.jpg".to_string(),
            "pz.jpg".to_string(),
            "nz.jpg".to_string(),
        ];
        let pano = CubeMapPanorama::new(faces);
        assert!(pano.is_complete());
    }

    #[test]
    fn test_cubemap_direction_to_face() {
        let pano = CubeMapPanorama::default();

        // +X direction -> face 0
        let (face, uv) = pano.direction_to_face_uv(DVec3::new(1.0, 0.0, 0.0));
        assert_eq!(face, 0);
        assert!((uv[0] - 0.5).abs() < 1e-10);
        assert!((uv[1] - 0.5).abs() < 1e-10);

        // -X direction -> face 1
        let (face, _) = pano.direction_to_face_uv(DVec3::new(-1.0, 0.0, 0.0));
        assert_eq!(face, 1);

        // +Y direction -> face 2
        let (face, _) = pano.direction_to_face_uv(DVec3::new(0.0, 1.0, 0.0));
        assert_eq!(face, 2);

        // -Y direction -> face 3
        let (face, _) = pano.direction_to_face_uv(DVec3::new(0.0, -1.0, 0.0));
        assert_eq!(face, 3);

        // +Z direction -> face 4
        let (face, _) = pano.direction_to_face_uv(DVec3::new(0.0, 0.0, 1.0));
        assert_eq!(face, 4);

        // -Z direction -> face 5
        let (face, _) = pano.direction_to_face_uv(DVec3::new(0.0, 0.0, -1.0));
        assert_eq!(face, 5);
    }

    #[test]
    fn test_equirectangular_builder() {
        let mut pano = EquirectangularPanorama::new("test.jpg");
        pano.set_radius(50000.0)
            .set_repeat_horizontal(2.0)
            .set_repeat_vertical(1.5)
            .set_credit("Test Credit");

        assert_eq!(pano.radius, 50000.0);
        assert_eq!(pano.repeat_horizontal, 2.0);
        assert_eq!(pano.repeat_vertical, 1.5);
        assert_eq!(pano.credit, Some("Test Credit".to_string()));
    }

    // ─── M6.3 additions ─────────────────────────────────────────────

    /// `EquirectangularPanorama.js` L117 hands the sampler
    /// `Cartesian2(-repeatHorizontal, repeatVertical)`; the comment on that line is
    /// "flip horizontally by default to match expected orientation of images inside
    /// a sphere, but allow user to override".
    #[test]
    fn texture_repeat_carries_the_upstream_horizontal_flip() {
        let pano = EquirectangularPanorama::default();
        assert_eq!(pano.texture_repeat(), DVec2::new(-1.0, 1.0));

        let mut repeated = EquirectangularPanorama::new("test.jpg");
        repeated.set_repeat_horizontal(2.0).set_repeat_vertical(0.5);
        assert_eq!(repeated.texture_repeat(), DVec2::new(-2.0, 0.5));

        // The flip is unconditional: it survives repeat == 0 and negative repeats,
        // which upstream also passes straight through to the sampler.
        let mut degenerate = EquirectangularPanorama::new("test.jpg");
        degenerate.set_repeat_horizontal(0.0).set_repeat_vertical(-3.0);
        assert_eq!(degenerate.texture_repeat(), DVec2::new(-0.0, -3.0));
    }

    /// [`EquirectangularPanorama::sample_uv`] is the upstream-faithful coordinate;
    /// the legacy [`EquirectangularPanorama::direction_to_uv`] keeps its positive
    /// horizontal repeat so its inverse [`EquirectangularPanorama::uv_to_direction`]
    /// stays a true round trip.
    #[test]
    fn sample_uv_is_the_horizontal_mirror_of_direction_to_uv() {
        let mut pano = EquirectangularPanorama::new("test.jpg");
        pano.set_repeat_horizontal(2.0).set_repeat_vertical(3.0);

        for dir in [
            DVec3::X,
            DVec3::Y,
            DVec3::Z,
            -DVec3::X,
            -DVec3::Y,
            -DVec3::Z,
            DVec3::new(1.0, 2.0, 3.0).normalize(),
            DVec3::new(-3.0, 1.0, -2.0).normalize(),
        ] {
            let base = pano.direction_to_uv(dir);
            let sample = pano.sample_uv(dir);

            // Bit-exact negation: IEEE-754 multiplication is sign-symmetric, so
            // `u * (-repeat)` and `-(u * repeat)` cannot drift apart.
            assert_eq!(sample.x.to_bits(), (-base[0]).to_bits(), "dir {dir:?}");
            assert_eq!(sample.y.to_bits(), base[1].to_bits(), "dir {dir:?}");

            // And that equals scaling the base uv by `texture_repeat()` elementwise,
            // which is literally what the GLSL/WGSL sampler does.
            let repeat = pano.texture_repeat();
            let u = base[0] / pano.repeat_horizontal;
            let v = base[1] / pano.repeat_vertical;
            assert!((sample.x - u * repeat.x).abs() < 1e-12, "dir {dir:?}");
            assert!((sample.y - v * repeat.y).abs() < 1e-12, "dir {dir:?}");
        }

        // Concrete upstream check: at lon = +pi/2 the base u is 0.75, so the flipped
        // sample u is -1.5 for repeat_horizontal = 2.
        let east = pano.sample_uv(DVec3::Y);
        assert!((east.x - (-0.75 * 2.0)).abs() < 1e-12, "{}", east.x);
        assert!((east.y - (0.5 * 3.0)).abs() < 1e-12, "{}", east.y);
    }

    /// `sample_uv` deliberately returns out-of-range coordinates; [`wrap_uv`] is the
    /// `GL_REPEAT` / `AddressMode::Repeat` half of the contract.
    #[test]
    fn wrap_uv_reproduces_gl_repeat_semantics() {
        assert_eq!(wrap_repeat(0.25), 0.25);
        assert_eq!(wrap_repeat(0.0), 0.0);
        assert_eq!(wrap_repeat(1.0), 0.0);
        assert_eq!(wrap_repeat(2.0), 0.0);
        assert_eq!(wrap_repeat(-0.25), 0.75);
        assert_eq!(wrap_repeat(1.5), 0.5);
        assert_eq!(wrap_repeat(-1.5), 0.5);
        assert_eq!(wrap_repeat(-2.0), 0.0);

        // Non-finite input degrades to "first texel" instead of propagating NaN into
        // every texture coordinate of the frame.
        assert_eq!(wrap_repeat(f64::NAN), 0.0);
        assert_eq!(wrap_repeat(f64::INFINITY), 0.0);
        assert_eq!(wrap_repeat(f64::NEG_INFINITY), 0.0);

        let wrapped = wrap_uv(DVec2::new(-0.25, 1.25));
        assert!((wrapped.x - 0.75).abs() < 1e-15, "{}", wrapped.x);
        assert!((wrapped.y - 0.25).abs() < 1e-15, "{}", wrapped.y);
        assert!(wrapped.x >= 0.0 && wrapped.x < 1.0);
        assert!(wrapped.y >= 0.0 && wrapped.y < 1.0);
    }

    /// The enum discriminants are the wire format for `PanoramaUniforms::mode` and
    /// `::source`; `shaders/panorama.wgsl` compares against `MODE_SKYBOX = 0u`,
    /// `MODE_BUBBLE = 1u`, `SOURCE_CUBEMAP = 0u`, `SOURCE_EQUIRECTANGULAR = 1u`.
    /// The adapter-side test
    /// `panorama_wgsl_mode_and_source_literals_match_the_domain_discriminants`
    /// closes the loop against the shader source text.
    #[test]
    fn placement_and_source_discriminants_match_the_shader() {
        assert_eq!(PanoramaPlacement::Skybox.as_u32(), 0);
        assert_eq!(PanoramaPlacement::Bubble.as_u32(), 1);
        assert_eq!(PanoramaSource::CubeMap.as_u32(), 0);
        assert_eq!(PanoramaSource::Equirectangular.as_u32(), 1);

        // Upstream's two pairings.
        let cube = CubeMapPanorama::default();
        assert_eq!(cube.placement(), PanoramaPlacement::Skybox);
        assert_eq!(cube.source(), PanoramaSource::CubeMap);

        let equirect = EquirectangularPanorama::default();
        assert_eq!(equirect.placement(), PanoramaPlacement::Bubble);
        assert_eq!(equirect.source(), PanoramaSource::Equirectangular);
    }

    /// The metres -> render-unit conversion changes **scale only**, never precision:
    /// the accessors still return `f64`.
    #[test]
    fn radius_render_units_converts_scale_without_narrowing_precision() {
        assert_eq!(PANORAMA_METERS_PER_RENDER_UNIT, 6_378_137.0);

        let pano = EquirectangularPanorama::default();
        let render_units = pano.radius_render_units();
        let expected = DEFAULT_PANORAMA_RADIUS / PANORAMA_METERS_PER_RENDER_UNIT;
        assert_eq!(render_units.to_bits(), expected.to_bits());

        // 100 km is a *local bubble*: ~1.57 % of the globe's 1.0 render unit. This
        // is the whole reason `PanoramaPlacement` has two members.
        assert!(
            render_units > 0.0156 && render_units < 0.0157,
            "expected ~0.015678 render units, got {render_units}"
        );
        assert!(
            render_units < 1.0,
            "the default panorama must be far smaller than the globe radius"
        );

        // Centre conversion is exact for whole-render-unit translations.
        let anchored = EquirectangularPanorama::with_transform(
            DMat4::from_translation(DVec3::new(6_378_137.0, 0.0, -6_378_137.0)),
            "test.jpg",
        );
        assert_eq!(
            anchored.center(),
            DVec3::new(6_378_137.0, 0.0, -6_378_137.0)
        );
        assert_eq!(anchored.center_render_units(), DVec3::new(1.0, 0.0, -1.0));

        assert_eq!(
            CubeMapPanorama::default().radius_render_units().to_bits(),
            expected.to_bits()
        );
    }

    /// `orientation()` is the accessor that reproduces upstream's `Matrix3`; the
    /// `DMat4` field it is derived from is untouched (see the module DEVIATION note).
    #[test]
    fn cube_orientation_drops_the_translation_the_upstream_matrix3_never_had() {
        let translated = CubeMapPanorama {
            transform: DMat4::from_translation(DVec3::new(5.0, 6.0, 7.0)),
            ..Default::default()
        };
        assert_eq!(translated.orientation(), DMat3::IDENTITY);

        let angle = std::f64::consts::FRAC_PI_2;
        let transform =
            DMat4::from_rotation_z(angle) * DMat4::from_translation(DVec3::new(5.0, 6.0, 7.0));
        let oriented = CubeMapPanorama {
            transform,
            ..Default::default()
        };
        let expected = DMat3::from_rotation_z(angle);
        for column in 0..3 {
            assert!(
                (oriented.orientation().col(column) - expected.col(column)).length() < 1e-15,
                "column {column}"
            );
        }

        // The equirectangular variant splits the same Matrix4 the other way: the
        // fourth column *is* meaningful there, because that panorama is a bubble
        // anchored in the world.
        let equirect = EquirectangularPanorama::with_transform(transform, "test.jpg");
        assert_eq!(equirect.center(), transform.w_axis.truncate());
        assert!((equirect.center() - DVec3::new(5.0, 6.0, 7.0)).length() > 1.0);
        for column in 0..3 {
            assert!(
                (equirect.orientation().col(column) - expected.col(column)).length() < 1e-15,
                "column {column}"
            );
        }
    }

    /// `direction_to_face_uv` (legacy) and `direction_to_face_uv_spec` must agree on
    /// the face **everywhere**, and on `(s, t)` everywhere except the two Z faces.
    #[test]
    fn cube_face_uv_legacy_and_spec_diverge_only_on_the_z_faces() {
        let pano = CubeMapPanorama::default();

        let mut state = 0x9E37_79B9_7F4A_7C15u64;
        let mut xorshift = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let component = |bits: u64, shift: u32| {
            (((bits >> shift) & 0xFFFF) as f64 / 32767.5) - 1.0
        };

        let mut z_face_divergences = 0usize;
        let mut swept = 0usize;
        for _ in 0..20_000 {
            let bits = xorshift();
            let raw = DVec3::new(
                component(bits, 0),
                component(bits, 16),
                component(bits, 32),
            );
            if raw.length_squared() < 1.0e-12 {
                continue; // a degenerate sample proves nothing about face addressing
            }
            let dir = raw.normalize();
            swept += 1;

            let (legacy_face, legacy_uv) = pano.direction_to_face_uv(dir);
            let (spec_face, spec_uv) = pano.direction_to_face_uv_spec(dir);

            assert_eq!(
                legacy_face, spec_face,
                "face selection diverged for {dir:?}"
            );

            let diverges = (legacy_uv[0] - spec_uv[0]).abs() > 1.0e-12
                || (legacy_uv[1] - spec_uv[1]).abs() > 1.0e-12;
            if diverges {
                assert!(
                    legacy_face >= 4,
                    "only +Z/-Z may diverge, got face {legacy_face} ({}) for {dir:?}",
                    CubeMapPanorama::FACE_NAMES[legacy_face]
                );
                z_face_divergences += 1;
            }
        }

        assert!(swept > 19_000, "only {swept} usable samples out of 20000");
        assert!(
            z_face_divergences > 0,
            "the sweep must actually witness the divergence, or it proves nothing"
        );

        // The exact divergence, spelled out: +Z flips `t`, -Z flips `s`.
        let tilted = DVec3::new(0.2, 0.3, 0.9).normalize();
        let (plus_z, legacy_uv) = pano.direction_to_face_uv(tilted);
        let (_, spec_uv) = pano.direction_to_face_uv_spec(tilted);
        assert_eq!(plus_z, 4);
        assert_eq!(legacy_uv[0].to_bits(), spec_uv[0].to_bits());
        assert!((legacy_uv[1] + spec_uv[1] - 1.0).abs() < 1.0e-15, "+Z: t is mirrored");

        let behind = DVec3::new(0.2, 0.3, -0.9).normalize();
        let (minus_z, legacy_uv) = pano.direction_to_face_uv(behind);
        let (_, spec_uv) = pano.direction_to_face_uv_spec(behind);
        assert_eq!(minus_z, 5);
        assert!((legacy_uv[0] + spec_uv[0] - 1.0).abs() < 1.0e-15, "-Z: s is mirrored");
        assert_eq!(legacy_uv[1].to_bits(), spec_uv[1].to_bits());
    }

    /// `direction_to_face_uv_spec` and `face_uv_to_direction_spec` are exact
    /// inverses over all six faces.
    #[test]
    fn cube_face_uv_spec_round_trips_in_both_directions() {
        let pano = CubeMapPanorama::default();

        for dir in [
            DVec3::X,
            DVec3::Y,
            DVec3::Z,
            -DVec3::X,
            -DVec3::Y,
            -DVec3::Z,
            DVec3::new(1.0, 2.0, 3.0).normalize(),
            DVec3::new(-3.0, 1.0, -2.0).normalize(),
            DVec3::new(0.1, -0.9, 0.4).normalize(),
        ] {
            let (face, uv) = pano.direction_to_face_uv_spec(dir);
            assert!(uv[0] >= 0.0 && uv[0] <= 1.0, "s out of range: {uv:?}");
            assert!(uv[1] >= 0.0 && uv[1] <= 1.0, "t out of range: {uv:?}");
            let back = pano.face_uv_to_direction_spec(face, uv);
            assert!(
                (back - dir).length() < 1.0e-12,
                "{dir:?} -> face {face} {uv:?} -> {back:?}"
            );
        }

        // And the other way: every face's centre maps back to its own major axis.
        let majors = [
            DVec3::X,
            -DVec3::X,
            DVec3::Y,
            -DVec3::Y,
            DVec3::Z,
            -DVec3::Z,
        ];
        for (face, expected) in majors.iter().enumerate() {
            let back = pano.face_uv_to_direction_spec(face, [0.5, 0.5]);
            assert!(
                (back - *expected).length() < 1.0e-15,
                "face {face} ({}) centre -> {back:?}, expected {expected:?}",
                CubeMapPanorama::FACE_NAMES[face]
            );
        }

        // Out-of-range face indices wrap instead of panicking.
        assert_eq!(
            pano.face_uv_to_direction_spec(6, [0.5, 0.5]),
            pano.face_uv_to_direction_spec(0, [0.5, 0.5])
        );
        assert_eq!(CubeMapPanorama::FACE_NAMES.len(), 6);
    }

    /// The camera-at-the-centre street-view case: the ray leaves the bubble centre
    /// and meets the **far** wall at exactly `radius`.
    #[test]
    fn ray_sphere_entry_hits_the_far_wall_from_inside_the_bubble() {
        let pano = EquirectangularPanorama::default();
        assert_eq!(pano.center(), DVec3::ZERO);
        assert_eq!(pano.radius, DEFAULT_PANORAMA_RADIUS);

        let hit = pano
            .ray_sphere_entry(DVec3::ZERO, DVec3::X)
            .expect("a centred ray must hit the bubble");
        assert_eq!(hit, DEFAULT_PANORAMA_RADIUS);

        // The sampling direction is then the ray direction itself, which is what
        // makes `direction_to_uv`/`sample_uv` valid straight from the camera ray.
        let hit_point = DVec3::ZERO + DVec3::X * hit;
        assert!(
            ((hit_point - pano.center()).normalize() - DVec3::X).length() < 1.0e-12
        );

        // From outside: the NEAR wall wins, matching upstream's depth-tested opaque
        // sphere with `cull: { enabled: false }`.
        let near = pano
            .ray_sphere_entry(DVec3::new(-300_000.0, 0.0, 0.0), DVec3::X)
            .expect("must hit");
        assert!((near - 200_000.0).abs() < 1.0e-9, "{near}");

        // A tangent-free miss, and a sphere entirely behind the origin.
        assert!(pano
            .ray_sphere_entry(DVec3::new(-300_000.0, 0.0, 0.0), DVec3::Y)
            .is_none());
        assert!(pano
            .ray_sphere_entry(DVec3::new(300_000.0, 0.0, 0.0), DVec3::X)
            .is_none());
    }

    /// No input can make [`ray_sphere_entry`] return NaN, infinity or a
    /// non-positive distance.
    #[test]
    fn ray_sphere_entry_rejects_degenerate_input_instead_of_returning_nan() {
        let pano = EquirectangularPanorama::default();

        assert!(pano.ray_sphere_entry(DVec3::ZERO, DVec3::ZERO).is_none());
        assert!(pano
            .ray_sphere_entry(DVec3::ZERO, DVec3::new(f64::NAN, 0.0, 0.0))
            .is_none());
        assert!(pano
            .ray_sphere_entry(DVec3::ZERO, DVec3::new(f64::INFINITY, 0.0, 0.0))
            .is_none());
        assert!(pano
            .ray_sphere_entry(DVec3::new(f64::NAN, 0.0, 0.0), DVec3::X)
            .is_none());

        let mut bad = pano.clone();
        bad.radius = f64::NAN;
        assert!(bad.ray_sphere_entry(DVec3::ZERO, DVec3::X).is_none());
        bad.radius = f64::INFINITY;
        assert!(bad.ray_sphere_entry(DVec3::ZERO, DVec3::X).is_none());
        bad.radius = -1.0;
        assert!(bad.ray_sphere_entry(DVec3::ZERO, DVec3::X).is_none());

        // A short-but-representable direction is normalised, not rejected: 1e-8
        // squares to 1e-16, which is eight orders of magnitude above the
        // `1.0e-24` squared-length epsilon.
        let short = ray_sphere_entry(
            DVec3::ZERO,
            DVec3::new(1.0e-8, 0.0, 0.0),
            DVec3::ZERO,
            1.0,
        )
        .expect("1e-8 squares to 1e-16, well above the degenerate epsilon");
        assert!((short - 1.0).abs() < 1.0e-12, "{short}");

        // 1e-20 squares to 1e-40, which is *below* the epsilon, so it is rejected
        // too: the guard is on the squared length, not on the length.
        assert!(
            ray_sphere_entry(
                DVec3::ZERO,
                DVec3::new(1.0e-20, 0.0, 0.0),
                DVec3::ZERO,
                1.0
            )
            .is_none(),
            "a squared length below the epsilon must be rejected, not normalised"
        );

        // 1e-200 squares to 1e-400, which underflows to exactly 0.0 in f64: the
        // guard must catch that rather than dividing by it.
        assert!(
            ray_sphere_entry(
                DVec3::ZERO,
                DVec3::new(1.0e-200, 0.0, 0.0),
                DVec3::ZERO,
                1.0
            )
            .is_none(),
            "underflow to zero must be rejected, not normalised into NaN"
        );

        // Whatever comes back is always finite and strictly positive.
        for dir in [DVec3::X, DVec3::Y, DVec3::Z, -DVec3::X] {
            if let Some(t) = pano.ray_sphere_entry(DVec3::ZERO, dir) {
                assert!(t.is_finite() && t > 0.0, "{dir:?} -> {t}");
            }
        }
    }

    /// The intersection is invariant under the metres -> render-unit rescaling,
    /// because origin, centre and radius are all divided by the same constant. That
    /// invariance is what lets the f64 domain reference validate the f32 GPU result.
    #[test]
    fn ray_sphere_entry_is_scale_invariant_under_render_unit_conversion() {
        let pano = EquirectangularPanorama::with_transform(
            DMat4::from_translation(DVec3::new(0.0, 0.0, PANORAMA_METERS_PER_RENDER_UNIT)),
            "test.jpg",
        );

        let metres = pano
            .ray_sphere_entry(DVec3::ZERO, DVec3::Z)
            .expect("hits in metres");
        let render_units = ray_sphere_entry(
            DVec3::ZERO,
            DVec3::Z,
            pano.center_render_units(),
            pano.radius_render_units(),
        )
        .expect("hits in render units");

        assert!((metres - 6_278_137.0).abs() < 1.0e-6, "{metres}");
        let relative =
            (metres / PANORAMA_METERS_PER_RENDER_UNIT - render_units).abs() / metres.abs();
        assert!(
            relative < 1.0e-12,
            "{metres} m == {render_units} ru only up to {relative}"
        );
    }

    /// `CubeMapPanorama.js` L189-192 builds a `2.0 x 2.0 x 2.0` box centred on the
    /// origin, so all eight corners sit at `+-SKYBOX_BOX_HALF_EXTENT`.
    #[test]
    fn skybox_box_vertices_are_the_unit_cube_corners() {
        let vertices = skybox_box_vertices();
        assert_eq!(vertices.len(), 8);

        for vertex in vertices {
            for component in [vertex.x, vertex.y, vertex.z] {
                assert_eq!(component.abs(), SKYBOX_BOX_HALF_EXTENT);
            }
        }

        let min = vertices
            .iter()
            .fold(DVec3::splat(f64::MAX), |a, b| a.min(*b));
        let max = vertices
            .iter()
            .fold(DVec3::splat(f64::MIN), |a, b| a.max(*b));
        assert_eq!(min, DVec3::splat(-SKYBOX_BOX_HALF_EXTENT));
        assert_eq!(max, DVec3::splat(SKYBOX_BOX_HALF_EXTENT));

        for i in 0..8 {
            for j in (i + 1)..8 {
                assert_ne!(vertices[i], vertices[j], "corners {i} and {j} coincide");
            }
        }
    }

    /// f64 CPU reference for `CubeMapPanoramaVS.glsl` L8-10. Pins the
    /// scale-then-orient-then-view-rotate order and the fact that `v_texCoord` is
    /// the **raw** box coordinate.
    #[test]
    fn skybox_vertex_transform_scales_then_orients_then_view_rotates() {
        let far = 200.0_f64;
        let box_position = DVec3::new(1.0, -1.0, 1.0);

        // Identity everything: p = far * position, clip = (p, 1).
        let identity = skybox_vertex_transform(
            DMat3::IDENTITY,
            DMat3::IDENTITY,
            DMat4::IDENTITY,
            far,
            box_position,
        );
        assert!(
            (identity.clip_position.xyz() - box_position * far).length() < 1.0e-9,
            "{}",
            identity.clip_position
        );
        assert_eq!(identity.clip_position.w, 1.0);
        // `v_texCoord = position.xyz` — the RAW box coordinate, never the scaled one.
        assert_eq!(identity.texture_coordinate, box_position);

        // The multiplication order is observable: swapping the orientation and the
        // view rotation gives a different eye-space point.
        let orientation = DMat3::from_rotation_z(std::f64::consts::FRAC_PI_2);
        let view_rotation = DMat3::from_rotation_x(std::f64::consts::FRAC_PI_4);
        let reference = skybox_vertex_transform(
            view_rotation,
            orientation,
            DMat4::IDENTITY,
            far,
            box_position,
        );
        let eye = view_rotation * (orientation * (box_position * far));
        assert!(
            (reference.clip_position.xyz() - eye).length() < 1.0e-9,
            "{}",
            reference.clip_position
        );

        let swapped = skybox_vertex_transform(
            orientation,
            view_rotation,
            DMat4::IDENTITY,
            far,
            box_position,
        );
        assert!(
            (swapped.clip_position.xyz() - reference.clip_position.xyz()).length() > 1.0e-6,
            "the test vectors must actually distinguish the two orders"
        );

        // Scaling by `czm_entireFrustum.y` is a pure similarity: doubling the far
        // plane doubles the eye-space point.
        let unit_far = skybox_vertex_transform(
            DMat3::IDENTITY,
            DMat3::IDENTITY,
            DMat4::IDENTITY,
            1.0,
            box_position,
        );
        let scaled_far = skybox_vertex_transform(
            DMat3::IDENTITY,
            DMat3::IDENTITY,
            DMat4::IDENTITY,
            far,
            box_position,
        );
        assert!(
            (scaled_far.clip_position.xyz() - unit_far.clip_position.xyz() * far).length() < 1.0e-9
        );

        // All eight box corners land on the far plane at identity view/projection.
        for vertex in skybox_box_vertices() {
            let out = skybox_vertex_transform(
                DMat3::IDENTITY,
                DMat3::IDENTITY,
                DMat4::IDENTITY,
                far,
                vertex,
            );
            assert!(
                (out.clip_position.xyz().length() - far * 3.0_f64.sqrt()).abs() < 1.0e-9,
                "{vertex:?} -> {}",
                out.clip_position
            );
            assert_eq!(out.texture_coordinate, vertex);
        }
    }

    /// The upstream `v_texCoord` is the panorama-**local** direction, which is why
    /// `shaders/panorama.wgsl` multiplies the world ray by the world->local
    /// `uniforms.transform` before sampling instead of by the forward transform.
    #[test]
    fn skybox_texture_coordinate_is_the_panorama_local_direction() {
        let orientation = DMat3::from_rotation_z(std::f64::consts::FRAC_PI_2);
        let out =
            skybox_vertex_transform(DMat3::IDENTITY, orientation, DMat4::IDENTITY, 100.0, DVec3::X);
        assert_eq!(out.texture_coordinate, DVec3::X);

        // The same vertex in world space points along +Y...
        let world_direction = (orientation * DVec3::X).normalize();
        assert!((world_direction - DVec3::Y).length() < 1.0e-15, "{world_direction:?}");

        // ...and the inverse orientation maps it straight back to the raw box
        // coordinate, which is the cube-map sampling direction.
        let local = orientation.inverse() * world_direction;
        assert!(
            (local - out.texture_coordinate.normalize()).length() < 1.0e-15,
            "{local:?}"
        );

        // A cube map is camera-centred, so the orientation never carries a
        // translation: applying it to the zero vector is still zero.
        assert_eq!(orientation * DVec3::ZERO, DVec3::ZERO);
    }
}
