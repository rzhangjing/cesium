//! 全景渲染（Equirectangular + CubeMap）。
//!
//! 映射到 CesiumJS：
//! - `Scene/EquirectangularPanorama.js`（266 行）
//! - `Scene/CubeMapPanorama.js`（352 行）
//! - `Scene/SkyBox.js`（164 行）—— **完全**委托给 `CubeMapPanorama`
//!   （L39-43 `this._panorama = new CubeMapPanorama({...})`，L100 注释
//!   "Delegate completely"），因此 cube-map 全景*就是* skybox 的真值源。
//! - `Scene/PanoramaProvider.js`
//! - `Shaders/SkyBoxVS.glsl`、`Shaders/SkyBoxFS.glsl`、
//!   `Shaders/CubeMapPanoramaVS.glsl`
//!
//! # f64 纪律
//! 本模块中每一个几何量都是 `f64` 并始终保持 `f64`。它被窄化
//! 为 `f32` 的唯一位置是 GPU uniform 边界
//! （`adapters/bevy-render/src/effects/panorama.rs::PanoramaUniforms`）。下面的 `*_render_units`
//! 访问器返回 `f64` 渲染单位——它们转换的是**尺度**，从不
//! 转换**精度**——因此调用方仍自行决定何时取整。
//!
//! # 两种摆放，两种 texture 布局
//! 上游恰好只提供了两种配对，本模块独立地建模两个轴
//! （[`PanoramaPlacement`] × [`PanoramaSource`]）：
//!
//! | upstream primitive           | placement | source        | transform type |
//! |------------------------------|-----------|---------------|----------------|
//! | `CubeMapPanorama` / `SkyBox` | `Skybox`  | `CubeMap`     | **`Matrix3`** |
//! | `EquirectangularPanorama`    | `Bubble`  | `Equirectangular` | `Matrix4` |
//!
//! ## 偏差 —— `CubeMapPanorama::transform` 是 `DMat4`，上游是 `Matrix3`
//! 上游 `CubeMapPanorama` 存储一个 **`Matrix3`**（`CubeMapPanorama.js` L143-149，
//! 在 `CubeMapPanoramaVS.glsl` L1 中绑定为 `uniform mat3 u_cubeMapPanoramaTransform`）：
//! cube-map skybox *总是*以相机为中心，因此它有朝向但无位置。本模块
//! 存储 `DMat4` 以与 [`EquirectangularPanorama`] 保持对称。因此
//! [`CubeMapPanorama::orientation`] 是重现上游 `Matrix3` 的访问器——它丢弃
//! 第四行和第四列，而上游本来就没有它们。`DMat4` 字段保持不动：改变
//! 它的类型将是对一个已发布的现有字段的破坏性语义变更。

use glam::{DMat3, DMat4, DVec2, DVec3, DVec4};

/// 以米为单位的默认全景半径。
///
/// 上游 `EquirectangularPanorama.js` L15 `const DEFAULT_RADIUS = 100000.0;`。
pub const DEFAULT_PANORAMA_RADIUS: f64 = 100000.0;

/// 每渲染单位的米数——项目级的尺度常量。
///
/// 与
/// `adapters/bevy-render/src/resources.rs::METERS_PER_RENDER_UNIT` 值一致的镜像，在此重复
/// 是因为领域层不得依赖适配器（DDD）。由
/// `adapters/bevy-render` 的 `panorama_meters_per_render_unit_matches_the_domain` 断言相等。
///
/// 在此尺度下，上游默认半径为
/// `100_000 / 6_378_137 = 0.015678` 渲染单位——一个大致为地球
/// 半径 1.6 % 的**局部 bubble**，而非无限远的天空。这就是
/// [`PanoramaPlacement`] 究竟为何有两个成员。
pub const PANORAMA_METERS_PER_RENDER_UNIT: f64 = 6_378_137.0;

/// 低于此平方长度时，一个方向被视为退化。
///
/// 对更短的向量做 `normalize` 得到 `0/0 = NaN`，而 NaN 会传播进由它派生的每一个
/// texture 坐标，静默地污染整帧。与大气首帧 `sun_direction == ZERO` bug
/// 属同类缺陷（M5 Ultra Review 发现项 H1）。在
/// `shaders/panorama.wgsl` 中以 f32 镜像为 `DEGENERATE_DIRECTION_SQUARED_EPSILON`；两者是
/// 独立字面量而非一次转换，因此双重舍入无法将它们分离。
pub const DEGENERATE_DIRECTION_SQUARED_EPSILON: f64 = 1.0e-24;

/// 上游 skybox 盒的半长，以盒局部单位计。
///
/// `CubeMapPanorama.js` L189-192：
/// `BoxGeometry.fromDimensions({ dimensions: new Cartesian3(2.0, 2.0, 2.0),
/// vertexFormat: VertexFormat.POSITION_ONLY })`——一个以原点为中心的 2×2×2 盒，
/// 因此每个角坐标都是 `±1`。
pub const SKYBOX_BOX_HALF_EXTENT: f64 = 1.0;

/// 全景相对于相机的摆放方式。
///
/// `u32` 判别值是线格式（wire format）：它们被逐字写入
/// `PanoramaUniforms::mode`，并在 `shaders/panorama.wgsl` 中与 `MODE_SKYBOX` / `MODE_BUBBLE`
/// 比较。因此重排变体是破坏性变更，
/// 并由 [`tests::placement_and_source_discriminants_match_the_shader`] 守护。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum PanoramaPlacement {
    /// 无限、以相机为中心。上游 `CubeMapPanorama` /
    /// `SkyBox`：`pass: Pass.ENVIRONMENT`（`CubeMapPanorama.js` L105-106，注释
    /// "render before everything else"）、`depthTest: { enabled: false }`、
    /// `depthMask: false`。摆放不携深度，因此 GPU 端写入
    /// 反转 Z 的远平面。
    Skybox = 0,
    /// 一个有限球体，半径为 [`EquirectangularPanorama::radius`] 米，由
    /// [`EquirectangularPanorama::transform`] 摆放。上游将其作为一个普通的不透明
    /// `Primitive` 渲染（`EquirectangularPanorama.js` L123-138，
    /// `translucent: false`），因此它正常地做深度测试和深度写入，且相机
    /// 可以在其内部——即街景情形。
    Bubble = 1,
}

impl PanoramaPlacement {
    /// 写入 `PanoramaUniforms::mode` 的线值。
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self as u32
    }
}

/// 全景图像在其 texture 中的布局方式。
///
/// 判别值是 `PanoramaUniforms::source` 的线格式；参见
/// [`PanoramaPlacement`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum PanoramaSource {
    /// 由方向寻址的六个方形面：`[+X, -X, +Y, -Y, +Z, -Z]`
    /// （上游 `SkyBox.js` `createEarthSkyBox` 使用 `px/mx/py/my/pz/mz`）。
    CubeMap = 0,
    /// 一张 2:1 图像，经度在 x、纬度在 y。上游
    /// `EquirectangularPanorama.js` L116 的注释："2:1 360 degrees equirectangular image path"。
    Equirectangular = 1,
}

impl PanoramaSource {
    /// 写入 `PanoramaUniforms::source` 的线值。
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self as u32
    }
}

/// 在球体上渲染的 equirectangular 全景。
///
/// 映射到 CesiumJS `Scene/EquirectangularPanorama.js`。
#[derive(Debug, Clone, PartialEq)]
pub struct EquirectangularPanorama {
    /// 定义位置与朝向的 4x4 变换矩阵。
    pub transform: DMat4,
    /// 图像 URL 或资源标识符。
    pub image: String,
    /// 以米为单位的全景球半径。
    pub radius: f64,
    /// texture 在水平方向重复的次数。
    pub repeat_horizontal: f64,
    /// texture 在垂直方向重复的次数。
    pub repeat_vertical: f64,
    /// 版权/署名字符串。
    pub credit: Option<String>,
    /// 全景是否可见。
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

    /// 由变换与图像创建。
    pub fn with_transform(transform: DMat4, image: impl Into<String>) -> Self {
        Self {
            transform,
            image: image.into(),
            ..Default::default()
        }
    }

    /// 设置半径。
    pub fn set_radius(&mut self, radius: f64) -> &mut Self {
        self.radius = radius;
        self
    }

    /// 设置水平重复。
    pub fn set_repeat_horizontal(&mut self, repeat: f64) -> &mut Self {
        self.repeat_horizontal = repeat;
        self
    }

    /// 设置垂直重复。
    pub fn set_repeat_vertical(&mut self, repeat: f64) -> &mut Self {
        self.repeat_vertical = repeat;
        self
    }

    /// 设置版权信息。
    pub fn set_credit(&mut self, credit: impl Into<String>) -> &mut Self {
        self.credit = Some(credit.into());
        self
    }

    /// 计算给定方向的 texture 坐标。
    ///
    /// direction 应为局部空间中的单位向量。
    /// 返回 [0, 1] 范围内的 (u, v)（在 repeat 之前）。
    pub fn direction_to_uv(&self, direction: glam::DVec3) -> [f64; 2] {
        // 完全性守护，与下方的 `ray_sphere_entry` 及 GPU 同胞
        // （`panorama.wgsl` `direction_to_equirect_uv`）一致：对零 /
        // 非有限方向做 `normalize()` 会产生 NaN，且即使经过 f64 归一化
        // `|z|` 仍可能是 `1.0 + ε`（舍入），因此裸的 `asin` 返回 NaN。钳制
        // 纬度参数（GPU 已经做 `asin(clamp(z, -1, 1))`），并在
        // 退化方向上回退到一个中性 UV，以便 NaN 永不逸出。
        let squared_length = direction.length_squared();
        if !squared_length.is_finite() || squared_length <= DEGENERATE_DIRECTION_SQUARED_EPSILON {
            return [0.5 * self.repeat_horizontal, 0.5 * self.repeat_vertical];
        }
        let dir = direction / squared_length.sqrt();
        // 经度：atan2(y, x) -> [-π, π] -> [0, 1]
        let lon = dir.y.atan2(dir.x);
        let u = (lon + std::f64::consts::PI) / std::f64::consts::TAU;

        // 纬度：asin(clamp(z, -1, 1)) -> [-π/2, π/2] -> [0, 1]
        let lat = dir.z.clamp(-1.0, 1.0).asin();
        let v = (lat + std::f64::consts::FRAC_PI_2) / std::f64::consts::PI;

        [u * self.repeat_horizontal, v * self.repeat_vertical]
    }

    /// 由 texture 坐标计算方向向量。
    ///
    /// UV 应在 [0, 1] 范围内（在除以 repeat 之后）。
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

    // ─── M6.3 补充（忠实于上游的朝向 / 投影语义）──

    /// [`PanoramaPlacement`] × [`PanoramaSource`] 的 texture 布局轴。
    #[inline]
    pub const fn source(&self) -> PanoramaSource {
        PanoramaSource::Equirectangular
    }

    /// 摆放轴：由 [`Self::transform`] 摆放的一个有限球体。
    #[inline]
    pub const fn placement(&self) -> PanoramaPlacement {
        PanoramaPlacement::Bubble
    }

    /// 递给采样器的 texture 重复向量，与上游构建的方式完全一致。
    ///
    /// `EquirectangularPanorama.js` L117：
    /// ```text
    /// repeat: new Cartesian2(-this._repeatHorizontal, this._repeatVertical),
    /// // flip horizontally by default to match expected orientation of images
    /// // inside a sphere, but allow user to override
    /// ```
    ///
    /// ## 偏差 —— [`Self::direction_to_uv`] **不**应用此翻转
    /// 预先存在的 [`Self::direction_to_uv`] 乘以*正的*
    /// `repeat_horizontal`，因此其 u 相对于上游是镜像的。这里
    /// 故意**不**纠正它的符号：[`Self::uv_to_direction`] 是它的精确
    /// 逆运算（`tests::test_equirectangular_uv_roundtrip` 断言了
    /// 1e-10 的往返），因此只取反其中一个会破坏这一对，
    /// 而两个都取则是对两个已发布方法的语义变更。
    /// 下方的 [`Self::sample_uv`] 是忠实于上游的访问器，而 GPU
    /// 路径（`shaders/panorama.wgsl::direction_to_equirect_uv`）使用这个 repeat
    /// 向量——因此渲染的图像与 CesiumJS 一致，而旧的那一对保持
    /// 逐字不变。
    #[inline]
    pub fn texture_repeat(&self) -> DVec2 {
        DVec2::new(-self.repeat_horizontal, self.repeat_vertical)
    }

    /// `direction` 的 GPU texture 坐标，忠实于上游。
    ///
    /// `direction` 是全景局部坐标系中的一个**单位**向量（即在
    /// [`Self::orientation`] 的逆应用之后——与 [`Self::direction_to_uv`]
    /// 工作的同一坐标系）。
    ///
    /// 等于将 [`Self::direction_to_uv`] 的 x 取反，这在代数上与
    /// 将基础 uv 乘以 [`Self::texture_repeat`] 相同：
    /// ```text
    ///   base = (u * repeat_h, v * repeat_v)
    ///   want = (u * (-repeat_h), v * repeat_v) = (-base.x, base.y)
    /// ```
    /// 而 IEEE-754 乘法是精确符号对称的（`(-a) * b == -(a * b)`
    /// 逐位相同），因此两种形式不会漂移。由
    /// [`tests::sample_uv_is_the_horizontal_mirror_of_direction_to_uv`] 断言。
    ///
    /// 只要 repeat 不为 `1`，结果通常**超出** `[0, 1]`，或者由于翻转而对 x 总是如此；
    /// 将它通过 [`wrap_uv`] 运行以得到 `GL_REPEAT` / `AddressMode::Repeat` 采样器会使用的
    /// 坐标。
    pub fn sample_uv(&self, direction: DVec3) -> DVec2 {
        let base = self.direction_to_uv(direction);
        DVec2::new(-base[0], base[1])
    }

    /// [`Self::transform`] 中仅旋转的部分。
    ///
    /// 上游通过 `Transforms.headingPitchRollToFixedFrame`（`EquirectangularPanorama.js`
    /// L46-61）从一个位置加上 heading/pitch/roll 合成它，因此左上角 3×3
    /// 是朝向，第四列是锚点位置——参见 [`Self::center`]。
    #[inline]
    pub fn orientation(&self) -> DMat3 {
        DMat3::from_cols(
            self.transform.x_axis.truncate(),
            self.transform.y_axis.truncate(),
            self.transform.z_axis.truncate(),
        )
    }

    /// 全景球体的锚点位置，以米为单位，与 [`Self::transform`] 处于同一坐标系。
    /// 这就是 bubble 的中心。
    #[inline]
    pub fn center(&self) -> DVec3 {
        self.transform.w_axis.truncate()
    }

    /// 以渲染单位表示的 [`Self::radius`]（仍为 `f64`）。
    ///
    /// 在上游默认值下这是 `100_000 / 6_378_137 = 0.015678…`——一个局部
    /// bubble，而非无限远的天空。参见 [`PANORAMA_METERS_PER_RENDER_UNIT`]。
    #[inline]
    pub fn radius_render_units(&self) -> f64 {
        self.radius / PANORAMA_METERS_PER_RENDER_UNIT
    }

    /// 以渲染单位表示的 [`Self::center`]（仍为 `f64`）。
    #[inline]
    pub fn center_render_units(&self) -> DVec3 {
        self.center() / PANORAMA_METERS_PER_RENDER_UNIT
    }

    /// `origin + t * direction` 进入此全景球体的最近正距离，以米为单位；
    /// 当射线从不到达时返回 `None`。
    ///
    /// 这是 `shaders/panorama.wgsl` 中 `ray_sphere_entry` 的 f64 CPU 参考，
    /// 后者驱动 `MODE_BUBBLE` 分支及其 `frag_depth`。使用几何形式（投影中心）
    /// 而非二次形式 `a t² + b t + c`，因为 `direction` 在其中已归一化，
    /// 因此 `a == 1`，整个 `2a` 分母——及其除零 NaN 路径——
    /// 都消失了。
    ///
    /// 对退化的/非有限的 `direction`、非有限或负的 `radius`、未命中，
    /// 或完全位于 origin 后方的球体返回 `None`。当 origin *在*球内
    /// 时，返回远的交点，这就是街景情形：相机位于 bubble 中心，
    /// 看到远壁的内侧。
    pub fn ray_sphere_entry(&self, origin: DVec3, direction: DVec3) -> Option<f64> {
        ray_sphere_entry(origin, direction, self.center(), self.radius)
    }
}

/// 最近的射线/球体正入射距离，或 `None`。
///
/// [`EquirectangularPanorama::ray_sphere_entry`] 的自由函数形式，供其
/// 球体不是全景的调用方使用（测试、CPU/GPU 交叉校验）。每一项拒绝都是一个
/// *完全性*守护，而非语义选择：NaN 和 `±inf` 永不逸出。
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

/// 按 `GL_REPEAT` / `AddressMode::Repeat` 的方式包裹一个 texture 坐标：
/// `x - floor(x)`，对有限输入给出 `[0, 1)` 内的结果。
///
/// 之所以需要，是因为 [`EquirectangularPanorama::sample_uv`] 故意返回
/// 越界的坐标（上游的水平翻转使每个方向的 x 都为负）。非有限输入
/// 产生 `0.0` 而非 NaN，因此一个垃圾 uniform 会退化为“采样第一个 texel”
/// 而不是污染整帧。
pub fn wrap_uv(uv: DVec2) -> DVec2 {
    DVec2::new(wrap_repeat(uv.x), wrap_repeat(uv.y))
}

/// [`wrap_uv`] 的标量部分。
pub fn wrap_repeat(value: f64) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    let wrapped = value - value.floor();
    // `floor` 对一个精确整数返回自身，因此 `wrapped` 为 `0.0`；达到 `1.0`
    // 的唯一方式是刚好低于一个整数的值的舍入误差。
    if wrapped >= 1.0 {
        return 0.0;
    }
    wrapped
}

/// 由 6 个面图像渲染的 cube map 全景。
///
/// 映射到 CesiumJS `Scene/CubeMapPanorama.js`。
#[derive(Debug, Clone, PartialEq)]
pub struct CubeMapPanorama {
    /// 4x4 变换矩阵。
    pub transform: DMat4,
    /// 6 个面的图像 URL：[+X, -X, +Y, -Y, +Z, -Z]。
    pub faces: [String; 6],
    /// 以米为单位的全景球半径。
    pub radius: f64,
    /// 版权/署名字符串。
    pub credit: Option<String>,
    /// 全景是否可见。
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
    /// 用 6 张面图像创建一个新的 cube map 全景。
    pub fn new(faces: [String; 6]) -> Self {
        Self {
            faces,
            ..Default::default()
        }
    }

    /// 检查是否所有面都有图像。
    pub fn is_complete(&self) -> bool {
        self.faces.iter().all(|f| !f.is_empty())
    }

    /// 判定一个方向向量映射到哪个面。
    ///
    /// 返回面索引（0-5）以及该面上的 (u, v) 坐标。
    pub fn direction_to_face_uv(&self, direction: glam::DVec3) -> (usize, [f64; 2]) {
        let dir = direction.normalize();
        let ax = dir.x.abs();
        let ay = dir.y.abs();
        let az = dir.z.abs();

        if ax >= ay && ax >= az {
            if dir.x > 0.0 {
                // +X 面
                let u = (-dir.z / ax + 1.0) * 0.5;
                let v = (-dir.y / ax + 1.0) * 0.5;
                (0, [u, v])
            } else {
                // -X 面
                let u = (dir.z / ax + 1.0) * 0.5;
                let v = (-dir.y / ax + 1.0) * 0.5;
                (1, [u, v])
            }
        } else if ay >= ax && ay >= az {
            if dir.y > 0.0 {
                // +Y 面
                let u = (dir.x / ay + 1.0) * 0.5;
                let v = (dir.z / ay + 1.0) * 0.5;
                (2, [u, v])
            } else {
                // -Y 面
                let u = (dir.x / ay + 1.0) * 0.5;
                let v = (-dir.z / ay + 1.0) * 0.5;
                (3, [u, v])
            }
        } else if dir.z > 0.0 {
            // +Z 面
            let u = (dir.x / az + 1.0) * 0.5;
            let v = (dir.y / az + 1.0) * 0.5;
            (4, [u, v])
        } else {
            // -Z 面
            let u = (dir.x / az + 1.0) * 0.5;
            let v = (-dir.y / az + 1.0) * 0.5;
            (5, [u, v])
        }
    }

    // ─── M6.3 补充（忠实于上游的朝向 / 投影语义）──

    /// [`PanoramaPlacement`] × [`PanoramaSource`] 的 texture 布局轴。
    #[inline]
    pub const fn source(&self) -> PanoramaSource {
        PanoramaSource::CubeMap
    }

    /// 摆放轴：无限且以相机为中心。
    #[inline]
    pub const fn placement(&self) -> PanoramaPlacement {
        PanoramaPlacement::Skybox
    }

    /// 上游的面顺序，即 `[+X, -X, +Y, -Y, +Z, -Z]`。
    ///
    /// 与 [`Self::faces`] 以及 `SkyBox.js::getDefaultSkyBoxUrl` /
    /// `createEarthSkyBox` 的 `px/mx/py/my/pz/mz` 后缀一致。
    pub const FACE_NAMES: [&'static str; 6] = ["+X", "-X", "+Y", "-Y", "+Z", "-Z"];

    /// 上游 `u_cubeMapPanoramaTransform` 的值：一个 **`Matrix3`**。
    ///
    /// `CubeMapPanorama.js` L143-149 存储一个 `Matrix3`，而
    /// `CubeMapPanoramaVS.glsl` L1 声明 `uniform mat3
    /// u_cubeMapPanoramaTransform`。cube-map skybox 总以相机为中心，
    /// 因此它有朝向却无位置——至于本结构为何仍携带一个 `DMat4`，参见模块级的
    /// 偏差说明。
    #[inline]
    pub fn orientation(&self) -> DMat3 {
        DMat3::from_cols(
            self.transform.x_axis.truncate(),
            self.transform.y_axis.truncate(),
            self.transform.z_axis.truncate(),
        )
    }

    /// 以渲染单位表示的 [`Self::radius`]（仍为 `f64`）。
    #[inline]
    pub fn radius_render_units(&self) -> f64 {
        self.radius / PANORAMA_METERS_PER_RENDER_UNIT
    }

    /// 完全按 OpenGL / WebGPU cube-map 规范定义的 cube 面寻址方式，
    /// 因而也与 wgpu `texture_cube` 的采样方式完全一致。
    ///
    /// 返回与 [`Self::direction_to_face_uv`] 相同的面索引——面**选择**
    /// 规则完全相同——但修正了旧方法与规范产生分歧的那两个面的 `(s, t)`：
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
    /// 其中 `s = (sc / |ma| + 1) / 2` 且 `t = (tc / |ma| + 1) / 2`。
    ///
    /// ## 偏差 —— [`Self::direction_to_face_uv`] 保持原样
    /// 旧方法并**不**就地修正。其已发布的行为由
    /// `tests::test_cubemap_direction_to_face` 以及下游调用方覆盖；静默地翻转两个面
    /// 会改变那些已经依赖它们的人的结果。这个忠于规范的同胞方法正是 GPU 路径
    /// 所验证的依据，而两者之间的分歧由
    /// [`tests::cube_face_uv_legacy_and_spec_diverge_only_on_the_z_faces`] 锁定。
    pub fn direction_to_face_uv_spec(&self, direction: DVec3) -> (usize, [f64; 2]) {
        let dir = direction.normalize();
        let ax = dir.x.abs();
        let ay = dir.y.abs();
        let az = dir.z.abs();

        // 面选择逐字复制自 `direction_to_face_uv`，使两个方法绝不会
        // 对*哪个*面产生分歧，只在它的 (s, t) 上不同。
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

    /// [`Self::direction_to_face_uv_spec`] 的逆运算：由面索引及其 `(s, t)`
    /// 重建单位方向。
    ///
    /// `face` 按 6 取模，因此一个越界的 uniform 会降级而非 panic。
    /// 在一组确定性方向上由
    /// [`tests::cube_face_uv_spec_round_trips_in_both_directions`] 双向回环验证。
    pub fn face_uv_to_direction_spec(&self, face: usize, uv: [f64; 2]) -> DVec3 {
        // s, t 在 [0, 1] -> sc/|ma|, tc/|ma| 在 [-1, 1]
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

/// 由规范的 `(sc, tc, ma)` 三元组得到一个 cube 面的 `(s, t)`：
/// `s = (sc / |ma| + 1) / 2`，`t = (tc / |ma| + 1) / 2`。
///
/// `ma` 以已取绝对值的主分量传入，因此对一个归一化方向它永不为零；
/// 零仍会返回 `0.5` 而非 NaN，因为
/// `0.0 / 0.0` 被调用方的面选择所守护。
#[inline]
fn face_uv(sc: f64, tc: f64, ma: f64) -> [f64; 2] {
    if ma <= 0.0 {
        return [0.5, 0.5];
    }
    [(sc / ma + 1.0) * 0.5, (tc / ma + 1.0) * 0.5]
}

// ─── 上游 skybox 顶点 shader 的 CPU 参考 ─────────────────────

/// [`skybox_vertex_transform`] 的结果：一个变换后的 skybox 顶点。
pub struct SkyBoxVertexOutput {
    /// `czm_projection * vec4(p, 1.0)`——要写入的裁剪空间位置。
    pub clip_position: DVec4,
    /// `position.xyz`——**未变换**的盒坐标，直接用作
    /// cube-map 采样方向（`v_texCoord = position.xyz`）。
    pub texture_coordinate: DVec3,
}

/// `Shaders/CubeMapPanoramaVS.glsl` L8-10 的 f64 CPU 参考（以及，
/// 将 `panorama_orientation` 换成 `czm_temeToPseudoFixed` 后，对应
/// `Shaders/SkyBoxVS.glsl` L7-9）：
/// ```glsl
/// vec3 p = czm_viewRotation * (u_cubeMapPanoramaTransform * (czm_entireFrustum.y * position));
/// gl_Position = czm_projection * vec4(p, 1.0);
/// v_texCoord = position.xyz;
/// ```
///
/// 类型由 `Renderer/AutomaticUniforms.js` 锁定：
/// * `czm_viewRotation` 是一个 **`mat3`**（L329 `uniform mat3 czm_viewRotation;`，
///   L341 `datatype: WebGLConstants.FLOAT_MAT3`）——视图矩阵中仅含旋转的部分，
///   正是它使 skybox 跟随相机而不同时平移。
/// * `czm_entireFrustum` 是一个 **`vec2`** `(near, far)`（L1064 `uniform vec2
///   czm_entireFrustum;`），因此 `.y` 是远平面距离。用它将单位盒缩放，
///   会把 skybox 推到远平面上。
///
/// 乘法顺序至关重要，并原样保留：先缩放，再定向，最后视图旋转。因为
/// `f64` 矩阵–向量乘法在舍入下不满足结合律，重新结合这三步会产生
/// 不同的末位结果——与 M5 Ultra Review 发现项 M2 同类的缺陷。
///
/// ## 为何这是*参考*而非 GPU 路径
/// `shaders/panorama.wgsl` 绘制一个全屏三角形，并在片段阶段重建光线，
/// 而非光栅化一个按远平面缩放的盒，因为 Bevy 使用无穷远反转投影，
/// 其远平面在无穷远（参见该文件头部的偏差 1，以及做了同样事情的
/// `bevy_core_pipeline-0.15.3/src/skybox/skybox.wgsl` L19-46）。本函数
/// 的存在是为了让测试能够证明两者在**采样方向**上一致——那是上游顶点阶段
/// 中唯一并入了片段路径的部分，即 `v_texCoord`。
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

/// 上游 skybox 盒的八个角点。
///
/// `CubeMapPanorama.js` L189-192 构建一个以原点为中心的 `2.0 × 2.0 × 2.0`
/// `BoxGeometry`，因此每个角坐标都是 `±SKYBOX_BOX_HALF_EXTENT`。顺序为
/// `-x` 最快，然后 `-y`，然后 `-z`。
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

/// 用于加载全景数据的全景 provider trait。
pub trait PanoramaProvider {
    /// 获取全景类型名称。
    fn provider_type(&self) -> &str;

    /// 检查 provider 是否就绪。
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

        // 测试前向方向（lon=0, lat=0）
        let dir = DVec3::new(1.0, 0.0, 0.0);
        let uv = pano.direction_to_uv(dir);
        assert!((uv[0] - 0.5).abs() < 1e-10); // lon=0 处 u = 0.5
        assert!((uv[1] - 0.5).abs() < 1e-10); // lat=0 处 v = 0.5

        // 回环
        let dir_back = pano.uv_to_direction(uv[0], uv[1]);
        assert!((dir_back - dir).length() < 1e-10);
    }

    #[test]
    fn test_direction_to_uv_clamps_and_guards_degenerate() {
        // FIX-PANO-UVCLAMP：`normalize` 后 `|z| == 1.0 + ε` 必须不产生 NaN
        //（与 GPU 的 `asin(clamp(z, -1, 1))` 一致）；而退化方向
        // 回退到一个中性的有限 UV 而非 NaN。
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

        // 北极（lat = π/2）
        let north = DVec3::new(0.0, 0.0, 1.0);
        let uv_north = pano.direction_to_uv(north);
        assert!((uv_north[1] - 1.0).abs() < 1e-10);

        // 南极（lat = -π/2）
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

        // +X 方向 -> 面 0
        let (face, uv) = pano.direction_to_face_uv(DVec3::new(1.0, 0.0, 0.0));
        assert_eq!(face, 0);
        assert!((uv[0] - 0.5).abs() < 1e-10);
        assert!((uv[1] - 0.5).abs() < 1e-10);

        // -X 方向 -> 面 1
        let (face, _) = pano.direction_to_face_uv(DVec3::new(-1.0, 0.0, 0.0));
        assert_eq!(face, 1);

        // +Y 方向 -> 面 2
        let (face, _) = pano.direction_to_face_uv(DVec3::new(0.0, 1.0, 0.0));
        assert_eq!(face, 2);

        // -Y 方向 -> 面 3
        let (face, _) = pano.direction_to_face_uv(DVec3::new(0.0, -1.0, 0.0));
        assert_eq!(face, 3);

        // +Z 方向 -> 面 4
        let (face, _) = pano.direction_to_face_uv(DVec3::new(0.0, 0.0, 1.0));
        assert_eq!(face, 4);

        // -Z 方向 -> 面 5
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

    // ─── M6.3 补充 ─────────────────────────────────────────────

    /// `EquirectangularPanorama.js` L117 向 sampler 传入
    /// `Cartesian2(-repeatHorizontal, repeatVertical)`；那一行的注释是
    /// "flip horizontally by default to match expected orientation of images inside
    /// a sphere, but allow user to override"。
    #[test]
    fn texture_repeat_carries_the_upstream_horizontal_flip() {
        let pano = EquirectangularPanorama::default();
        assert_eq!(pano.texture_repeat(), DVec2::new(-1.0, 1.0));

        let mut repeated = EquirectangularPanorama::new("test.jpg");
        repeated.set_repeat_horizontal(2.0).set_repeat_vertical(0.5);
        assert_eq!(repeated.texture_repeat(), DVec2::new(-2.0, 0.5));

        // 这个翻转是无条件的：它在 repeat == 0 和负 repeat 时仍保留，
        // 而上游也把这些值直接传给 sampler。
        let mut degenerate = EquirectangularPanorama::new("test.jpg");
        degenerate.set_repeat_horizontal(0.0).set_repeat_vertical(-3.0);
        assert_eq!(degenerate.texture_repeat(), DVec2::new(-0.0, -3.0));
    }

    /// [`EquirectangularPanorama::sample_uv`] 是忠于上游的坐标；
    /// 旧方法 [`EquirectangularPanorama::direction_to_uv`] 保持其正的水平 repeat，
    /// 以便其逆运算 [`EquirectangularPanorama::uv_to_direction`]
    /// 仍是一个真正的回环。
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

            // 逐位精确的取负：IEEE-754 乘法对符号对称，因此
            // `u * (-repeat)` 与 `-(u * repeat)` 不会分离。
            assert_eq!(sample.x.to_bits(), (-base[0]).to_bits(), "dir {dir:?}");
            assert_eq!(sample.y.to_bits(), base[1].to_bits(), "dir {dir:?}");

            // 并且它等于将 base uv 逐元素乘以 `texture_repeat()`，
            // 这正是 GLSL/WGSL sampler 所做的。
            let repeat = pano.texture_repeat();
            let u = base[0] / pano.repeat_horizontal;
            let v = base[1] / pano.repeat_vertical;
            assert!((sample.x - u * repeat.x).abs() < 1e-12, "dir {dir:?}");
            assert!((sample.y - v * repeat.y).abs() < 1e-12, "dir {dir:?}");
        }

        // 具体的上游校验：在 lon = +pi/2 处 base u 为 0.75，因此对于
        // repeat_horizontal = 2，翻转后的采样 u 为 -1.5。
        let east = pano.sample_uv(DVec3::Y);
        assert!((east.x - (-0.75 * 2.0)).abs() < 1e-12, "{}", east.x);
        assert!((east.y - (0.5 * 3.0)).abs() < 1e-12, "{}", east.y);
    }

    /// `sample_uv` 故意返回越界坐标；[`wrap_uv`] 是契约中
    /// `GL_REPEAT` / `AddressMode::Repeat` 的那一半。
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

        // 非有限输入降级为“首个 texel”，而非将 NaN 传播进整帧的每一个
        // texture 坐标。
        assert_eq!(wrap_repeat(f64::NAN), 0.0);
        assert_eq!(wrap_repeat(f64::INFINITY), 0.0);
        assert_eq!(wrap_repeat(f64::NEG_INFINITY), 0.0);

        let wrapped = wrap_uv(DVec2::new(-0.25, 1.25));
        assert!((wrapped.x - 0.75).abs() < 1e-15, "{}", wrapped.x);
        assert!((wrapped.y - 0.25).abs() < 1e-15, "{}", wrapped.y);
        assert!(wrapped.x >= 0.0 && wrapped.x < 1.0);
        assert!(wrapped.y >= 0.0 && wrapped.y < 1.0);
    }

    /// 枚举的判别值是 `PanoramaUniforms::mode` 和 `::source` 的线格式；
    /// `shaders/panorama.wgsl` 与 `MODE_SKYBOX = 0u`、
    /// `MODE_BUBBLE = 1u`、`SOURCE_CUBEMAP = 0u`、`SOURCE_EQUIRECTANGULAR = 1u` 比较。
    /// 适配器端的测试
    /// `panorama_wgsl_mode_and_source_literals_match_the_domain_discriminants`
    /// 与 shader 源码文本闭环验证。
    #[test]
    fn placement_and_source_discriminants_match_the_shader() {
        assert_eq!(PanoramaPlacement::Skybox.as_u32(), 0);
        assert_eq!(PanoramaPlacement::Bubble.as_u32(), 1);
        assert_eq!(PanoramaSource::CubeMap.as_u32(), 0);
        assert_eq!(PanoramaSource::Equirectangular.as_u32(), 1);

        // 上游的两种配对。
        let cube = CubeMapPanorama::default();
        assert_eq!(cube.placement(), PanoramaPlacement::Skybox);
        assert_eq!(cube.source(), PanoramaSource::CubeMap);

        let equirect = EquirectangularPanorama::default();
        assert_eq!(equirect.placement(), PanoramaPlacement::Bubble);
        assert_eq!(equirect.source(), PanoramaSource::Equirectangular);
    }

    /// 米 -> 渲染单位的换算**只**改变尺度，从不改变精度：
    /// 访问器仍返回 `f64`。
    #[test]
    fn radius_render_units_converts_scale_without_narrowing_precision() {
        assert_eq!(PANORAMA_METERS_PER_RENDER_UNIT, 6_378_137.0);

        let pano = EquirectangularPanorama::default();
        let render_units = pano.radius_render_units();
        let expected = DEFAULT_PANORAMA_RADIUS / PANORAMA_METERS_PER_RENDER_UNIT;
        assert_eq!(render_units.to_bits(), expected.to_bits());

        // 100 km 是一个*局部 bubble*：约占地球 1.0 渲染单位的 1.57 %。这正是
        // `PanoramaPlacement` 有两个成员的全部原因。
        assert!(
            render_units > 0.0156 && render_units < 0.0157,
            "expected ~0.015678 render units, got {render_units}"
        );
        assert!(
            render_units < 1.0,
            "the default panorama must be far smaller than the globe radius"
        );

        // 对于整渲染单位的平移，中心换算是精确的。
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

    /// `orientation()` 是重现上游 `Matrix3` 的访问器；派生它的
    /// `DMat4` 字段保持不变（参见模块级的偏差说明）。
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

        // equirectangular 变体以另一种方式拆分同一个 Matrix4：在那里
        // 第四列*确实*是有意义的，因为那个全景是一个锚定在世界中的 bubble。
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

    /// `direction_to_face_uv`（旧）与 `direction_to_face_uv_spec` 必须在**所有**
    /// 地方就面达成一致，并在除两个 Z 面之外的所有地方就 `(s, t)` 一致。
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
                continue; // 一个退化样本证明不了任何关于面寻址的东西
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

        // 确切的分歧，逐字列出：+Z 翻转 `t`，-Z 翻转 `s`。
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

    /// `direction_to_face_uv_spec` 与 `face_uv_to_direction_spec` 在全部六个面上
    /// 是精确的互逆。
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

        // 另一个方向：每个面的中心都映射回自身的主轴。
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

        // 越界的面索引会回绕而非 panic。
        assert_eq!(
            pano.face_uv_to_direction_spec(6, [0.5, 0.5]),
            pano.face_uv_to_direction_spec(0, [0.5, 0.5])
        );
        assert_eq!(CubeMapPanorama::FACE_NAMES.len(), 6);
    }

    /// 相机在中心的街景情形：光线从 bubble 中心出发，
    /// 恰好在 `radius` 处遇到**远**墙。
    #[test]
    fn ray_sphere_entry_hits_the_far_wall_from_inside_the_bubble() {
        let pano = EquirectangularPanorama::default();
        assert_eq!(pano.center(), DVec3::ZERO);
        assert_eq!(pano.radius, DEFAULT_PANORAMA_RADIUS);

        let hit = pano
            .ray_sphere_entry(DVec3::ZERO, DVec3::X)
            .expect("a centred ray must hit the bubble");
        assert_eq!(hit, DEFAULT_PANORAMA_RADIUS);

        // 采样方向随后就是光线方向本身，这正是使
        // `direction_to_uv`/`sample_uv` 直接从相机光线起就有效的原因。
        let hit_point = DVec3::ZERO + DVec3::X * hit;
        assert!(
            ((hit_point - pano.center()).normalize() - DVec3::X).length() < 1.0e-12
        );

        // 从外部：近墙胜出，与上游那个做了深度测试的不透明球体
        // `cull: { enabled: false }` 一致。
        let near = pano
            .ray_sphere_entry(DVec3::new(-300_000.0, 0.0, 0.0), DVec3::X)
            .expect("must hit");
        assert!((near - 200_000.0).abs() < 1.0e-9, "{near}");

        // 一个无切点的错过，以及一个完全在原点后方的球体。
        assert!(pano
            .ray_sphere_entry(DVec3::new(-300_000.0, 0.0, 0.0), DVec3::Y)
            .is_none());
        assert!(pano
            .ray_sphere_entry(DVec3::new(300_000.0, 0.0, 0.0), DVec3::X)
            .is_none());
    }

    /// 没有任何输入能使 [`ray_sphere_entry`] 返回 NaN、无穷大或
    /// 非正距离。
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

        // 一个短但可表示的方向会被归一化而非拒绝：1e-8
        // 平方得 1e-16，比 `1.0e-24` 的平方长度 epsilon 高八个数量级。
        let short = ray_sphere_entry(
            DVec3::ZERO,
            DVec3::new(1.0e-8, 0.0, 0.0),
            DVec3::ZERO,
            1.0,
        )
        .expect("1e-8 squares to 1e-16, well above the degenerate epsilon");
        assert!((short - 1.0).abs() < 1.0e-12, "{short}");

        // 1e-20 平方得 1e-40，它*低于* epsilon，因此也会被拒绝：
        // 守护是针对平方长度，而非长度。
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

        // 1e-200 平方得 1e-400，它在 f64 中下溢为恰好 0.0：
        // 守护必须捕获那种情况而不是去除以它。
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

        // 无论返回什么总是有限且严格为正的。
        for dir in [DVec3::X, DVec3::Y, DVec3::Z, -DVec3::X] {
            if let Some(t) = pano.ray_sphere_entry(DVec3::ZERO, dir) {
                assert!(t.is_finite() && t > 0.0, "{dir:?} -> {t}");
            }
        }
    }

    /// 交点在米 -> 渲染单位的重缩下放是不变的，
    /// 因为原点、中心和半径都除以同一个常量。那种不变性正是
    /// 使 f64 领域参考能验证 f32 GPU 结果的原因。
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

    /// `CubeMapPanorama.js` L189-192 构建一个以原点为中心的 `2.0 x 2.0 x 2.0`
    /// 盒，因此全部八个角都位于 `+-SKYBOX_BOX_HALF_EXTENT`。
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

    /// `CubeMapPanoramaVS.glsl` L8-10 的 f64 CPU 参考。锁定
    /// 先缩放再定向再视图旋转的顺序，以及 `v_texCoord` 是
    /// **原始**盒坐标这一事实。
    #[test]
    fn skybox_vertex_transform_scales_then_orients_then_view_rotates() {
        let far = 200.0_f64;
        let box_position = DVec3::new(1.0, -1.0, 1.0);

        // 全部取单位阵：p = far * position，clip = (p, 1)。
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
        // `v_texCoord = position.xyz`——原始的盒坐标，绝非缩放后的那个。
        assert_eq!(identity.texture_coordinate, box_position);

        // 乘法顺序是可观察的：交换定向和视图旋转
        // 会得到不同的眼空间点。
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

        // 乘以 `czm_entireFrustum.y` 是一个纯粹的相似变换：将远平面加倍
        // 会将眼空间点加倍。
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

        // 在单位视图/投影下，八个盒角都落在远平面上。
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

    /// 上游的 `v_texCoord` 是全景**局部**方向，这就是为什么
    /// `shaders/panorama.wgsl` 在采样前将世界光线乘以世界->局部的
    /// `uniforms.transform`，而非前向变换。
    #[test]
    fn skybox_texture_coordinate_is_the_panorama_local_direction() {
        let orientation = DMat3::from_rotation_z(std::f64::consts::FRAC_PI_2);
        let out =
            skybox_vertex_transform(DMat3::IDENTITY, orientation, DMat4::IDENTITY, 100.0, DVec3::X);
        assert_eq!(out.texture_coordinate, DVec3::X);

        // 同一个顶点在世界空间中指向 +Y...
        let world_direction = (orientation * DVec3::X).normalize();
        assert!((world_direction - DVec3::Y).length() < 1.0e-15, "{world_direction:?}");

        // ...而逆定向将它直接映回原始盒
        // 坐标，那正是 cube-map 的采样方向。
        let local = orientation.inverse() * world_direction;
        assert!(
            (local - out.texture_coordinate.normalize()).length() < 1.0e-15,
            "{local:?}"
        );

        // cube map 以相机为中心，因此定向从不携带
        // 平移：将它作用于零向量仍得零。
        assert_eq!(orientation * DVec3::ZERO, DVec3::ZERO);
    }
}
