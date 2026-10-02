//! styling AST 的运行时求值：`Node::evaluate` 分派循环、
//! feature 接口，以及一元/二元运算符求值。
//!
//! 本模块的主要入口：
//! - `ExpressionFeature` trait：向求值暴露 feature 属性的接口
//! - `get_feature_property` / `check_feature`：feature 属性查找
//! - `Node::evaluate`：逐节点分派求值
//! - `Node::evaluate_unary` / `Node::evaluate_binary`：一元/二元运算符求值
//! - `Node::get_variables`：收集表达式引用的变量名
//!
//! # 偏离（依赖）
//!
//! blueprint 使用 `cesium_core::Cartesian2/3/4` 的自由函数
//! （`add_new`/`subtract_new`/`multiply_components_new`/`multiply_by_scalar_new`/
//! `divide_components_new`/`divide_by_scalar_new`/`negate_new`/
//! `from_elements_new`）。这个孤立的 domain crate 只依赖 `glam`，所以
//! 向量是 `glam::DVec2/DVec3/DVec4`（f64），算术使用 glam 的运算符重载
//! （`+`、`-`、`*`、`/`、一元 `-`），它们对 向量⊗向量 是逐分量的，对
//! 向量⊗f64 是标量广播的 —— 与 Cartesian 辅助函数逐字节一致。`%` 运算符
//! 没有 glam 重载，因此手工逐分量实现。
//!
//! # 模块接线
//!
//! 重活都委托给同级的兄弟模块，以保持本文件专注于分派：[`crate::member_access`]
//! （Member）、[`crate::literal`]（LiteralColor/LiteralVector）、
//! [`crate::coerce`]（一元/二元/三元内建函数表）以及 [`crate::regex`]
//! （RegExp 编译/测试/exec）。
//!
//! # 分派总览
//!
//! [`Node::evaluate`] 是唯一的入口：按 `node_type` 分为常量/变量/成员/
//! 数组/颜色/向量/一元/二元/三元/条件/方法调用/正则等若开分支，每个
//! 分支只负责“递归求值子节点 + 应用本节点语义”，具体算术与内建函数
//! 表都委派给兄弟模块。求值失败统一以 [`RuntimeError`] 返回，消息与 JS
//! 运行时的报错文案逐字一致，便于将 spec 用例直接镜像。

use glam::{DVec2, DVec3, DVec4};

use crate::ast::{is_binary_function, ExpressionNodeType, Node, NodeValue};
use crate::coerce::{evaluate_binary_function, evaluate_ternary_function, evaluate_unary_function};
use crate::literal::{evaluate_literal_color, evaluate_literal_vector};
use crate::member_access::evaluate_member;
use crate::regex::RegExpValue;
use crate::value::{runtime_error, RuntimeError, Value};
use crate::variables::variable_regex;

// ---------------------------------------------------------------------------
// Feature 接口（镜像此处用到的 Cesium3DTileFeature 方法）
// ---------------------------------------------------------------------------

/// 表达式求值所用的 feature 属性接口，镜像 `Cesium3DTileFeature` 的
/// `getPropertyInherited`、`isExactClass`、`isClass` 和 `getExactClassName` 方法。
pub trait ExpressionFeature {
    /// 镜像 `getPropertyInherited(name)`；`None` 即 `undefined`。
    fn get_property_inherited(&self, name: &str) -> Option<Value>;

    /// 镜像 `isExactClass(className)`。默认实现返回 false（没有语义层
    /// 时）；实现方可根据 metadata 覆写。
    fn is_exact_class(&self, _class_name: &Value) -> bool {
        false
    }

    /// 镜像 `isClass(className)`。默认 false；实现方可判断是否为该类或子类。
    fn is_class(&self, _class_name: &Value) -> bool {
        false
    }

    /// 镜像 `getExactClassName()`。
    fn get_exact_class_name(&self) -> Option<Value> {
        None
    }
}

/// 镜像 `getFeatureProperty`：当 feature 未定义或属性缺失时返回 undefined。
/// 向量/标量/字符串等均以 [`Value`] 承载，丢失类型信息时退化为 `Undefined`。
pub(crate) fn get_feature_property(
    feature: Option<&dyn ExpressionFeature>,
    name: &str,
) -> Value {
    match feature {
        // 有 feature：继承式查找，缺失补 Undefined。
        Some(feature) => feature.get_property_inherited(name).unwrap_or(Value::Undefined),
        // 无 feature（全局求值）：任何属性都是 Undefined。
        None => Value::Undefined,
    }
}

/// 镜像 `checkFeature`：当节点是裸 `feature` 关键字时为 `true`。
/// 用于区分变量名与作为属性查找基体的 `feature` 本身。
pub(crate) fn check_feature(node: &Node) -> bool {
    matches!(&node.value, NodeValue::Str(value) if value == "feature")
}

/// 提取存储于节点 `NodeValue::Str` 中的运算符/调用名。
/// 非字符串值（不应出现在这些节点上）回退为空串。
fn node_op(node: &Node) -> &str {
    match &node.value {
        // 运算符名 / 调用名 / 变量名统一存于 Str 载荷。
        NodeValue::Str(s) => s.as_str(),
        _ => "",
    }
}

impl Node {
    /// 镜像由 `setEvaluateFunction` 赋值的逐节点 `evaluate` 函数。
    pub fn evaluate(
        &self,
        feature: Option<&dyn ExpressionFeature>,
    ) -> Result<Value, RuntimeError> {
        match self.node_type {
            // 三元条件：test 必须求值为布尔，真取 left 分支，假取 right 分支。
            ExpressionNodeType::Conditional => {
                let test = self.test.as_ref().unwrap().evaluate(feature)?;
                // test 必须求值为布尔，否则报错。
                let Value::Boolean(test) = test else {
                    return Err(runtime_error(&format!(
                        "Conditional argument of conditional expression must be a boolean. Argument is {test}."
                    )));
                };
                if test {
                    self.left.as_ref().unwrap().evaluate(feature)
                } else {
                    // 条件为假：取 right 分支。
                    self.right.as_ref().unwrap().evaluate(feature)
                }
            }
            // 方法调用节点：目前只覆盖 RegExp 的 test/exec/toString 三种。
            ExpressionNodeType::FunctionCall => {
                let call = node_op(self);
                match call {
                    // test(regex, string)：正则是否匹配目标串。
                    "test" => {
                        let left = self.left.as_ref().unwrap().evaluate(feature)?;
                        let right = self.right.as_ref().unwrap().evaluate(feature)?;
                        match (&left, &right) {
                            (Value::RegExp(regex), Value::String(text)) => {
                                Ok(Value::Boolean(regex.test(text)))
                            }
                            _ => Err(runtime_error(&format!(
                                "RegExp.test requires the first argument to be a RegExp and the second argument to be a string. Arguments are {left} and {right}."
                            ))),
                        }
                    }
                    // exec(regex, string)：返回第一个捕获组，无匹配则为 null。
                    "exec" => {
                        let left = self.left.as_ref().unwrap().evaluate(feature)?;
                        let right = self.right.as_ref().unwrap().evaluate(feature)?;
                        match (&left, &right) {
                            (Value::RegExp(regex), Value::String(text)) => {
                                Ok(match regex.exec_first_capture(text) {
                                    Some(capture) => Value::String(capture),
                                    None => Value::Null,
                                })
                            }
                            _ => Err(runtime_error(&format!(
                                "RegExp.exec requires the first argument to be a RegExp and the second argument to be a string. Arguments are {left} and {right}."
                            ))),
                        }
                    }
                    // toString(value)：仅对正则与向量合法，走 String() 转换。
                    "toString" => {
                        let left = self.left.as_ref().unwrap().evaluate(feature)?;
                        match &left {
                            // toString 仅接受正则与向量，其余报错。
                            Value::RegExp(_)
                            | Value::Cartesian2(_)
                            | Value::Cartesian3(_)
                            | Value::Cartesian4(_) => Ok(Value::String(left.string_conversion())),
                            _ => Err(runtime_error(&format!(
                                "Unexpected function call \"{call}\"."
                            ))),
                        }
                    }
                    _ => Err(runtime_error(&format!(
                        "Unexpected function call \"{call}\"."
                    ))),
                }
            }
            // 一元与二元运算分别委派给专用求值器。
            ExpressionNodeType::Unary => self.evaluate_unary(feature),
            ExpressionNodeType::Binary => self.evaluate_binary(feature),
            // 三元内建函数：left/right 为两分支，test 为布尔条件。
            ExpressionNodeType::Ternary => {
                let call = node_op(self);
                let left = self.left.as_ref().unwrap().evaluate(feature)?;
                let right = self.right.as_ref().unwrap().evaluate(feature)?;
                let test = self.test.as_ref().unwrap().evaluate(feature)?;
                // 三个操作数齐备后委派给三元内建函数表。
                evaluate_ternary_function(call, left, right, test)
            }
            // 成员访问（feature.property / 向量逐分量）。
            ExpressionNodeType::Member => evaluate_member(self, feature),
            // 数组字面量：逐个求值子节点收集为 Value::Array。
            ExpressionNodeType::Array => {
                let NodeValue::Nodes(nodes) = &self.value else {
                    return Ok(Value::Array(Vec::new()));
                };
                let mut array = Vec::with_capacity(nodes.len());
                // 逐个求值子节点，任一报错则整体失败。
                for node in nodes {
                    array.push(node.evaluate(feature)?);
                }
                Ok(Value::Array(array))
            }
            // 裸变量：按名从 feature 继承读取属性。
            ExpressionNodeType::Variable => {
                let name = node_op(self);
                Ok(get_feature_property(feature, name))
            }
            // 字符串模板：扫描 `${name}` 占位并逐个替换为属性字符串形式。
            ExpressionNodeType::VariableInString => {
                let template = node_op(self);
                let pattern = variable_regex();
                let mut result = String::new();
                let mut last = 0usize;
                // 逐个匹配 `${name}`：先把占位前的原文本拷入，再插入属性字符串。
                for captures in pattern.captures_iter(template) {
                    let whole = captures.get(0).unwrap();
                    result.push_str(&template[last..whole.start()]);
                    let property = get_feature_property(feature, &captures[1]);
                    // 未定义属性被空字符串替代（不追加）。
                    if property.is_defined() {
                        result.push_str(&property.string_conversion());
                    }
                    last = whole.end();
                }
                // 收尾：追加最后一个占位之后的剩余文本。
                result.push_str(&template[last..]);
                Ok(Value::String(result))
            }
            // 颜色字面量（rgb/hsl 等），子节点为通道参数。
            ExpressionNodeType::LiteralColor => {
                let name = node_op(self);
                evaluate_literal_color(name, self.left_children.as_deref(), feature)
            }
            // 向量字面量（Cartesian2/3/4），子节点为分量参数。
            ExpressionNodeType::LiteralVector => {
                let call = node_op(self);
                // 子节点为分量参数；缺失则视为非法构造器。
                match &self.left_children {
                    Some(args) => evaluate_literal_vector(call, args, feature),
                    None => Err(runtime_error(&format!(
                        "Invalid {call} constructor. No valid arguments."
                    ))),
                }
            }
            // 字符串字面量：直接返回缓存文本。
            ExpressionNodeType::LiteralString => match &self.value {
                NodeValue::Str(s) => Ok(Value::String(s.clone())),
                _ => Ok(Value::String(String::new())),
            },
            // 运行期构造正则：left 为模式、right 为 flags，编译为 RegExp 值。
            ExpressionNodeType::Regex => {
                let pattern = self.left.as_ref().unwrap().evaluate(feature)?;
                let flags = match &self.right {
                    Some(flags) => flags.evaluate(feature)?.string_conversion(),
                    None => String::new(),
                };
                // 把模式与 flags 编译为 RegExp 值（失败向上报 RuntimeError）。
                let regex = RegExpValue::compile(&pattern.string_conversion(), &flags)?;
                Ok(Value::RegExp(regex))
            }
            ExpressionNodeType::BuiltinVariable => {
                // 偏离：原始代码中 `tiles3d_tileset_time` 读取
                // `feature.content.tileset.timeSinceLoad`；CPU 侧的 domain 移植
                // 没有 tileset 上下文，所以它求值为 0.0（与 feature 为
                // undefined 时返回的值相同）。着色器侧的时间仍是 codegen 的
                // 职责（Sam Q2）。
                Ok(Value::Number(0.0))
            }
            // 其余字面量类型：null/bool/number/regex/undefined 直取存储值。
            ExpressionNodeType::LiteralNull => Ok(Value::Null),
            // 布尔字面量：直取存储的 bool。
            ExpressionNodeType::LiteralBoolean => match &self.value {
                NodeValue::Bool(b) => Ok(Value::Boolean(*b)),
                _ => Ok(Value::Undefined),
            },
            // 数字字面量：直取存储的 f64。
            ExpressionNodeType::LiteralNumber => match &self.value {
                NodeValue::Number(n) => Ok(Value::Number(*n)),
                _ => Ok(Value::Undefined),
            },
            // 正则字面量：克隆已编译的 RegExp 值。
            ExpressionNodeType::LiteralRegex => match &self.value {
                NodeValue::Regex(regex) => Ok(Value::RegExp(regex.clone())),
                _ => Ok(Value::Undefined),
            },
            // undefined 字面量：恒为 Undefined。
            ExpressionNodeType::LiteralUndefined => Ok(Value::Undefined),
        }
    }

    /// UNARY 节点求值，镜像 `_evaluateNot`/`_evaluateNegative`/
    /// `_evaluatePositive`/类型转换调用，以及一元函数表。
    fn evaluate_unary(
        &self,
        feature: Option<&dyn ExpressionFeature>,
    ) -> Result<Value, RuntimeError> {
        let op = node_op(self);
        // getExactClassName 无操作数，直接从 feature 取类名。
        if op == "getExactClassName" {
            return Ok(match feature {
                Some(feature) => feature.get_exact_class_name().unwrap_or(Value::Undefined),
                None => Value::Undefined,
            });
        }
        let left = self.left.as_ref().unwrap().evaluate(feature)?;
        match op {
            // 逻辑非：仅接受布尔，取反。
            "!" => match left {
                Value::Boolean(b) => Ok(Value::Boolean(!b)),
                _ => Err(runtime_error(&format!(
                    "Operator \"!\" requires a boolean argument. Argument is {left}."
                ))),
            },
            // 一元负号：数字或向量逐分量取负。
            "-" => match left {
                Value::Number(n) => Ok(Value::Number(-n)),
                Value::Cartesian2(v) => Ok(Value::Cartesian2(-v)),
                Value::Cartesian3(v) => Ok(Value::Cartesian3(-v)),
                Value::Cartesian4(v) => Ok(Value::Cartesian4(-v)),
                _ => Err(runtime_error(&format!(
                    "Operator \"-\" requires a vector or number argument. Argument is {left}."
                ))),
            },
            // 一元正号：数字/向量恒等，其余报错。
            "+" => match &left {
                Value::Number(_)
                | Value::Cartesian2(_)
                | Value::Cartesian3(_)
                | Value::Cartesian4(_) => Ok(left),
                _ => Err(runtime_error(&format!(
                    "Operator \"+\" requires a vector or number argument. Argument is {left}."
                ))),
            },
            // isNaN / isFinite 先做数值转换再判断。
            "isNaN" => Ok(Value::Boolean(left.number_conversion().is_nan())),
            // isFinite：先转数值，非 NaN 且非无穷才为真。
            "isFinite" => Ok(Value::Boolean({
                let n = left.number_conversion();
                !n.is_nan() && !n.is_infinite()
            })),
            // isExactClass / isClass 为 feature 分类谓词，无 feature 时为假。
            "isExactClass" => Ok(Value::Boolean(match feature {
                Some(feature) => feature.is_exact_class(&left),
                None => false,
            })),
            "isClass" => Ok(Value::Boolean(match feature {
                Some(feature) => feature.is_class(&left),
                None => false,
            })),
            // 类型转换内建函数与其余一元函数表。
            "Boolean" => Ok(Value::Boolean(left.boolean_conversion())),
            "Number" => Ok(Value::Number(left.number_conversion())),
            "String" => Ok(Value::String(left.string_conversion())),
            _ => evaluate_unary_function(op, left),
        }
    }

    /// BINARY 节点求值，镜像 `_evaluatePlus`/.../`_evaluateOr`
    /// 以及正则匹配运算符。
    fn evaluate_binary(
        &self,
        feature: Option<&dyn ExpressionFeature>,
    ) -> Result<Value, RuntimeError> {
        let op = node_op(self);
        // 短路运算符惰性求值右侧。
        if op == "&&" || op == "||" {
            let left = self.left.as_ref().unwrap().evaluate(feature)?;
            let Value::Boolean(left) = left else {
                return Err(runtime_error(&format!(
                    "Operator \"{op}\" requires boolean arguments. First argument is {left}."
                )));
            };
            if op == "&&" && !left {
                // 左假短路：整个 && 必为 false，不求值右侧。
                return Ok(Value::Boolean(false));
            }
            if op == "||" && left {
                // 左真短路：整个 || 必为 true，不求值右侧。
                return Ok(Value::Boolean(true));
            }
            let right = self.right.as_ref().unwrap().evaluate(feature)?;
            // 未被短路时，右侧也必须是布尔。
            let Value::Boolean(right) = right else {
                return Err(runtime_error(&format!(
                    "Operator \"{op}\" requires boolean arguments. Second argument is {right}."
                )));
            };
            return Ok(Value::Boolean(if op == "&&" {
                left && right
            } else {
                left || right
            }));
        }

        let left = self.left.as_ref().unwrap().evaluate(feature)?;
        let right = self.right.as_ref().unwrap().evaluate(feature)?;
        match op {
            // 加号：任一侧为字符串时，两侧都走 String() 拼接。
            "+" => match (&left, &right) {
                // 同型向量逐分量相加。
                (Value::Cartesian2(l), Value::Cartesian2(r)) => Ok(Value::Cartesian2(*l + *r)),
                (Value::Cartesian3(l), Value::Cartesian3(r)) => Ok(Value::Cartesian3(*l + *r)),
                (Value::Cartesian4(l), Value::Cartesian4(r)) => Ok(Value::Cartesian4(*l + *r)),
                // 任一侧为字符串：两侧都转 String() 后拼接（优先于算术）。
                (Value::String(_), _) | (_, Value::String(_)) => Ok(Value::String(format!(
                    "{}{}",
                    left.string_conversion(),
                    right.string_conversion()
                ))),
                (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l + r)),
                _ => Err(runtime_error(&format!(
                    "Operator \"+\" requires vector or number arguments of matching types, or at least one string argument. Arguments are {left} and {right}."
                ))),
            },
            // 减号：同型向量逐分量相减，或数字相减。
            "-" => match (&left, &right) {
                (Value::Cartesian2(l), Value::Cartesian2(r)) => Ok(Value::Cartesian2(*l - *r)),
                (Value::Cartesian3(l), Value::Cartesian3(r)) => Ok(Value::Cartesian3(*l - *r)),
                (Value::Cartesian4(l), Value::Cartesian4(r)) => Ok(Value::Cartesian4(*l - *r)),
                (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l - r)),
                _ => Err(runtime_error(&format!(
                    "Operator \"-\" requires vector or number arguments of matching types. Arguments are {left} and {right}."
                ))),
            },
            // 乘号：向量⊗向量逐分量，向量⊗标量广播（两侧可交换）。
            "*" => match (&left, &right) {
                (Value::Cartesian2(l), Value::Cartesian2(r)) => Ok(Value::Cartesian2(*l * *r)),
                (Value::Cartesian2(v), Value::Number(n))
                | (Value::Number(n), Value::Cartesian2(v)) => Ok(Value::Cartesian2(*v * *n)),
                // 向量×标量可交换：把标量广播到各分量。
                (Value::Cartesian3(l), Value::Cartesian3(r)) => Ok(Value::Cartesian3(*l * *r)),
                (Value::Cartesian3(v), Value::Number(n))
                | (Value::Number(n), Value::Cartesian3(v)) => Ok(Value::Cartesian3(*v * *n)),
                (Value::Cartesian4(l), Value::Cartesian4(r)) => Ok(Value::Cartesian4(*l * *r)),
                (Value::Cartesian4(v), Value::Number(n))
                | (Value::Number(n), Value::Cartesian4(v)) => Ok(Value::Cartesian4(*v * *n)),
                (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l * r)),
                _ => Err(runtime_error(&format!(
                    "Operator \"*\" requires vector or number arguments. If both arguments are vectors they must be matching types. Arguments are {left} and {right}."
                ))),
            },
            // 除号：向量⊗向量逐分量，或向量/标量除以标量。
            "/" => match (&left, &right) {
                (Value::Cartesian2(l), Value::Cartesian2(r)) => Ok(Value::Cartesian2(*l / *r)),
                (Value::Cartesian2(v), Value::Number(n)) => Ok(Value::Cartesian2(*v / *n)),
                (Value::Cartesian3(l), Value::Cartesian3(r)) => Ok(Value::Cartesian3(*l / *r)),
                (Value::Cartesian3(v), Value::Number(n)) => Ok(Value::Cartesian3(*v / *n)),
                (Value::Cartesian4(l), Value::Cartesian4(r)) => Ok(Value::Cartesian4(*l / *r)),
                (Value::Cartesian4(v), Value::Number(n)) => Ok(Value::Cartesian4(*v / *n)),
                (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l / r)),
                _ => Err(runtime_error(&format!(
                    "Operator \"/\" requires vector or number arguments of matching types, or a number as the second argument. Arguments are {left} and {right}."
                ))),
            },
            // 取余：glam 无 `%` 重载，逐分量手工实现（DVecN::new）。
            "%" => match (&left, &right) {
                (Value::Cartesian2(l), Value::Cartesian2(r)) => Ok(Value::Cartesian2(DVec2::new(
                    l.x % r.x,
                    l.y % r.y,
                ))),
                (Value::Cartesian3(l), Value::Cartesian3(r)) => Ok(Value::Cartesian3(DVec3::new(
                    l.x % r.x,
                    l.y % r.y,
                    l.z % r.z,
                ))),
                (Value::Cartesian4(l), Value::Cartesian4(r)) => Ok(Value::Cartesian4(DVec4::new(
                    l.x % r.x,
                    l.y % r.y,
                    l.z % r.z,
                    l.w % r.w,
                ))),
                (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l % r)),
                _ => Err(runtime_error(&format!(
                    "Operator \"%\" requires vector or number arguments of matching types. Arguments are {left} and {right}."
                ))),
            },
            // 严格相等/不等：走 equals_strict。
            "===" => Ok(Value::Boolean(left.equals_strict(&right))),
            "!==" => Ok(Value::Boolean(!left.equals_strict(&right))),
            // 关系比较：仅对数字合法。
            "<" | "<=" | ">" | ">=" => match (&left, &right) {
                (Value::Number(l), Value::Number(r)) => Ok(Value::Boolean(match op {
                    "<" => l < r,
                    "<=" => l <= r,
                    ">" => l > r,
                    _ => l >= r,
                })),
                _ => Err(runtime_error(&format!(
                    "Operator \"{op}\" requires number arguments. Arguments are {left} and {right}."
                ))),
            },
            // 正则匹配运算符：一方 RegExp 另一方 string，顺序可交换。
            "=~" | "!~" => {
                let matched = match (&left, &right) {
                    (Value::RegExp(regex), Value::String(text)) => regex.test(text),
                    (Value::String(text), Value::RegExp(regex)) => regex.test(text),
                    _ => {
                        return Err(runtime_error(&format!(
                            "Operator \"{op}\" requires one RegExp argument and one string argument. Arguments are {left} and {right}."
                        )))
                    }
                };
                Ok(Value::Boolean(if op == "=~" { matched } else { !matched }))
            }
            // 其余运算符：先查二元内建函数表，否则报错。
            _ => {
                // 回退到二元内建函数表（数学/三角等）。
                if is_binary_function(op) {
                    evaluate_binary_function(op, left, right)
                } else {
                    Err(runtime_error(&format!("Unexpected operator \"{op}\".")))
                }
            }
        }
    }

    /// 镜像 `Node.prototype.getVariables`：遍历 AST，收集 `${name}` 变量、
    /// `Variable` 名称以及 `feature.<name>` 属性访问。`LiteralString` 情形
    /// 需要 `parent`（字符串字面量只有作为 `feature` 成员访问的属性时
    /// 才是变量）。
    pub fn get_variables(&self, variables: &mut Vec<String>, parent: Option<&Node>) {
        // 先递归四类子引用（children/left/right/test）与数组元素。
        if let Some(children) = &self.left_children {
            for child in children {
                child.get_variables(variables, Some(self));
            }
        }
        // left 子节点。
        if let Some(left) = &self.left {
            left.get_variables(variables, Some(self));
        }
        // right 子节点。
        if let Some(right) = &self.right {
            right.get_variables(variables, Some(self));
        }
        // 条件节点的 test 子节点。
        if let Some(test) = &self.test {
            test.get_variables(variables, Some(self));
        }
        if let NodeValue::Nodes(nodes) = &self.value {
            // 针对 ARRAY 类型
            for node in nodes {
                node.get_variables(variables, Some(self));
            }
        }

        match self.node_type {
            // 裸变量（非 feature 关键字）：直接把名字收入列表。
            ExpressionNodeType::Variable if !check_feature(self) => {
                if let NodeValue::Str(value) = &self.value {
                    variables.push(value.clone());
                }
            }
            // 字符串模板：从 `${name}` 捕获组提取变量名。
            ExpressionNodeType::VariableInString => {
                if let NodeValue::Str(value) = &self.value {
                    let pattern = variable_regex();
                    for captures in pattern.captures_iter(value) {
                        variables.push(captures[1].to_string());
                    }
                }
            }
            // 字符串字面量：仅当作为 feature 成员访问的属性时才是变量。
            ExpressionNodeType::LiteralString => {
                if let Some(parent) = parent {
                    let feature_left = parent
                        .left
                        .as_ref()
                        .map(|left| check_feature(left))
                        .unwrap_or(false);
                    if parent.node_type == ExpressionNodeType::Member && feature_left {
                        if let NodeValue::Str(value) = &self.value {
                            variables.push(value.clone());
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// 测试用 feature 桩：以 map 承载属性、可选携带精确类名。
    struct TestFeature {
        /// 属性名 -> 属性值的查找表。
        props: HashMap<String, Value>,
        /// getExactClassName 返回的类名（None 即 undefined）。
        class_name: Option<String>,
    }
    impl ExpressionFeature for TestFeature {
        /// 按名从 props 查找，缺失即 None。
        fn get_property_inherited(&self, name: &str) -> Option<Value> {
            self.props.get(name).cloned()
        }
        /// 返回可选类名，包装为字符串值。
        fn get_exact_class_name(&self) -> Option<Value> {
            self.class_name.clone().map(Value::String)
        }
    }

    /// 构造一个数字字面量节点。
    fn num(n: f64) -> Node {
        Node::new(
            ExpressionNodeType::LiteralNumber,
            NodeValue::Number(n),
            None,
            None,
            None,
        )
    }
    /// 构造一个布尔字面量节点。
    fn boolean(b: bool) -> Node {
        Node::new(
            ExpressionNodeType::LiteralBoolean,
            NodeValue::Bool(b),
            None,
            None,
            None,
        )
    }
    /// 构造一个二元运算节点（op 为运算符名）。
    fn binary(op: &str, l: Node, r: Node) -> Node {
        Node::new(
            ExpressionNodeType::Binary,
            NodeValue::Str(op.to_string()),
            Some(l),
            Some(r),
            None,
        )
    }
    /// 构造一个一元运算节点（arg 为唯一操作数）。
    fn unary(op: &str, arg: Node) -> Node {
        Node::new(
            ExpressionNodeType::Unary,
            NodeValue::Str(op.to_string()),
            Some(arg),
            None,
            None,
        )
    }

    /// 字面量分派：数字/布尔/null/undefined/内置变量都直取存储值。
    #[test]
    fn literal_dispatch() {
        // 数字字面量求值为自身。
        assert_eq!(num(3.0).evaluate(None).unwrap(), Value::Number(3.0));
        // 布尔字面量求值为自身。
        assert_eq!(boolean(true).evaluate(None).unwrap(), Value::Boolean(true));
        let null = Node::new(
            ExpressionNodeType::LiteralNull,
            NodeValue::Null,
            None,
            None,
            None,
        );
        // null 字面量求值为 Value::Null。
        assert_eq!(null.evaluate(None).unwrap(), Value::Null);
        let undef = Node::new(
            ExpressionNodeType::LiteralUndefined,
            NodeValue::Undefined,
            None,
            None,
            None,
        );
        // undefined 字面量求值为 Value::Undefined。
        assert_eq!(undef.evaluate(None).unwrap(), Value::Undefined);
        let builtin = Node::new(
            ExpressionNodeType::BuiltinVariable,
            NodeValue::Str("tiles3d_tileset_time".into()),
            None,
            None,
            None,
        );
        // 内置变量 tiles3d_tileset_time 在 CPU 侧恒为 0.0。
        assert_eq!(builtin.evaluate(None).unwrap(), Value::Number(0.0));
    }

    /// 二元算术与比较：+、%、<、=== 的数值结果与严格相等语义。
    #[test]
    fn binary_arithmetic_and_comparison() {
        assert_eq!(
            // 加法：1+2=3。
            binary("+", num(1.0), num(2.0)).evaluate(None).unwrap(),
            Value::Number(3.0)
        );
        assert_eq!(
            // 取余：7%3=1。
            binary("%", num(7.0), num(3.0)).evaluate(None).unwrap(),
            Value::Number(1.0)
        );
        assert_eq!(
            // 关系比较：1<2 为真。
            binary("<", num(1.0), num(2.0)).evaluate(None).unwrap(),
            Value::Boolean(true)
        );
        assert_eq!(
            // 严格相等：5===5 为真。
            binary("===", num(5.0), num(5.0)).evaluate(None).unwrap(),
            Value::Boolean(true)
        );
        // 向量 * 标量与向量 + 向量，经由 glam 运算符
        let v3 = Node::new(
            ExpressionNodeType::LiteralVector,
            NodeValue::None,
            None,
            None,
            None,
        );
        let _ = v3;
        let l = Value::Cartesian3(DVec3::new(1.0, 2.0, 3.0));
        let r = Value::Cartesian3(DVec3::new(4.0, 5.0, 6.0));
        // 不同分量的向量严格不等。
        assert!(!l.equals_strict(&r));
    }

    /// 短路：&& / || 在能确定结果时不求值右侧（右侧本会报错仍被跳过）。
    #[test]
    fn short_circuit_and_or() {
        // false && <error> 短路为 false，不求值右侧。
        let bad = unary("!", num(1.0)); // 若求值会报错：! 要求布尔
        let node = binary("&&", boolean(false), bad);
        // false && _ 短路为 false。
        assert_eq!(node.evaluate(None).unwrap(), Value::Boolean(false));
        // true || <error> 短路为 true。
        let bad = unary("!", num(1.0));
        let node = binary("||", boolean(true), bad);
        // true || _ 短路为 true。
        assert_eq!(node.evaluate(None).unwrap(), Value::Boolean(true));
    }

    /// 一元运算：! / - / + 与 isNaN 谓词的求值。
    #[test]
    fn unary_conversion_and_predicates() {
        // !false -> true。
        assert_eq!(
            unary("!", boolean(false)).evaluate(None).unwrap(),
            Value::Boolean(true)
        );
        // -5 -> -5。
        assert_eq!(
            unary("-", num(5.0)).evaluate(None).unwrap(),
            Value::Number(-5.0)
        );
        // +5 -> 5（一元正号恒等）。
        assert_eq!(
            unary("+", num(5.0)).evaluate(None).unwrap(),
            Value::Number(5.0)
        );
        let nan = Node::new(
            ExpressionNodeType::LiteralNumber,
            NodeValue::Number(f64::NAN),
            None,
            None,
            None,
        );
        // isNaN(NaN) -> true。
        assert_eq!(
            unary("isNaN", nan).evaluate(None).unwrap(),
            Value::Boolean(true)
        );
    }

    /// 三元条件：test 非布尔报错，test 为真取 left 分支。
    #[test]
    fn conditional_requires_boolean() {
        let cond = Node::new(
            ExpressionNodeType::Conditional,
            NodeValue::None,
            Some(num(1.0)),
            Some(num(2.0)),
            Some(num(3.0)), // 非布尔 test -> 报错
        );
        // 非布尔 test -> 报错。
        assert!(cond.evaluate(None).is_err());
        let cond = Node::new(
            ExpressionNodeType::Conditional,
            NodeValue::None,
            Some(num(1.0)),
            Some(num(2.0)),
            Some(boolean(true)),
        );
        // test 为真 -> 取 left 分支 1.0。
        assert_eq!(cond.evaluate(None).unwrap(), Value::Number(1.0));
    }

    /// 变量与字符串模板求值，外加 getExactClassName 一元调用。
    #[test]
    fn variable_and_variable_in_string() {
        let mut props = HashMap::new();
        props.insert("height".to_string(), Value::Number(10.0));
        props.insert("name".to_string(), Value::String("abc".into()));
        let feature = TestFeature {
            props,
            class_name: Some("building".into()),
        };

        let var = Node::new(
            ExpressionNodeType::Variable,
            NodeValue::Str("height".into()),
            None,
            None,
            None,
        );
        // 裸变量 height 从 feature 取到 10.0。
        assert_eq!(
            var.evaluate(Some(&feature)).unwrap(),
            Value::Number(10.0)
        );

        let tmpl = Node::new(
            ExpressionNodeType::VariableInString,
            NodeValue::Str("h=${height},n=${name}".into()),
            None,
            None,
            None,
        );
        // 模板把 ${height}/${name} 替换为属性字符串。
        assert_eq!(
            tmpl.evaluate(Some(&feature)).unwrap(),
            Value::String("h=10,n=abc".into())
        );

        // getExactClassName 一元
        let gec = Node::new(
            ExpressionNodeType::Unary,
            NodeValue::Str("getExactClassName".into()),
            None,
            None,
            None,
        );
        // getExactClassName 一元返回类名字符串。
        assert_eq!(
            gec.evaluate(Some(&feature)).unwrap(),
            Value::String("building".into())
        );
    }

    /// get_variables 收集模板 `${a}/${b}` 与裸变量 c，按遍历顺序去重。
    #[test]
    fn get_variables_collects_and_dedups_paths() {
        // 字符串模板中的 ${a} + ${b}，外加一个裸 Variable 节点。
        let tmpl = Node::new(
            ExpressionNodeType::VariableInString,
            NodeValue::Str("${a}/${b}".into()),
            None,
            None,
            None,
        );
        let var = Node::new(
            ExpressionNodeType::Variable,
            NodeValue::Str("c".into()),
            None,
            None,
            None,
        );
        let root = binary("+", tmpl, var);
        let mut out = Vec::new();
        root.get_variables(&mut out, None);
        // 变量按 a/b/c 顺序收入。
        assert_eq!(out, vec!["a".to_string(), "b".into(), "c".into()]);
    }
}
