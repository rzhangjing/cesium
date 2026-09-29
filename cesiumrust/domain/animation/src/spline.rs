//! 样条插值系统。
//!
//! 映射到 CesiumJS：
//! - `Core/Spline.js`（基础）
//! - `Core/LinearSpline.js`
//! - `Core/CatmullRomSpline.js`
//! - `Core/HermiteSpline.js`
//! - `Core/QuaternionSpline.js`
//! - `Core/SteppedSpline.js`（在 CesiumJS 中经由 SteppedSpline）
//! - `Core/ConstantSpline.js`（经由 MorphWeightSpline）

use glam::{DQuat, DVec3};

// ============================================================================
// Spline 特质
// ============================================================================

/// 通用的样条操作。
pub trait Spline {
    /// 获取时间值。
    fn times(&self) -> &[f64];

    /// 查找给定时间对应的时间区间索引。
    /// 返回满足 times[i] <= time <= times[i+1] 的索引 i。
    fn find_time_interval(&self, time: f64) -> usize {
        let times = self.times();
        if times.is_empty() {
            return 0;
        }
        if time <= times[0] {
            return 0;
        }
        let last = times.len() - 1;
        if time >= times[last] {
            return last.saturating_sub(1);
        }
        // 二分查找
        let mut lo = 0;
        let mut hi = last;
        while lo < hi {
            let mid = (lo + hi) / 2;
            if times[mid] <= time && time < times[mid + 1] {
                return mid;
            } else if times[mid] > time {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        lo.min(last.saturating_sub(1))
    }

    /// 将时间环绕到样条的周期。
    fn wrap_time(&self, time: f64) -> f64 {
        let times = self.times();
        if times.len() < 2 {
            return time;
        }
        let start = times[0];
        let end = times[times.len() - 1];
        let duration = end - start;
        if duration <= 0.0 {
            return start;
        }
        let mut t = (time - start) % duration;
        if t < 0.0 {
            t += duration;
        }
        start + t
    }

    /// 将时间夹取到样条的范围内。
    fn clamp_time(&self, time: f64) -> f64 {
        let times = self.times();
        if times.is_empty() {
            return time;
        }
        time.clamp(times[0], times[times.len() - 1])
    }
}

// ============================================================================
// LinearSpline
// ============================================================================

/// 分段线性插值样条。
///
/// 映射到 CesiumJS `Core/LinearSpline.js`。
#[derive(Debug, Clone, PartialEq)]
pub struct LinearSpline {
    /// 时间值（严格递增）。
    pub times: Vec<f64>,
    /// 控制点。
    pub points: Vec<DVec3>,
}

impl LinearSpline {
    /// 创建一个新的线性样条。
    pub fn new(times: Vec<f64>, points: Vec<DVec3>) -> Self {
        assert!(points.len() >= 2, "points.length must be >= 2");
        assert_eq!(times.len(), points.len(), "times and points must match");
        Self { times, points }
    }

    /// 在给定时间处求样条的值。
    pub fn evaluate(&self, time: f64) -> DVec3 {
        let i = self.find_time_interval(time);
        let t0 = self.times[i];
        let t1 = self.times[i + 1];
        let u = if (t1 - t0).abs() > 1e-15 {
            (time - t0) / (t1 - t0)
        } else {
            0.0
        };
        self.points[i].lerp(self.points[i + 1], u)
    }
}

impl Spline for LinearSpline {
    fn times(&self) -> &[f64] {
        &self.times
    }
}

// ============================================================================
// CatmullRomSpline
// ============================================================================

/// 用于平滑 C1 连续曲线的 Catmull-Rom 样条。
///
/// 映射到 CesiumJS `Core/CatmullRomSpline.js`。
#[derive(Debug, Clone, PartialEq)]
pub struct CatmullRomSpline {
    /// 时间值。
    pub times: Vec<f64>,
    /// 控制点。
    pub points: Vec<DVec3>,
    /// 首点处的切线。
    pub first_tangent: DVec3,
    /// 末点处的切线。
    pub last_tangent: DVec3,
}

impl CatmullRomSpline {
    /// 创建一个带有自动计算切线的新 Catmull-Rom 样条。
    ///
    /// 映射到 CesiumJS CatmullRomSpline 构造函数（不带 firstTangent/lastTangent）。
    pub fn new(times: Vec<f64>, points: Vec<DVec3>) -> Self {
        assert!(points.len() >= 2, "points.length must be >= 2");
        assert_eq!(times.len(), points.len(), "times and points must match");

        let n = points.len();
        let (first_tangent, last_tangent) = if n > 2 {
            // CesiumJS: firstTangent = (2*points[1] - points[2] - points[0]) * 0.5
            let ft = (points[1] * 2.0 - points[2] - points[0]) * 0.5;
            // CesiumJS: lastTangent = (points[n-1] - 2*points[n-2] + points[n-3]) * 0.5
            let lt = (points[n - 1] - points[n - 2] * 2.0 + points[n - 3]) * 0.5;
            (ft, lt)
        } else {
            (points[1] - points[0], points[1] - points[0])
        };

        Self {
            times,
            points,
            first_tangent,
            last_tangent,
        }
    }

    /// 使用显式切线创建。
    ///
    /// 映射到带 firstTangent/lastTangent 的 CesiumJS CatmullRomSpline 构造函数。
    pub fn with_tangents(
        times: Vec<f64>,
        points: Vec<DVec3>,
        first_tangent: DVec3,
        last_tangent: DVec3,
    ) -> Self {
        assert!(points.len() >= 2, "points.length must be >= 2");
        assert_eq!(times.len(), points.len(), "times and points must match");
        Self {
            times,
            points,
            first_tangent,
            last_tangent,
        }
    }

    /// 在给定时间处求样条的值。
    ///
    /// 首/末段使用 Hermite 基，内部段使用 Catmull-Rom 矩阵。
    pub fn evaluate(&self, time: f64) -> DVec3 {
        let n = self.points.len();
        if n < 3 {
            // 回退到线性
            let t0 = self.times[0];
            let inv_span = 1.0 / (self.times[1] - t0);
            let u = (time - t0) * inv_span;
            return self.points[0].lerp(self.points[1], u);
        }

        let i = self.find_time_interval(time);
        let t0 = self.times[i];
        let t1 = self.times[i + 1];
        let u = if (t1 - t0).abs() > 1e-15 {
            (time - t0) / (t1 - t0)
        } else {
            0.0
        };

        let u2 = u * u;
        let u3 = u2 * u;

        if i == 0 {
            // 首段：带 firstTangent 的 Hermite
            let p0 = self.points[0];
            let p1 = self.points[1];
            let m0 = self.first_tangent;
            let m1 = (self.points[2] - self.points[0]) * 0.5;

            let h00 = 2.0 * u3 - 3.0 * u2 + 1.0;
            let h10 = u3 - 2.0 * u2 + u;
            let h01 = -2.0 * u3 + 3.0 * u2;
            let h11 = u3 - u2;

            p0 * h00 + m0 * h10 + p1 * h01 + m1 * h11
        } else if i == n - 2 {
            // 末段：带 lastTangent 的 Hermite
            let p0 = self.points[i];
            let p1 = self.points[i + 1];
            let m0 = (self.points[i + 1] - self.points[i - 1]) * 0.5;
            let m1 = self.last_tangent;

            let h00 = 2.0 * u3 - 3.0 * u2 + 1.0;
            let h10 = u3 - 2.0 * u2 + u;
            let h01 = -2.0 * u3 + 3.0 * u2;
            let h11 = u3 - u2;

            p0 * h00 + m0 * h10 + p1 * h01 + m1 * h11
        } else {
            // 内部：Catmull-Rom 系数矩阵
            // Matrix: [-0.5, 1.5, -1.5, 0.5; 1.0, -2.5, 2.0, -0.5; -0.5, 0.0, 0.5, 0.0; 0.0, 1.0, 0.0, 0.0]
            let p0 = self.points[i - 1];
            let p1 = self.points[i];
            let p2 = self.points[i + 1];
            let p3 = self.points[i + 2];

            let c0 = -0.5 * u3 + u2 - 0.5 * u;
            let c1 = 1.5 * u3 - 2.5 * u2 + 1.0;
            let c2 = -1.5 * u3 + 2.0 * u2 + 0.5 * u;
            let c3 = 0.5 * u3 - 0.5 * u2;

            p0 * c0 + p1 * c1 + p2 * c2 + p3 * c3
        }
    }
}

impl Spline for CatmullRomSpline {
    fn times(&self) -> &[f64] {
        &self.times
    }
}

// ============================================================================
// HermiteSpline
// ============================================================================

/// 带显式切线的 Hermite 样条。
///
/// 映射到 CesiumJS `Core/HermiteSpline.js`。
/// inTangents 与 outTangents 的长度为 `points.len() - 1`。
/// 对于段 [i, i+1]：out_tangents[i] 是 points[i] 处的出向切线，
/// in_tangents[i] 是 points[i+1] 处的入向切线。
#[derive(Debug, Clone, PartialEq)]
pub struct HermiteSpline {
    /// 时间值。
    pub times: Vec<f64>,
    /// 控制点。
    pub points: Vec<DVec3>,
    /// 入向切线（长度 = points.len() - 1）。
    pub in_tangents: Vec<DVec3>,
    /// 出向切线（长度 = points.len() - 1）。
    pub out_tangents: Vec<DVec3>,
}

impl HermiteSpline {
    /// 创建一个新的 Hermite 样条。
    /// in_tangents 与 out_tangents 的长度必须等于 points.len() - 1。
    pub fn new(
        times: Vec<f64>,
        points: Vec<DVec3>,
        in_tangents: Vec<DVec3>,
        out_tangents: Vec<DVec3>,
    ) -> Self {
        assert!(points.len() >= 2, "points.length must be >= 2");
        assert_eq!(times.len(), points.len(), "times and points must match");
        assert_eq!(
            in_tangents.len(),
            points.len() - 1,
            "inTangents.length must be points.length - 1"
        );
        assert_eq!(
            out_tangents.len(),
            points.len() - 1,
            "outTangents.length must be points.length - 1"
        );
        Self {
            times,
            points,
            in_tangents,
            out_tangents,
        }
    }

    /// 由每个点处共享的切线创建 C1 连续样条。
    /// 映射到 CesiumJS `HermiteSpline.createC1`。
    pub fn create_c1(times: Vec<f64>, points: Vec<DVec3>, tangents: Vec<DVec3>) -> Self {
        assert!(points.len() >= 2, "points.length must be >= 2");
        assert_eq!(times.len(), points.len(), "times and points must match");
        assert_eq!(tangents.len(), points.len(), "tangents and points must match");
        let out_tangents = tangents[..tangents.len() - 1].to_vec();
        let in_tangents = tangents[1..].to_vec();
        Self { times, points, in_tangents, out_tangents }
    }

    /// 创建自然三次样条（C2 连续）。
    /// 映射到 CesiumJS `HermiteSpline.createNaturalCubic`。
    pub fn create_natural_cubic(times: Vec<f64>, points: Vec<DVec3>) -> Self {
        assert!(points.len() >= 2, "points.length must be >= 2");
        assert_eq!(times.len(), points.len(), "times and points must match");

        if points.len() < 3 {
            let tangent = points[1] - points[0];
            return Self {
                times,
                points,
                in_tangents: vec![tangent],
                out_tangents: vec![tangent],
            };
        }

        let tangents = generate_natural(&points);
        let out_tangents = tangents[..tangents.len() - 1].to_vec();
        let in_tangents = tangents[1..].to_vec();
        Self { times, points, in_tangents, out_tangents }
    }

    /// 创建夹取三次样条（C2，带指定的端点切线）。
    /// 映射到 CesiumJS `HermiteSpline.createClampedCubic`。
    pub fn create_clamped_cubic(
        times: Vec<f64>,
        points: Vec<DVec3>,
        first_tangent: DVec3,
        last_tangent: DVec3,
    ) -> Self {
        assert!(points.len() >= 2, "points.length must be >= 2");
        assert_eq!(times.len(), points.len(), "times and points must match");

        if points.len() < 3 {
            let tangent = points[1] - points[0];
            return Self {
                times,
                points,
                in_tangents: vec![tangent],
                out_tangents: vec![tangent],
            };
        }

        let tangents = generate_clamped(&points, first_tangent, last_tangent);
        let out_tangents = tangents[..tangents.len() - 1].to_vec();
        let in_tangents = tangents[1..].to_vec();
        Self { times, points, in_tangents, out_tangents }
    }

    /// 在给定时间处求样条的值。
    /// 使用带 timesDelta 缩放的 CesiumJS hermite 系数矩阵。
    pub fn evaluate(&self, time: f64) -> DVec3 {
        let i = self.find_time_interval(time);
        let t0 = self.times[i];
        let t1 = self.times[i + 1];
        let times_delta = t1 - t0;
        let u = if times_delta.abs() > 1e-15 {
            (time - t0) / times_delta
        } else {
            0.0
        };

        let u2 = u * u;
        let u3 = u2 * u;
        // Hermite 基来自 hermiteCoefficientMatrix，切线系数按 timesDelta 缩放
        let coef_start = 2.0 * u3 - 3.0 * u2 + 1.0;
        let coef_end = -2.0 * u3 + 3.0 * u2;
        let coef_out = (u3 - 2.0 * u2 + u) * times_delta;
        let coef_in = (u3 - u2) * times_delta;

        self.points[i] * coef_start
            + self.points[i + 1] * coef_end
            + self.out_tangents[i] * coef_out
            + self.in_tangents[i] * coef_in
    }
}

impl Spline for HermiteSpline {
    fn times(&self) -> &[f64] {
        &self.times
    }
}

/// 使用 Thomas 算法求解三对角方程组。
/// 映射到 CesiumJS `TridiagonalSystemSolver.solve`。
pub fn tridiagonal_solve(
    lower: &[f64],
    diagonal: &[f64],
    upper: &[f64],
    right: &[DVec3],
) -> Vec<DVec3> {
    let n = right.len();
    let mut c = vec![0.0f64; upper.len()];
    let mut d = vec![DVec3::ZERO; n];
    let mut x = vec![DVec3::ZERO; n];

    c[0] = upper[0] / diagonal[0];
    d[0] = right[0] * (1.0 / diagonal[0]);

    for i in 1..c.len() {
        let scalar = 1.0 / (diagonal[i] - c[i - 1] * lower[i - 1]);
        c[i] = upper[i] * scalar;
        d[i] = (right[i] - d[i - 1] * lower[i - 1]) * scalar;
    }

    let i = c.len();
    let scalar = 1.0 / (diagonal[i] - c[i - 1] * lower[i - 1]);
    d[i] = (right[i] - d[i - 1] * lower[i - 1]) * scalar;

    x[n - 1] = d[n - 1];
    for i in (0..n - 1).rev() {
        x[i] = d[i] - x[i + 1] * c[i];
    }

    x
}

/// 为自然三次样条生成切线。
/// 映射到 CesiumJS `generateNatural`。
fn generate_natural(points: &[DVec3]) -> Vec<DVec3> {
    let n = points.len();
    let mut l = vec![0.0f64; n - 1];
    let mut u = vec![0.0f64; n - 1];
    let mut d = vec![0.0f64; n];
    let mut r = vec![DVec3::ZERO; n];

    l[0] = 1.0;
    u[0] = 1.0;
    d[0] = 2.0;
    r[0] = (points[1] - points[0]) * 3.0;

    for i in 1..n - 1 {
        l[i] = 1.0;
        u[i] = 1.0;
        d[i] = 4.0;
        r[i] = (points[i + 1] - points[i - 1]) * 3.0;
    }

    d[n - 1] = 2.0;
    r[n - 1] = (points[n - 1] - points[n - 2]) * 3.0;

    tridiagonal_solve(&l, &d, &u, &r)
}

/// 为夹取三次样条生成切线。
/// 映射到 CesiumJS `generateClamped`。
fn generate_clamped(points: &[DVec3], first_tangent: DVec3, last_tangent: DVec3) -> Vec<DVec3> {
    let n = points.len();
    let mut l = vec![0.0f64; n - 1];
    let mut u = vec![0.0f64; n - 1];
    let mut d = vec![0.0f64; n];
    let mut r = vec![DVec3::ZERO; n];

    l[0] = 1.0;
    d[0] = 1.0;
    u[0] = 0.0;
    r[0] = first_tangent;

    for i in 1..n - 2 {
        l[i] = 1.0;
        u[i] = 1.0;
        d[i] = 4.0;
        r[i] = (points[i + 1] - points[i - 1]) * 3.0;
    }

    let i = n - 2;
    l[i] = 0.0;
    u[i] = 1.0;
    d[i] = 4.0;
    r[i] = (points[i + 1] - points[i - 1]) * 3.0;

    d[n - 1] = 1.0;
    r[n - 1] = last_tangent;

    tridiagonal_solve(&l, &d, &u, &r)
}

// ============================================================================
// QuaternionSpline
// ============================================================================

/// 使用 SLERP 插值的四元数样条。
///
/// 映射到 CesiumJS `Core/QuaternionSpline.js`。
#[derive(Debug, Clone, PartialEq)]
pub struct QuaternionSpline {
    /// 时间值。
    pub times: Vec<f64>,
    /// 四元数控制点。
    pub points: Vec<DQuat>,
}

impl QuaternionSpline {
    /// 创建一个新的四元数样条。
    pub fn new(times: Vec<f64>, points: Vec<DQuat>) -> Self {
        assert!(points.len() >= 2, "points.length must be >= 2");
        assert_eq!(times.len(), points.len(), "times and points must match");
        Self { times, points }
    }

    /// 使用 SLERP 在给定时间处求样条的值。
    pub fn evaluate(&self, time: f64) -> DQuat {
        let i = self.find_time_interval(time);
        let t0 = self.times[i];
        let t1 = self.times[i + 1];
        let u = if (t1 - t0).abs() > 1e-15 {
            (time - t0) / (t1 - t0)
        } else {
            0.0
        };
        self.points[i].slerp(self.points[i + 1], u)
    }
}

impl Spline for QuaternionSpline {
    fn times(&self) -> &[f64] {
        &self.times
    }
}

// ============================================================================
// SteppedSpline
// ============================================================================

/// 阶梯（分段常量）样条 —— 保持取值直到下一个关键帧。
///
/// 映射到 CesiumJS `Core/SteppedSpline.js`。
#[derive(Debug, Clone, PartialEq)]
pub struct SteppedSpline {
    /// 时间值。
    pub times: Vec<f64>,
    /// 控制点。
    pub points: Vec<DVec3>,
}

impl SteppedSpline {
    /// 创建一个新的阶梯样条。
    pub fn new(times: Vec<f64>, points: Vec<DVec3>) -> Self {
        assert!(points.len() >= 2, "points.length must be >= 2");
        assert_eq!(times.len(), points.len(), "times and points must match");
        Self { times, points }
    }

    /// 在给定时间处求样条的值（返回上一个关键帧的值）。
    pub fn evaluate(&self, time: f64) -> DVec3 {
        let i = self.find_time_interval(time);
        self.points[i]
    }
}

impl Spline for SteppedSpline {
    fn times(&self) -> &[f64] {
        &self.times
    }
}

// ============================================================================
// ConstantSpline
// ============================================================================

/// 常量样条 —— 始终返回同一个值。
///
/// 映射到 CesiumJS `Core/ConstantSpline.js` / MorphWeightSpline。
#[derive(Debug, Clone, PartialEq)]
pub struct ConstantSpline {
    /// 常量值。
    pub value: DVec3,
    /// 时间范围（用于接口兼容）。
    pub times: Vec<f64>,
}

impl ConstantSpline {
    /// 创建一个新的常量样条。
    pub fn new(value: DVec3) -> Self {
        Self {
            value,
            times: vec![0.0, 1.0],
        }
    }

    /// 使用特定的时间范围创建。
    pub fn with_time_range(value: DVec3, start: f64, end: f64) -> Self {
        Self {
            value,
            times: vec![start, end],
        }
    }

    /// 求值（始终返回常量值）。
    pub fn evaluate(&self, _time: f64) -> DVec3 {
        self.value
    }

    /// 对于常量样条，wrapTime 始终返回 0.0。
    pub fn wrap_time(&self, _time: f64) -> f64 {
        0.0
    }

    /// 对于常量样条，clampTime 始终返回 0.0。
    pub fn clamp_time(&self, _time: f64) -> f64 {
        0.0
    }
}

impl Spline for ConstantSpline {
    fn times(&self) -> &[f64] {
        &self.times
    }
}

// ============================================================================
// MorphWeightSpline
// ============================================================================

/// 用于形变目标权重（标量值）的样条。
///
/// 映射到 CesiumJS `Core/MorphWeightSpline.js`。
#[derive(Debug, Clone, PartialEq)]
pub struct MorphWeightSpline {
    /// 时间值。
    pub times: Vec<f64>,
    /// 权重值（通常在 0.0 到 1.0 之间）。
    pub weights: Vec<f64>,
}

impl MorphWeightSpline {
    /// 创建一个新的形变权重样条。
    pub fn new(times: Vec<f64>, weights: Vec<f64>) -> Self {
        assert!(weights.len() >= 2, "weights.length must be >= 2");
        assert_eq!(times.len(), weights.len(), "times and weights must match");
        Self { times, weights }
    }

    /// 在给定时间处求权重（线性插值）。
    pub fn evaluate(&self, time: f64) -> f64 {
        let i = self.find_time_interval(time);
        let t0 = self.times[i];
        let t1 = self.times[i + 1];
        let u = if (t1 - t0).abs() > 1e-15 {
            (time - t0) / (t1 - t0)
        } else {
            0.0
        };
        self.weights[i] + (self.weights[i + 1] - self.weights[i]) * u
    }
}

impl Spline for MorphWeightSpline {
    fn times(&self) -> &[f64] {
        &self.times
    }
}

// ============================================================================
// ScalarSpline
// ============================================================================

/// 用于标量值的样条（线性插值）。
#[derive(Debug, Clone, PartialEq)]
pub struct ScalarSpline {
    /// 时间值。
    pub times: Vec<f64>,
    /// 标量值。
    pub values: Vec<f64>,
}

impl ScalarSpline {
    /// 创建一个新的标量样条。
    pub fn new(times: Vec<f64>, values: Vec<f64>) -> Self {
        assert!(values.len() >= 2, "values.length must be >= 2");
        assert_eq!(times.len(), values.len(), "times and values must match");
        Self { times, values }
    }

    /// 在给定时间处求标量值。
    pub fn evaluate(&self, time: f64) -> f64 {
        let i = self.find_time_interval(time);
        let t0 = self.times[i];
        let t1 = self.times[i + 1];
        let u = if (t1 - t0).abs() > 1e-15 {
            (time - t0) / (t1 - t0)
        } else {
            0.0
        };
        self.values[i] + (self.values[i + 1] - self.values[i]) * u
    }
}

impl Spline for ScalarSpline {
    fn times(&self) -> &[f64] {
        &self.times
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;

    #[test]
    fn test_linear_spline_endpoints() {
        let spline = LinearSpline::new(
            vec![0.0, 1.0, 2.0],
            vec![
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(1.0, 1.0, 1.0),
                DVec3::new(2.0, 0.0, 0.0),
            ],
        );
        let p0 = spline.evaluate(0.0);
        assert!((p0 - DVec3::new(0.0, 0.0, 0.0)).length() < 1e-10);
        let p1 = spline.evaluate(1.0);
        assert!((p1 - DVec3::new(1.0, 1.0, 1.0)).length() < 1e-10);
        let p2 = spline.evaluate(2.0);
        assert!((p2 - DVec3::new(2.0, 0.0, 0.0)).length() < 1e-10);
    }

    #[test]
    fn test_linear_spline_midpoint() {
        let spline = LinearSpline::new(
            vec![0.0, 2.0],
            vec![DVec3::new(0.0, 0.0, 0.0), DVec3::new(4.0, 2.0, 0.0)],
        );
        let mid = spline.evaluate(1.0);
        assert!((mid - DVec3::new(2.0, 1.0, 0.0)).length() < 1e-10);
    }

    #[test]
    fn test_catmull_rom_spline_endpoints() {
        let spline = CatmullRomSpline::new(
            vec![0.0, 1.0, 2.0, 3.0],
            vec![
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(1.0, 1.0, 0.0),
                DVec3::new(2.0, 0.0, 0.0),
                DVec3::new(3.0, 1.0, 0.0),
            ],
        );
        let p0 = spline.evaluate(0.0);
        assert!((p0 - DVec3::new(0.0, 0.0, 0.0)).length() < 1e-10);
        let p3 = spline.evaluate(3.0);
        assert!((p3 - DVec3::new(3.0, 1.0, 0.0)).length() < 1e-6);
    }

    #[test]
    fn test_hermite_spline() {
        let spline = HermiteSpline::new(
            vec![0.0, 1.0],
            vec![DVec3::new(0.0, 0.0, 0.0), DVec3::new(1.0, 1.0, 0.0)],
            vec![DVec3::new(1.0, 0.0, 0.0)],
            vec![DVec3::new(1.0, 0.0, 0.0)],
        );
        let p0 = spline.evaluate(0.0);
        assert!((p0 - DVec3::new(0.0, 0.0, 0.0)).length() < 1e-10);
        let p1 = spline.evaluate(1.0);
        assert!((p1 - DVec3::new(1.0, 1.0, 0.0)).length() < 1e-10);
    }

    #[test]
    fn test_quaternion_spline() {
        let q0 = DQuat::IDENTITY;
        let q1 = DQuat::from_rotation_z(FRAC_PI_2);
        let spline = QuaternionSpline::new(vec![0.0, 1.0], vec![q0, q1]);
        let r0 = spline.evaluate(0.0);
        assert!((r0.x - q0.x).abs() < 1e-10);
        assert!((r0.w - q0.w).abs() < 1e-10);
        let r1 = spline.evaluate(1.0);
        assert!((r1.z - q1.z).abs() < 1e-6);
    }

    #[test]
    fn test_stepped_spline() {
        let spline = SteppedSpline::new(
            vec![0.0, 1.0, 2.0],
            vec![
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(1.0, 1.0, 1.0),
                DVec3::new(2.0, 2.0, 2.0),
            ],
        );
        let p = spline.evaluate(0.5);
        assert!((p - DVec3::new(0.0, 0.0, 0.0)).length() < 1e-10);
        let p = spline.evaluate(1.5);
        assert!((p - DVec3::new(1.0, 1.0, 1.0)).length() < 1e-10);
    }

    #[test]
    fn test_natural_cubic() {
        let spline = HermiteSpline::create_natural_cubic(
            vec![0.0, 1.0, 2.0, 3.0],
            vec![
                DVec3::new(1.0, 0.0, 0.0),
                DVec3::new(0.0, 1.0, FRAC_PI_2),
                DVec3::new(-1.0, 0.0, std::f64::consts::PI),
                DVec3::new(0.0, -1.0, 3.0 * FRAC_PI_2),
            ],
        );
        let p0 = spline.evaluate(0.0);
        assert!((p0 - DVec3::new(1.0, 0.0, 0.0)).length() < 1e-10);
    }

    #[test]
    fn test_wrap_time() {
        let spline = LinearSpline::new(
            vec![0.0, 1.0, 2.0],
            vec![DVec3::ZERO, DVec3::ONE, DVec3::ZERO],
        );
        assert!((spline.wrap_time(2.5) - 0.5).abs() < 1e-10);
        assert!((spline.wrap_time(-0.5) - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_clamp_time() {
        let spline = LinearSpline::new(
            vec![0.0, 1.0, 2.0],
            vec![DVec3::ZERO, DVec3::ONE, DVec3::ZERO],
        );
        assert!((spline.clamp_time(-1.0) - 0.0).abs() < 1e-10);
        assert!((spline.clamp_time(5.0) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_find_time_interval() {
        let spline = LinearSpline::new(
            vec![0.0, 1.0, 2.0, 3.0],
            vec![DVec3::ZERO, DVec3::ONE, DVec3::ZERO, DVec3::ONE],
        );
        assert_eq!(spline.find_time_interval(0.5), 0);
        assert_eq!(spline.find_time_interval(1.5), 1);
        assert_eq!(spline.find_time_interval(2.5), 2);
        assert_eq!(spline.find_time_interval(0.0), 0);
        assert_eq!(spline.find_time_interval(3.0), 2);
    }
}
