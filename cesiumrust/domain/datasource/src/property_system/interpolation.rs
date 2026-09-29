//! 采样属性的插值算法。
//!
//! 映射到 CesiumJS：
//! - `Core/LinearApproximation.js`
//! - `Core/LagrangePolynomialApproximation.js`
//! - `Core/HermitePolynomialApproximation.js`
//! - `DataSources/ExtrapolationType.js`
//!
//! 所有算法都在打包的 `f64` 表上操作，与 CesiumJS 完全一致：
//! - `x_table`：自变量值（时间，以秒计），递增顺序。
//! - `y_table`：因变量值；当每个样本有 `y_stride` 个分量时，其布局
//!   为 `{p1, q1, w1, p2, q2, w2, ...}`。

use cesium_geospatial::math_utils::factorial;

/// 决定当查询超出可用数据边界时，插值的结果如何被外推。
///
/// 映射到 CesiumJS `DataSources/ExtrapolationType.js`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ExtrapolationType {
    /// 不进行外推；在样本范围之外的值为 undefined。
    #[default]
    None,
    /// 在样本数据范围之外时使用第一个或最后一个值。
    Hold,
    /// 对外推该值。
    Extrapolate,
}

impl ExtrapolationType {
    /// CesiumJS 所使用的数值（NONE=0、HOLD=1、EXTRAPOLATE=2）。
    pub fn to_u32(self) -> u32 {
        match self {
            ExtrapolationType::None => 0,
            ExtrapolationType::Hold => 1,
            ExtrapolationType::Extrapolate => 2,
        }
    }

    /// 由 CesiumJS 的数值创建外推类型。
    pub fn from_u32(value: u32) -> Self {
        match value {
            1 => ExtrapolationType::Hold,
            2 => ExtrapolationType::Extrapolate,
            _ => ExtrapolationType::None,
        }
    }
}

/// 用于插值打包因变量表的算法。
///
/// 映射到 CesiumJS `InterpolationAlgorithm` 接口，由
/// `LinearApproximation`、`LagrangePolynomialApproximation` 与
/// `HermitePolynomialApproximation` 实现。
pub trait InterpolationAlgorithm: Send + Sync {
    /// 算法类型名（`"Linear"`、`"Lagrange"`、`"Hermite"`）。
    fn name(&self) -> &'static str;

    /// 给定所需次数，返回插值所需的数据点数量。
    ///
    /// `input_order` 是输入的阶数（0 表示仅有数据，
    /// 1 表示数据及其导数，依此类推）。
    fn get_required_data_points(&self, degree: usize, input_order: usize) -> usize;

    /// 此算法是否实现 `interpolate`（即支持
    /// 导数输入/输出）。在 CesiumJS 中仅 Hermite 支持。
    fn supports_derivatives(&self) -> bool {
        false
    }

    /// 使用该算法插值（零阶，无导数）。
    ///
    /// 返回一个包含 `y_stride` 个插值分量的 `Vec`。
    fn interpolate_order_zero(
        &self,
        x: f64,
        x_table: &[f64],
        y_table: &[f64],
        y_stride: usize,
    ) -> Vec<f64>;

    /// 使用导数输入与输出进行插值。
    ///
    /// 默认实现（用于那些在 CesiumJS 中未定义
    /// `interpolate` 的算法）回退到零阶插值，并将导数输出
    /// 填充为零。
    fn interpolate(
        &self,
        x: f64,
        x_table: &[f64],
        y_table: &[f64],
        y_stride: usize,
        _input_order: usize,
        output_order: usize,
    ) -> Vec<f64> {
        let mut result = self.interpolate_order_zero(x, x_table, y_table, y_stride);
        result.resize(y_stride * (output_order + 1), 0.0);
        result
    }
}

/// 线性插值。
///
/// 映射到 CesiumJS `Core/LinearApproximation.js`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LinearApproximation;

impl InterpolationAlgorithm for LinearApproximation {
    fn name(&self) -> &'static str {
        "Linear"
    }

    /// 由于线性插值只能生成一次多项式，
    /// 因此总是返回 2。
    fn get_required_data_points(&self, _degree: usize, _input_order: usize) -> usize {
        2
    }

    fn interpolate_order_zero(
        &self,
        x: f64,
        x_table: &[f64],
        y_table: &[f64],
        y_stride: usize,
    ) -> Vec<f64> {
        debug_assert_eq!(
            x_table.len(),
            2,
            "The xTable provided to the linear interpolator must have exactly two elements."
        );
        debug_assert!(
            y_stride > 0,
            "There must be at least 1 dependent variable for each independent variable."
        );

        let mut result = vec![0.0; y_stride];
        let x0 = x_table[0];
        let x1 = x_table[1];
        debug_assert_ne!(x0, x1, "Divide by zero error: xTable[0] and xTable[1] are equal");

        for i in 0..y_stride {
            let y0 = y_table[i];
            let y1 = y_table[i + y_stride];
            result[i] = ((y1 - y0) * x + x1 * y0 - x0 * y1) / (x1 - x0);
        }
        result
    }
}

/// Lagrange 多项式插值。
///
/// 映射到 CesiumJS `Core/LagrangePolynomialApproximation.js`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LagrangePolynomialApproximation;

impl InterpolationAlgorithm for LagrangePolynomialApproximation {
    fn name(&self) -> &'static str {
        "Lagrange"
    }

    fn get_required_data_points(&self, degree: usize, _input_order: usize) -> usize {
        (degree + 1).max(2)
    }

    fn interpolate_order_zero(
        &self,
        x: f64,
        x_table: &[f64],
        y_table: &[f64],
        y_stride: usize,
    ) -> Vec<f64> {
        let mut result = vec![0.0; y_stride];
        let length = x_table.len();

        for i in 0..length {
            let mut coefficient = 1.0;
            for j in 0..length {
                if j != i {
                    let diff_x = x_table[i] - x_table[j];
                    coefficient *= (x - x_table[j]) / diff_x;
                }
            }
            for j in 0..y_stride {
                result[j] += coefficient * y_table[i * y_stride + j];
            }
        }
        result
    }
}

/// Hermite 多项式插值（支持导数的
/// 差商）。
///
/// 映射到 CesiumJS `Core/HermitePolynomialApproximation.js`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HermitePolynomialApproximation;

impl InterpolationAlgorithm for HermitePolynomialApproximation {
    fn name(&self) -> &'static str {
        "Hermite"
    }

    fn get_required_data_points(&self, degree: usize, input_order: usize) -> usize {
        ((degree + 1) / (input_order + 1)).max(2)
    }

    fn supports_derivatives(&self) -> bool {
        true
    }

    fn interpolate_order_zero(
        &self,
        x: f64,
        x_table: &[f64],
        y_table: &[f64],
        y_stride: usize,
    ) -> Vec<f64> {
        let length = x_table.len();
        let mut result = vec![0.0; y_stride];
        if length == 0 || y_stride == 0 {
            return result;
        }

        // coefficients[s][i] 为分量 s 保存差商表的行。
        let mut coefficients: Vec<Vec<Vec<f64>>> =
            (0..y_stride).map(|_| (0..length).map(|_| Vec::new()).collect()).collect();

        let z_indices: Vec<usize> = (0..length).collect();

        let mut highest_non_zero_coef = length - 1;
        for (s, coef_s) in coefficients.iter_mut().enumerate() {
            for &z in &z_indices {
                let index = z * y_stride + s;
                coef_s[0].push(y_table[index]);
            }

            for i in 1..length {
                let mut non_zero_coefficients = false;
                for j in 0..(length - i) {
                    let zj = x_table[z_indices[j]];
                    let zn = x_table[z_indices[j + i]];

                    let numerator;
                    if zn - zj <= 0.0 {
                        let index = z_indices[j] * y_stride + y_stride * i + s;
                        numerator = y_table[index];
                        coef_s[i].push(numerator / factorial(i as u32) as f64);
                    } else {
                        numerator = coef_s[i - 1][j + 1] - coef_s[i - 1][j];
                        coef_s[i].push(numerator / (zn - zj));
                    }
                    non_zero_coefficients = non_zero_coefficients || numerator != 0.0;
                }

                if !non_zero_coefficients {
                    highest_non_zero_coef = i - 1;
                }
            }
        }

        // 在 interpolateOrderZero 中，外层循环只会以 d = 0 运行
        // （在 CesiumJS 中为 `for (d = 0, len = 0; d <= len; d++)`）。
        // 显式的索引循环模仿 CesiumJS 的差商累加，
        // 无法用普通的迭代器 zip 表达。
        #[allow(clippy::needless_range_loop)]
        for i in 0..=highest_non_zero_coef {
            let temp_term =
                calculate_coefficient_term(x, &z_indices, x_table, 0, i, &mut Vec::new());
            for (result_s, coef_s) in result.iter_mut().zip(coefficients.iter()) {
                *result_s += coef_s[i][0] * temp_term;
            }
        }

        result
    }

    fn interpolate(
        &self,
        x: f64,
        x_table: &[f64],
        y_table: &[f64],
        y_stride: usize,
        input_order: usize,
        output_order: usize,
    ) -> Vec<f64> {
        let result_length = y_stride * (output_order + 1);
        let mut result = vec![0.0; result_length];

        let length = x_table.len();
        // zIndices 数组保存我们所查看范围内 xTable 值
        // 地址的副本。
        let z_len = length * (input_order + 1);
        let mut z_indices = vec![0usize; z_len];
        for i in 0..length {
            for j in 0..(input_order + 1) {
                z_indices[i * (input_order + 1) + j] = i;
            }
        }

        let tmp = z_len * (z_len + 1) / 2;
        let mut coefficients = vec![0.0f64; y_stride * tmp];
        let highest_non_zero_coef = fill_coefficient_list(
            &mut coefficients,
            &z_indices,
            x_table,
            y_table,
            y_stride,
            input_order,
        );

        let loop_stop = highest_non_zero_coef.min(output_order as isize);
        if loop_stop < 0 {
            return result;
        }
        for d in 0..=loop_stop as usize {
            for i in d..=highest_non_zero_coef as usize {
                let temp_term = calculate_coefficient_term(
                    x,
                    &z_indices,
                    x_table,
                    d,
                    i,
                    &mut Vec::new(),
                );
                let dim_two = row_offset(i, z_len);
                for s in 0..y_stride {
                    let dim_one = s * tmp;
                    let coef = coefficients[dim_one + dim_two];
                    result[s + d * y_stride] += coef * temp_term;
                }
            }
        }

        result
    }
}

/// 差商行 `i` 在打包系数缓冲区内的偏移。
///
/// 行 `i` 保存 `z_len - i` 个条目；CesiumJS 将其计算为
/// `Math.floor((i * (1 - i)) / 2) + zIndicesLength * i`。
fn row_offset(i: usize, z_len: usize) -> usize {
    let signed = (i as isize) * (1 - i as isize) / 2 + (z_len * i) as isize;
    signed as usize
}

fn fill_coefficient_list(
    coefficients: &mut [f64],
    z_indices: &[usize],
    x_table: &[f64],
    y_table: &[f64],
    y_stride: usize,
    input_order: usize,
) -> isize {
    let mut highest_non_zero: isize = -1;
    let z_len = z_indices.len();
    let tmp = z_len * (z_len + 1) / 2;

    for s in 0..y_stride {
        let dim_one = s * tmp;

        for j in 0..z_len {
            let index = z_indices[j] * y_stride * (input_order + 1) + s;
            coefficients[dim_one + j] = y_table[index];
        }

        for i in 1..z_len {
            let mut coef_index = 0usize;
            let dim_two = row_offset(i, z_len);
            let mut non_zero_coefficients = false;

            for j in 0..(z_len - i) {
                let zj = x_table[z_indices[j]];
                let zn = x_table[z_indices[j + i]];

                let numerator;
                if zn - zj <= 0.0 {
                    let index = z_indices[j] * y_stride * (input_order + 1) + y_stride * i + s;
                    numerator = y_table[index];
                    coefficients[dim_one + dim_two + coef_index] =
                        numerator / factorial(i as u32) as f64;
                    coef_index += 1;
                } else {
                    let dim_two_minus_one = row_offset(i - 1, z_len);
                    numerator = coefficients[dim_one + dim_two_minus_one + j + 1]
                        - coefficients[dim_one + dim_two_minus_one + j];
                    coefficients[dim_one + dim_two + coef_index] = numerator / (zn - zj);
                    coef_index += 1;
                }
                non_zero_coefficients = non_zero_coefficients || numerator != 0.0;
            }

            if non_zero_coefficients {
                highest_non_zero = highest_non_zero.max(i as isize);
            }
        }
    }

    highest_non_zero
}

/// 计算 Newton 形式系数多项式的一项，或它的某个导数。
///
/// 当 `deriv_order == 0` 时，这是对所有非保留的 `i < term_order`
/// 求 `(x - xTable[zIndices[i]])` 的乘积。当 `deriv_order > 0` 时，
/// 它是该乘积的 `deriv_order` 阶导数，通过递归地
/// 对所有逐一保留一个因子的方式求和计算得出。
fn calculate_coefficient_term(
    x: f64,
    z_indices: &[usize],
    x_table: &[f64],
    deriv_order: usize,
    term_order: usize,
    reserved_indices: &mut Vec<usize>,
) -> f64 {
    if deriv_order > 0 {
        let mut result = 0.0;
        for i in 0..term_order {
            if !reserved_indices.contains(&i) {
                reserved_indices.push(i);
                result += calculate_coefficient_term(
                    x,
                    z_indices,
                    x_table,
                    deriv_order - 1,
                    term_order,
                    reserved_indices,
                );
                reserved_indices.pop();
            }
        }
        return result;
    }

    let mut result = 1.0;
    for i in 0..term_order {
        if !reserved_indices.contains(&i) {
            result *= x - x_table[z_indices[i]];
        }
    }
    result
}

/// 内置的插值算法，可按值选择。
///
/// 此枚举分发到 [`LinearApproximation`]、
/// [`LagrangePolynomialApproximation`] 与 [`HermitePolynomialApproximation`]，
/// 并由 `SampledProperty` 用于存储所选的算法。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InterpolationAlgorithmKind {
    #[default]
    Linear,
    Lagrange,
    Hermite,
}

impl InterpolationAlgorithmKind {
    /// 返回此种类对应的算法对象。
    pub fn algorithm(&self) -> &'static dyn InterpolationAlgorithm {
        match self {
            InterpolationAlgorithmKind::Linear => &LinearApproximation,
            InterpolationAlgorithmKind::Lagrange => &LagrangePolynomialApproximation,
            InterpolationAlgorithmKind::Hermite => &HermitePolynomialApproximation,
        }
    }

    /// 解析 CesiumJS 的算法类型名（`"Linear"`、`"Lagrange"`、
    /// `"Hermite"`）。
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Linear" => Some(InterpolationAlgorithmKind::Linear),
            "Lagrange" => Some(InterpolationAlgorithmKind::Lagrange),
            "Hermite" => Some(InterpolationAlgorithmKind::Hermite),
            _ => None,
        }
    }
}

impl InterpolationAlgorithm for InterpolationAlgorithmKind {
    fn name(&self) -> &'static str {
        self.algorithm().name()
    }

    fn get_required_data_points(&self, degree: usize, input_order: usize) -> usize {
        self.algorithm().get_required_data_points(degree, input_order)
    }

    fn supports_derivatives(&self) -> bool {
        self.algorithm().supports_derivatives()
    }

    fn interpolate_order_zero(
        &self,
        x: f64,
        x_table: &[f64],
        y_table: &[f64],
        y_stride: usize,
    ) -> Vec<f64> {
        self.algorithm()
            .interpolate_order_zero(x, x_table, y_table, y_stride)
    }

    fn interpolate(
        &self,
        x: f64,
        x_table: &[f64],
        y_table: &[f64],
        y_stride: usize,
        input_order: usize,
        output_order: usize,
    ) -> Vec<f64> {
        self.algorithm()
            .interpolate(x, x_table, y_table, y_stride, input_order, output_order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-12;

    #[test]
    fn test_extrapolation_type_roundtrip() {
        assert_eq!(ExtrapolationType::None.to_u32(), 0);
        assert_eq!(ExtrapolationType::Hold.to_u32(), 1);
        assert_eq!(ExtrapolationType::Extrapolate.to_u32(), 2);
        assert_eq!(ExtrapolationType::from_u32(0), ExtrapolationType::None);
        assert_eq!(ExtrapolationType::from_u32(1), ExtrapolationType::Hold);
        assert_eq!(ExtrapolationType::from_u32(2), ExtrapolationType::Extrapolate);
        assert_eq!(ExtrapolationType::default(), ExtrapolationType::None);
    }

    #[test]
    fn test_linear_required_data_points() {
        assert_eq!(LinearApproximation.get_required_data_points(1, 0), 2);
        assert_eq!(LinearApproximation.get_required_data_points(5, 0), 2);
        assert_eq!(LinearApproximation.get_required_data_points(9, 2), 2);
    }

    #[test]
    fn test_linear_interpolate_midpoint() {
        // y = 2x + 1，在 x = 0 与 x = 10 处采样（每个样本两个分量）。
        let x_table = [0.0, 10.0];
        let y_table = [1.0, -1.0, 21.0, 19.0];
        let result = LinearApproximation.interpolate_order_zero(5.0, &x_table, &y_table, 2);
        assert!((result[0] - 11.0).abs() < EPS);
        assert!((result[1] - 9.0).abs() < EPS);
    }

    #[test]
    fn test_linear_interpolate_at_nodes() {
        let x_table = [-4.0, 2.0];
        let y_table = [3.0, 7.0];
        let r0 = LinearApproximation.interpolate_order_zero(-4.0, &x_table, &y_table, 1);
        let r1 = LinearApproximation.interpolate_order_zero(2.0, &x_table, &y_table, 1);
        assert!((r0[0] - 3.0).abs() < EPS);
        assert!((r1[0] - 7.0).abs() < EPS);
    }

    #[test]
    fn test_linear_negative_x_extrapolates() {
        // xTable 值是相对的（距最后一个样本的秒数）且可能
        // 为负；公式仍须成立。
        let x_table = [-10.0, 0.0];
        let y_table = [0.0, 100.0];
        let result = LinearApproximation.interpolate_order_zero(-5.0, &x_table, &y_table, 1);
        assert!((result[0] - 50.0).abs() < EPS);
    }

    #[test]
    fn test_lagrange_required_data_points() {
        assert_eq!(LagrangePolynomialApproximation.get_required_data_points(0, 0), 2);
        assert_eq!(LagrangePolynomialApproximation.get_required_data_points(1, 0), 2);
        assert_eq!(LagrangePolynomialApproximation.get_required_data_points(2, 0), 3);
        assert_eq!(LagrangePolynomialApproximation.get_required_data_points(7, 0), 8);
    }

    #[test]
    fn test_lagrange_quadratic_exact() {
        // y = x^2 - 2x + 3，在 x = -1、0、2 处采样。
        let x_table = [-1.0, 0.0, 2.0];
        let y_table = [6.0, 3.0, 3.0];
        for &x in &[0.5, 1.0, -0.5, 1.7] {
            let expected = x * x - 2.0 * x + 3.0;
            let result =
                LagrangePolynomialApproximation.interpolate_order_zero(x, &x_table, &y_table, 1);
            assert!(
                (result[0] - expected).abs() < EPS,
                "x={x}: got {}, expected {expected}",
                result[0]
            );
        }
    }

    #[test]
    fn test_lagrange_multi_component() {
        // 两个分量：p = x、q = x^3，在 x = 0、1、2、3 处。
        let x_table = [0.0, 1.0, 2.0, 3.0];
        let y_table = [0.0, 0.0, 1.0, 1.0, 2.0, 8.0, 3.0, 27.0];
        let result =
            LagrangePolynomialApproximation.interpolate_order_zero(1.5, &x_table, &y_table, 2);
        assert!((result[0] - 1.5).abs() < EPS);
        assert!((result[1] - 3.375).abs() < EPS);
    }

    #[test]
    fn test_hermite_required_data_points() {
        assert_eq!(HermitePolynomialApproximation.get_required_data_points(1, 0), 2);
        assert_eq!(HermitePolynomialApproximation.get_required_data_points(3, 0), 4);
        // 带导数（inputOrder=1）：(degree+1)/2 个点。
        assert_eq!(HermitePolynomialApproximation.get_required_data_points(3, 1), 2);
        assert_eq!(HermitePolynomialApproximation.get_required_data_points(5, 1), 3);
        assert_eq!(HermitePolynomialApproximation.get_required_data_points(0, 0), 2);
    }

    #[test]
    fn test_hermite_order_zero_matches_lagrange() {
        // 对于互异的点且无导数时，Hermite 零阶就是
        // Newton 形式的多项式插值：对 4 个点的三次式精确。
        let x_table = [-3.0, -1.0, 1.0, 2.0];
        let f = |x: f64| 2.0 * x * x * x - x * x + 4.0 * x - 7.0;
        let y_table: Vec<f64> = x_table.iter().map(|&x| f(x)).collect();
        for &x in &[-2.0, 0.0, 0.5, 1.5] {
            let result =
                HermitePolynomialApproximation.interpolate_order_zero(x, &x_table, &y_table, 1);
            assert!(
                (result[0] - f(x)).abs() < 1e-9,
                "x={x}: got {}, expected {}",
                result[0],
                f(x)
            );
        }
    }

    #[test]
    fn test_hermite_order_zero_constant_data() {
        // 全相等的样本：highestNonZeroCoef 收缩为 0。
        let x_table = [0.0, 1.0, 2.0];
        let y_table = [5.0, 5.0, 5.0];
        let result =
            HermitePolynomialApproximation.interpolate_order_zero(0.7, &x_table, &y_table, 1);
        assert!((result[0] - 5.0).abs() < EPS);
    }

    #[test]
    fn test_hermite_with_derivatives_cubic() {
        // 经典的三次 Hermite：f(t) = t^3 于 [0, 1]。
        // f(0)=0，f'(0)=0，f(1)=1，f'(1)=3。
        // yTable 每点的布局：[value, derivative]。
        let x_table = [0.0, 1.0];
        let y_table = [0.0, 0.0, 1.0, 3.0];
        let result =
            HermitePolynomialApproximation.interpolate(0.5, &x_table, &y_table, 1, 1, 1);
        // result[0] = f(0.5) = 0.125，result[1] = f'(0.5) = 0.75。
        assert!(
            (result[0] - 0.125).abs() < 1e-9,
            "value: got {}",
            result[0]
        );
        assert!(
            (result[1] - 0.75).abs() < 1e-9,
            "derivative: got {}",
            result[1]
        );
    }

    #[test]
    fn test_hermite_with_derivatives_at_nodes() {
        let x_table = [-1.0, 2.0];
        // f(x) = x^2：f(-1)=1，f'(-1)=-2，f(2)=4，f'(2)=4。
        let y_table = [1.0, -2.0, 4.0, 4.0];
        for &x in &[-1.0, 0.0, 2.0] {
            let result =
                HermitePolynomialApproximation.interpolate(x, &x_table, &y_table, 1, 1, 1);
            assert!((result[0] - x * x).abs() < 1e-9, "x={x}: got {}", result[0]);
            assert!((result[1] - 2.0 * x).abs() < 1e-9, "x={x}: got {}", result[1]);
        }
    }

    #[test]
    fn test_hermite_three_points_with_derivatives() {
        // f(x) = x^5 - x，在 -1、0、1 处连同导数采样。
        let x_table = [-1.0, 0.0, 1.0];
        let f = |x: f64| x.powi(5) - x;
        let df = |x: f64| 5.0 * x.powi(4) - 1.0;
        let mut y_table = Vec::new();
        for &x in &x_table {
            y_table.push(f(x));
            y_table.push(df(x));
        }
        // 3 个点 * 2 个值 = 6 个 z-index；次数至多 5 → 对 x^5 精确。
        let result =
            HermitePolynomialApproximation.interpolate(0.5, &x_table, &y_table, 1, 1, 1);
        assert!((result[0] - f(0.5)).abs() < 1e-9, "got {}", result[0]);
        assert!((result[1] - df(0.5)).abs() < 1e-9, "got {}", result[1]);
    }

    #[test]
    fn test_kind_dispatch_and_names() {
        assert_eq!(InterpolationAlgorithmKind::Linear.name(), "Linear");
        assert_eq!(InterpolationAlgorithmKind::Lagrange.name(), "Lagrange");
        assert_eq!(InterpolationAlgorithmKind::Hermite.name(), "Hermite");
        assert!(!InterpolationAlgorithmKind::Linear.supports_derivatives());
        assert!(!InterpolationAlgorithmKind::Lagrange.supports_derivatives());
        assert!(InterpolationAlgorithmKind::Hermite.supports_derivatives());
        assert_eq!(
            InterpolationAlgorithmKind::from_name("Hermite"),
            Some(InterpolationAlgorithmKind::Hermite)
        );
        assert_eq!(InterpolationAlgorithmKind::from_name("Bogus"), None);
        assert_eq!(
            InterpolationAlgorithmKind::default(),
            InterpolationAlgorithmKind::Linear
        );
    }

    #[test]
    fn test_kind_interpolate_fallback_zero_fills_derivatives() {
        // 非 Hermite 算法回退到零阶并填充零。
        let x_table = [0.0, 10.0];
        let y_table = [1.0, 21.0];
        let result = InterpolationAlgorithmKind::Linear.interpolate(5.0, &x_table, &y_table, 1, 0, 1);
        assert!((result[0] - 11.0).abs() < EPS);
        assert_eq!(result[1], 0.0);
    }
}
