//! Cloud rendering system (CumulusCloud + CloudCollection).
//!
//! Maps to CesiumJS:
//! - `Scene/CumulusCloud.js`
//! - `Scene/CloudCollection.js`
//! - `Scene/CloudType.js`

// legacy CesiumJS-port style debt (deferred.md #18); revisit at M13 lint-cleanup 或本文件在其里程碑被重写时
#![allow(clippy::field_reassign_with_default)]
use glam::{DVec2, DVec3};

// ─── M6.6 additive constants (CloudCollection.js / CloudNoiseFS.glsl / CloudCollectionFS.glsl) ───
//
// Everything below this line is ADDITIVE (M6.6): the original 426-line data model
// (CumulusCloud / CloudCollection / CloudType) and its tests are untouched. The
// constants mirror the upstream truth source byte-for-byte where one exists; the
// physically-based scattering constants (HG phase `g`, Beer-Lambert extinction) are
// an ADDITIVE extension — upstream CesiumJS cumulus clouds use the Gardner (1985)
// sine-texture + Worley-FBM erosion model with a SINGLE ray/ellipsoid intersection,
// NOT a multi-step HG/Beer-Lambert volume march. Both paths are provided; see the
// module report + `docs/deviations.md#dev-032` (draft).

/// Edge length of the cubic noise volume, in voxels.
/// Mirrors `CloudCollection.js` `_textureSliceWidth = 128` (L118). 128³ = 2_097_152
/// voxels; at 3 Worley channels the CPU f64 reference is ~50 MB and the GPU RGBA8
/// upload is 8 MB (see the memory-budget note in the M6.6 report).
pub const NOISE_TEXTURE_DIMENSIONS: usize = 128;

/// Number of rows the upstream 2D noise atlas packs the 128³ volume into.
/// Mirrors `CloudCollection.js` `_noiseTextureRows = 4` (L119).
pub const NOISE_TEXTURE_ROWS: usize = 4;

/// Number of Worley channels stored per voxel (worley0/1/2 → RGB).
/// Mirrors `CloudNoiseFS.glsl` `main` (L88-91).
pub const NOISE_CHANNELS: usize = 3;

/// Henyey-Greenstein anisotropy for in-scattering (ADDITIVE, not upstream).
/// `g = 0.6` is the standard cumulus forward-scattering lobe.
pub const HG_PHASE_G: f64 = 0.6;

/// Beer-Lambert extinction coefficient (ADDITIVE, not upstream), per render unit.
pub const BEER_LAMBERT_EXTINCTION: f64 = 0.1;

/// Ellipsoid shrink factor applied to `maximumSize` before intersection.
/// Mirrors `CloudCollectionFS.glsl` `main` `ellipsoidScale = 0.82 * v_maximumSize` (L240).
pub const ELLIPSOID_SCALE_FACTOR: f64 = 0.82;

/// Minimum raymarch step count for the ADDITIVE volumetric path.
pub const RAYMARCH_STEPS_MIN: usize = 8;
/// Maximum raymarch step count for the ADDITIVE volumetric path.
pub const RAYMARCH_STEPS_MAX: usize = 16;
/// Default raymarch step count (mid-range, quality/cost balance).
pub const RAYMARCH_STEPS_DEFAULT: usize = 12;

// Gardner (1985) "Visual Simulation of Clouds" texture constants — mirror
// `CloudCollectionFS.glsl` L129-159.
/// Contrast of the Gardner texture pattern (`T0`, L129).
pub const GARDNER_T0: f64 = 0.6;
/// Normalisation coefficient (`k`, L130).
pub const GARDNER_K: f64 = 0.1;
/// Base octave coefficient (`C0`, L131).
pub const GARDNER_C0: f64 = 0.8;
/// Base X frequency (`FX0`, L132).
pub const GARDNER_FX0: f64 = 0.6;
/// Base Y frequency (`FY0`, L133).
pub const GARDNER_FY0: f64 = 0.6;
/// Gardner octave count (`octaves`, L134).
pub const GARDNER_OCTAVES: usize = 5;
/// Ambient / scattered-light fraction (`a`, L151).
pub const CLOUD_AMBIENT_FRACTION: f64 = 0.5;
/// Texture-shading fraction (`t`, L152).
pub const CLOUD_TEXTURE_FRACTION: f64 = 0.4;
/// Specular fraction (`s`, L153).
pub const CLOUD_SPECULAR_FRACTION: f64 = 0.25;
/// Fixed cloud light direction — mirror `CloudCollectionFS.glsl` L159
/// `normalize(vec3(0.2, -1.0, 0.7))`.
pub const CLOUD_LIGHT_DIR: DVec3 = DVec3::new(0.2, -1.0, 0.7);

/// Worley FBM iteration cap — mirror `CloudNoiseFS.glsl` `MAX_FBM_ITERATIONS` (L60).
pub const MAX_FBM_ITERATIONS: usize = 10;
/// Worley FBM base persistence — mirror `CloudNoiseFS.glsl` L65.
pub const WORLEY_FBM_PERSISTENCE: f64 = 0.625;
/// Small surface offset to avoid self-intersection — mirror `czm_epsilon2`
/// (`CloudCollectionFS.glsl` L92).
pub const CZM_EPSILON2: f64 = 1e-5;

#[inline]
fn fract(x: f64) -> f64 {
    x - x.floor()
}

#[inline]
fn fract3(v: DVec3) -> DVec3 {
    DVec3::new(fract(v.x), fract(v.y), fract(v.z))
}

#[inline]
fn floor3(v: DVec3) -> DVec3 {
    DVec3::new(v.x.floor(), v.y.floor(), v.z.floor())
}

/// Mirror of `CloudNoiseFS.glsl` / `CloudCollectionFS.glsl` `wrap` (L6-13 / L10-17):
/// positive modulo that stays in `[0, range_length)` for negative inputs too.
pub fn wrap(value: f64, range_length: f64) -> f64 {
    if value < 0.0 {
        let abs_value = value.abs();
        let mod_value = abs_value % range_length;
        (range_length - mod_value) % range_length
    } else {
        value % range_length
    }
}

/// Component-wise [`wrap`] — mirror `wrapVec` (CloudNoiseFS.glsl L15-19).
pub fn wrap_vec(value: DVec3, range_length: f64) -> DVec3 {
    DVec3::new(
        wrap(value.x, range_length),
        wrap(value.y, range_length),
        wrap(value.z, range_length),
    )
}

/// Mirror of `CloudNoiseFS.glsl` `random3` (L21-25): a hash-like pseudo-random
/// point in `[0,1)³` from a cell centre. CPU f64 reference — the GPU
/// (`cloud_noise.wgsl`) mirrors the same expression in f32.
pub fn worley_random3(p: DVec3) -> DVec3 {
    let dot1 = p.dot(DVec3::new(127.1, 311.7, 932.8));
    let dot2 = p.dot(DVec3::new(269.5, 183.3, 421.4));
    DVec3::new(
        fract((dot1 - dot2).sin()),
        fract((dot1 * dot2).cos()),
        fract(dot1 * dot2),
    )
}

/// Mirror of `CloudNoiseFS.glsl` `getWorleyCellPoint` (L29-36).
fn worley_cell_point(
    center_cell: DVec3,
    offset: DVec3,
    detail: f64,
    noise_offset: DVec3,
    slice_width: f64,
) -> DVec3 {
    let cell = wrap_vec(center_cell + offset, slice_width / detail);
    let cell = cell + floor3(noise_offset / detail);
    offset + worley_random3(cell)
}

/// Mirror of `CloudNoiseFS.glsl` `worleyNoise` (L38-58): the shortest distance from
/// `p` (scaled by `freq`) to the nearest jittered cell centre over the 3×3×3
/// neighbourhood. Result is in `[0, ~1.5]` (a cell diagonal).
pub fn worley_noise(
    p: DVec3,
    freq: f64,
    detail: f64,
    noise_offset: DVec3,
    slice_width: f64,
) -> f64 {
    let center_cell = floor3(p * freq);
    let point_in_cell = fract3(p * freq);
    let mut shortest_distance = 1000.0_f64;
    for z in -1..=1_i32 {
        for y in -1..=1_i32 {
            for x in -1..=1_i32 {
                let offset = DVec3::new(x as f64, y as f64, z as f64);
                let point =
                    worley_cell_point(center_cell, offset, detail, noise_offset, slice_width);
                let distance = (point_in_cell - point).length();
                if distance < shortest_distance {
                    shortest_distance = distance;
                }
            }
        }
    }
    shortest_distance
}

/// Mirror of `CloudNoiseFS.glsl` `worleyFBMNoise` (L62-76): `octaves` of Worley at
/// doubling frequency / halving persistence, summed.
pub fn worley_fbm(
    p: DVec3,
    octaves: usize,
    scale: f64,
    detail: f64,
    noise_offset: DVec3,
    slice_width: f64,
) -> f64 {
    let mut noise = 0.0_f64;
    let mut freq = 1.0_f64;
    let mut persistence = WORLEY_FBM_PERSISTENCE;
    for i in 0..MAX_FBM_ITERATIONS {
        if i >= octaves {
            break;
        }
        noise += worley_noise(p * scale, freq * scale, detail, noise_offset, slice_width)
            * persistence;
        persistence *= 0.5;
        freq *= 2.0;
    }
    noise
}

/// Cloud type enumeration.
///
/// Maps to CesiumJS `Scene/CloudType.js`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CloudType {
    /// Cumulus clouds (billboard-based).
    #[default]
    Cumulus,
}

/// A single cumulus cloud billboard in the 3D scene.
///
/// Maps to CesiumJS `Scene/CumulusCloud.js`.
#[derive(Debug, Clone, PartialEq)]
pub struct CumulusCloud {
    /// Whether the cloud is visible.
    pub show: bool,
    /// World position of the cloud.
    pub position: DVec3,
    /// Billboard scale (width, height) in meters.
    pub scale: [f64; 2],
    /// Maximum size of the cloud volume (x, y, z) in meters.
    pub maximum_size: DVec3,
    /// Cross-section slice through the cloud [0, 1], or negative for no slice.
    pub slice: f64,
    /// Brightness multiplier [0, 1].
    pub brightness: f64,
    /// Cloud color as RGBA [0, 1].
    pub color: [f64; 4],
    /// Internal index in the collection.
    index: i32,
}

impl Default for CumulusCloud {
    fn default() -> Self {
        Self {
            show: true,
            position: DVec3::ZERO,
            scale: [20.0, 12.0],
            maximum_size: DVec3::new(20.0, 12.0, 12.0_f64 / 1.5),
            slice: -1.0,
            brightness: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
            index: -1,
        }
    }
}

impl CumulusCloud {
    /// Create a new cumulus cloud with position and maximum size.
    pub fn new(position: DVec3, maximum_size: DVec3) -> Self {
        let scale = [maximum_size.x, maximum_size.y];
        Self {
            position,
            scale,
            maximum_size,
            ..Default::default()
        }
    }

    /// Create with full options.
    pub fn with_options(
        position: DVec3,
        scale: [f64; 2],
        maximum_size: DVec3,
        slice: f64,
        brightness: f64,
        color: [f64; 4],
    ) -> Self {
        Self {
            show: true,
            position,
            scale,
            maximum_size,
            slice,
            brightness,
            color,
            index: -1,
        }
    }

    /// Get the cloud's index in the collection.
    pub fn index(&self) -> i32 {
        self.index
    }

    /// Compute the effective billboard dimensions considering slice.
    pub fn effective_dimensions(&self) -> [f64; 2] {
        if self.slice >= 0.0 && self.slice <= 1.0 {
            // Sliced clouds appear smaller
            let factor = 1.0 - (self.slice - 0.5).abs() * 0.5;
            [self.scale[0] * factor, self.scale[1] * factor]
        } else {
            self.scale
        }
    }

    /// Check if the slice value is in the recommended range [0.1, 0.9].
    pub fn is_slice_recommended(&self) -> bool {
        self.slice < 0.0 || (self.slice >= 0.1 && self.slice <= 0.9)
    }
}

/// A renderable collection of clouds in the 3D scene.
///
/// Maps to CesiumJS `Scene/CloudCollection.js`.
#[derive(Debug, Clone)]
pub struct CloudCollection {
    /// Whether to display the clouds.
    pub show: bool,
    /// Desired amount of detail in the noise texture.
    pub noise_detail: f64,
    /// Desired translation of data in noise texture.
    pub noise_offset: DVec3,
    /// For debugging: render billboards with opaque color.
    pub debug_billboards: bool,
    /// For debugging: render clouds as opaque ellipsoids.
    pub debug_ellipsoids: bool,
    /// The clouds in this collection.
    clouds: Vec<CumulusCloud>,
    /// Whether the collection needs a GPU buffer update.
    dirty: bool,
}

impl Default for CloudCollection {
    fn default() -> Self {
        Self {
            show: true,
            noise_detail: 16.0,
            noise_offset: DVec3::ZERO,
            debug_billboards: false,
            debug_ellipsoids: false,
            clouds: Vec::new(),
            dirty: true,
        }
    }
}

impl CloudCollection {
    /// Create a new empty cloud collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with noise parameters.
    pub fn with_noise(noise_detail: f64, noise_offset: DVec3) -> Self {
        Self {
            noise_detail,
            noise_offset,
            ..Default::default()
        }
    }

    /// Add a cloud to the collection. Returns the cloud's index.
    pub fn add(&mut self, mut cloud: CumulusCloud) -> usize {
        let index = self.clouds.len();
        cloud.index = index as i32;
        self.clouds.push(cloud);
        self.dirty = true;
        index
    }

    /// Remove a cloud by index.
    pub fn remove(&mut self, index: usize) -> Option<CumulusCloud> {
        if index < self.clouds.len() {
            let cloud = self.clouds.remove(index);
            // Reindex remaining clouds
            for (i, c) in self.clouds.iter_mut().enumerate().skip(index) {
                c.index = i as i32;
            }
            self.dirty = true;
            Some(cloud)
        } else {
            None
        }
    }

    /// Remove all clouds.
    pub fn remove_all(&mut self) {
        self.clouds.clear();
        self.dirty = true;
    }

    /// Get a cloud by index.
    pub fn get(&self, index: usize) -> Option<&CumulusCloud> {
        self.clouds.get(index)
    }

    /// Get a mutable cloud by index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut CumulusCloud> {
        if index < self.clouds.len() {
            self.dirty = true;
            self.clouds.get_mut(index)
        } else {
            None
        }
    }

    /// Get the number of clouds.
    pub fn len(&self) -> usize {
        self.clouds.len()
    }

    /// Check if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.clouds.is_empty()
    }

    /// Get all clouds.
    pub fn clouds(&self) -> &[CumulusCloud] {
        &self.clouds
    }

    /// Get visible clouds only.
    pub fn visible_clouds(&self) -> impl Iterator<Item = &CumulusCloud> {
        self.clouds.iter().filter(|c| c.show)
    }

    /// Check if the collection needs a GPU update.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark the collection as clean (after GPU update).
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Compute the total bounding sphere of all visible clouds.
    pub fn compute_bounding_sphere(&self) -> Option<(DVec3, f64)> {
        let visible: Vec<_> = self.visible_clouds().collect();
        if visible.is_empty() {
            return None;
        }

        // Simple centroid + max distance approach
        let mut center = DVec3::ZERO;
        for cloud in &visible {
            center += cloud.position;
        }
        center /= visible.len() as f64;

        let mut max_dist = 0.0_f64;
        for cloud in &visible {
            let dist = (cloud.position - center).length()
                + cloud.maximum_size.length() * 0.5;
            max_dist = max_dist.max(dist);
        }

        Some((center, max_dist))
    }
}

// ─── Perlin gradient noise (ADDITIVE — the "Perlin" half of Perlin-Worley) ──────

/// Classic Perlin permutation table (Ken Perlin's 2002 improved-noise ordering).
/// Deterministic; the GPU `cloud_noise.wgsl` mirrors the same table.
const PERLIN_PERM: [u8; 256] = [
    151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225, 140, 36, 103, 30, 69,
    142, 8, 99, 37, 240, 21, 10, 23, 190, 6, 148, 247, 120, 234, 75, 0, 26, 197, 62, 94, 252, 219,
    203, 117, 35, 11, 32, 57, 177, 33, 88, 237, 149, 56, 87, 174, 20, 125, 136, 171, 168, 68, 175,
    74, 165, 71, 134, 139, 48, 27, 166, 77, 146, 158, 231, 83, 111, 229, 122, 60, 211, 133, 230,
    220, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54, 65, 25, 63, 161, 1, 216, 80, 73, 209,
    76, 132, 187, 208, 89, 18, 169, 200, 196, 135, 130, 116, 188, 159, 86, 164, 100, 109, 198,
    173, 186, 3, 64, 52, 217, 226, 250, 124, 123, 5, 202, 38, 147, 118, 126, 255, 82, 85, 212,
    207, 206, 59, 227, 47, 16, 58, 17, 182, 189, 28, 42, 223, 183, 170, 213, 119, 248, 152, 2, 44,
    154, 163, 70, 221, 153, 101, 155, 167, 43, 172, 9, 129, 22, 39, 253, 19, 98, 108, 110, 79,
    113, 224, 232, 178, 185, 112, 104, 218, 246, 97, 228, 251, 34, 242, 193, 238, 210, 144, 12,
    191, 179, 162, 241, 81, 51, 145, 235, 249, 14, 239, 107, 49, 192, 214, 31, 181, 199, 106, 157,
    184, 84, 204, 176, 115, 121, 50, 45, 127, 4, 150, 254, 138, 236, 205, 93, 222, 114, 67, 29,
    24, 72, 243, 141, 128, 195, 78, 66, 215, 61, 156, 180,
];

#[inline]
fn perlin_fade(t: f64) -> f64 {
    // 6t⁵ − 15t⁴ + 10t³ (NO FMA contraction — three separate roundings).
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn perlin_perm(index: usize) -> usize {
    PERLIN_PERM[index & 255] as usize
}

fn perlin_grad(hash: usize, x: f64, y: f64, z: f64) -> f64 {
    // 12 gradient directions (Perlin's improved set), reduced to the 8 cube corners.
    let h = hash & 15;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 { y } else if h == 12 || h == 14 { x } else { z };
    let u_term = if (h & 1) == 0 { u } else { -u };
    let v_term = if (h & 2) == 0 { v } else { -v };
    u_term + v_term
}

/// Classic improved-Perlin 3D gradient noise in approximately `[-1, 1]`.
/// ADDITIVE: upstream CesiumJS cumulus clouds store pure Worley-FBM channels; the
/// Perlin base is the low-frequency "shape" half of the standard Perlin-Worley
/// cloud model (Decima / Hillaire), provided here for the volumetric path.
pub fn perlin_noise_3d(p: DVec3) -> f64 {
    let x = p.x.floor() as i64;
    let y = p.y.floor() as i64;
    let z = p.z.floor() as i64;
    let xf = p.x - x as f64;
    let yf = p.y - y as f64;
    let zf = p.z - z as f64;
    let u = perlin_fade(xf);
    let v = perlin_fade(yf);
    let w = perlin_fade(zf);
    let xi = (x & 255) as usize;
    let yi = (y & 255) as usize;
    let zi = (z & 255) as usize;
    let aa = perlin_perm(perlin_perm(xi) + yi);
    let ab = perlin_perm(perlin_perm(xi) + yi + 1);
    let ba = perlin_perm(perlin_perm(xi + 1) + yi);
    let bb = perlin_perm(perlin_perm(xi + 1) + yi + 1);
    let aaa = perlin_grad(perlin_perm(aa + zi), xf, yf, zf);
    let baa = perlin_grad(perlin_perm(ba + zi), xf - 1.0, yf, zf);
    let aba = perlin_grad(perlin_perm(ab + zi), xf, yf - 1.0, zf);
    let bba = perlin_grad(perlin_perm(bb + zi), xf - 1.0, yf - 1.0, zf);
    let aab = perlin_grad(perlin_perm(aa + zi + 1), xf, yf, zf - 1.0);
    let bab = perlin_grad(perlin_perm(ba + zi + 1), xf - 1.0, yf, zf - 1.0);
    let abb = perlin_grad(perlin_perm(ab + zi + 1), xf, yf - 1.0, zf - 1.0);
    let bbb = perlin_grad(perlin_perm(bb + zi + 1), xf - 1.0, yf - 1.0, zf - 1.0);
    let x1 = lerp(lerp(aaa, baa, u), lerp(aba, bba, u), v);
    let x2 = lerp(lerp(aab, bab, u), lerp(abb, bbb, u), v);
    lerp(x1, x2, w)
}

#[inline]
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    // Two roundings (NO FMA): a + (b − a)·t keeps the subtraction and the
    // multiply/add distinct, matching the WGSL reference.
    a + (b - a) * t
}

/// A CPU f64 reference noise volume: `dimensions³` voxels × [`NOISE_CHANNELS`]
/// Worley-FBM channels, mirroring `CloudNoiseFS.glsl` `main` (L78-92).
///
/// The GPU path (`cloud_noise.wgsl`) generates the same volume into a `texture_3d`;
/// this struct is the deterministic reference used by the unit tests and (optionally)
/// as a CPU-upload fallback when a 3D texture is unavailable. `data` is laid out
/// channel-major within a voxel and z-slowest:
/// `index = ((z·dim + y)·dim + x)·CHANNELS + c`.
#[derive(Debug, Clone, PartialEq)]
pub struct NoiseVolume {
    /// Edge length in voxels (production = [`NOISE_TEXTURE_DIMENSIONS`] = 128).
    pub dimensions: usize,
    /// Worley-FBM detail divisor — mirror `u_noiseDetail` (default 16).
    pub detail: f64,
    /// Noise translation — mirror `u_noiseOffset`.
    pub noise_offset: DVec3,
    /// Flat voxel data, `dimensions³ · NOISE_CHANNELS` f64 values in `[0, 1]`.
    pub data: Vec<f64>,
}

impl NoiseVolume {
    /// Generates a `dimensions³` Worley-FBM volume. Mirrors `CloudNoiseFS.glsl`
    /// `main`: each voxel centre `position = (x, y, z) / detail` yields three
    /// clamped `worley_fbm(position, 3 octaves, scale ∈ {1, 2, 3})` channels.
    ///
    /// Cost is `dimensions³ · 3 · (3 octaves · 27 cells)`; production 128³ is a
    /// one-off GPU/upload step, while tests use a small `dimensions` (≤ 16).
    pub fn generate(dimensions: usize, detail: f64, noise_offset: DVec3) -> Self {
        let slice_width = dimensions as f64;
        let mut data = vec![0.0_f64; dimensions * dimensions * dimensions * NOISE_CHANNELS];
        for z in 0..dimensions {
            for y in 0..dimensions {
                for x in 0..dimensions {
                    let position = DVec3::new(x as f64, y as f64, z as f64) / detail;
                    let base = ((z * dimensions + y) * dimensions + x) * NOISE_CHANNELS;
                    for (c, scale) in [1.0_f64, 2.0, 3.0].iter().enumerate() {
                        let worley = worley_fbm(position, 3, *scale, detail, noise_offset, slice_width);
                        data[base + c] = worley.clamp(0.0, 1.0);
                    }
                }
            }
        }
        Self {
            dimensions,
            detail,
            noise_offset,
            data,
        }
    }

    /// Fetches the raw 3-channel value at integer voxel `(x, y, z)` (wrapped).
    pub fn voxel(&self, x: usize, y: usize, z: usize) -> [f64; NOISE_CHANNELS] {
        let d = self.dimensions;
        let wx = x.rem_euclid(d);
        let wy = y.rem_euclid(d);
        let wz = z.rem_euclid(d);
        let base = ((wz * d + wy) * d + wx) * NOISE_CHANNELS;
        [
            self.data[base],
            self.data[base + 1],
            self.data[base + 2],
        ]
    }

    /// Trilinear interpolation of the volume at a continuous voxel-space `position`.
    /// Mirrors `CloudCollectionFS.glsl` `sampleNoiseTexture` (L51-65): recenter by
    /// half the slice width, then `floor`/`fract` + 3-axis `mix`.
    pub fn sample_trilinear(&self, position: DVec3) -> [f64; NOISE_CHANNELS] {
        let d = self.dimensions as f64;
        let recentered = position + DVec3::splat(d / 2.0);
        let lerp_value = fract3(recentered);
        let voxel_index = floor3(recentered);
        let ix = voxel_index.x as isize;
        let iy = voxel_index.y as isize;
        let iz = voxel_index.z as isize;
        let s = |dx: isize, dy: isize, dz: isize| {
            self.voxel((ix + dx) as usize, (iy + dy) as usize, (iz + dz) as usize)
        };
        let mix3 = |a: [f64; 3], b: [f64; 3], t: f64| {
            [
                lerp(a[0], b[0], t),
                lerp(a[1], b[1], t),
                lerp(a[2], b[2], t),
            ]
        };
        let x00 = mix3(s(0, 0, 0), s(1, 0, 0), lerp_value.x);
        let x10 = mix3(s(0, 1, 0), s(1, 1, 0), lerp_value.x);
        let x01 = mix3(s(0, 0, 1), s(1, 0, 1), lerp_value.x);
        let x11 = mix3(s(0, 1, 1), s(1, 1, 1), lerp_value.x);
        let y0 = mix3(x00, x10, lerp_value.y);
        let y1 = mix3(x01, x11, lerp_value.y);
        mix3(y0, y1, lerp_value.z)
    }

    /// Packs the volume into RGBA8 bytes for the GPU upload boundary (the single
    /// f64 → u8 projection). Alpha is 255; RGB are the three Worley channels.
    /// Layout matches a `128 × (128·ROWS)` 2D atlas OR a `128³` 3D texture
    /// (z-slowest), so the same bytes serve either the atlas or the D3 route.
    pub fn to_rgba8_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.data.len() / NOISE_CHANNELS * 4);
        for voxel in self.data.chunks_exact(NOISE_CHANNELS) {
            for &channel in voxel {
                out.push((channel.clamp(0.0, 1.0) * 255.0).round() as u8);
            }
            out.push(255);
        }
        out
    }
}

// ─── Billboard geometry (mirror CloudCollectionVS.glsl) ──────────────────────

/// Two-triangle quad indices — mirror `CloudCollection.js` `textureIndices` (L451).
pub const BILLBOARD_INDICES: [u32; 6] = [0, 1, 2, 0, 2, 3];

/// The four corner UVs of a cloud billboard — mirror the `coordinates` attribute
/// (`CloudCollectionVS.glsl` L24 / L34 `offset = dir - vec2(0.5, 0.5)`).
pub const BILLBOARD_CORNER_UVS: [[f64; 2]; 4] =
    [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

/// A camera-facing quad (4 world-space vertices) for one cumulus cloud.
///
/// Mirrors `CloudCollectionVS.glsl`: the corners are offset from the cloud centre
/// by `scale * (uv - 0.5)` along the camera right/up basis, so every billboard is
/// parallel to the view plane (screen-aligned, as upstream does in eye space).
#[derive(Debug, Clone, PartialEq)]
pub struct BillboardGeometry {
    /// World-space corner positions (metric f64), matching [`BILLBOARD_CORNER_UVS`].
    pub positions: [[f64; 3]; 4],
    /// Per-corner UVs in `[0,1]²`.
    pub uvs: [[f64; 2]; 4],
    /// Triangle indices ([`BILLBOARD_INDICES`]).
    pub indices: [u32; 6],
    /// The cloud centre the quad is built around.
    pub center: DVec3,
    /// Unit normal guaranteed to point FROM the cloud TOWARD the camera.
    pub normal: DVec3,
}

impl CloudCollection {
    /// Builds one camera-facing billboard quad for `cloud`.
    ///
    /// `cam_right` / `cam_up` are the (world-space) camera basis vectors; the quad
    /// lies in the plane they span, centred on `cloud.position`, sized by
    /// [`CumulusCloud::effective_dimensions`]. `camera_position` is used only to
    /// orient [`BillboardGeometry::normal`] toward the eye (handedness-robust).
    pub fn build_billboard(
        cloud: &CumulusCloud,
        camera_position: DVec3,
        cam_right: DVec3,
        cam_up: DVec3,
    ) -> BillboardGeometry {
        let dims = cloud.effective_dimensions();
        let right = cam_right.normalize();
        let up = cam_up.normalize();
        let mut normal = right.cross(up);
        // Guarantee the normal faces the camera regardless of basis handedness.
        if normal.dot(camera_position - cloud.position) < 0.0 {
            normal = -normal;
        }
        let mut positions = [[0.0_f64; 3]; 4];
        for (i, uv) in BILLBOARD_CORNER_UVS.iter().enumerate() {
            // offset = dir - 0.5 ; scaledOffset = scale * offset (VS L34-35).
            let offset = DVec2::new(uv[0] - 0.5, uv[1] - 0.5);
            let scaled = DVec2::new(dims[0] * offset.x, dims[1] * offset.y);
            let p = cloud.position + right * scaled.x + up * scaled.y;
            positions[i] = [p.x, p.y, p.z];
        }
        BillboardGeometry {
            positions,
            uvs: BILLBOARD_CORNER_UVS,
            indices: BILLBOARD_INDICES,
            center: cloud.position,
            normal,
        }
    }

    /// Builds billboards for every visible cloud (skips `show == false`).
    pub fn build_geometry(
        &self,
        camera_position: DVec3,
        cam_right: DVec3,
        cam_up: DVec3,
    ) -> Vec<BillboardGeometry> {
        self.visible_clouds()
            .map(|cloud| Self::build_billboard(cloud, camera_position, cam_right, cam_up))
            .collect()
    }
}

// ─── Gardner (1985) texture + intensity (mirror CloudCollectionFS.glsl) ───────

/// Mirror of `CloudCollectionFS.glsl` `phaseShift2D` (L118-120).
fn phase_shift_2d(p: DVec2, freq: DVec2) -> DVec2 {
    let half_pi = std::f64::consts::FRAC_PI_2;
    DVec2::new(half_pi * (freq.y * p.y).sin(), half_pi * (freq.x * p.x).sin())
}

/// Mirror of `CloudCollectionFS.glsl` `phaseShift3D` (L122-124).
fn phase_shift_3d(p: DVec3, freq: DVec2) -> DVec2 {
    let s = (freq.x * p.z).sin();
    phase_shift_2d(DVec2::new(p.x, p.y), freq)
        + DVec2::new(std::f64::consts::PI * s, std::f64::consts::PI * s)
}

/// Mirror of `CloudCollectionFS.glsl` `T` (L136-149): Gardner's sine-sum cloud
/// texture function. `Ci *= 0.707` and `FXY *= 2.0` happen BEFORE each octave's use.
pub fn gardner_texture(point: DVec3) -> f64 {
    let mut sum = DVec2::ZERO;
    let mut ci = GARDNER_C0;
    let mut fxy = DVec2::new(GARDNER_FX0, GARDNER_FY0);
    for _ in 1..=GARDNER_OCTAVES {
        let pxy = phase_shift_3d(point, fxy);
        ci *= 0.707;
        fxy *= 2.0;
        let sin_term = DVec2::new(
            (fxy.x * point.x + pxy.x).sin(),
            (fxy.y * point.y + pxy.y).sin(),
        );
        sum += ci * sin_term + DVec2::new(GARDNER_T0, GARDNER_T0);
    }
    GARDNER_K * sum.x * sum.y
}

/// Mirror of `CloudCollectionFS.glsl` `I` (L155-157): combines diffuse (`id`),
/// specular (`is`) and texture (`it`) terms with the fixed ambient/texture/specular
/// fractions.
pub fn cloud_intensity(id: f64, is: f64, it: f64) -> f64 {
    let a = CLOUD_AMBIENT_FRACTION;
    let t = CLOUD_TEXTURE_FRACTION;
    let s = CLOUD_SPECULAR_FRACTION;
    (1.0 - a) * ((1.0 - t) * ((1.0 - s) * id + s * is) + t * it) + a
}

// ─── Ray / ellipsoid intersection (mirror CloudCollectionFS.glsl) ─────────────

/// A ray/ellipsoid intersection: the surface `point`, the unit-sphere `normal`, and
/// the ray parameter `t`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EllipsoidHit {
    pub point: DVec3,
    pub normal: DVec3,
    pub t: f64,
}

/// Mirror of `CloudCollectionFS.glsl` `intersectSphere` (L68-94): intersection with
/// a unit sphere of radius 0.5 at the origin, honouring the optional `slice` plane.
pub fn intersect_sphere(origin: DVec3, dir: DVec3, slice: f64) -> Option<EllipsoidHit> {
    let a = dir.dot(dir);
    let b = origin.dot(dir);
    let c = origin.dot(origin) - 0.25;
    let discriminant = (b * b) - (a * c);
    if discriminant < 0.0 {
        return None;
    }
    let root = discriminant.sqrt();
    let mut t = (-b - root) / a;
    if t < 0.0 {
        t = (-b + root) / a;
    }
    let mut point = origin + dir * t;
    if slice >= 0.0 {
        // point.z = (slice / 2.0) - 0.5 (GLSL swizzle write → whole-vector rebuild).
        point = DVec3::new(point.x, point.y, slice / 2.0 - 0.5);
        if point.length() > 0.5 {
            return None;
        }
    }
    let normal = point.normalize();
    let point = point - CZM_EPSILON2 * normal;
    Some(EllipsoidHit { point, normal, t })
}

/// Mirror of `CloudCollectionFS.glsl` `intersectEllipsoid` (L98-113): transforms the
/// ray into unit-sphere space, intersects, then maps the point back. The `normal`
/// stays in unit-sphere space (upstream does NOT rescale it).
pub fn intersect_ellipsoid(
    origin: DVec3,
    dir: DVec3,
    center: DVec3,
    scale: DVec3,
    slice: f64,
) -> Option<EllipsoidHit> {
    if scale.x <= 0.01 || scale.y < 0.01 || scale.z < 0.01 {
        return None;
    }
    let o = (origin - center) / scale;
    let d = dir / scale;
    let mut hit = intersect_sphere(o, d, slice)?;
    hit.point = (hit.point * scale) + center;
    Some(hit)
}

/// Mirror of `CloudCollectionFS.glsl` `drawCloud` (L161-230): the FAITHFUL upstream
/// cumulus shading — a single ellipsoid intersection, Gardner texture + Worley-FBM
/// erosion, returning premultiplied `rgba` (alpha = translucency `TR`).
///
/// `noise` / `noise_detail` drive the `sampleNoiseTexture(u_noiseDetail * point)`
/// call (L175). Returns `[0,0,0,0]` on a miss (upstream returns `vec4(0.0)`).
///
/// `#[allow(clippy::too_many_arguments)]`: the 9-argument list is a deliberate
/// 1:1 mirror of upstream `drawCloud`'s uniform inputs (ray, ellipsoid, shading,
/// noise). Collapsing it into param structs would obscure the domain-WGSL parity
/// this reference exists to guarantee, and there are no production callers
/// yet (the adapter constructs these from GPU uniforms). See `docs/deviations.md#dev-032`.
#[allow(clippy::too_many_arguments)]
pub fn draw_cloud(
    ray_origin: DVec3,
    ray_dir: DVec3,
    center: DVec3,
    scale: DVec3,
    slice: f64,
    brightness: f64,
    color: [f64; 4],
    noise: &NoiseVolume,
    noise_detail: f64,
) -> [f64; 4] {
    let hit = match intersect_ellipsoid(ray_origin, ray_dir, center, scale, slice) {
        Some(hit) => hit,
        None => return [0.0; 4],
    };
    let light_dir = CLOUD_LIGHT_DIR.normalize();
    let id = hit.normal.dot(-light_dir).clamp(0.0, 1.0); // diffuse
    let is = (-light_dir).dot(-ray_dir).max(0.0).powi(2); // specular
    let it = gardner_texture(hit.point); // texture
    let intensity = cloud_intensity(id, is, it);
    let shaded = intensity * brightness.clamp(0.1, 1.0);

    let n = noise.sample_trilinear(hit.point * noise_detail);
    let w = n[0];
    let w2 = n[1];
    let w3 = n[2];

    let nd_dot = hit.normal.dot(-ray_dir).clamp(0.0, 1.0);
    let mut tr = nd_dot.powi(3) - w; // translucency
    tr *= 1.3;
    let minus_dot = 0.5 - nd_dot;
    tr -= (minus_dot * w2).min(0.0);
    tr -= 0.8 * (minus_dot + 0.25) * w3;

    let mut shading = lerp(1.0 - 0.8 * w * w, 1.0, id * tr);
    shading = (shading + 0.2).clamp(0.3, 1.0);

    // finalColor = mix(vec3(0.5), shading * color, 1.15); return vec4(finalColor, TR) * v_color
    let sc_r = shading * shaded;
    let fr = lerp(0.5, sc_r, 1.15);
    let alpha = tr.clamp(0.0, 1.0);
    [
        fr * color[0],
        fr * color[1],
        fr * color[2],
        alpha * color[3],
    ]
}

// ─── Physically-based scattering (ADDITIVE — NOT upstream cumulus) ───────────

/// Henyey-Greenstein phase function (normalised over the sphere). `g = 0` is
/// isotropic; `g > 0` is forward-scattering. ADDITIVE extension: upstream cumulus
/// uses the Gardner specular/diffuse terms, not a phase function.
pub fn henyey_greenstein(cos_theta: f64, g: f64) -> f64 {
    let g2 = g * g;
    let denom = 1.0 + g2 - 2.0 * g * cos_theta;
    // Guard the degenerate denominator (g → ±1 with cos_theta → ±1).
    if denom.abs() < 1e-12 {
        return 0.0;
    }
    (1.0 - g2) / (4.0 * std::f64::consts::PI * denom.powf(1.5))
}

/// Beer-Lambert transmittance `exp(-density · extinction · distance)` over a step.
pub fn beer_lambert_transmittance(density: f64, extinction: f64, distance: f64) -> f64 {
    (-(density * extinction * distance)).exp()
}

/// Result of the ADDITIVE volumetric raymarch through one cloud ellipsoid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaymarchResult {
    /// Accumulated transmittance after the march (in `(0, 1]`).
    pub transmittance: f64,
    /// Accumulated in-scattered radiance (HG phase × absorbed energy).
    pub scattered: f64,
    /// Number of steps actually taken.
    pub steps_taken: usize,
}

/// The near/far ray parameters where the ray enters/exits the unit-sphere-space
/// ellipsoid (both roots, near clamped to 0). Returns `None` on a miss.
fn ellipsoid_interval(
    origin: DVec3,
    dir: DVec3,
    center: DVec3,
    scale: DVec3,
) -> Option<(f64, f64)> {
    if scale.x <= 0.01 || scale.y < 0.01 || scale.z < 0.01 {
        return None;
    }
    let o = (origin - center) / scale;
    let d = dir / scale;
    let a = d.dot(d);
    let b = o.dot(d);
    let c = o.dot(o) - 0.25;
    let discriminant = (b * b) - (a * c);
    if discriminant < 0.0 {
        return None;
    }
    let root = discriminant.sqrt();
    let t0 = ((-b - root) / a).max(0.0);
    let t1 = (-b + root) / a;
    if t1 <= t0 {
        return None;
    }
    Some((t0, t1))
}

/// ADDITIVE volumetric raymarch: marches `steps` (clamped to
/// `[RAYMARCH_STEPS_MIN, RAYMARCH_STEPS_MAX]`) samples between the ellipsoid entry
/// and exit, accumulating Beer-Lambert extinction and HG in-scattering. This is the
/// physically-based alternative to upstream's single-intersection `drawCloud`; both
/// are exposed so the render path can choose fidelity vs cost.
///
/// `#[allow(clippy::too_many_arguments)]`: same rationale as [`draw_cloud`] — the
/// signature mirrors the volumetric raymarch uniform set (ray, ellipsoid, march,
/// noise, scattering) rather than an idiomatic Rust grouping.
#[allow(clippy::too_many_arguments)]
pub fn raymarch_density(
    ray_origin: DVec3,
    ray_dir: DVec3,
    center: DVec3,
    scale: DVec3,
    steps: usize,
    noise: &NoiseVolume,
    noise_detail: f64,
    extinction: f64,
    cos_theta: f64,
) -> RaymarchResult {
    let steps = steps.clamp(RAYMARCH_STEPS_MIN, RAYMARCH_STEPS_MAX);
    let (t0, t1) = match ellipsoid_interval(ray_origin, ray_dir, center, scale) {
        Some(interval) => interval,
        None => {
            return RaymarchResult {
                transmittance: 1.0,
                scattered: 0.0,
                steps_taken: 0,
            }
        }
    };
    let dt = (t1 - t0) / steps as f64;
    let phase = henyey_greenstein(cos_theta, HG_PHASE_G);
    let mut transmittance = 1.0_f64;
    let mut scattered = 0.0_f64;
    let mut steps_taken = 0_usize;
    for i in 0..steps {
        let t = t0 + (i as f64 + 0.5) * dt;
        let world_point = ray_origin + ray_dir * t;
        let density = noise.sample_trilinear(world_point * noise_detail)[0].clamp(0.0, 1.0);
        let step_transmittance = beer_lambert_transmittance(density, extinction, dt);
        scattered += transmittance * (1.0 - step_transmittance) * phase;
        transmittance *= step_transmittance;
        steps_taken += 1;
    }
    RaymarchResult {
        transmittance,
        scattered,
        steps_taken,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cumulus_cloud_default() {
        let cloud = CumulusCloud::default();
        assert!(cloud.show);
        assert_eq!(cloud.position, DVec3::ZERO);
        assert_eq!(cloud.scale, [20.0, 12.0]);
        assert_eq!(cloud.slice, -1.0);
        assert_eq!(cloud.brightness, 1.0);
        assert_eq!(cloud.color, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_cumulus_cloud_new() {
        let pos = DVec3::new(100.0, 200.0, 300.0);
        let size = DVec3::new(30.0, 20.0, 15.0);
        let cloud = CumulusCloud::new(pos, size);
        assert_eq!(cloud.position, pos);
        assert_eq!(cloud.maximum_size, size);
        assert_eq!(cloud.scale, [30.0, 20.0]);
    }

    #[test]
    fn test_cumulus_cloud_with_options() {
        let cloud = CumulusCloud::with_options(
            DVec3::new(1.0, 2.0, 3.0),
            [25.0, 15.0],
            DVec3::new(25.0, 15.0, 10.0),
            0.5,
            0.8,
            [0.9, 0.9, 0.9, 1.0],
        );
        assert_eq!(cloud.scale, [25.0, 15.0]);
        assert_eq!(cloud.slice, 0.5);
        assert_eq!(cloud.brightness, 0.8);
    }

    #[test]
    fn test_cumulus_cloud_effective_dimensions() {
        let mut cloud = CumulusCloud::default();
        // No slice (negative) => full scale
        assert_eq!(cloud.effective_dimensions(), [20.0, 12.0]);

        // Slice at 0.5 => factor = 1.0
        cloud.slice = 0.5;
        let dims = cloud.effective_dimensions();
        assert!((dims[0] - 20.0).abs() < 1e-10);

        // Slice at 0.0 => factor = 0.75
        cloud.slice = 0.0;
        let dims = cloud.effective_dimensions();
        assert!((dims[0] - 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_cumulus_cloud_slice_recommended() {
        let mut cloud = CumulusCloud::default();
        assert!(cloud.is_slice_recommended()); // -1.0 is ok

        cloud.slice = 0.5;
        assert!(cloud.is_slice_recommended());

        cloud.slice = 0.05;
        assert!(!cloud.is_slice_recommended());

        cloud.slice = 0.95;
        assert!(!cloud.is_slice_recommended());
    }

    #[test]
    fn test_cloud_collection_default() {
        let collection = CloudCollection::new();
        assert!(collection.show);
        assert_eq!(collection.noise_detail, 16.0);
        assert_eq!(collection.noise_offset, DVec3::ZERO);
        assert!(collection.is_empty());
    }

    #[test]
    fn test_cloud_collection_add_remove() {
        let mut collection = CloudCollection::new();
        let idx = collection.add(CumulusCloud::new(DVec3::new(1.0, 2.0, 3.0), DVec3::new(20.0, 12.0, 8.0)));
        assert_eq!(idx, 0);
        assert_eq!(collection.len(), 1);

        let idx2 = collection.add(CumulusCloud::new(DVec3::new(4.0, 5.0, 6.0), DVec3::new(15.0, 9.0, 9.0)));
        assert_eq!(idx2, 1);
        assert_eq!(collection.len(), 2);

        let removed = collection.remove(0);
        assert!(removed.is_some());
        assert_eq!(collection.len(), 1);
        // Remaining cloud should be reindexed
        assert_eq!(collection.get(0).unwrap().index(), 0);
    }

    #[test]
    fn test_cloud_collection_remove_all() {
        let mut collection = CloudCollection::new();
        collection.add(CumulusCloud::default());
        collection.add(CumulusCloud::default());
        collection.add(CumulusCloud::default());
        assert_eq!(collection.len(), 3);

        collection.remove_all();
        assert!(collection.is_empty());
    }

    #[test]
    fn test_cloud_collection_visible_clouds() {
        let mut collection = CloudCollection::new();
        collection.add(CumulusCloud::default());
        let mut hidden = CumulusCloud::default();
        hidden.show = false;
        collection.add(hidden);
        collection.add(CumulusCloud::default());

        assert_eq!(collection.len(), 3);
        assert_eq!(collection.visible_clouds().count(), 2);
    }

    #[test]
    fn test_cloud_collection_dirty() {
        let mut collection = CloudCollection::new();
        assert!(collection.is_dirty());

        collection.mark_clean();
        assert!(!collection.is_dirty());

        collection.add(CumulusCloud::default());
        assert!(collection.is_dirty());
    }

    #[test]
    fn test_cloud_collection_bounding_sphere() {
        let mut collection = CloudCollection::new();
        assert!(collection.compute_bounding_sphere().is_none());

        collection.add(CumulusCloud::new(DVec3::new(0.0, 0.0, 0.0), DVec3::new(10.0, 10.0, 10.0)));
        collection.add(CumulusCloud::new(DVec3::new(100.0, 0.0, 0.0), DVec3::new(10.0, 10.0, 10.0)));

        let (center, radius) = collection.compute_bounding_sphere().unwrap();
        assert!((center.x - 50.0).abs() < 1e-10);
        assert!(radius > 50.0);
    }

    #[test]
    fn test_cloud_collection_with_noise() {
        let collection = CloudCollection::with_noise(32.0, DVec3::new(1.0, 2.0, 3.0));
        assert_eq!(collection.noise_detail, 32.0);
        assert_eq!(collection.noise_offset, DVec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_cloud_type() {
        assert_eq!(CloudType::default(), CloudType::Cumulus);
    }

    // ─── M6.6 additive tests: noise / billboard / intersection / scattering ────

    #[test]
    fn noise_worley_fbm_non_negative_monotone_in_octaves_and_deterministic() {
        let p = DVec3::new(0.3, 0.7, 0.1);
        let w1 = worley_fbm(p, 1, 1.0, 16.0, DVec3::ZERO, 128.0);
        let w3 = worley_fbm(p, 3, 1.0, 16.0, DVec3::ZERO, 128.0);
        assert!(w1 >= 0.0 && w1.is_finite(), "worley distance is non-negative");
        // Each octave adds a non-negative term (persistence > 0) ⇒ 3 ≥ 1.
        assert!(w3 >= w1 - 1e-12, "more octaves only add energy");
        // Deterministic reference (the GPU f32 mirror is cross-checked loosely).
        assert_eq!(w3, worley_fbm(p, 3, 1.0, 16.0, DVec3::ZERO, 128.0));
    }

    #[test]
    fn noise_volume_generate_is_bounded_and_deterministic() {
        let v = NoiseVolume::generate(8, 8.0, DVec3::ZERO);
        assert_eq!(v.data.len(), 8 * 8 * 8 * NOISE_CHANNELS);
        assert!(
            v.data.iter().all(|&x| (0.0..=1.0).contains(&x)),
            "worley channels clamped to [0,1]"
        );
        assert_eq!(v, NoiseVolume::generate(8, 8.0, DVec3::ZERO), "deterministic");
        // Trilinear at an exact integer voxel (recentered) returns that voxel.
        let vox = v.voxel(2, 3, 4);
        let sampled = v.sample_trilinear(DVec3::new(2.0, 3.0, 4.0) - DVec3::splat(4.0));
        for c in 0..NOISE_CHANNELS {
            assert!((sampled[c] - vox[c]).abs() < 1e-12, "channel {c} exact");
        }
    }

    #[test]
    fn noise_to_rgba8_packs_four_bytes_with_opaque_alpha() {
        let v = NoiseVolume::generate(4, 8.0, DVec3::ZERO);
        let bytes = v.to_rgba8_bytes();
        assert_eq!(bytes.len(), 4 * 4 * 4 * 4, "RGBA8 = 4 bytes/voxel");
        assert!(
            bytes.iter().skip(3).step_by(4).all(|&a| a == 255),
            "alpha channel is opaque"
        );
    }

    #[test]
    fn perlin_noise_is_bounded_continuous_and_roughly_zero_mean() {
        let mut min = f64::MAX;
        let mut max = f64::MIN;
        let mut sum = 0.0;
        let n = 1000;
        for i in 0..n {
            let fi = i as f64;
            let p = DVec3::new(fi * 0.013, (fi * 0.021).sin(), fi * 0.007);
            let v = perlin_noise_3d(p);
            assert!(v.is_finite());
            min = min.min(v);
            max = max.max(v);
            sum += v;
        }
        assert!(min >= -1.5 && max <= 1.5, "Perlin bounded ≈ [-1,1]: [{min}, {max}]");
        assert!((sum / n as f64).abs() < 0.3, "roughly zero-mean");
        // Continuity: points 1e-3 apart differ by < 0.05.
        let a = perlin_noise_3d(DVec3::new(1.0, 2.0, 3.0));
        let b = perlin_noise_3d(DVec3::new(1.001, 2.0, 3.0));
        assert!((a - b).abs() < 0.05, "Lipschitz-continuous");
    }

    #[test]
    fn billboard_faces_camera_and_is_symmetric_about_center() {
        let cloud = CumulusCloud::new(DVec3::ZERO, DVec3::new(20.0, 12.0, 8.0));
        let cam_pos = DVec3::new(0.0, 0.0, 100.0);
        let bb = CloudCollection::build_billboard(&cloud, cam_pos, DVec3::X, DVec3::Y);
        // Normal points from the cloud toward the camera (+Z here).
        assert!(bb.normal.dot(cam_pos - bb.center) > 0.0, "faces camera");
        assert!((bb.normal - DVec3::Z).length() < 1e-9, "right×up = +Z");
        // Centroid of the 4 corners == the cloud centre.
        let mut centroid = DVec3::ZERO;
        for p in &bb.positions {
            centroid += DVec3::from(*p);
        }
        centroid /= 4.0;
        assert!((centroid - bb.center).length() < 1e-9, "symmetric");
        // X span == effective width (20), Y span == effective height (12).
        assert!((bb.positions[1][0] - bb.positions[0][0] - 20.0).abs() < 1e-9);
        assert!((bb.positions[3][1] - bb.positions[0][1] - 12.0).abs() < 1e-9);
        assert_eq!(bb.indices, BILLBOARD_INDICES);
    }

    #[test]
    fn build_geometry_emits_only_visible_clouds() {
        let mut c = CloudCollection::new();
        c.add(CumulusCloud::default());
        let mut hidden = CumulusCloud::default();
        hidden.show = false;
        c.add(hidden);
        c.add(CumulusCloud::default());
        let geoms = c.build_geometry(DVec3::new(0.0, 0.0, 100.0), DVec3::X, DVec3::Y);
        assert_eq!(geoms.len(), 2, "hidden cloud skipped");
    }

    #[test]
    fn intersect_sphere_honours_slice_plane() {
        let hit = intersect_sphere(DVec3::new(0.0, 0.0, -5.0), DVec3::new(0.0, 0.0, 1.0), 0.5)
            .expect("hits the unit sphere");
        // slice = 0.5 ⇒ point.z = 0.25 - 0.5 = -0.25 (± the epsilon normal offset).
        assert!((hit.point.z - (-0.25)).abs() < 1e-3, "z pinned to slice plane");
        assert!(hit.point.length() <= 0.5 + 1e-9, "inside the sphere");
    }

    #[test]
    fn intersect_ellipsoid_hits_misses_and_rejects_degenerate_scale() {
        let center = DVec3::ZERO;
        let scale = DVec3::new(10.0, 10.0, 10.0);
        // Ray down +Z through the centre hits.
        assert!(intersect_ellipsoid(
            DVec3::new(0.0, 0.0, -50.0),
            DVec3::new(0.0, 0.0, 1.0),
            center,
            scale,
            -1.0
        )
        .is_some());
        // Ray parallel to X at z = -50 never approaches the ellipsoid ⇒ miss.
        assert!(intersect_ellipsoid(
            DVec3::new(0.0, 0.0, -50.0),
            DVec3::new(1.0, 0.0, 0.0),
            center,
            scale,
            -1.0
        )
        .is_none());
        // Degenerate scale (< 0.01 on an axis) is rejected.
        assert!(intersect_ellipsoid(
            DVec3::new(0.0, 0.0, -50.0),
            DVec3::new(0.0, 0.0, 1.0),
            center,
            DVec3::new(0.001, 10.0, 10.0),
            -1.0
        )
        .is_none());
    }

    #[test]
    fn gardner_texture_is_finite_and_bounded() {
        for p in [
            DVec3::ZERO,
            DVec3::new(1.0, 2.0, 3.0),
            DVec3::new(-5.0, 0.5, 2.0),
        ] {
            let t = gardner_texture(p);
            assert!(t.is_finite(), "T finite at {p:?}");
            assert!(t.abs() < 10.0, "T bounded (k·sum.x·sum.y): {t}");
        }
    }

    #[test]
    fn cloud_intensity_matches_analytic_endpoints() {
        // I(1,1,1) = 1 (all terms saturated); I(0,0,0) = a (ambient only).
        assert!((cloud_intensity(1.0, 1.0, 1.0) - 1.0).abs() < 1e-12);
        assert!((cloud_intensity(0.0, 0.0, 0.0) - CLOUD_AMBIENT_FRACTION).abs() < 1e-12);
    }

    #[test]
    fn draw_cloud_returns_premultiplied_rgba_and_zero_on_miss() {
        let noise = NoiseVolume::generate(8, 8.0, DVec3::ZERO);
        let center = DVec3::ZERO;
        let scale = DVec3::new(10.0, 10.0, 10.0);
        let rgba = draw_cloud(
            DVec3::new(0.0, 0.0, -50.0),
            DVec3::new(0.0, 0.0, 1.0),
            center,
            scale,
            -1.0,
            1.0,
            [1.0, 1.0, 1.0, 1.0],
            &noise,
            0.5,
        );
        assert!(rgba.iter().all(|c| c.is_finite()), "finite colour");
        assert!((0.0..=1.0).contains(&rgba[3]), "alpha = TR clamped to [0,1]");
        // A perpendicular ray that never approaches the ellipsoid ⇒ vec4(0).
        let miss = draw_cloud(
            DVec3::new(0.0, 0.0, -50.0),
            DVec3::new(1.0, 0.0, 0.0),
            center,
            scale,
            -1.0,
            1.0,
            [1.0, 1.0, 1.0, 1.0],
            &noise,
            0.5,
        );
        assert_eq!(miss, [0.0; 4], "miss returns transparent black");
    }

    #[test]
    fn henyey_greenstein_is_isotropic_at_zero_and_forward_at_positive_g() {
        let iso = henyey_greenstein(0.3, 0.0);
        assert!(
            (iso - 1.0 / (4.0 * std::f64::consts::PI)).abs() < 1e-12,
            "g = 0 ⇒ 1/(4π) for all angles"
        );
        let forward = henyey_greenstein(1.0, HG_PHASE_G);
        let backward = henyey_greenstein(-1.0, HG_PHASE_G);
        assert!(forward > backward, "g = 0.6 forward-scatters");
        assert!(forward.is_finite() && backward.is_finite());
    }

    #[test]
    fn beer_lambert_is_unit_at_zero_distance_and_monotone_decreasing() {
        assert!((beer_lambert_transmittance(0.5, 0.1, 0.0) - 1.0).abs() < 1e-12);
        let t1 = beer_lambert_transmittance(0.5, 0.1, 1.0);
        let t2 = beer_lambert_transmittance(0.5, 0.1, 2.0);
        assert!(t1 < 1.0 && t2 < t1, "transmittance falls with distance");
        assert!(t1 > 0.0 && t2 > 0.0, "never negative");
    }

    #[test]
    fn raymarch_miss_is_fully_transparent() {
        let noise = NoiseVolume::generate(8, 8.0, DVec3::ZERO);
        let r = raymarch_density(
            DVec3::new(0.0, 0.0, -50.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::ZERO,
            DVec3::new(10.0, 10.0, 10.0),
            12,
            &noise,
            0.5,
            0.3,
            1.0,
        );
        assert!((r.transmittance - 1.0).abs() < 1e-12);
        assert_eq!(r.scattered, 0.0);
        assert_eq!(r.steps_taken, 0);
    }

    #[test]
    fn raymarch_clamps_step_count_to_the_documented_range() {
        let noise = NoiseVolume::generate(8, 8.0, DVec3::ZERO);
        let (origin, dir, center, scale) = (
            DVec3::new(0.0, 0.0, -50.0),
            DVec3::new(0.0, 0.0, 1.0),
            DVec3::ZERO,
            DVec3::new(10.0, 10.0, 10.0),
        );
        let low = raymarch_density(origin, dir, center, scale, 1, &noise, 0.5, 0.3, 1.0);
        assert_eq!(low.steps_taken, RAYMARCH_STEPS_MIN, "clamped up to MIN");
        let high = raymarch_density(origin, dir, center, scale, 1000, &noise, 0.5, 0.3, 1.0);
        assert_eq!(high.steps_taken, RAYMARCH_STEPS_MAX, "clamped down to MAX");
    }

    #[test]
    fn raymarch_converges_as_step_count_grows() {
        let noise = NoiseVolume::generate(8, 8.0, DVec3::ZERO);
        let (origin, dir, center, scale) = (
            DVec3::new(0.0, 0.0, -50.0),
            DVec3::new(0.0, 0.0, 1.0),
            DVec3::ZERO,
            DVec3::new(10.0, 10.0, 10.0),
        );
        let r8 = raymarch_density(origin, dir, center, scale, 8, &noise, 0.5, 0.3, 1.0);
        let r12 = raymarch_density(origin, dir, center, scale, 12, &noise, 0.5, 0.3, 1.0);
        let r16 = raymarch_density(origin, dir, center, scale, 16, &noise, 0.5, 0.3, 1.0);
        for r in [r8, r12, r16] {
            assert!(r.transmittance > 0.0 && r.transmittance <= 1.0 + 1e-12);
            assert!(r.scattered >= 0.0, "in-scattered radiance non-negative");
        }
        // Midpoint-rule Riemann convergence: successive differences shrink.
        assert!(
            (r16.scattered - r12.scattered).abs()
                <= (r12.scattered - r8.scattered).abs() + 1e-9,
            "scattered converges"
        );
        assert!(
            (r16.transmittance - r12.transmittance).abs()
                <= (r12.transmittance - r8.transmittance).abs() + 1e-9,
            "transmittance converges"
        );
    }
}
