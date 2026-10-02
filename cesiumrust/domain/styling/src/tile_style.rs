//! 3D Tiles Styling 语言实现。
//!
//! 本模块实现声明式 3D Tiles Styling 语言：
//! - 声明式 styling 表达式
//! - 基于属性的条件
//! - 颜色与 show 表达式

use std::collections::HashMap;

/// 求值为某个值的样式表达式。
///
/// 支持常量、属性引用与条件表达式三类形态。
#[derive(Debug, Clone, PartialEq)]
pub enum StyleExpression {
    /// 常量颜色 [r, g, b, a]（0.0-1.0）。
    Color([f64; 4]),
    /// 常量布尔值。
    Bool(bool),
    /// 常量数字。
    Number(f64),
    /// 常量字符串。
    String(String),
    /// 对 feature 属性的引用：`${propertyName}`。
    Property(String),
    /// 条件表达式：`condition ? true_expr : false_expr`。
    Conditional {
        /// 条件表达式。
        condition: Box<StyleExpression>,
        /// 条件为真时求值的表达式。
        true_expr: Box<StyleExpression>,
        /// 条件为假时求值的表达式。
        false_expr: Box<StyleExpression>,
    },
    /// 比较：`left op right`。
    Compare {
        /// 左操作数。
        left: Box<StyleExpression>,
        /// 比较运算符。
        op: CompareOp,
        /// 右操作数。
        right: Box<StyleExpression>,
    },
    /// 逻辑与：`a && b`。
    And(Box<StyleExpression>, Box<StyleExpression>),
    /// 逻辑或：`a || b`。
    Or(Box<StyleExpression>, Box<StyleExpression>),
    /// 逻辑非：`!expr`。
    Not(Box<StyleExpression>),
    /// 算术：`left op right`。
    Arithmetic {
        /// 左操作数。
        left: Box<StyleExpression>,
        /// 算术运算符。
        op: ArithmeticOp,
        /// 右操作数。
        right: Box<StyleExpression>,
    },
    /// 函数调用：`func(args...)`。
    Function {
        /// 函数名。
        name: String,
        /// 参数。
        args: Vec<StyleExpression>,
    },
}

/// 比较运算符。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    /// 等于 (==)。
    Equal,
    /// 不等于 (!=)。
    NotEqual,
    /// 小于 (<)。
    LessThan,
    /// 小于等于 (<=)。
    LessThanOrEqual,
    /// 大于 (>)。
    GreaterThan,
    /// 大于等于 (>=)。
    GreaterThanOrEqual,
}

/// 算术运算符。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithmeticOp {
    /// 加法 (+)。
    Add,
    /// 减法 (-)。
    Subtract,
    /// 乘法 (*)。
    Multiply,
    /// 除法 (/)。
    Divide,
    /// 取模 (%)。
    Modulo,
}

/// 用于表达式求值的 feature 属性值。
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    /// 布尔值。
    Bool(bool),
    /// 数值。
    Number(f64),
    /// 字符串值。
    String(String),
    /// 颜色值 [r, g, b, a]。
    Color([f64; 4]),
}

impl StyleExpression {
    /// 在给定的 feature 属性下求值表达式。
    pub fn evaluate(&self, properties: &HashMap<String, PropertyValue>) -> PropertyValue {
        match self {
            // 常量：直接回取为对应的 PropertyValue。
            Self::Color(c) => PropertyValue::Color(*c),
            Self::Bool(b) => PropertyValue::Bool(*b),
            Self::Number(n) => PropertyValue::Number(*n),
            Self::String(s) => PropertyValue::String(s.clone()),
            // 属性引用：按名查找，缺失时退化为 0.0。
            Self::Property(name) => properties
                .get(name)
                .cloned()
                .unwrap_or(PropertyValue::Number(0.0)),
            Self::Conditional {
                condition,
                true_expr,
                false_expr,
            } => {
                // 先求值条件，按真值性选择分支。
                let cond_result = condition.evaluate(properties);
                if Self::is_truthy(&cond_result) {
                    true_expr.evaluate(properties)
                } else {
                    false_expr.evaluate(properties)
                }
            }
            Self::Compare { left, op, right } => {
                // 两侧求值后按运算符比较，产出布尔。
                let l = left.evaluate(properties);
                let r = right.evaluate(properties);
                PropertyValue::Bool(Self::compare(&l, *op, &r))
            }
            Self::And(a, b) => {
                // 逻辑与：两侧都求值（无短路），均真才为真。
                let a_result = a.evaluate(properties);
                let b_result = b.evaluate(properties);
                PropertyValue::Bool(Self::is_truthy(&a_result) && Self::is_truthy(&b_result))
            }
            Self::Or(a, b) => {
                // 逻辑或：两侧都求值，任一为真即为真。
                let a_result = a.evaluate(properties);
                let b_result = b.evaluate(properties);
                PropertyValue::Bool(Self::is_truthy(&a_result) || Self::is_truthy(&b_result))
            }
            Self::Not(expr) => {
                // 逻辑非：取操作数的真值性再取反。
                let result = expr.evaluate(properties);
                PropertyValue::Bool(!Self::is_truthy(&result))
            }
            Self::Arithmetic { left, op, right } => {
                // 算术：两侧求值后交给 arithmetic 逐型处理。
                let l = left.evaluate(properties);
                let r = right.evaluate(properties);
                Self::arithmetic(&l, *op, &r)
            }
            Self::Function { name, args } => {
                // 内置函数：委派给 evaluate_function 的名表。
                Self::evaluate_function(name, args, properties)
            }
        }
    }

    /// 检查一个值是否为真值（truthy）。
    fn is_truthy(value: &PropertyValue) -> bool {
        match value {
            PropertyValue::Bool(b) => *b,
            PropertyValue::Number(n) => *n != 0.0,
            PropertyValue::String(s) => !s.is_empty(),
            PropertyValue::Color(_) => true,
        }
    }

    /// 比较两个值。
    fn compare(left: &PropertyValue, op: CompareOp, right: &PropertyValue) -> bool {
        match (left, right) {
            // 同为数值：逐运算符比较（相等用 1e-10 容差）。
            (PropertyValue::Number(l), PropertyValue::Number(r)) => match op {
                CompareOp::Equal => (l - r).abs() < 1e-10,
                CompareOp::NotEqual => (l - r).abs() >= 1e-10,
                CompareOp::LessThan => l < r,
                CompareOp::LessThanOrEqual => l <= r,
                CompareOp::GreaterThan => l > r,
                CompareOp::GreaterThanOrEqual => l >= r,
            },
            // 同为字符串：仅相等/不等有意义，序关系返回 false。
            (PropertyValue::String(l), PropertyValue::String(r)) => match op {
                CompareOp::Equal => l == r,
                CompareOp::NotEqual => l != r,
                _ => false,
            },
            // 同为布尔：仅相等/不等有意义。
            (PropertyValue::Bool(l), PropertyValue::Bool(r)) => match op {
                CompareOp::Equal => l == r,
                CompareOp::NotEqual => l != r,
                _ => false,
            },
            // 类型不匹配：无法比较，一律 false。
            _ => false,
        }
    }

    /// 对两个值执行算术运算。
    fn arithmetic(left: &PropertyValue, op: ArithmeticOp, right: &PropertyValue) -> PropertyValue {
        match (left, right) {
            // 仅数值与数值之间做算术；除/模遇零除数退化为 0.0。
            (PropertyValue::Number(l), PropertyValue::Number(r)) => {
                let result = match op {
                    ArithmeticOp::Add => l + r,
                    ArithmeticOp::Subtract => l - r,
                    ArithmeticOp::Multiply => l * r,
                    ArithmeticOp::Divide => if *r != 0.0 { l / r } else { 0.0 },
                    ArithmeticOp::Modulo => if *r != 0.0 { l % r } else { 0.0 },
                };
                PropertyValue::Number(result)
            }
            // 非数值操作数：回退为 0.0。
            _ => PropertyValue::Number(0.0),
        }
    }

    /// 求值一个内置函数。
    fn evaluate_function(
        name: &str,
        args: &[StyleExpression],
        properties: &HashMap<String, PropertyValue>,
    ) -> PropertyValue {
        match name {
            // 颜色构造器：color("#fff") 或 color(r,g,b,a)。
            "color" => {
                // color(cssColor) 或 color(r, g, b, a)
                if args.len() == 1 {
                    if let StyleExpression::String(css) = &args[0] {
                        return PropertyValue::Color(Self::parse_css_color(css));
                    }
                }
                if args.len() >= 3 {
                    let r = Self::get_number_arg(args, 0, properties);
                    let g = Self::get_number_arg(args, 1, properties);
                    let b = Self::get_number_arg(args, 2, properties);
                    let a = if args.len() > 3 {
                        Self::get_number_arg(args, 3, properties)
                    } else {
                        1.0
                    };
                    return PropertyValue::Color([r, g, b, a]);
                }
                PropertyValue::Color([1.0, 1.0, 1.0, 1.0])
            }
            // rgb(r,g,b)：分量取 0-255，除以 255 归一到 0-1，alpha 固定 1.0。
            "rgb" => {
                let r = Self::get_number_arg(args, 0, properties) / 255.0;
                let g = Self::get_number_arg(args, 1, properties) / 255.0;
                let b = Self::get_number_arg(args, 2, properties) / 255.0;
                PropertyValue::Color([r, g, b, 1.0])
            }
            // rgba(r,g,b,a)：同 rgb，但第四参数作为归一化 alpha。
            "rgba" => {
                let r = Self::get_number_arg(args, 0, properties) / 255.0;
                let g = Self::get_number_arg(args, 1, properties) / 255.0;
                let b = Self::get_number_arg(args, 2, properties) / 255.0;
                let a = Self::get_number_arg(args, 3, properties);
                PropertyValue::Color([r, g, b, a])
            }
            // abs(v)：取绝对值。
            "abs" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.abs())
            }
            // sqrt(v)：平方根。
            "sqrt" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.sqrt())
            }
            // min(a,b)：取两参数中较小值。
            "min" => {
                let a = Self::get_number_arg(args, 0, properties);
                let b = Self::get_number_arg(args, 1, properties);
                PropertyValue::Number(a.min(b))
            }
            // max(a,b)：取两参数中较大值。
            "max" => {
                let a = Self::get_number_arg(args, 0, properties);
                let b = Self::get_number_arg(args, 1, properties);
                PropertyValue::Number(a.max(b))
            }
            // clamp(v,min,max)：把 v 钳制到 [min,max] 闭区间。
            "clamp" => {
                let v = Self::get_number_arg(args, 0, properties);
                let min = Self::get_number_arg(args, 1, properties);
                let max = Self::get_number_arg(args, 2, properties);
                PropertyValue::Number(v.clamp(min, max))
            }
            // 三角函数
            // cos(v)：余弦（v 以弧度计）。
            "cos" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.cos())
            }
            // sin(v)：正弦（v 以弧度计）。
            "sin" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.sin())
            }
            // tan(v)：正切（v 以弧度计）。
            "tan" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.tan())
            }
            // acos(v)：反余弦，返回弧度。
            "acos" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.acos())
            }
            // asin(v)：反正弦，返回弧度。
            "asin" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.asin())
            }
            // atan(v)：反正切（单参数），返回弧度。
            "atan" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.atan())
            }
            // atan2(y,x)：双参数反正切，按象限确定角度。
            "atan2" => {
                let y = Self::get_number_arg(args, 0, properties);
                let x = Self::get_number_arg(args, 1, properties);
                PropertyValue::Number(y.atan2(x))
            }
            // 角度转换
            // radians(v)：角度转弧度。
            "radians" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.to_radians())
            }
            // degrees(v)：弧度转角度。
            "degrees" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.to_degrees())
            }
            // 取整 / 符号
            // sign(v)：取符号，正→1、负→-1、零→0。
            "sign" => {
                let v = Self::get_number_arg(args, 0, properties);
                let s = if v > 0.0 { 1.0 } else if v < 0.0 { -1.0 } else { 0.0 };
                PropertyValue::Number(s)
            }
            // floor(v)：向下取整。
            "floor" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.floor())
            }
            // ceil(v)：向上取整。
            "ceil" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.ceil())
            }
            // round(v)：四舍五入到最近整数。
            "round" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.round())
            }
            // fract(v)：取小数部分（v - floor(v)）。
            "fract" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v - v.floor())
            }
            // 指数 / 对数
            // exp(v)：自然指数 e^v。
            "exp" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.exp())
            }
            // exp2(v)：以 2 为底的指数 2^v。
            "exp2" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.exp2())
            }
            // log(v)：自然对数 ln(v)。
            "log" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.ln())
            }
            // log2(v)：以 2 为底的对数。
            "log2" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.log2())
            }
            // pow(base,exponent)：幂运算 base^exponent。
            "pow" => {
                let base = Self::get_number_arg(args, 0, properties);
                let exponent = Self::get_number_arg(args, 1, properties);
                PropertyValue::Number(base.powf(exponent))
            }
            // mod(a,b)：取模，除数为 0 时退化为 0.0。
            "mod" => {
                let a = Self::get_number_arg(args, 0, properties);
                let b = Self::get_number_arg(args, 1, properties);
                PropertyValue::Number(if b != 0.0 { a % b } else { 0.0 })
            }
            // 插值
            // mix(a,b,t)：线性插值，t=0 取 a、t=1 取 b。
            "mix" => {
                let a = Self::get_number_arg(args, 0, properties);
                let b = Self::get_number_arg(args, 1, properties);
                let t = Self::get_number_arg(args, 2, properties);
                PropertyValue::Number(a * (1.0 - t) + b * t)
            }
            // HSL 颜色构造器
            // hsl(h,s,l)：由色相/饱和度/亮度构造颜色，alpha 固定 1.0。
            "hsl" => {
                let h = Self::get_number_arg(args, 0, properties);
                let s = Self::get_number_arg(args, 1, properties);
                let l = Self::get_number_arg(args, 2, properties);
                let rgb = Self::hsl_to_rgb(h, s, l);
                PropertyValue::Color([rgb[0], rgb[1], rgb[2], 1.0])
            }
            // hsla(h,s,l,a)：同 hsl，但第四参数作为 alpha。
            "hsla" => {
                let h = Self::get_number_arg(args, 0, properties);
                let s = Self::get_number_arg(args, 1, properties);
                let l = Self::get_number_arg(args, 2, properties);
                let a = Self::get_number_arg(args, 3, properties);
                let rgb = Self::hsl_to_rgb(h, s, l);
                PropertyValue::Color([rgb[0], rgb[1], rgb[2], a])
            }
            // 向量运算（对以 Color 编码的 vec3/vec4 数组进行操作）
            // length(v)：向量各分量平方和开方（欧几里得范数）。
            "length" => {
                let v = Self::get_vec_arg(args, 0, properties);
                let len: f64 = v.iter().map(|c| c * c).sum::<f64>().sqrt();
                PropertyValue::Number(len)
            }
            // normalize(v)：单位化向量；零向量时原样返回避免除零。
            "normalize" => {
                let v = Self::get_vec_arg(args, 0, properties);
                let len: f64 = v.iter().map(|c| c * c).sum::<f64>().sqrt();
                if len > 0.0 {
                    let n: Vec<f64> = v.iter().map(|c| c / len).collect();
                    Self::vec_to_property(&n)
                } else {
                    Self::vec_to_property(&v)
                }
            }
            // distance(a,b)：两点间的欧几里得距离。
            "distance" => {
                let a = Self::get_vec_arg(args, 0, properties);
                let b = Self::get_vec_arg(args, 1, properties);
                let d: f64 = a.iter().zip(b.iter()).map(|(x, y)| (x - y) * (x - y)).sum::<f64>().sqrt();
                PropertyValue::Number(d)
            }
            // dot(a,b)：点积，对应分量相乘后求和。
            "dot" => {
                let a = Self::get_vec_arg(args, 0, properties);
                let b = Self::get_vec_arg(args, 1, properties);
                let d: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
                PropertyValue::Number(d)
            }
            // cross(a,b)：三维叉积；任一向量不足三维时退化为 0.0。
            "cross" => {
                let a = Self::get_vec_arg(args, 0, properties);
                let b = Self::get_vec_arg(args, 1, properties);
                if a.len() >= 3 && b.len() >= 3 {
                    let result = vec![
                        a[1] * b[2] - a[2] * b[1],
                        a[2] * b[0] - a[0] * b[2],
                        a[0] * b[1] - a[1] * b[0],
                    ];
                    Self::vec_to_property(&result)
                } else {
                    PropertyValue::Number(0.0)
                }
            }
            _ => PropertyValue::Number(0.0),
        }
    }

    /// 获取一个数值参数。
    fn get_number_arg(
        args: &[StyleExpression],
        index: usize,
        properties: &HashMap<String, PropertyValue>,
    ) -> f64 {
        if let Some(arg) = args.get(index) {
            // 仅当参数求值为 Number 时取用，其余情形回退为 0.0。
            if let PropertyValue::Number(n) = arg.evaluate(properties) {
                return n;
            }
        }
        0.0
    }

    /// 获取一个向量参数（来自 Color 或 Number）。
    fn get_vec_arg(
        args: &[StyleExpression],
        index: usize,
        properties: &HashMap<String, PropertyValue>,
    ) -> Vec<f64> {
        if let Some(arg) = args.get(index) {
            // Color 展开为四分量，Number 退化为单分量，其余走末尾默认。
            match arg.evaluate(properties) {
                PropertyValue::Color(c) => return vec![c[0], c[1], c[2], c[3]],
                PropertyValue::Number(n) => return vec![n],
                _ => {}
            }
        }
        vec![0.0, 0.0, 0.0]
    }

    /// 将向量转换回 PropertyValue。
    fn vec_to_property(v: &[f64]) -> PropertyValue {
        // 按向量长度回映：4/3 分量视作颜色，1 分量视作标量。
        match v.len() {
            4 => PropertyValue::Color([v[0], v[1], v[2], v[3]]),
            3 => PropertyValue::Color([v[0], v[1], v[2], 1.0]),
            1 => PropertyValue::Number(v[0]),
            _ => PropertyValue::Number(0.0),
        }
    }

    /// 将 HSL 转换为 RGB。h 属于 [0,360]，s 属于 [0,1]，l 属于 [0,1]。
    fn hsl_to_rgb(h: f64, s: f64, l: f64) -> [f64; 3] {
        // 将色相回绕到 [0,360)，避免负值与越界。
        let h = ((h % 360.0) + 360.0) % 360.0;
        // 色度（chroma）：由亮度与饱和度导出。
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        // 次大分量 x：落在色相环当前扇区上的中间值。
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        // 匹配项 m：把亮度对齐到实际 l，最后加回各分量。
        let m = l - c / 2.0;
        // 六个 60° 扇区分别决定 (r1,g1,b1) 的排列。
        let (r1, g1, b1) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };
        [r1 + m, g1 + m, b1 + m]
    }

    /// 解析一个 CSS 颜色字符串。
    fn parse_css_color(css: &str) -> [f64; 4] {
        let css = css.trim().to_lowercase();
        match css.as_str() {
            "red" => [1.0, 0.0, 0.0, 1.0],
            "green" => [0.0, 0.5, 0.0, 1.0],
            "blue" => [0.0, 0.0, 1.0, 1.0],
            "white" => [1.0, 1.0, 1.0, 1.0],
            "black" => [0.0, 0.0, 0.0, 1.0],
            "yellow" => [1.0, 1.0, 0.0, 1.0],
            "cyan" => [0.0, 1.0, 1.0, 1.0],
            "magenta" => [1.0, 0.0, 1.0, 1.0],
            "orange" => [1.0, 0.647, 0.0, 1.0],
            "gray" | "grey" => [0.5, 0.5, 0.5, 1.0],
            _ => {
                // 尝试十六进制格式
                if css.starts_with('#') {
                    Self::parse_hex_color(&css)
                } else {
                    [1.0, 1.0, 1.0, 1.0]
                }
            }
        }
    }

    /// 解析一个十六进制颜色字符串。
    fn parse_hex_color(hex: &str) -> [f64; 4] {
        let hex = hex.trim_start_matches('#');
        match hex.len() {
            // 6 位：rrggbb，alpha 固定 1.0。
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255) as f64 / 255.0;
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255) as f64 / 255.0;
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255) as f64 / 255.0;
                [r, g, b, 1.0]
            }
            // 8 位：rrggbbaa，第四字节作为 alpha。
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255) as f64 / 255.0;
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255) as f64 / 255.0;
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255) as f64 / 255.0;
                let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255) as f64 / 255.0;
                [r, g, b, a]
            }
            _ => [1.0, 1.0, 1.0, 1.0],
        }
    }
}

/// 一个 3D Tiles 样式定义。
///
/// 样式包含 show/color/pointSize/label 等表达式字段。
#[derive(Debug, Clone, Default)]
pub struct TileStyle {
    /// show 表达式（可见性）。
    pub show: Option<StyleExpression>,
    /// 颜色表达式。
    pub color: Option<StyleExpression>,
    /// point size 表达式。
    pub point_size: Option<StyleExpression>,
    /// 元属性（键值表达式）。
    pub meta: HashMap<String, StyleExpression>,
}

impl TileStyle {
    /// 创建一个空样式。
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建一个带常量颜色的样式。
    pub fn with_color(color: [f64; 4]) -> Self {
        // 直接把常量颜色包成 Color 表达式，其余字段取默认（空）。
        Self {
            color: Some(StyleExpression::Color(color)),
            ..Default::default()
        }
    }

    /// 为某个 feature 求值 show 表达式。
    pub fn evaluate_show(&self, properties: &HashMap<String, PropertyValue>) -> bool {
        match &self.show {
            Some(expr) => {
                // show 必须求值为布尔，否则默认可见。
                if let PropertyValue::Bool(b) = expr.evaluate(properties) {
                    b
                } else {
                    true
                }
            }
            // show 缺失：默认显示。
            None => true,
        }
    }

    /// 为某个 feature 求值颜色表达式。
    pub fn evaluate_color(&self, properties: &HashMap<String, PropertyValue>) -> [f64; 4] {
        match &self.color {
            Some(expr) => {
                // 颜色表达式非 Color 时退化为不透明白。
                if let PropertyValue::Color(c) = expr.evaluate(properties) {
                    c
                } else {
                    [1.0, 1.0, 1.0, 1.0]
                }
            }
            // color 缺失：默认不透明白。
            None => [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// 为某个 feature 求值 point size 表达式。
    pub fn evaluate_point_size(&self, properties: &HashMap<String, PropertyValue>) -> f64 {
        match &self.point_size {
            Some(expr) => {
                // point size 非 Number 时退化为 1.0。
                if let PropertyValue::Number(n) = expr.evaluate(properties) {
                    n
                } else {
                    1.0
                }
            }
            // point size 缺失：默认 1.0。
            None => 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个空属性表供常量表达式求值。
    fn empty_props() -> HashMap<String, PropertyValue> {
        HashMap::new()
    }

    /// 常量颜色表达式直接回取其 rgba。
    #[test]
    fn test_constant_color() {
        // 常量颜色不经属性表，直接回取。
        let expr = StyleExpression::Color([1.0, 0.0, 0.0, 1.0]);
        let result = expr.evaluate(&empty_props());
        assert_eq!(result, PropertyValue::Color([1.0, 0.0, 0.0, 1.0]));
    }

    /// 属性引用 `${height}` 从 feature 取到对应数值。
    #[test]
    fn test_property_reference() {
        // 向属性表注入 height=100，供引用查找。
        let mut props = HashMap::new();
        props.insert("height".to_string(), PropertyValue::Number(100.0));

        let expr = StyleExpression::Property("height".to_string());
        let result = expr.evaluate(&props);
        assert_eq!(result, PropertyValue::Number(100.0));
    }

    /// 比较表达式：height > 50 为真。
    #[test]
    fn test_comparison() {
        // 构造 height > 50 的比较节点。
        let mut props = HashMap::new();
        props.insert("height".to_string(), PropertyValue::Number(100.0));

        let expr = StyleExpression::Compare {
            left: Box::new(StyleExpression::Property("height".to_string())),
            op: CompareOp::GreaterThan,
            right: Box::new(StyleExpression::Number(50.0)),
        };

        let result = expr.evaluate(&props);
        assert_eq!(result, PropertyValue::Bool(true));
    }

    /// 条件表达式：type=="building" 时取红色分支。
    #[test]
    fn test_conditional() {
        // type=="building" 命中真分支（红色）。
        let mut props = HashMap::new();
        props.insert("type".to_string(), PropertyValue::String("building".to_string()));

        let expr = StyleExpression::Conditional {
            condition: Box::new(StyleExpression::Compare {
                left: Box::new(StyleExpression::Property("type".to_string())),
                op: CompareOp::Equal,
                right: Box::new(StyleExpression::String("building".to_string())),
            }),
            true_expr: Box::new(StyleExpression::Color([1.0, 0.0, 0.0, 1.0])),
            false_expr: Box::new(StyleExpression::Color([0.0, 0.0, 1.0, 1.0])),
        };

        let result = expr.evaluate(&props);
        assert_eq!(result, PropertyValue::Color([1.0, 0.0, 0.0, 1.0]));
    }

    /// 算术表达式：10 * 5 = 50。
    #[test]
    fn test_arithmetic() {
        // 纯常量乘法：无需属性表。
        let expr = StyleExpression::Arithmetic {
            left: Box::new(StyleExpression::Number(10.0)),
            op: ArithmeticOp::Multiply,
            right: Box::new(StyleExpression::Number(5.0)),
        };

        let result = expr.evaluate(&empty_props());
        assert_eq!(result, PropertyValue::Number(50.0));
    }

    /// 逻辑与：true && false = false。
    #[test]
    fn test_logical_and() {
        let expr = StyleExpression::And(
            Box::new(StyleExpression::Bool(true)),
            Box::new(StyleExpression::Bool(false)),
        );

        let result = expr.evaluate(&empty_props());
        assert_eq!(result, PropertyValue::Bool(false));
    }

    /// 逻辑或：true || false = true。
    #[test]
    fn test_logical_or() {
        let expr = StyleExpression::Or(
            Box::new(StyleExpression::Bool(true)),
            Box::new(StyleExpression::Bool(false)),
        );

        let result = expr.evaluate(&empty_props());
        assert_eq!(result, PropertyValue::Bool(true));
    }

    /// 逻辑非：!true = false。
    #[test]
    fn test_not() {
        let expr = StyleExpression::Not(Box::new(StyleExpression::Bool(true)));
        let result = expr.evaluate(&empty_props());
        assert_eq!(result, PropertyValue::Bool(false));
    }

    /// color("red") 函数解析为命名红色。
    #[test]
    fn test_color_function() {
        let expr = StyleExpression::Function {
            name: "color".to_string(),
            args: vec![StyleExpression::String("red".to_string())],
        };

        let result = expr.evaluate(&empty_props());
        assert_eq!(result, PropertyValue::Color([1.0, 0.0, 0.0, 1.0]));
    }

    /// rgb(255,128,0) 归一到 0..1 分量。
    #[test]
    fn test_rgb_function() {
        let expr = StyleExpression::Function {
            name: "rgb".to_string(),
            args: vec![
                StyleExpression::Number(255.0),
                StyleExpression::Number(128.0),
                StyleExpression::Number(0.0),
            ],
        };

        let result = expr.evaluate(&empty_props());
        if let PropertyValue::Color(c) = result {
            assert!((c[0] - 1.0).abs() < 0.01);
            assert!((c[1] - 0.502).abs() < 0.01);
            assert!((c[2] - 0.0).abs() < 0.01);
        } else {
            panic!("Expected color");
        }
    }

    /// TileStyle 聚合 show/color 表达式的求值。
    #[test]
    fn test_tile_style_evaluate() {
        let style = TileStyle {
            show: Some(StyleExpression::Compare {
                left: Box::new(StyleExpression::Property("height".to_string())),
                op: CompareOp::GreaterThan,
                right: Box::new(StyleExpression::Number(0.0)),
            }),
            color: Some(StyleExpression::Color([0.0, 1.0, 0.0, 1.0])),
            ..Default::default()
        };

        let mut props = HashMap::new();
        props.insert("height".to_string(), PropertyValue::Number(50.0));

        assert!(style.evaluate_show(&props));
        assert_eq!(style.evaluate_color(&props), [0.0, 1.0, 0.0, 1.0]);
    }

    /// 十六进制颜色 #FF8000 解析。
    #[test]
    fn test_hex_color_parsing() {
        let color = StyleExpression::parse_hex_color("#FF8000");
        assert!((color[0] - 1.0).abs() < 0.01);
        assert!((color[1] - 0.502).abs() < 0.01);
        assert!((color[2] - 0.0).abs() < 0.01);
    }

    /// clamp(150, 0, 100) 钳制到上限 100。
    #[test]
    fn test_clamp_function() {
        let expr = StyleExpression::Function {
            name: "clamp".to_string(),
            args: vec![
                StyleExpression::Number(150.0),
                StyleExpression::Number(0.0),
                StyleExpression::Number(100.0),
            ],
        };

        let result = expr.evaluate(&empty_props());
        assert_eq!(result, PropertyValue::Number(100.0));
    }
}
