//! 3D Tiles Styling 语言实现。
//!
//! 镜像 CesiumJS `Scene/Cesium3DTileStyle.js`：
//! - 声明式 styling 表达式
//! - 基于属性的条件
//! - 颜色与 show 表达式

use std::collections::HashMap;

/// 求值为某个值的样式表达式。
///
/// 镜像 CesiumJS `Scene/Expression.js`
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
            Self::Color(c) => PropertyValue::Color(*c),
            Self::Bool(b) => PropertyValue::Bool(*b),
            Self::Number(n) => PropertyValue::Number(*n),
            Self::String(s) => PropertyValue::String(s.clone()),
            Self::Property(name) => properties
                .get(name)
                .cloned()
                .unwrap_or(PropertyValue::Number(0.0)),
            Self::Conditional {
                condition,
                true_expr,
                false_expr,
            } => {
                let cond_result = condition.evaluate(properties);
                if Self::is_truthy(&cond_result) {
                    true_expr.evaluate(properties)
                } else {
                    false_expr.evaluate(properties)
                }
            }
            Self::Compare { left, op, right } => {
                let l = left.evaluate(properties);
                let r = right.evaluate(properties);
                PropertyValue::Bool(Self::compare(&l, *op, &r))
            }
            Self::And(a, b) => {
                let a_result = a.evaluate(properties);
                let b_result = b.evaluate(properties);
                PropertyValue::Bool(Self::is_truthy(&a_result) && Self::is_truthy(&b_result))
            }
            Self::Or(a, b) => {
                let a_result = a.evaluate(properties);
                let b_result = b.evaluate(properties);
                PropertyValue::Bool(Self::is_truthy(&a_result) || Self::is_truthy(&b_result))
            }
            Self::Not(expr) => {
                let result = expr.evaluate(properties);
                PropertyValue::Bool(!Self::is_truthy(&result))
            }
            Self::Arithmetic { left, op, right } => {
                let l = left.evaluate(properties);
                let r = right.evaluate(properties);
                Self::arithmetic(&l, *op, &r)
            }
            Self::Function { name, args } => {
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
            (PropertyValue::Number(l), PropertyValue::Number(r)) => match op {
                CompareOp::Equal => (l - r).abs() < 1e-10,
                CompareOp::NotEqual => (l - r).abs() >= 1e-10,
                CompareOp::LessThan => l < r,
                CompareOp::LessThanOrEqual => l <= r,
                CompareOp::GreaterThan => l > r,
                CompareOp::GreaterThanOrEqual => l >= r,
            },
            (PropertyValue::String(l), PropertyValue::String(r)) => match op {
                CompareOp::Equal => l == r,
                CompareOp::NotEqual => l != r,
                _ => false,
            },
            (PropertyValue::Bool(l), PropertyValue::Bool(r)) => match op {
                CompareOp::Equal => l == r,
                CompareOp::NotEqual => l != r,
                _ => false,
            },
            _ => false,
        }
    }

    /// 对两个值执行算术运算。
    fn arithmetic(left: &PropertyValue, op: ArithmeticOp, right: &PropertyValue) -> PropertyValue {
        match (left, right) {
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
            "rgb" => {
                let r = Self::get_number_arg(args, 0, properties) / 255.0;
                let g = Self::get_number_arg(args, 1, properties) / 255.0;
                let b = Self::get_number_arg(args, 2, properties) / 255.0;
                PropertyValue::Color([r, g, b, 1.0])
            }
            "rgba" => {
                let r = Self::get_number_arg(args, 0, properties) / 255.0;
                let g = Self::get_number_arg(args, 1, properties) / 255.0;
                let b = Self::get_number_arg(args, 2, properties) / 255.0;
                let a = Self::get_number_arg(args, 3, properties);
                PropertyValue::Color([r, g, b, a])
            }
            "abs" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.abs())
            }
            "sqrt" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.sqrt())
            }
            "min" => {
                let a = Self::get_number_arg(args, 0, properties);
                let b = Self::get_number_arg(args, 1, properties);
                PropertyValue::Number(a.min(b))
            }
            "max" => {
                let a = Self::get_number_arg(args, 0, properties);
                let b = Self::get_number_arg(args, 1, properties);
                PropertyValue::Number(a.max(b))
            }
            "clamp" => {
                let v = Self::get_number_arg(args, 0, properties);
                let min = Self::get_number_arg(args, 1, properties);
                let max = Self::get_number_arg(args, 2, properties);
                PropertyValue::Number(v.clamp(min, max))
            }
            // 三角函数
            "cos" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.cos())
            }
            "sin" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.sin())
            }
            "tan" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.tan())
            }
            "acos" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.acos())
            }
            "asin" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.asin())
            }
            "atan" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.atan())
            }
            "atan2" => {
                let y = Self::get_number_arg(args, 0, properties);
                let x = Self::get_number_arg(args, 1, properties);
                PropertyValue::Number(y.atan2(x))
            }
            // 角度转换
            "radians" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.to_radians())
            }
            "degrees" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.to_degrees())
            }
            // 取整 / 符号
            "sign" => {
                let v = Self::get_number_arg(args, 0, properties);
                let s = if v > 0.0 { 1.0 } else if v < 0.0 { -1.0 } else { 0.0 };
                PropertyValue::Number(s)
            }
            "floor" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.floor())
            }
            "ceil" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.ceil())
            }
            "round" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.round())
            }
            "fract" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v - v.floor())
            }
            // 指数 / 对数
            "exp" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.exp())
            }
            "exp2" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.exp2())
            }
            "log" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.ln())
            }
            "log2" => {
                let v = Self::get_number_arg(args, 0, properties);
                PropertyValue::Number(v.log2())
            }
            "pow" => {
                let base = Self::get_number_arg(args, 0, properties);
                let exponent = Self::get_number_arg(args, 1, properties);
                PropertyValue::Number(base.powf(exponent))
            }
            "mod" => {
                let a = Self::get_number_arg(args, 0, properties);
                let b = Self::get_number_arg(args, 1, properties);
                PropertyValue::Number(if b != 0.0 { a % b } else { 0.0 })
            }
            // 插值
            "mix" => {
                let a = Self::get_number_arg(args, 0, properties);
                let b = Self::get_number_arg(args, 1, properties);
                let t = Self::get_number_arg(args, 2, properties);
                PropertyValue::Number(a * (1.0 - t) + b * t)
            }
            // HSL 颜色构造器
            "hsl" => {
                let h = Self::get_number_arg(args, 0, properties);
                let s = Self::get_number_arg(args, 1, properties);
                let l = Self::get_number_arg(args, 2, properties);
                let rgb = Self::hsl_to_rgb(h, s, l);
                PropertyValue::Color([rgb[0], rgb[1], rgb[2], 1.0])
            }
            "hsla" => {
                let h = Self::get_number_arg(args, 0, properties);
                let s = Self::get_number_arg(args, 1, properties);
                let l = Self::get_number_arg(args, 2, properties);
                let a = Self::get_number_arg(args, 3, properties);
                let rgb = Self::hsl_to_rgb(h, s, l);
                PropertyValue::Color([rgb[0], rgb[1], rgb[2], a])
            }
            // 向量运算（对以 Color 编码的 vec3/vec4 数组进行操作）
            "length" => {
                let v = Self::get_vec_arg(args, 0, properties);
                let len: f64 = v.iter().map(|c| c * c).sum::<f64>().sqrt();
                PropertyValue::Number(len)
            }
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
            "distance" => {
                let a = Self::get_vec_arg(args, 0, properties);
                let b = Self::get_vec_arg(args, 1, properties);
                let d: f64 = a.iter().zip(b.iter()).map(|(x, y)| (x - y) * (x - y)).sum::<f64>().sqrt();
                PropertyValue::Number(d)
            }
            "dot" => {
                let a = Self::get_vec_arg(args, 0, properties);
                let b = Self::get_vec_arg(args, 1, properties);
                let d: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
                PropertyValue::Number(d)
            }
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
        match v.len() {
            4 => PropertyValue::Color([v[0], v[1], v[2], v[3]]),
            3 => PropertyValue::Color([v[0], v[1], v[2], 1.0]),
            1 => PropertyValue::Number(v[0]),
            _ => PropertyValue::Number(0.0),
        }
    }

    /// 将 HSL 转换为 RGB。h 属于 [0,360]，s 属于 [0,1]，l 属于 [0,1]。
    fn hsl_to_rgb(h: f64, s: f64, l: f64) -> [f64; 3] {
        let h = ((h % 360.0) + 360.0) % 360.0;
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = l - c / 2.0;
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
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255) as f64 / 255.0;
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255) as f64 / 255.0;
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255) as f64 / 255.0;
                [r, g, b, 1.0]
            }
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
/// 镜像 CesiumJS `Scene/Cesium3DTileStyle.js`
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
        Self {
            color: Some(StyleExpression::Color(color)),
            ..Default::default()
        }
    }

    /// 为某个 feature 求值 show 表达式。
    pub fn evaluate_show(&self, properties: &HashMap<String, PropertyValue>) -> bool {
        match &self.show {
            Some(expr) => {
                if let PropertyValue::Bool(b) = expr.evaluate(properties) {
                    b
                } else {
                    true
                }
            }
            None => true,
        }
    }

    /// 为某个 feature 求值颜色表达式。
    pub fn evaluate_color(&self, properties: &HashMap<String, PropertyValue>) -> [f64; 4] {
        match &self.color {
            Some(expr) => {
                if let PropertyValue::Color(c) = expr.evaluate(properties) {
                    c
                } else {
                    [1.0, 1.0, 1.0, 1.0]
                }
            }
            None => [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// 为某个 feature 求值 point size 表达式。
    pub fn evaluate_point_size(&self, properties: &HashMap<String, PropertyValue>) -> f64 {
        match &self.point_size {
            Some(expr) => {
                if let PropertyValue::Number(n) = expr.evaluate(properties) {
                    n
                } else {
                    1.0
                }
            }
            None => 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_props() -> HashMap<String, PropertyValue> {
        HashMap::new()
    }

    #[test]
    fn test_constant_color() {
        let expr = StyleExpression::Color([1.0, 0.0, 0.0, 1.0]);
        let result = expr.evaluate(&empty_props());
        assert_eq!(result, PropertyValue::Color([1.0, 0.0, 0.0, 1.0]));
    }

    #[test]
    fn test_property_reference() {
        let mut props = HashMap::new();
        props.insert("height".to_string(), PropertyValue::Number(100.0));

        let expr = StyleExpression::Property("height".to_string());
        let result = expr.evaluate(&props);
        assert_eq!(result, PropertyValue::Number(100.0));
    }

    #[test]
    fn test_comparison() {
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

    #[test]
    fn test_conditional() {
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

    #[test]
    fn test_arithmetic() {
        let expr = StyleExpression::Arithmetic {
            left: Box::new(StyleExpression::Number(10.0)),
            op: ArithmeticOp::Multiply,
            right: Box::new(StyleExpression::Number(5.0)),
        };

        let result = expr.evaluate(&empty_props());
        assert_eq!(result, PropertyValue::Number(50.0));
    }

    #[test]
    fn test_logical_and() {
        let expr = StyleExpression::And(
            Box::new(StyleExpression::Bool(true)),
            Box::new(StyleExpression::Bool(false)),
        );

        let result = expr.evaluate(&empty_props());
        assert_eq!(result, PropertyValue::Bool(false));
    }

    #[test]
    fn test_logical_or() {
        let expr = StyleExpression::Or(
            Box::new(StyleExpression::Bool(true)),
            Box::new(StyleExpression::Bool(false)),
        );

        let result = expr.evaluate(&empty_props());
        assert_eq!(result, PropertyValue::Bool(true));
    }

    #[test]
    fn test_not() {
        let expr = StyleExpression::Not(Box::new(StyleExpression::Bool(true)));
        let result = expr.evaluate(&empty_props());
        assert_eq!(result, PropertyValue::Bool(false));
    }

    #[test]
    fn test_color_function() {
        let expr = StyleExpression::Function {
            name: "color".to_string(),
            args: vec![StyleExpression::String("red".to_string())],
        };

        let result = expr.evaluate(&empty_props());
        assert_eq!(result, PropertyValue::Color([1.0, 0.0, 0.0, 1.0]));
    }

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

    #[test]
    fn test_hex_color_parsing() {
        let color = StyleExpression::parse_hex_color("#FF8000");
        assert!((color[0] - 1.0).abs() < 0.01);
        assert!((color[1] - 0.502).abs() < 0.01);
        assert!((color[2] - 0.0).abs() < 0.01);
    }

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
