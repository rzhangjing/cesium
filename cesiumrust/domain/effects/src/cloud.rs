//! 云渲染系统（CumulusCloud + CloudCollection）。
//!
//! 建模三种形态：
//! - 单朵 cumulus 云（billboard 表示）
//! - 云集合（批量管理与 GPU buffer 更新）
//! - 云类型枚举（当前仅 Cumulus）

// 遗留 CesiumJS 移植风格技术债（deferred.md #18）；在 M13 lint 清理时重新审视，或本文件在其里程碑被重写时
#![allow(clippy::field_reassign_with_default)]
use glam::{DVec2, DVec3};

// ─── M6.6 新增常量（云集合 / noise 生成 / 集合片元三者的 shader 语义）───
//
// 本行以下的一切都是新增的（M6.6）：原有的 426 行数据模型
// （CumulusCloud / CloudCollection / CloudType）及其测试保持不变。只要存在参考真值源，
// 下面的常量就逐字节镜像它；基于物理的散射常量（HG 相位 `g`、Beer-Lambert 消光）
// 是一个新增扩展——参考实现的 cumulus 云使用 Gardner (1985)
// 正弦 texture + Worley-FBM 侵蚀模型，配合单次光线/椭球相交，
// 而非多步 HG/Beer-Lambert 体积行进。两条路径都提供；参见
// 模块报告 + `docs/deviations.md#dev-032`（草稿）。

/// 立方 noise 体积的边长，以体素计。
/// 镜像参考实现的 `_textureSliceWidth = 128`。128³ = 2_097_152
/// 个体素；在 3 个 Worley 通道下，CPU f64 参考约占 50 MB，而 GPU RGBA8
/// 上传占 8 MB（参见 M6.6 报告中的内存预算说明）。
pub const NOISE_TEXTURE_DIMENSIONS: usize = 128;

/// 上游 2D noise 图集将 128³ 体积打包成的行数。
/// 镜像参考实现的 `_noiseTextureRows = 4`。
pub const NOISE_TEXTURE_ROWS: usize = 4;

/// 每个体素存储的 Worley 通道数（worley0/1/2 → RGB）。
/// 镜像参考实现 noise 生成 shader 的 `main`。
pub const NOISE_CHANNELS: usize = 3;

/// 内散射的 Henyey-Greenstein 各向异性（新增，非上游）。
/// `g = 0.6` 是标准的 cumulus 前向散射瓣。
pub const HG_PHASE_G: f64 = 0.6;

/// Beer-Lambert 消光系数（新增，非上游），每渲染单位。
pub const BEER_LAMBERT_EXTINCTION: f64 = 0.1;

/// 相交前应用于 `maximumSize` 的椭球收缩因子。
/// 镜像参考实现片元 shader `main` 的 `ellipsoidScale = 0.82 * v_maximumSize`。
pub const ELLIPSOID_SCALE_FACTOR: f64 = 0.82;

/// 新增体积路径的最小 raymarch 步数。
pub const RAYMARCH_STEPS_MIN: usize = 8;
/// 新增体积路径的最大 raymarch 步数。
pub const RAYMARCH_STEPS_MAX: usize = 16;
/// 默认 raymarch 步数（中程，质量/开销平衡）。
pub const RAYMARCH_STEPS_DEFAULT: usize = 12;

// Gardner (1985)《Visual Simulation of Clouds》texture 常量——镜像
// 参考实现片元 shader 的对应常量定义段。
/// Gardner texture 图案的对比度（`T0`，L129）。
pub const GARDNER_T0: f64 = 0.6;
/// 归一化系数（`k`，L130）。
pub const GARDNER_K: f64 = 0.1;
/// 基础八度系数（`C0`，L131）。
pub const GARDNER_C0: f64 = 0.8;
/// 基础 X 频率（`FX0`，L132）。
pub const GARDNER_FX0: f64 = 0.6;
/// 基础 Y 频率（`FY0`，L133）。
pub const GARDNER_FY0: f64 = 0.6;
/// Gardner 八度数（`octaves`，L134）。
pub const GARDNER_OCTAVES: usize = 5;
/// 环境/散射光比例（`a`，L151）。
pub const CLOUD_AMBIENT_FRACTION: f64 = 0.5;
/// texture 着色比例（`t`，L152）。
pub const CLOUD_TEXTURE_FRACTION: f64 = 0.4;
/// 镜面反射比例（`s`，L153）。
pub const CLOUD_SPECULAR_FRACTION: f64 = 0.25;
/// 固定的云光照方向——镜像参考实现片元 shader 的
/// `normalize(vec3(0.2, -1.0, 0.7))`。
pub const CLOUD_LIGHT_DIR: DVec3 = DVec3::new(0.2, -1.0, 0.7);

/// Worley FBM 迭代上限——镜像参考实现 noise shader 的 `MAX_FBM_ITERATIONS`。
pub const MAX_FBM_ITERATIONS: usize = 10;
/// Worley FBM 基础持续度——镜像参考实现 noise shader 的持续度常量。
pub const WORLEY_FBM_PERSISTENCE: f64 = 0.625;
/// 避免自相交的小表面偏移——镜像 `czm_epsilon2`
/// （参考实现片元 shader 的对应常量）。
pub const CZM_EPSILON2: f64 = 1e-5;

/// 取小数部分（`x - floor(x)`），结果落在 `[0, 1)`。
#[inline]
fn fract(x: f64) -> f64 {
    x - x.floor()
}

/// 逐分量的 [`fract`]，用于三维向量。
#[inline]
fn fract3(v: DVec3) -> DVec3 {
    DVec3::new(fract(v.x), fract(v.y), fract(v.z))
}

/// 逐分量向下取整，用于三维向量。
#[inline]
fn floor3(v: DVec3) -> DVec3 {
    DVec3::new(v.x.floor(), v.y.floor(), v.z.floor())
}

/// 参考实现 noise / 片元 shader 中 `wrap` 的镜像：
/// 正模运算，对负输入也保持在 `[0, range_length)` 内。
pub fn wrap(value: f64, range_length: f64) -> f64 {
    if value < 0.0 {
        let abs_value = value.abs();
        let mod_value = abs_value % range_length;
        (range_length - mod_value) % range_length
    } else {
        value % range_length
    }
}

/// 逐分量的 [`wrap`]——镜像参考实现的 `wrapVec`。
pub fn wrap_vec(value: DVec3, range_length: f64) -> DVec3 {
    DVec3::new(
        wrap(value.x, range_length),
        wrap(value.y, range_length),
        wrap(value.z, range_length),
    )
}

/// 参考实现 `random3` 的镜像：由单元格中心得到一个
/// 类哈希的伪随机点，位于 `[0,1)³`。CPU f64 参考——GPU
/// （`cloud_noise.wgsl`）以 f32 镜像同一个表达式。
pub fn worley_random3(p: DVec3) -> DVec3 {
    let dot1 = p.dot(DVec3::new(127.1, 311.7, 932.8));
    let dot2 = p.dot(DVec3::new(269.5, 183.3, 421.4));
    DVec3::new(
        fract((dot1 - dot2).sin()),
        fract((dot1 * dot2).cos()),
        fract(dot1 * dot2),
    )
}

/// 参考实现 `getWorleyCellPoint` 的镜像：定位邻域单元格内的抖动特征点。
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

/// 参考实现 `worleyNoise` 的镜像：`p`（乘以 `freq`）到
/// 3×3×3 邻域内最近抖动单格中心的最短距离。结果在
/// `[0, ~1.5]`（一个单元格对角线）。
pub fn worley_noise(
    p: DVec3,
    freq: f64,
    detail: f64,
    noise_offset: DVec3,
    slice_width: f64,
) -> f64 {
    // 先把查询点缩放到 Worley 网格，分离所在单元格与单元格局部坐标。
    let center_cell = floor3(p * freq);
    let point_in_cell = fract3(p * freq);
    // 用足够大的哨兵值起始终于取最小距离。
    let mut shortest_distance = 1000.0_f64;
    // 遍历 3×3×3 邻域单元格，逐一生成抖动特征点并取最近距离。
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

/// 参考实现 `worleyFBMNoise`：`octaves` 个 Worley，
/// 频率倍增 / 持续度减半，求和。
pub fn worley_fbm(
    p: DVec3,
    octaves: usize,
    scale: f64,
    detail: f64,
    noise_offset: DVec3,
    slice_width: f64,
) -> f64 {
    // 累加器、当前频率、当前振幅（持续度逐八度减半）。
    let mut noise = 0.0_f64;
    let mut freq = 1.0_f64;
    let mut persistence = WORLEY_FBM_PERSISTENCE;
    for i in 0..MAX_FBM_ITERATIONS {
        // 达到请求的八度数即停（受 MAX_FBM_ITERATIONS 上限约束）。
        if i >= octaves {
            break;
        }
        // 叠加本八度的 Worley 噪声，按当前振幅加权。
        noise += worley_noise(p * scale, freq * scale, detail, noise_offset, slice_width)
            * persistence;
        // 下一八度：振幅减半、频率倍增。
        persistence *= 0.5;
        freq *= 2.0;
    }
    noise
}

/// 云类型枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CloudType {
    /// Cumulus 云（基于 billboard）。
    #[default]
    Cumulus,
}

/// 3D 场景中的单个 cumulus 云 billboard。
#[derive(Debug, Clone, PartialEq)]
pub struct CumulusCloud {
    /// 云是否可见。
    pub show: bool,
    /// 云的世界位置。
    pub position: DVec3,
    /// billboard 尺度（宽，高），以米计。
    pub scale: [f64; 2],
    /// 云体积的最大尺寸 (x, y, z)，以米计。
    pub maximum_size: DVec3,
    /// 云的横截面切片 [0, 1]，若为负则无切片。
    pub slice: f64,
    /// 亮度乘数 [0, 1]。
    pub brightness: f64,
    /// 以 RGBA 表示的云颜色 [0, 1]。
    pub color: [f64; 4],
    /// 在集合中的内部索引。
    index: i32,
}

impl Default for CumulusCloud {
    /// 默认云：可见、原点位置、20×12 billboard、默认体积、无切片、全亮白色。
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
    /// 由位置与最大尺寸创建一个新的 cumulus 云。
    pub fn new(position: DVec3, maximum_size: DVec3) -> Self {
        let scale = [maximum_size.x, maximum_size.y];
        Self {
            position,
            scale,
            maximum_size,
            ..Default::default()
        }
    }

    /// 用完整选项创建。
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

    /// 获取云在集合中的索引。
    pub fn index(&self) -> i32 {
        self.index
    }

    /// 考虑 slice 后计算有效的 billboard 尺寸。
    pub fn effective_dimensions(&self) -> [f64; 2] {
        if self.slice >= 0.0 && self.slice <= 1.0 {
            // 被切片的云看起来更小
            let factor = 1.0 - (self.slice - 0.5).abs() * 0.5;
            [self.scale[0] * factor, self.scale[1] * factor]
        } else {
            self.scale
        }
    }

    /// 检查 slice 值是否处于推荐范围 [0.1, 0.9] 内。
    pub fn is_slice_recommended(&self) -> bool {
        self.slice < 0.0 || (self.slice >= 0.1 && self.slice <= 0.9)
    }
}

/// 3D 场景中可渲染的云集合。
#[derive(Debug, Clone)]
pub struct CloudCollection {
    /// 是否显示云。
    pub show: bool,
    /// noise texture 中期望的细节量。
    pub noise_detail: f64,
    /// noise texture 中期望的数据平移量。
    pub noise_offset: DVec3,
    /// 调试用：以不透明颜色渲染 billboard。
    pub debug_billboards: bool,
    /// 调试用：将云渲染为不透明椭球。
    pub debug_ellipsoids: bool,
    /// 本集合中的云。
    clouds: Vec<CumulusCloud>,
    /// 集合是否需要 GPU buffer 更新。
    dirty: bool,
}

impl Default for CloudCollection {
    /// 默认集合：显示云、noise_detail=16、零偏移、无调试模式、空列表、初始为 dirty。
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
    /// 创建一个空的云集合。
    pub fn new() -> Self {
        Self::default()
    }

    /// 由 noise 参数创建。
    pub fn with_noise(noise_detail: f64, noise_offset: DVec3) -> Self {
        Self {
            noise_detail,
            noise_offset,
            ..Default::default()
        }
    }

    /// 向集合添加一朵云。返回云的索引。
    pub fn add(&mut self, mut cloud: CumulusCloud) -> usize {
        let index = self.clouds.len();
        cloud.index = index as i32;
        self.clouds.push(cloud);
        self.dirty = true;
        index
    }

    /// 按索引移除一朵云。
    pub fn remove(&mut self, index: usize) -> Option<CumulusCloud> {
        if index < self.clouds.len() {
            let cloud = self.clouds.remove(index);
            // 重新索引剩余的云
            for (i, c) in self.clouds.iter_mut().enumerate().skip(index) {
                c.index = i as i32;
            }
            self.dirty = true;
            Some(cloud)
        } else {
            None
        }
    }

    /// 移除所有云。
    pub fn remove_all(&mut self) {
        self.clouds.clear();
        self.dirty = true;
    }

    /// 按索引获取一朵云。
    pub fn get(&self, index: usize) -> Option<&CumulusCloud> {
        self.clouds.get(index)
    }

    /// 按索引获取一朵云的可变引用。
    pub fn get_mut(&mut self, index: usize) -> Option<&mut CumulusCloud> {
        if index < self.clouds.len() {
            self.dirty = true;
            self.clouds.get_mut(index)
        } else {
            None
        }
    }

    /// 获取云的数量。
    pub fn len(&self) -> usize {
        self.clouds.len()
    }

    /// 检查集合是否为空。
    pub fn is_empty(&self) -> bool {
        self.clouds.is_empty()
    }

    /// 获取所有云。
    pub fn clouds(&self) -> &[CumulusCloud] {
        &self.clouds
    }

    /// 仅获取可见的云。
    pub fn visible_clouds(&self) -> impl Iterator<Item = &CumulusCloud> {
        self.clouds.iter().filter(|c| c.show)
    }

    /// 检查集合是否需要 GPU 更新。
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// 将集合标记为干净（在 GPU 更新之后）。
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// 计算所有可见云的总包围球。
    pub fn compute_bounding_sphere(&self) -> Option<(DVec3, f64)> {
        let visible: Vec<_> = self.visible_clouds().collect();
        if visible.is_empty() {
            return None;
        }

        // 简单的形心 + 最大距离方法
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

// ─── Perlin 梯度 noise（新增——Perlin-Worley 中的“Perlin”那一半）───────

/// 经典 Perlin 置换表（Ken Perlin 2002 改进 noise 的排序）。
/// 确定性；GPU `cloud_noise.wgsl` 镜像同一张表。
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

/// Perlin 平滑插值曲线 `6t⁵ − 15t⁴ + 10t³`，输入在 `[0, 1]`。
#[inline]
fn perlin_fade(t: f64) -> f64 {
    // 6t⁵ − 15t⁴ + 10t³（不做 FMA 收缩——三次独立舍入）。
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// 排列表查找：将任意整数掩码到 `[0, 255]` 后取置换值。
#[inline]
fn perlin_perm(index: usize) -> usize {
    PERLIN_PERM[index & 255] as usize
}

/// 依据哈希值选取梯度方向并与偏移向量点乘，返回单角贡献。
fn perlin_grad(hash: usize, x: f64, y: f64, z: f64) -> f64 {
    // 12 个梯度方向（Perlin 的改进集），归约到 8 个立方角。
    let h = hash & 15;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 { y } else if h == 12 || h == 14 { x } else { z };
    let u_term = if (h & 1) == 0 { u } else { -u };
    let v_term = if (h & 2) == 0 { v } else { -v };
    u_term + v_term
}

/// 近似位于 `[-1, 1]` 的经典改进 Perlin 3D 梯度 noise。
/// 新增：参考实现的 cumulus 云存储纯 Worley-FBM 通道；
/// Perlin 基底是标准 Perlin-Worley 云模型（Decima / Hillaire）中低频的“形状”
/// 那一半，在此为体积路径提供。
pub fn perlin_noise_3d(p: DVec3) -> f64 {
    // 取所在单位立方体的整数角坐标与单元格局部小数坐标。
    let x = p.x.floor() as i64;
    let y = p.y.floor() as i64;
    let z = p.z.floor() as i64;
    let xf = p.x - x as f64;
    let yf = p.y - y as f64;
    let zf = p.z - z as f64;
    // 对三个轴的小数坐标做平滑曲线，得到插值权重。
    let u = perlin_fade(xf);
    let v = perlin_fade(yf);
    let w = perlin_fade(zf);
    // 掩码到排列表范围后，级联查表得到 8 个角的哈希。
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
    // 先在 x 方向插值 4 对角，再在 y、z 方向逐级插值收敛为标量。
    let x1 = lerp(lerp(aaa, baa, u), lerp(aba, bba, u), v);
    let x2 = lerp(lerp(aab, bab, u), lerp(abb, bbb, u), v);
    lerp(x1, x2, w)
}

/// 线性插值 `a + (b − a)·t`，`t` 在 `[0, 1]`。
#[inline]
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    // 两次舍入（无 FMA）：a + (b − a)·t 使减法与乘/加保持独立，
    // 与 WGSL 参考一致。
    a + (b - a) * t
}

/// 一个 CPU f64 参考 noise 体积：`dimensions³` 个体素 × [`NOISE_CHANNELS`]
/// 个 Worley-FBM 通道，镜像参考实现 noise shader 的 `main`。
///
/// GPU 路径（`cloud_noise.wgsl`）将同一个体积生成到一个 `texture_3d`；
/// 本结构是单元测试使用的确定性参考，并（可选地）在 3D texture
/// 不可用时作为 CPU 上传回退。`data` 布局为体素内 channel-major、z 最慢：
/// `index = ((z·dim + y)·dim + x)·CHANNELS + c`。
#[derive(Debug, Clone, PartialEq)]
pub struct NoiseVolume {
    /// 以体素计的边长（生产环境 = [`NOISE_TEXTURE_DIMENSIONS`] = 128）。
    pub dimensions: usize,
    /// Worley-FBM 细节除数——镜像 `u_noiseDetail`（默认 16）。
    pub detail: f64,
    /// noise 平移——镜像 `u_noiseOffset`。
    pub noise_offset: DVec3,
    /// 扁平体素数据，`dimensions³ · NOISE_CHANNELS` 个 `[0, 1]` 内的 f64 值。
    pub data: Vec<f64>,
}

impl NoiseVolume {
    /// 生成一个 `dimensions³` 的 Worley-FBM 体积。镜像参考实现 noise shader 的
    /// `main`：每个体素中心 `position = (x, y, z) / detail` 产生三个
    /// 被钳制的 `worley_fbm(position, 3 octaves, scale ∈ {1, 2, 3})` 通道。
    ///
    /// 开销为 `dimensions³ · 3 · (3 octaves · 27 cells)`；生产环境 128³ 是一个
    /// 一次性的 GPU/上传步骤，而测试使用较小的 `dimensions`（≤ 16）。
    pub fn generate(dimensions: usize, detail: f64, noise_offset: DVec3) -> Self {
        // 切片宽度即体积边长，供 wrap 归一使用。
        let slice_width = dimensions as f64;
        // 扁平体素缓冲：每体素 NOISE_CHANNELS 个通道。
        let mut data = vec![0.0_f64; dimensions * dimensions * dimensions * NOISE_CHANNELS];
        for z in 0..dimensions {
            for y in 0..dimensions {
                for x in 0..dimensions {
                    // 体素中心缩放到 Worley 空间坐标。
                    let position = DVec3::new(x as f64, y as f64, z as f64) / detail;
                    // z 最慢、体素内 channel-major 的线性索引基址。
                    let base = ((z * dimensions + y) * dimensions + x) * NOISE_CHANNELS;
                    // 三个通道分别用 scale 1/2/3 的 Worley-FBM 填充并钳到 [0,1]。
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

    /// 获取整数体素 `(x, y, z)` 处的原始 3 通道值（回绕）。
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

    /// 在连续的体素空间 `position` 处对体积做三线性插值。
    /// 镜像参考实现片元 shader 的 `sampleNoiseTexture`：先以
    /// 半个切片宽度重新居中，然后 `floor`/`fract` + 三轴 `mix`。
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

    /// 将体积打包为 RGBA8 字节用于 GPU 上传边界（唯一的
    /// f64 → u8 投影）。Alpha 为 255；RGB 为三个 Worley 通道。
    /// 布局要么匹配一个 `128 × (128·ROWS)` 的 2D 图集，要么匹配一个 `128³` 的 3D texture
    /// （z 最慢），因此同一批字节要么服务于图集要么服务于 D3 路径。
    pub fn to_rgba8_bytes(&self) -> Vec<u8> {
        // 预分配：每体素由 3 通道扩为 4 字节 RGBA。
        let mut out = Vec::with_capacity(self.data.len() / NOISE_CHANNELS * 4);
        for voxel in self.data.chunks_exact(NOISE_CHANNELS) {
            // 每通道 [0,1] → 8-bit，RGB 之后补不透明 alpha=255。
            for &channel in voxel {
                out.push((channel.clamp(0.0, 1.0) * 255.0).round() as u8);
            }
            out.push(255);
        }
        out
    }
}

// ─── Billboard 几何（镜像参考实现的顶点 shader）──────────────────────

/// 两三角形四边形索引——镜像参考实现的 `textureIndices`。
pub const BILLBOARD_INDICES: [u32; 6] = [0, 1, 2, 0, 2, 3];

/// 一朵云 quad 的四个角 UV——镜像参考实现顶点的 `coordinates` 属性
/// （`offset = dir - vec2(0.5, 0.5)`）。
pub const BILLBOARD_CORNER_UVS: [[f64; 2]; 4] =
    [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

/// 一朵 cumulus 云的面向相机四边形（4 个世界空间顶点）。
///
/// 镜像参考实现的顶点 shader：各角由云中心沿相机的 right/up 基偏移
/// `scale * (uv - 0.5)`，因此每个 billboard 都平行于视平面（与参考在眼空间中所做的一样，屏幕对齐）。
#[derive(Debug, Clone, PartialEq)]
pub struct BillboardGeometry {
    /// 世界空间角位置（度量 f64），与 [`BILLBOARD_CORNER_UVS`] 对应。
    pub positions: [[f64; 3]; 4],
    /// 每角在 `[0,1]²` 内的 UV。
    pub uvs: [[f64; 2]; 4],
    /// 三角索引（[`BILLBOARD_INDICES`]）。
    pub indices: [u32; 6],
    /// 四边形构建围绕的云中心。
    pub center: DVec3,
    /// 保证从云指向相机（FROM 云 TOWARD 相机）的单位法线。
    pub normal: DVec3,
}

impl CloudCollection {
    /// 为 `cloud` 构建一个面向相机的 billboard 四边形。
    ///
    /// `cam_right` / `cam_up` 是（世界空间的）相机基向量；四边形位于
    /// 它们张成的平面上，以 `cloud.position` 为中心，由
    /// [`CumulusCloud::effective_dimensions`] 定尺。`camera_position` 仅用于
    /// 将 [`BillboardGeometry::normal`] 转向眼睛（对坐标系手性稳健）。
    pub fn build_billboard(
        cloud: &CumulusCloud,
        camera_position: DVec3,
        cam_right: DVec3,
        cam_up: DVec3,
    ) -> BillboardGeometry {
        // 采用考虑 slice 后的有效尺寸，并把相机基归一化。
        let dims = cloud.effective_dimensions();
        let right = cam_right.normalize();
        let up = cam_up.normalize();
        // 由 right×up 得到四边形法线，随后按手性翻正。
        let mut normal = right.cross(up);
        // 保证无论基的手性如何，法线都面向相机。
        if normal.dot(camera_position - cloud.position) < 0.0 {
            normal = -normal;
        }
        // 逐角把 UV 映射为以中心为原点的偏移，再沿相机基展开到世界空间。
        let mut positions = [[0.0_f64; 3]; 4];
        for (i, uv) in BILLBOARD_CORNER_UVS.iter().enumerate() {
            // offset = uv - 0.5，scaled = 尺寸 × offset（参考实现顶点主流程）。
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

    /// 为每一朵可见云构建 billboard（跳过 `show == false`）。
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

// ─── Gardner (1985) texture + 强度（镜像参考实现片元 shader）───────

/// 参考实现 `phaseShift2D` 的镜像：二维正弦相移。
fn phase_shift_2d(p: DVec2, freq: DVec2) -> DVec2 {
    let half_pi = std::f64::consts::FRAC_PI_2;
    DVec2::new(half_pi * (freq.y * p.y).sin(), half_pi * (freq.x * p.x).sin())
}

/// 参考实现 `phaseShift3D` 的镜像：引入 z 维相位偏移。
fn phase_shift_3d(p: DVec3, freq: DVec2) -> DVec2 {
    let s = (freq.x * p.z).sin();
    phase_shift_2d(DVec2::new(p.x, p.y), freq)
        + DVec2::new(std::f64::consts::PI * s, std::f64::consts::PI * s)
}

/// 参考实现 `T` 的镜像：Gardner 的正弦和云
/// texture 函数。`Ci *= 0.707` 和 `FXY *= 2.0` 在每个八度使用之前发生。
pub fn gardner_texture(point: DVec3) -> f64 {
    let mut sum = DVec2::ZERO;
    let mut ci = GARDNER_C0;
    let mut fxy = DVec2::new(GARDNER_FX0, GARDNER_FY0);
    for _ in 1..=GARDNER_OCTAVES {
        // 每个八度：先算相移，再衰减振幅、倍增频率。
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

/// 参考实现 `I` 的镜像：以固定的环境/texture/镜面
/// 比例组合漫反射（`id`）、镜面（`is`）与 texture（`it`）项。
pub fn cloud_intensity(id: f64, is: f64, it: f64) -> f64 {
    // 按环境/texture/镜面的嵌套比例混合三项（Gardner 的 I 表达式）。
    let a = CLOUD_AMBIENT_FRACTION;
    let t = CLOUD_TEXTURE_FRACTION;
    let s = CLOUD_SPECULAR_FRACTION;
    (1.0 - a) * ((1.0 - t) * ((1.0 - s) * id + s * is) + t * it) + a
}

// ─── 光线 / 椭球相交（镜像参考实现片元 shader）─────────────

/// 一个光线/椭球相交：表面 `point`、单位球 `normal`，以及
/// 光线参数 `t`。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EllipsoidHit {
    pub point: DVec3,
    pub normal: DVec3,
    pub t: f64,
}

/// 参考实现 `intersectSphere` 的镜像：与原点处
/// 半径 0.5 的单位球相交，并尊重可选的 `slice` 平面。
pub fn intersect_sphere(origin: DVec3, dir: DVec3, slice: f64) -> Option<EllipsoidHit> {
    // 二次方程 |o + t·d|² = 0.5² 的系数（半径 0.5 → r² = 0.25）。
    let a = dir.dot(dir);
    let b = origin.dot(dir);
    let c = origin.dot(origin) - 0.25;
    // 判别式为负即无实交点，直接返回 None。
    let discriminant = (b * b) - (a * c);
    if discriminant < 0.0 {
        return None;
    }
    // 取入射（较近）交点，若为负则改用出射交点。
    let root = discriminant.sqrt();
    let mut t = (-b - root) / a;
    if t < 0.0 {
        t = (-b + root) / a;
    }
    let mut point = origin + dir * t;
    if slice >= 0.0 {
        // point.z = (slice / 2.0) - 0.5（GLSL swizzle 写入 → 整向量重建）。
        point = DVec3::new(point.x, point.y, slice / 2.0 - 0.5);
        if point.length() > 0.5 {
            return None;
        }
    }
    let normal = point.normalize();
    let point = point - CZM_EPSILON2 * normal;
    Some(EllipsoidHit { point, normal, t })
}

/// 参考实现 `intersectEllipsoid` 的镜像：将光线
/// 变换到单位球空间，相交，再将点映回。`normal` 保持在单位球空间
/// （上游并**不**对它重新缩放）。
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

/// 参考实现 `drawCloud` 的镜像：忠实的参考
/// cumulus 着色——单次椭球相交，Gardner texture + Worley-FBM 侵蚀，
/// 返回预乘的 `rgba`（alpha = 半透明度 `TR`）。
///
/// `noise` / `noise_detail` 驱动 `sampleNoiseTexture(u_noiseDetail * point)`
/// 调用（L175）。错过时返回 `[0,0,0,0]`（上游返回 `vec4(0.0)`）。
///
/// `#[allow(clippy::too_many_arguments)]`：9 参数的列表是有意对上游 `drawCloud`
/// 的 uniform 输入（光线、椭球、着色、noise）做 1:1 镜像。将它们
/// 折叠进参数结构体会模糊本参考所必须保证的领域-WGSL 平价，且尚无
/// 生产调用方（适配器从 GPU uniform 构造这些参数）。参见 `docs/deviations.md#dev-032`。
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
    // 单次椭球相交：错过即完全透明（返回 [0,0,0,0]）。
    let hit = match intersect_ellipsoid(ray_origin, ray_dir, center, scale, slice) {
        Some(hit) => hit,
        None => return [0.0; 4],
    };
    // 固定光照方向下计算三类着色分量：漫反射、镜面、Gardner texture。
    let light_dir = CLOUD_LIGHT_DIR.normalize();
    let id = hit.normal.dot(-light_dir).clamp(0.0, 1.0); // 漫反射
    let is = (-light_dir).dot(-ray_dir).max(0.0).powi(2); // 镜面
    let it = gardner_texture(hit.point); // texture
    // 按固定比例合成三项，再乘亮度得到明暗强度。
    let intensity = cloud_intensity(id, is, it);
    let shaded = intensity * brightness.clamp(0.1, 1.0);

    // 采样 noise 体积得到三个 Worley 侵蚀通道 w/w2/w3。
    let n = noise.sample_trilinear(hit.point * noise_detail);
    let w = n[0];
    let w2 = n[1];
    let w3 = n[2];

    // 视线-法线夹角驱动的基础半透明度，再逐通道做侵蚀修正。
    let nd_dot = hit.normal.dot(-ray_dir).clamp(0.0, 1.0);
    let mut tr = nd_dot.powi(3) - w; // 半透明度
    tr *= 1.3;
    let minus_dot = 0.5 - nd_dot;
    tr -= (minus_dot * w2).min(0.0);
    tr -= 0.8 * (minus_dot + 0.25) * w3;

    // 依侵蚀量调制明暗，并钳制到合理亮度范围。
    let mut shading = lerp(1.0 - 0.8 * w * w, 1.0, id * tr);
    shading = (shading + 0.2).clamp(0.3, 1.0);

    // 最终色 = mix(灰底, 着色×颜色, 1.15)，返回时再乘云色（预乘 alpha = TR）。
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

// ─── 基于物理的散射（新增——非上游 cumulus）───────────

/// Henyey-Greenstein 相位函数（在球面上归一化）。`g = 0` 是
/// 各向同性；`g > 0` 是前向散射。新增扩展：上游 cumulus
/// 使用 Gardner 的镜面/漫反射项，而非相位函数。
pub fn henyey_greenstein(cos_theta: f64, g: f64) -> f64 {
    let g2 = g * g;
    let denom = 1.0 + g2 - 2.0 * g * cos_theta;
    // 守护退化的分母（g → ±1 且 cos_theta → ±1）。
    if denom.abs() < 1e-12 {
        return 0.0;
    }
    (1.0 - g2) / (4.0 * std::f64::consts::PI * denom.powf(1.5))
}

/// 一个步长上的 Beer-Lambert 透射率 `exp(-density · extinction · distance)`。
pub fn beer_lambert_transmittance(density: f64, extinction: f64, distance: f64) -> f64 {
    (-(density * extinction * distance)).exp()
}

/// 新增体积 raymarch 穿过一个云椭球的结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaymarchResult {
    /// 行进后的累积透射率（在 `(0, 1]` 内）。
    pub transmittance: f64,
    /// 累积的内散射辐射（HG 相位 × 吸收能量）。
    pub scattered: f64,
    /// 实际取的步数。
    pub steps_taken: usize,
}

/// 光线进入/离开单位球空间椭球的近/远光线参数
/// （两个根，近端钳制到 0）。错过时返回 `None`。
fn ellipsoid_interval(
    origin: DVec3,
    dir: DVec3,
    center: DVec3,
    scale: DVec3,
) -> Option<(f64, f64)> {
    // 退化尺度直接判负；否则把光线变换到单位球空间。
    if scale.x <= 0.01 || scale.y < 0.01 || scale.z < 0.01 {
        return None;
    }
    let o = (origin - center) / scale;
    let d = dir / scale;
    // 单位球（半径 0.5）上的二次方程系数。
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

/// 新增的体积 raymarch：在椭球入口与出口之间行进 `steps`
/// （钳制到 `[RAYMARCH_STEPS_MIN, RAYMARCH_STEPS_MAX]`）个采样，累积 Beer-Lambert
/// 消光与 HG 内散射。这是上游单次相交 `drawCloud` 的基于物理的替代
/// 方案；两者都对外暴露，以便渲染路径在保真度与开销之间选择。
///
/// `#[allow(clippy::too_many_arguments)]`：与 [`draw_cloud`] 同样的理由——该
/// 签名镜像体积 raymarch 的 uniform 集（光线、椭球、行进、
/// noise、散射），而非惯用的 Rust 分组。
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
    // 步数钳制到允许区间，避免开销失控。
    let steps = steps.clamp(RAYMARCH_STEPS_MIN, RAYMARCH_STEPS_MAX);
    // 求光线与椭球的进/出区间；错过即完全透明、无散射。
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
    // 均分间距，并预先算出固定 HG 相位。
    let dt = (t1 - t0) / steps as f64;
    let phase = henyey_greenstein(cos_theta, HG_PHASE_G);
    let mut transmittance = 1.0_f64;
    let mut scattered = 0.0_f64;
    let mut steps_taken = 0_usize;
    // 取每步中点采样密度，累积 Beer-Lambert 消光与 HG 内散射能量。
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
        // 无切片（负值）=> 完整尺度
        assert_eq!(cloud.effective_dimensions(), [20.0, 12.0]);

        // 切片在 0.5 => factor = 1.0
        cloud.slice = 0.5;
        let dims = cloud.effective_dimensions();
        assert!((dims[0] - 20.0).abs() < 1e-10);

        // 切片在 0.0 => factor = 0.75
        cloud.slice = 0.0;
        let dims = cloud.effective_dimensions();
        assert!((dims[0] - 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_cumulus_cloud_slice_recommended() {
        let mut cloud = CumulusCloud::default();
        assert!(cloud.is_slice_recommended()); // -1.0 是可以的

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
        // 剩余的云应被重新索引
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

    // ─── M6.6 新增测试：noise / billboard / 相交 / 散射 ────

    #[test]
    fn noise_worley_fbm_non_negative_monotone_in_octaves_and_deterministic() {
        let p = DVec3::new(0.3, 0.7, 0.1);
        let w1 = worley_fbm(p, 1, 1.0, 16.0, DVec3::ZERO, 128.0);
        let w3 = worley_fbm(p, 3, 1.0, 16.0, DVec3::ZERO, 128.0);
        assert!(w1 >= 0.0 && w1.is_finite(), "worley distance is non-negative");
        // 每个八度都添加一个非负项（持续度 > 0）⇒ 3 ≥ 1。
        assert!(w3 >= w1 - 1e-12, "more octaves only add energy");
        // 确定性参考（GPU f32 镜像会宽松地交叉校验）。
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
        // 在恰好为整数体素（重新居中）处的三线性插值返回那个体素。
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
        // 连续性：相距 1e-3 的点差异 < 0.05。
        let a = perlin_noise_3d(DVec3::new(1.0, 2.0, 3.0));
        let b = perlin_noise_3d(DVec3::new(1.001, 2.0, 3.0));
        assert!((a - b).abs() < 0.05, "Lipschitz-continuous");
    }

    #[test]
    fn billboard_faces_camera_and_is_symmetric_about_center() {
        let cloud = CumulusCloud::new(DVec3::ZERO, DVec3::new(20.0, 12.0, 8.0));
        let cam_pos = DVec3::new(0.0, 0.0, 100.0);
        let bb = CloudCollection::build_billboard(&cloud, cam_pos, DVec3::X, DVec3::Y);
        // 法线从云指向相机（此处为 +Z）。
        assert!(bb.normal.dot(cam_pos - bb.center) > 0.0, "faces camera");
        assert!((bb.normal - DVec3::Z).length() < 1e-9, "right×up = +Z");
        // 4 个角的形心 == 云中心。
        let mut centroid = DVec3::ZERO;
        for p in &bb.positions {
            centroid += DVec3::from(*p);
        }
        centroid /= 4.0;
        assert!((centroid - bb.center).length() < 1e-9, "symmetric");
        // X 展开 == 有效宽度 (20)，Y 展开 == 有效高度 (12)。
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
        // slice = 0.5 ⇒ point.z = 0.25 - 0.5 = -0.25（± epsilon 法线偏移）。
        assert!((hit.point.z - (-0.25)).abs() < 1e-3, "z pinned to slice plane");
        assert!(hit.point.length() <= 0.5 + 1e-9, "inside the sphere");
    }

    #[test]
    fn intersect_ellipsoid_hits_misses_and_rejects_degenerate_scale() {
        let center = DVec3::ZERO;
        let scale = DVec3::new(10.0, 10.0, 10.0);
        // 沿 +Z 穿过中心的光线会命中。
        assert!(intersect_ellipsoid(
            DVec3::new(0.0, 0.0, -50.0),
            DVec3::new(0.0, 0.0, 1.0),
            center,
            scale,
            -1.0
        )
        .is_some());
        // 在 z = -50 处平行于 X 的光线从不靠近椭球 ⇒ 错过。
        assert!(intersect_ellipsoid(
            DVec3::new(0.0, 0.0, -50.0),
            DVec3::new(1.0, 0.0, 0.0),
            center,
            scale,
            -1.0
        )
        .is_none());
        // 退化尺度（某轴 < 0.01）会被拒绝。
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
        // I(1,1,1) = 1（所有项饱和）；I(0,0,0) = a（仅环境项）。
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
        // 一条从不靠近椭球的垂直光线 ⇒ vec4(0)。
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
        // 中点法则 Riemann 收敛：连续差值逐渐缩小。
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
