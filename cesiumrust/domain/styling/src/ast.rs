//! 3D Tiles Styling 表达式引擎的运行时 AST。
//!
//! 该模块聚合了运行时 AST 的各组成部分：
//! - `ExpressionNodeType`              ← `expression_node_type.rs`（19 个变体）
//! - `NodeValue` + `Node`              ← L307-366
//! - `JsepNode` + `JsepLiteral`        ← L634-670
//! - 运算符表                    ← L903-906
//! - `create_runtime_ast` + `parse_*`  ← L909-1569（jsep-AST → 运行时-AST 桥）
//! - `vector_component`/`member_access`← L1889-1954
//!
//! # M7-A 作用域说明（基础层）
//!
//! * 本模块从 jsep AST 构建**运行时 AST**。Pratt 解析器
//!   （tokens → [`JsepNode`]）与所有求值（`evaluate_*`）都在 M7-B。
//! * M7-B：[`parse_call`] 的 `regExp(...)` 分支现在委托给
//!   `crate::regex::parse_regex`（`regex` crate 在离线环境下可用），因此
//!   对字面模式会构造出 [`NodeValue::Regex`]。其他每一个
//!   `parse_call` 分支（color/rgb/hsl/rgba/hsla/vec2-4/unary/binary/ternary/
//!   Boolean/Number/String/test/exec/toString）都是纯粹的结构化节点构建。
//! * 完整的预处理流水线（`replaceDefines`/`removeBackslashes`/
//!   `replaceVariables`）位于 `variables.rs`（M7-B）；此处只保留
//!   `replace_backslashes`（`parse_literal` 需要）。

use crate::regex::RegExpValue;
use crate::value::{runtime_error, RuntimeError, Value};

// ---------------------------------------------------------------------------
// 节点类型（ExpressionNodeType）
// ---------------------------------------------------------------------------

/// 运行时 AST 节点的类型。判别值镜像原 JS 对象的数值；
/// [`ExpressionNodeType::is_literal_type`] 依赖 `>= LiteralNull` 的排序，
/// 与原 `isLiteralType` 完全一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ExpressionNodeType {
    /// 一个 `${name}` 变量引用。
    Variable = 0,
    /// 一元运算符或单参数函数。
    Unary = 1,
    /// 二元运算符或双参数函数。
    Binary = 2,
    /// 三参数函数。
    Ternary = 3,
    /// 条件 `? :` 表达式。
    Conditional = 4,
    /// 成员访问（`.` 或 `[]`）。
    Member = 5,
    /// 对对象的函数调用（例如 `regExp(...).test(...)`）。
    FunctionCall = 6,
    /// 数组字面量。
    Array = 7,
    /// 在求值时构造的正则表达式。
    Regex = 8,
    /// 含 `${name}` 占位符的字符串字面量。
    VariableInString = 9,
    /// `null` 字面量。
    LiteralNull = 10,
    /// 布尔字面量。
    LiteralBoolean = 11,
    /// 数字字面量。
    LiteralNumber = 12,
    /// 字符串字面量。
    LiteralString = 13,
    /// 颜色字面量（`color`/`rgb`/`rgba`/`hsl`/`hsla`）。
    LiteralColor = 14,
    /// 向量字面量（`vec2`/`vec3`/`vec4`）。
    LiteralVector = 15,
    /// 正则表达式字面量（预编译）。
    LiteralRegex = 16,
    /// `undefined` 字面量。
    LiteralUndefined = 17,
    /// 内建变量（例如 `tiles3d_tileset_time`）。
    BuiltinVariable = 18,
}

impl ExpressionNodeType {
    /// 当节点持有一个字面值时返回 `true`，镜像
    /// `isLiteralType`：`node._type >= ExpressionNodeType.LITERAL_NULL`。
    pub fn is_literal_type(self) -> bool {
        self >= ExpressionNodeType::LiteralNull
    }
}

// ---------------------------------------------------------------------------
// 运行时 AST 节点
// ---------------------------------------------------------------------------

/// 运行时 AST 节点的负载，镜像 JS 的 `_value`。
#[derive(Debug, Clone)]
pub enum NodeValue {
    /// 无值（例如 `getExactClassName`）。
    None,
    /// JS `null` 值。
    Null,
    /// JS `undefined` 值。
    Undefined,
    /// 布尔标量。
    Bool(bool),
    /// 双精度数值标量。
    Number(f64),
    /// 字符串标量（关键字/属性名/字面量）。
    Str(String),
    /// 子表达式（ARRAY 节点类型）。
    Nodes(Vec<Node>),
    /// 一个预编译的正则表达式（LITERAL_REGEX）。M7-A：从不
    /// 构造（正则编译推迟到 M7-B）；见模块文档。
    Regex(RegExpValue),
}

/// 一个运行时 AST 节点，对应引擎的 `Node` 构造器。
/// `_left` 既可以是单个节点，也可以是参数节点数组
/// （LITERAL_COLOR / LITERAL_VECTOR）；`left_children` 持有后者。
#[derive(Debug, Clone)]
pub struct Node {
    /// 节点类型（决定求值分支）。
    pub node_type: ExpressionNodeType,
    /// 节点携带的字面量/子节点集合值。
    pub value: NodeValue,
    /// 左操作数（二元/一元的主体；也可能是字面量入参）。
    pub left: Option<Box<Node>>,
    /// `left` 为多子节点数组时使用（color/vector 参数）。
    pub left_children: Option<Vec<Node>>,
    /// 右操作数。
    pub right: Option<Box<Node>>,
    /// 三元条件表达式的 test 子节点。
    pub test: Option<Box<Node>>,
}

impl Node {
    /// 构建一个带可选单个 `left`/`right`/`test` 子节点的节点。
    pub fn new(
        node_type: ExpressionNodeType,
        value: NodeValue,
        left: Option<Node>,
        right: Option<Node>,
        test: Option<Node>,
    ) -> Node {
        Node {
            node_type,
            value,
            left: left.map(Box::new),
            left_children: None,
            right: right.map(Box::new),
            test: test.map(Box::new),
        }
    }

    /// 构建一个其参数位于 `left_children` 中的节点（color/vector）。
    pub fn with_children(
        node_type: ExpressionNodeType,
        value: NodeValue,
        children: Vec<Node>,
    ) -> Node {
        Node {
            node_type,
            value,
            left: None,
            left_children: Some(children),
            right: None,
            test: None,
        }
    }
}

// ---------------------------------------------------------------------------
// jsep AST（由 M7-B 解析器产出，被 create_runtime_ast 消费）
// ---------------------------------------------------------------------------

/// 由解析器在 [`create_runtime_ast`] 之前产出的 jsep 风格 AST。
#[derive(Debug, Clone)]
pub enum JsepNode {
    /// 字面量（null/布尔/数字/字符串）。
    Literal(JsepLiteral),
    /// 标识符（变量名/函数名/关键字）。
    Identifier(String),
    /// `this` 表达式（styling 语言中非法，保留以对齐 jsep 形态）。
    ThisExpression,
    /// 一元运算（单操作数）。
    Unary {
        /// 一元运算符文本（`!`/`-`/`+`）。
        operator: String,
        /// 被作用的操作数。
        argument: Box<JsepNode>,
    },
    /// 二元运算（左右两操作数）。
    Binary {
        /// 二元运算符文本。
        operator: String,
        /// 左操作数。
        left: Box<JsepNode>,
        /// 右操作数。
        right: Box<JsepNode>,
    },
    /// 条件表达式 `test ? consequent : alternate`。
    Conditional {
        /// 条件测试表达式。
        test: Box<JsepNode>,
        /// 条件为真时求值的分支。
        consequent: Box<JsepNode>,
        /// 条件为假时求值的分支。
        alternate: Box<JsepNode>,
    },
    /// 成员访问（`.` 或 `[]`）。
    Member {
        /// 被访问的对象表达式。
        object: Box<JsepNode>,
        /// 被访问的属性表达式。
        property: Box<JsepNode>,
        /// 是否为方括号计算访问（`a[0]`）。
        computed: bool,
    },
    /// 函数调用（callee + 实参列表）。
    Call {
        /// 被调用的表达式（函数名/成员链）。
        callee: Box<JsepNode>,
        /// 实参列表。
        arguments: Vec<JsepNode>,
    },
    /// 数组字面量（元素列表）。
    Array(Vec<JsepNode>),
}

/// 一个 jsep 字面值。
#[derive(Debug, Clone)]
pub enum JsepLiteral {
    /// JS `null`。
    Null,
    /// 布尔字面值。
    Boolean(bool),
    /// 数字字面值。
    Number(f64),
    /// 字符串字面值。
    Str(String),
}

// ---------------------------------------------------------------------------
// createRuntimeAst（jsep AST → 运行时节点）
// ---------------------------------------------------------------------------

/// jsep/Cesium 接受的一元运算符（`!`、一元 `-`、一元 `+`）。
pub const UNARY_OPERATORS: [&str; 3] = ["!", "-", "+"];
/// 二元运算符，镜像 `jsep.binary_ops` 加上 Cesium 的
/// 定制 `addBinaryOp("=~", 0)` / `addBinaryOp("!~", 0)`。
pub const BINARY_OPERATORS: [&str; 15] = [
    "+", "-", "*", "/", "%", "===", "!==", ">", ">=", "<", "<=", "&&", "||", "!~", "=~",
];

const BACKSLASH_REPLACEMENT: &str = "@#%";

/// 镜像 `replaceBackslashes`：`"@#%"` → `\`。
pub(crate) fn replace_backslashes(expression: &str) -> String {
    // 用占位串还原真正的反斜杠
    expression.replace(BACKSLASH_REPLACEMENT, "\\")
}

/// 将一个 jsep 字面量转换为运行时 AST 节点。
///
/// 四类字面量各自映射到对应的 Literal* 节点类型；字符串若含 `${`
/// 占位符则记为 VariableInString（延迟到求值再展开），否则做反斜杠还原。
fn parse_literal(literal: &JsepLiteral) -> Node {
    // 按字面量种类分支：Null / Boolean / Number / Str
    match literal {
        JsepLiteral::Null => Node::new(
            // null → LiteralNull，无子节点。
            ExpressionNodeType::LiteralNull,
            NodeValue::Null,
            None,
            None,
            None,
        ),
        JsepLiteral::Boolean(value) => Node::new(
            // 布尔 → LiteralBoolean，值存入 NodeValue::Bool。
            ExpressionNodeType::LiteralBoolean,
            NodeValue::Bool(*value),
            None,
            None,
            None,
        ),
        JsepLiteral::Number(value) => Node::new(
            // 数字 → LiteralNumber，值存入 NodeValue::Number。
            ExpressionNodeType::LiteralNumber,
            NodeValue::Number(*value),
            None,
            None,
            None,
        ),
        JsepLiteral::Str(value) => {
            // 含 `${` 则归为 VariableInString（延迟展开），否则作反斜杠还原后存为字面串。
            if value.contains("${") {
                Node::new(
                    ExpressionNodeType::VariableInString,
                    NodeValue::Str(value.clone()),
                    None,
                    None,
                    None,
                )
            } else {
                Node::new(
                    ExpressionNodeType::LiteralString,
                    NodeValue::Str(replace_backslashes(value)),
                    None,
                    None,
                    None,
                )
            }
        }
    }
}

/// 判断一个标识符是否为 `czm_` 前缀的内置变量。
fn is_variable(name: &str) -> bool {
    // czm_ 前缀是 styling 语言声明变量引用的约定
    name.starts_with("czm_")
}

/// 去掉 `czm_` 前缀取出真实属性名（跳过前 4 个字符）。
fn get_property_name(variable: &str) -> &str {
    &variable[4..]
}

/// 将一个标识符名解析为变量、内建变量或关键字常量节点。
///
/// `czm_` 前缀走变量路径（区分 tiles3d_ 内建变量），`NaN`/`Infinity`/
/// `undefined` 走关键字常量，其余未知名报错。
fn parse_keywords_and_variables(name: &str) -> Result<Node, RuntimeError> {
    if is_variable(name) {
        // czm_ 内置变量走 BuiltinVariable，其余 czm_ 走普通 Variable
        let property = get_property_name(name);
        // tiles3d_ 前缀是引擎内置变量（如 tileset_time），单独标为 BuiltinVariable。
        if property.starts_with("tiles3d_") {
            return Ok(Node::new(
                ExpressionNodeType::BuiltinVariable,
                NodeValue::Str(property.to_string()),
                None,
                None,
                None,
            ));
        }
        return Ok(Node::new(
            ExpressionNodeType::Variable,
            NodeValue::Str(property.to_string()),
            None,
            None,
            None,
        ));
    } else if name == "NaN" {
        // NaN 关键字 → 值为 f64::NAN 的数字字面量。
        return Ok(Node::new(
            ExpressionNodeType::LiteralNumber,
            NodeValue::Number(f64::NAN),
            None,
            None,
            None,
        ));
    } else if name == "Infinity" {
        // Infinity 关键字 → 值为 +∞ 的数字字面量。
        return Ok(Node::new(
            ExpressionNodeType::LiteralNumber,
            NodeValue::Number(f64::INFINITY),
            None,
            None,
            None,
        ));
    } else if name == "undefined" {
        // undefined 关键字 → LiteralUndefined。
        return Ok(Node::new(
            ExpressionNodeType::LiteralUndefined,
            NodeValue::Undefined,
            None,
            None,
            None,
        ));
    }

    Err(runtime_error(&format!("{name} is not defined.")))
}

/// 镜像 `parseMathConstant`（对未知常量返回 None，如同
/// 原函数返回 undefined）。仅识别 `PI` 与 `E` 两个 Math 常量。
fn parse_math_constant(name: &str) -> Option<Node> {
    // 仅 PI / E 两个 Math 常量；其余返回 None（同原 JS 的 undefined）。
    if name == "PI" {
        Some(Node::new(
            ExpressionNodeType::LiteralNumber,
            NodeValue::Number(std::f64::consts::PI),
            None,
            None,
            None,
        ))
    } else if name == "E" {
        Some(Node::new(
            ExpressionNodeType::LiteralNumber,
            NodeValue::Number(std::f64::consts::E),
            None,
            None,
            None,
        ))
    } else {
        None
    }
}

/// 镜像 `parseNumberConstant`。
fn parse_number_constant(name: &str) -> Option<Node> {
    // 仅 POSITIVE_INFINITY 一个 Number 常量。
    if name == "POSITIVE_INFINITY" {
        Some(Node::new(
            ExpressionNodeType::LiteralNumber,
            NodeValue::Number(f64::INFINITY),
            None,
            None,
            None,
        ))
    } else {
        None
    }
}

/// 是否为单参数数学/向量一元函数（abs/sqrt/cos 等）。
fn is_unary_function(call: &str) -> bool {
    // 白名单：标量一元函数加上 length/normalize 两个向量一元函数。
    matches!(
        call,
        "abs" | "sqrt" | "cos" | "sin" | "tan" | "acos" | "asin" | "atan" | "radians"
            | "degrees" | "sign" | "floor" | "ceil" | "round" | "exp" | "exp2" | "log"
            | "log2" | "fract" | "length" | "normalize"
    )
}

/// 是否为双参数二元函数（atan2/pow/min/max 等）。
pub(crate) fn is_binary_function(call: &str) -> bool {
    // 白名单：双参数函数（distance/dot 取标量，cross 取向量）。
    matches!(call, "atan2" | "pow" | "min" | "max" | "distance" | "dot" | "cross")
}

/// 是否为三参数函数（clamp/mix）。
fn is_ternary_function(call: &str) -> bool {
    // 白名单：仅 clamp 与 mix 两个三参数函数。
    matches!(call, "clamp" | "mix")
}

/// 解析成员访问表达式：先处理 `Math.X`/`Number.X` 命名空间常量，
/// 否则按点访问（dot）或方括号访问（brackets）构建 Member 节点。
fn parse_member_expression(
    object: &JsepNode,
    property: &JsepNode,
    computed: bool,
) -> Result<Node, RuntimeError> {
    let object_name = match object {
        // 仅当对象/属性是直接标识符时才尝试解析 Math/Number 命名空间常量
        JsepNode::Identifier(name) => Some(name.as_str()),
        _ => None,
    };
    let property_name = match property {
        JsepNode::Identifier(name) => Some(name.as_str()),
        _ => None,
    };
    if object_name == Some("Math") {
        // Math 命名空间：仅常量可解，否则拒绝（函数调用不走此处）。
        // 属性非标识符（如 Math[expr]）时 property_name 为 None，落入错误。
        if let Some(name) = property_name {
            if let Some(node) = parse_math_constant(name) {
                return Ok(node);
            }
        }
        return Err(runtime_error("Cannot parse expression."));
    } else if object_name == Some("Number") {
        // Number 命名空间：仅 POSITIVE_INFINITY 等常量可解。
        if let Some(name) = property_name {
            if let Some(node) = parse_number_constant(name) {
                return Ok(node);
            }
        }
        return Err(runtime_error("Cannot parse expression."));
    }

    let obj = create_runtime_ast(object)?;
    if computed {
        // 方括号访问 a[i]：property 作为表达式递归求值。
        let val = create_runtime_ast(property)?;
        return Ok(Node::new(
            ExpressionNodeType::Member,
            NodeValue::Str("brackets".to_string()),
            Some(obj),
            Some(val),
            None,
        ));
    }

    // 点访问 a.b：property 必须是标识符，存为字面串成员名。
    let property_name = match property_name {
        Some(name) => name,
        None => return Err(runtime_error("Cannot parse expression.")),
    };
    let val = Node::new(
        ExpressionNodeType::LiteralString,
        NodeValue::Str(property_name.to_string()),
        None,
        None,
        None,
    );
    Ok(Node::new(
        ExpressionNodeType::Member,
        NodeValue::Str("dot".to_string()),
        Some(obj),
        Some(val),
        None,
    ))
}

/// 镜像 `parseCall`。
///
/// 偏离（M7-A）：`regExp(...)` 分支返回一个错误而非构建一个正则节点；
/// 正则编译落在 M7-B `regex.rs`
/// （`parse_regex` + `RegExpValue::compile`）。所有其他分支都是忠实的。
fn parse_call(callee: &JsepNode, arguments: &[JsepNode]) -> Result<Node, RuntimeError> {
    // 先处理成员函数调用（test/exec/toString），再处理命名构造器/全局函数
    let args_length = arguments.len();

    // 成员函数调用
    // 当 callee 是非计算成员（a.test(...)）时，先按方法名分派。
    if let JsepNode::Member {
        object,
        property,
        computed: false,
    } = callee
    {
        let call = match property.as_ref() {
            JsepNode::Identifier(name) => name.as_str(),
            _ => return Err(runtime_error("Cannot parse expression.")),
        };
        if call == "test" || call == "exec" {
            // 确保这是在一个有效类型上调用
            // test/exec 只能出现在 regExp(...) 的成员链上，否则报错。
            let is_reg_exp = matches!(
                object.as_ref(),
                JsepNode::Call { callee, .. }
                    if matches!(callee.as_ref(), JsepNode::Identifier(name) if name == "regExp")
            );
            if !is_reg_exp {
                return Err(runtime_error(&format!("{call} is not a function.")));
            }
            if args_length == 0 {
                // 空参：test 退化为 false 常量，exec 退化为 null 常量。
                if call == "test" {
                    return Ok(Node::new(
                        ExpressionNodeType::LiteralBoolean,
                        NodeValue::Bool(false),
                        None,
                        None,
                        None,
                    ));
                }
                return Ok(Node::new(
                    ExpressionNodeType::LiteralNull,
                    NodeValue::Null,
                    None,
                    None,
                    None,
                ));
            }
            let left = create_runtime_ast(object)?;
            // 有参：构造 FunctionCall 节点，left=正则对象，right=目标串。
            let right = create_runtime_ast(&arguments[0])?;
            return Ok(Node::new(
                ExpressionNodeType::FunctionCall,
                NodeValue::Str(call.to_string()),
                Some(left),
                Some(right),
                None,
            ));
        } else if call == "toString" {
            // toString 仅一个接收者操作数，无额外实参。
            let val = create_runtime_ast(object)?;
            return Ok(Node::new(
                ExpressionNodeType::FunctionCall,
                NodeValue::Str("toString".to_string()),
                Some(val),
                None,
                None,
            ));
        }

        return Err(runtime_error(&format!(
            "Unexpected function call \"{call}\"."
        )));
    }

    // 非成员函数调用
    // callee 为普通标识符时取函数名，否则拒绝。
    let call = match callee {
        JsepNode::Identifier(name) => name.as_str(),
        _ => return Err(runtime_error("Unexpected function call.")),
    };
    if call == "color" {
        // 无参 color() 退化为白色 LiteralColor
        if args_length == 0 {
            return Ok(Node::new(
                ExpressionNodeType::LiteralColor,
                NodeValue::Str("color".to_string()),
                None,
                None,
                None,
            ));
        }
        let val = create_runtime_ast(&arguments[0])?;
        // 单分量时 children 仅 1 个；多分量（r,g,b,a）逐个追加。
        let mut children = vec![val];
        if args_length > 1 {
            children.push(create_runtime_ast(&arguments[1])?);
        }
        return Ok(Node::with_children(
            ExpressionNodeType::LiteralColor,
            NodeValue::Str("color".to_string()),
            children,
        ));
    } else if call == "rgb" || call == "hsl" {
        // rgb/hsl 需要三个分量参数
        // 三个分量按入参顺序存入 left_children。
        if args_length < 3 {
            return Err(runtime_error(&format!("{call} requires three arguments.")));
        }
        let children = vec![
            create_runtime_ast(&arguments[0])?,
            create_runtime_ast(&arguments[1])?,
            create_runtime_ast(&arguments[2])?,
        ];
        return Ok(Node::with_children(
            ExpressionNodeType::LiteralColor,
            NodeValue::Str(call.to_string()),
            children,
        ));
    } else if call == "rgba" || call == "hsla" {
        // rgba/hsla 需要四个分量参数
        // 少于四个拒绝；四分量按序存入 left_children。
        if args_length < 4 {
            return Err(runtime_error(&format!("{call} requires four arguments.")));
        }
        let children = vec![
            create_runtime_ast(&arguments[0])?,
            create_runtime_ast(&arguments[1])?,
            create_runtime_ast(&arguments[2])?,
            create_runtime_ast(&arguments[3])?,
        ];
        return Ok(Node::with_children(
            ExpressionNodeType::LiteralColor,
            NodeValue::Str(call.to_string()),
            children,
        ));
    } else if call == "vec2" || call == "vec3" || call == "vec4" {
        // 在求值时检查无效构造器
        // 向量构造器参数个数不定，逐个递归后打包为 children。
        let mut children = Vec::with_capacity(args_length);
        for argument in arguments {
            children.push(create_runtime_ast(argument)?);
        }
        return Ok(Node::with_children(
            ExpressionNodeType::LiteralVector,
            NodeValue::Str(call.to_string()),
            children,
        ));
    } else if call == "isNaN" || call == "isFinite" {
        // isNaN/isFinite：无参时给常量，否则作为一元运算
        // 无参时：isNaN()→true、isFinite()→false（对齐 JS 对 undefined 的强制转换）。
        if args_length == 0 {
            let value = call == "isNaN";
            return Ok(Node::new(
                ExpressionNodeType::LiteralBoolean,
                NodeValue::Bool(value),
                None,
                None,
                None,
            ));
        }
        let val = create_runtime_ast(&arguments[0])?;
        return Ok(Node::new(
            ExpressionNodeType::Unary,
            NodeValue::Str(call.to_string()),
            Some(val),
            None,
            None,
        ));
    } else if call == "isExactClass" || call == "isClass" {
        // 分类判定函数要求恰好一个参数
        // 参数不足/过多都拒绝；恰好一个则作为一元运算。
        if args_length != 1 {
            return Err(runtime_error(&format!(
                "{call} requires exactly one argument."
            )));
        }
        let val = create_runtime_ast(&arguments[0])?;
        return Ok(Node::new(
            ExpressionNodeType::Unary,
            NodeValue::Str(call.to_string()),
            Some(val),
            None,
            None,
        ));
    } else if call == "getExactClassName" {
        // getExactClassName 不接受任何参数，产出无操作数的一元节点
        // 传入任何参数都拒绝。
        if !arguments.is_empty() {
            return Err(runtime_error(&format!(
                "{call} does not take any argument."
            )));
        }
        return Ok(Node::new(
            ExpressionNodeType::Unary,
            NodeValue::Str(call.to_string()),
            None,
            None,
            None,
        ));
    } else if is_unary_function(call) {
        // 一元数学/向量函数：校验单参数
        // 由 is_unary_function 白名单命中，构造带单子节点的 Unary。
        if args_length != 1 {
            return Err(runtime_error(&format!(
                "{call} requires exactly one argument."
            )));
        }
        let val = create_runtime_ast(&arguments[0])?;
        return Ok(Node::new(
            ExpressionNodeType::Unary,
            NodeValue::Str(call.to_string()),
            Some(val),
            None,
            None,
        ));
    } else if is_binary_function(call) {
        // 二元函数：校验双参数
        // 由 is_binary_function 命中，左右各递归后构造 Binary。
        if args_length != 2 {
            return Err(runtime_error(&format!(
                "{call} requires exactly two arguments."
            )));
        }
        let left = create_runtime_ast(&arguments[0])?;
        let right = create_runtime_ast(&arguments[1])?;
        return Ok(Node::new(
            ExpressionNodeType::Binary,
            NodeValue::Str(call.to_string()),
            Some(left),
            Some(right),
            None,
        ));
    } else if is_ternary_function(call) {
        // 三元函数：校验三参数
        // 三参数分别存入 left/right/test，构造 Ternary。
        if args_length != 3 {
            return Err(runtime_error(&format!(
                "{call} requires exactly three arguments."
            )));
        }
        let left = create_runtime_ast(&arguments[0])?;
        let right = create_runtime_ast(&arguments[1])?;
        let test = create_runtime_ast(&arguments[2])?;
        return Ok(Node::new(
            ExpressionNodeType::Ternary,
            NodeValue::Str(call.to_string()),
            Some(left),
            Some(right),
            Some(test),
        ));
    } else if call == "Boolean" {
        // Boolean()/Number()/String() 是 JS 全局构造器的强制转换
        // 无参 Boolean() 按 JS 语义得 false。
        if args_length == 0 {
            return Ok(Node::new(
                ExpressionNodeType::LiteralBoolean,
                NodeValue::Bool(false),
                None,
                None,
                None,
            ));
        }
        let val = create_runtime_ast(&arguments[0])?;
        return Ok(Node::new(
            ExpressionNodeType::Unary,
            NodeValue::Str("Boolean".to_string()),
            Some(val),
            None,
            None,
        ));
    } else if call == "Number" {
        // 无参 Number() 得 0.0；有参则作为一元强制转换。
        if args_length == 0 {
            return Ok(Node::new(
                ExpressionNodeType::LiteralNumber,
                NodeValue::Number(0.0),
                None,
                None,
                None,
            ));
        }
        let val = create_runtime_ast(&arguments[0])?;
        return Ok(Node::new(
            ExpressionNodeType::Unary,
            NodeValue::Str("Number".to_string()),
            Some(val),
            None,
            None,
        ));
    } else if call == "String" {
        // 无参 String() 得空串；有参则作为一元强制转换。
        if args_length == 0 {
            return Ok(Node::new(
                ExpressionNodeType::LiteralString,
                NodeValue::Str(String::new()),
                None,
                None,
                None,
            ));
        }
        let val = create_runtime_ast(&arguments[0])?;
        return Ok(Node::new(
            ExpressionNodeType::Unary,
            NodeValue::Str("String".to_string()),
            Some(val),
            None,
            None,
        ));
    } else if call == "regExp" {
        // M7-B：委托给 `regex.rs`（`parse_regex` + `RegExpValue::compile`）。
        return crate::regex::parse_regex(arguments);
    }

    Err(runtime_error(&format!(
        "Unexpected function call \"{call}\"."
    )))
}

/// 递归地将 jsep AST 转换为运行时 AST 节点。
///
/// 这是 M7-A 的核心入口：按 [`JsepNode`] 的种类分派到字面量、调用、
/// 变量、运算符、条件、成员访问与数组各分支。
pub fn create_runtime_ast(ast: &JsepNode) -> Result<Node, RuntimeError> {
    // 按 jsep 节点种类分派到对应的构建分支，递归下降构造运行时 AST
    match ast {
        // 字面量直接转成对应的 Literal* 节点
        // 委派给 parse_literal 处理四类字面量。
        JsepNode::Literal(literal) => Ok(parse_literal(literal)),
        // 函数调用交由 parse_call 校验参数个数与构造器
        // 调用节点的 callee/arguments 全交由 parse_call 分派。
        JsepNode::Call { callee, arguments } => parse_call(callee, arguments),
        // 标识符解析为变量引用或 NaN/Infinity 等关键字常量
        JsepNode::Identifier(name) => parse_keywords_and_variables(name),
        // this 不是 styling 语言的合法根，按未定义名处理以复用错误路径
        // 传字符串 "this" 进入关键字解析，自然命中未定义分支。
        JsepNode::ThisExpression => parse_keywords_and_variables("this"),
        // 一元运算：先递归构造操作数，再校验运算符是否受支持
        JsepNode::Unary { operator, argument } => {
            // 先构造子节点，再校验运算符属于 UNARY_OPERATORS。
            let child = create_runtime_ast(argument)?;
            if UNARY_OPERATORS.contains(&operator.as_str()) {
                Ok(Node::new(
                    ExpressionNodeType::Unary,
                    NodeValue::Str(operator.clone()),
                    Some(child),
                    None,
                    None,
                ))
            } else {
                Err(runtime_error(&format!(
                    "Unexpected operator \"{operator}\"."
                )))
            }
        }
        JsepNode::Binary {
            operator,
            left,
            right,
        } => {
            // 二元运算：两侧各递归构造，再校验运算符白名单
            // 先递归两侧，保证子节点错误优先暴露。
            let left = create_runtime_ast(left)?;
            let right = create_runtime_ast(right)?;
            if BINARY_OPERATORS.contains(&operator.as_str()) {
                Ok(Node::new(
                    ExpressionNodeType::Binary,
                    NodeValue::Str(operator.clone()),
                    Some(left),
                    Some(right),
                    None,
                ))
            } else {
                Err(runtime_error(&format!(
                    "Unexpected operator \"{operator}\"."
                )))
            }
        }
        JsepNode::Conditional {
            test,
            consequent,
            alternate,
        } => {
            // 三元条件：test/consequent/alternate 三部分分别递归构造
            // consequent → left、alternate → right、test → test。
            let test = create_runtime_ast(test)?;
            let left = create_runtime_ast(consequent)?;
            let right = create_runtime_ast(alternate)?;
            Ok(Node::new(
                ExpressionNodeType::Conditional,
                NodeValue::Str("?".to_string()),
                Some(left),
                Some(right),
                Some(test),
            ))
        }
        JsepNode::Member {
            object,
            property,
            computed,
        } => parse_member_expression(object, property, *computed),
        JsepNode::Array(elements) => {
            // 数组：逐元素递归构造后打包成 NodeValue::Nodes
            // 任一元素报错则整体失败（用 ? 短路）。
            let mut children = Vec::with_capacity(elements.len());
            for element in elements {
                children.push(create_runtime_ast(element)?);
            }
            Ok(Node::new(
                ExpressionNodeType::Array,
                NodeValue::Nodes(children),
                None,
                None,
                None,
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// 成员访问辅助（镜像 `.r/.g/.b/.a`、`.x/.y/.z/.w`、`[0]-[3]`）
// ---------------------------------------------------------------------------

/// 向量上的分量访问，镜像 `.r/.g/.b/.a`、`.x/.y/.z/.w`
/// 和 `[0]-[3]` 成员处理。
pub fn vector_component(property: &Value, member: &Value) -> Option<Value> {
    // 先把成员归一成分量名：数字索引 0..3 映射 x/y/z/w，字符串原样用
    // 非数字也非字符串的成员无法定位分量，直接 None。
    let name: &str = match member {
        Value::Number(n) => match n {
            0.0 => "x",
            1.0 => "y",
            2.0 => "z",
            3.0 => "w",
            _ => return None,
        },
        Value::String(s) => s.as_str(),
        _ => return None,
    };
    // 按向量维数匹配可用分量名，超出维度返回 None
    match property {
        Value::Cartesian2(v) => match name {
            "r" | "x" => Some(Value::Number(v.x)),
            "g" | "y" => Some(Value::Number(v.y)),
            _ => None,
        },
        Value::Cartesian3(v) => match name {
            "r" | "x" => Some(Value::Number(v.x)),
            "g" | "y" => Some(Value::Number(v.y)),
            "b" | "z" => Some(Value::Number(v.z)),
            _ => None,
        },
        Value::Cartesian4(v) => match name {
            "r" | "x" => Some(Value::Number(v.x)),
            "g" | "y" => Some(Value::Number(v.y)),
            "b" | "z" => Some(Value::Number(v.z)),
            "a" | "w" => Some(Value::Number(v.w)),
            _ => None,
        },
        _ => None,
    }
}

/// 对数组和字符串的通用成员访问（JS `property[member]`）。
pub fn member_access(property: &Value, member: &Value) -> Value {
    match property {
        // 数组：成员须为非负整数索引或可解析为 usize 的数字串
        // 仅接受整数值数字或纯数字串作为索引。
        Value::Array(items) => {
            let index = match member {
                Value::Number(n) if *n >= 0.0 && n.fract() == 0.0 => Some(*n as usize),
                Value::String(s) => s.parse::<usize>().ok(),
                _ => None,
            };
            index
                .and_then(|i| items.get(i).cloned())
                .unwrap_or(Value::Undefined)
        }
        // 字符串：支持 length 属性与按索引取字符
        Value::String(s) => match member {
            Value::String(m) if m == "length" => Value::Number(s.chars().count() as f64),
            Value::String(m) => m
                .parse::<usize>()
                .ok()
                .and_then(|i| s.chars().nth(i))
                .map(|c| Value::String(c.to_string()))
                .unwrap_or(Value::Undefined),
            Value::Number(n) if *n >= 0.0 && n.fract() == 0.0 => s
                .chars()
                .nth(*n as usize)
                .map(|c| Value::String(c.to_string()))
                .unwrap_or(Value::Undefined),
            _ => Value::Undefined,
        },
        _ => Value::Undefined,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{DVec2, DVec3, DVec4};

    /// 测试辅助：构造一个数值字面量 jsep 节点。
    fn num(v: f64) -> JsepNode {
        // 把 Rust f64 包装成 jsep 数字字面量。
        JsepNode::Literal(JsepLiteral::Number(v))
    }
    /// 测试辅助：构造一个标识符 jsep 节点。
    fn ident(name: &str) -> JsepNode {
        // 把名称包装成 jsep 标识符节点。
        JsepNode::Identifier(name.to_string())
    }
    /// 测试辅助：将节点装箱以便填入各 `Box<JsepNode>` 字段。
    fn boxed(n: JsepNode) -> Box<JsepNode> {
        Box::new(n)
    }

    // --- ExpressionNodeType：19 个变体 + 判别值 + is_literal_type ---

    #[test]
    /// 校验 ExpressionNodeType 的 19 个变体及其判别值 0..=18。
    fn node_type_discriminants_and_count() {
        // 抽查几个关键判别值，确保与 JS 数值一致。
        assert_eq!(ExpressionNodeType::Variable as u8, 0);
        assert_eq!(ExpressionNodeType::VariableInString as u8, 9);
        assert_eq!(ExpressionNodeType::LiteralNull as u8, 10);
        assert_eq!(ExpressionNodeType::LiteralRegex as u8, 16);
        assert_eq!(ExpressionNodeType::BuiltinVariable as u8, 18);
        // 共 19 个变体：判别值 0..=18。
        let all = [
            ExpressionNodeType::Variable,
            ExpressionNodeType::Unary,
            ExpressionNodeType::Binary,
            ExpressionNodeType::Ternary,
            ExpressionNodeType::Conditional,
            ExpressionNodeType::Member,
            ExpressionNodeType::FunctionCall,
            ExpressionNodeType::Array,
            ExpressionNodeType::Regex,
            ExpressionNodeType::VariableInString,
            ExpressionNodeType::LiteralNull,
            ExpressionNodeType::LiteralBoolean,
            ExpressionNodeType::LiteralNumber,
            ExpressionNodeType::LiteralString,
            ExpressionNodeType::LiteralColor,
            ExpressionNodeType::LiteralVector,
            ExpressionNodeType::LiteralRegex,
            ExpressionNodeType::LiteralUndefined,
            ExpressionNodeType::BuiltinVariable,
        ];
        assert_eq!(all.len(), 19);
    }

    #[test]
    /// is_literal_type 应在判别值 >= LiteralNull(10) 时为真。
    fn is_literal_type_ordering() {
        // 非字面类型（Variable/VariableInString）应为 false。
        assert!(!ExpressionNodeType::Variable.is_literal_type());
        assert!(!ExpressionNodeType::VariableInString.is_literal_type());
        assert!(ExpressionNodeType::LiteralNull.is_literal_type());
        assert!(ExpressionNodeType::LiteralNumber.is_literal_type());
        assert!(ExpressionNodeType::BuiltinVariable.is_literal_type());
    }

    // --- create_runtime_ast：字面量 ---

    #[test]
    /// 数值字面量应产出 LiteralNumber 节点并携带 Number 值。
    fn literal_number_node() {
        // 数字 5.0 应得到 LiteralNumber 且值回取为 5.0。
        let node = create_runtime_ast(&num(5.0)).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::LiteralNumber);
        assert!(matches!(node.value, NodeValue::Number(n) if n == 5.0));
    }

    #[test]
    /// 不含占位符的字符串应产出 LiteralString 节点。
    fn literal_string_node() {
        // 无占位符的普通字符串走 LiteralString 分支。
        let node =
            create_runtime_ast(&JsepNode::Literal(JsepLiteral::Str("hello".into()))).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::LiteralString);
        assert!(matches!(&node.value, NodeValue::Str(s) if s == "hello"));
    }

    #[test]
    /// 含 `${...}` 的字符串应归为 VariableInString。
    fn string_with_placeholder_is_variable_in_string() {
        // `${x}` 含占位符，应归为 VariableInString 而非 LiteralString。
        let node =
            create_runtime_ast(&JsepNode::Literal(JsepLiteral::Str("${x}".into()))).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::VariableInString);
    }

    #[test]
    /// null 与 boolean 字面量各自的节点类型与值。
    fn literal_null_and_boolean_nodes() {
        // null 与 true 分别落到 LiteralNull / LiteralBoolean。
        let n = create_runtime_ast(&JsepNode::Literal(JsepLiteral::Null)).unwrap();
        assert_eq!(n.node_type, ExpressionNodeType::LiteralNull);
        assert!(matches!(n.value, NodeValue::Null));

        let b = create_runtime_ast(&JsepNode::Literal(JsepLiteral::Boolean(true))).unwrap();
        assert_eq!(b.node_type, ExpressionNodeType::LiteralBoolean);
        assert!(matches!(b.value, NodeValue::Bool(true)));
    }

    // --- create_runtime_ast：关键字与变量 ---

    #[test]
    /// czm_ 变量映射为 Variable，czm_tiles3d_ 前缀映射为 BuiltinVariable。
    fn variable_and_builtin_nodes() {
        // 去前缀后的属性名应分别落在 Variable / BuiltinVariable 的 Str 值上。
        let v = create_runtime_ast(&ident("czm_height")).unwrap();
        assert_eq!(v.node_type, ExpressionNodeType::Variable);
        assert!(matches!(&v.value, NodeValue::Str(s) if s == "height"));

        let b = create_runtime_ast(&ident("czm_tiles3d_tileset_time")).unwrap();
        assert_eq!(b.node_type, ExpressionNodeType::BuiltinVariable);
        assert!(matches!(&b.value, NodeValue::Str(s) if s == "tiles3d_tileset_time"));
    }

    #[test]
    /// undefined/NaN/Infinity 关键字各自产出对应字面量节点。
    fn keyword_literals() {
        // 三个关键字各自映射到对应的字面量类型/值。
        let u = create_runtime_ast(&ident("undefined")).unwrap();
        assert_eq!(u.node_type, ExpressionNodeType::LiteralUndefined);

        let n = create_runtime_ast(&ident("NaN")).unwrap();
        assert!(matches!(n.value, NodeValue::Number(x) if x.is_nan()));

        let i = create_runtime_ast(&ident("Infinity")).unwrap();
        assert!(matches!(i.value, NodeValue::Number(x) if x == f64::INFINITY));
    }

    #[test]
    /// 未知标识符应报 "is not defined" 错误。
    fn undefined_identifier_errors() {
        // 非 czm_ 且非关键字的 foo 应报未定义。
        let err = create_runtime_ast(&ident("foo")).unwrap_err();
        assert!(err.message().contains("is not defined"));
    }

    // --- create_runtime_ast：运算符 ---

    #[test]
    /// 合法二元运算符产出 Binary 节点并带左右子节点。
    fn binary_node() {
        // 合法的 `+` 应产出带左右子节点的 Binary 节点。
        let b = JsepNode::Binary {
            operator: "+".into(),
            left: boxed(num(1.0)),
            right: boxed(num(2.0)),
        };
        let node = create_runtime_ast(&b).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::Binary);
        assert!(matches!(&node.value, NodeValue::Str(s) if s == "+"));
        assert!(node.left.is_some() && node.right.is_some());
    }

    #[test]
    /// 非法二元运算符 `**` 应报错。
    fn binary_bad_operator_errors() {
        // `**` 不在二元运算符白名单，应报错。
        let b = JsepNode::Binary {
            operator: "**".into(),
            left: boxed(num(1.0)),
            right: boxed(num(2.0)),
        };
        let err = create_runtime_ast(&b).unwrap_err();
        assert!(err.message().contains("Unexpected operator"));
    }

    #[test]
    /// 合法一元 `-` 产出 Unary，非法 `?` 报错。
    fn unary_node_and_bad_operator() {
        // 一元 `-` 合法产出 Unary；一元 `?` 非法应报错。
        let u = JsepNode::Unary {
            operator: "-".into(),
            argument: boxed(num(1.0)),
        };
        let node = create_runtime_ast(&u).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::Unary);

        let bad = JsepNode::Unary {
            operator: "?".into(),
            argument: boxed(num(1.0)),
        };
        assert!(create_runtime_ast(&bad).is_err());
    }

    #[test]
    /// 条件表达式产出 Conditional 并带 test/left/right。
    fn conditional_node() {
        // 条件节点应同时携带 test/left/right 三个子节点。
        let c = JsepNode::Conditional {
            test: boxed(num(1.0)),
            consequent: boxed(num(2.0)),
            alternate: boxed(num(3.0)),
        };
        let node = create_runtime_ast(&c).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::Conditional);
        assert!(node.test.is_some() && node.left.is_some() && node.right.is_some());
    }

    #[test]
    /// 数组节点应将元素存入 NodeValue::Nodes。
    fn array_node() {
        // 数组的两个元素应存入 NodeValue::Nodes。
        let a = JsepNode::Array(vec![num(1.0), num(2.0)]);
        let node = create_runtime_ast(&a).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::Array);
        match &node.value {
            NodeValue::Nodes(children) => assert_eq!(children.len(), 2),
            _ => panic!("expected Nodes"),
        }
    }

    // --- create_runtime_ast：成员访问 ---

    #[test]
    /// 点访问与方括号访问分别得到 dot/brackets 成员节点。
    fn member_dot_and_brackets() {
        // computed=false 得 dot，computed=true 得 brackets 成员节点。
        let dot = JsepNode::Member {
            object: boxed(ident("czm_a")),
            property: boxed(ident("b")),
            computed: false,
        };
        let node = create_runtime_ast(&dot).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::Member);
        assert!(matches!(&node.value, NodeValue::Str(s) if s == "dot"));

        let brackets = JsepNode::Member {
            object: boxed(ident("czm_a")),
            property: boxed(num(0.0)),
            computed: true,
        };
        let node = create_runtime_ast(&brackets).unwrap();
        assert!(matches!(&node.value, NodeValue::Str(s) if s == "brackets"));
    }

    #[test]
    /// Math.PI / Number.POSITIVE_INFINITY 解析为常量，Math.FOO 报错。
    fn math_and_number_constants() {
        // Math.PI / Number.POSITIVE_INFINITY 解为常量，Math.FOO 未知报错。
        let pi = JsepNode::Member {
            object: boxed(ident("Math")),
            property: boxed(ident("PI")),
            computed: false,
        };
        let node = create_runtime_ast(&pi).unwrap();
        assert!(matches!(node.value, NodeValue::Number(n) if (n - std::f64::consts::PI).abs() < 1e-15));

        let pos_inf = JsepNode::Member {
            object: boxed(ident("Number")),
            property: boxed(ident("POSITIVE_INFINITY")),
            computed: false,
        };
        let node = create_runtime_ast(&pos_inf).unwrap();
        assert!(matches!(node.value, NodeValue::Number(n) if n == f64::INFINITY));

        let bad = JsepNode::Member {
            object: boxed(ident("Math")),
            property: boxed(ident("FOO")),
            computed: false,
        };
        assert!(create_runtime_ast(&bad).is_err());
    }

    // --- create_runtime_ast：调用 ---

    #[test]
    /// vec3(...) 产出 LiteralVector 并携带三个子节点。
    fn call_vector_constructor() {
        // vec3 的三个实参应展开为 left_children 中的三个子节点。
        let c = JsepNode::Call {
            callee: boxed(ident("vec3")),
            arguments: vec![num(1.0), num(2.0), num(3.0)],
        };
        let node = create_runtime_ast(&c).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::LiteralVector);
        assert_eq!(node.left_children.as_ref().unwrap().len(), 3);
    }

    #[test]
    /// abs/pow/clamp 分别归为 Unary/Binary/Ternary 函数调用。
    fn call_unary_binary_ternary_functions() {
        // abs/pow/clamp 分别校验为 Unary/Binary/Ternary 三种节点类型。
        let abs = JsepNode::Call {
            callee: boxed(ident("abs")),
            arguments: vec![num(-1.0)],
        };
        assert_eq!(create_runtime_ast(&abs).unwrap().node_type, ExpressionNodeType::Unary);

        let pow = JsepNode::Call {
            callee: boxed(ident("pow")),
            arguments: vec![num(2.0), num(3.0)],
        };
        assert_eq!(create_runtime_ast(&pow).unwrap().node_type, ExpressionNodeType::Binary);

        let clamp = JsepNode::Call {
            callee: boxed(ident("clamp")),
            arguments: vec![num(1.0), num(2.0), num(3.0)],
        };
        assert_eq!(create_runtime_ast(&clamp).unwrap().node_type, ExpressionNodeType::Ternary);
    }

    #[test]
    /// 无参 color() 产出 LiteralColor 节点。
    fn call_color_no_args() {
        // 无参 color() 退化为白色 LiteralColor 节点。
        let c = JsepNode::Call {
            callee: boxed(ident("color")),
            arguments: vec![],
        };
        let node = create_runtime_ast(&c).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::LiteralColor);
    }

    #[test]
    /// 无参 Boolean() 产出 Bool(false) 字面量。
    fn call_boolean_no_args_is_false_literal() {
        // 无参 Boolean() 按 JS 语义得 false 常量。
        let c = JsepNode::Call {
            callee: boxed(ident("Boolean")),
            arguments: vec![],
        };
        let node = create_runtime_ast(&c).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::LiteralBoolean);
        assert!(matches!(node.value, NodeValue::Bool(false)));
    }

    #[test]
    /// regExp('ab') 现在应构造 LiteralRegex 节点（M7-B）。
    fn call_regexp_builds_literal_regex_node() {
        // regExp('ab') 现在委派给 regex.rs，得到携带模式串的 LiteralRegex。
        let c = JsepNode::Call {
            callee: boxed(ident("regExp")),
            arguments: vec![JsepNode::Literal(JsepLiteral::Str("ab".into()))],
        };
        let node = create_runtime_ast(&c).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::LiteralRegex);
        assert!(matches!(&node.value, NodeValue::Regex(re) if re.source == "ab"));
    }

    #[test]
    /// 未知函数调用应报 "Unexpected function call"。
    fn call_unknown_function_errors() {
        // 未知函数名 nope 应报 Unexpected function call。
        let c = JsepNode::Call {
            callee: boxed(ident("nope")),
            arguments: vec![],
        };
        let err = create_runtime_ast(&c).unwrap_err();
        assert!(err.message().contains("Unexpected function call"));
    }

    #[test]
    /// this 表达式既非变量亦非关键字，应报 "is not defined"。
    fn this_expression_is_not_defined() {
        // `this` 既不是 czm_ 变量也不是关键字 → "this is not defined."
        let err = create_runtime_ast(&JsepNode::ThisExpression).unwrap_err();
        assert!(err.message().contains("is not defined"));
    }

    // --- vector_component ---

    #[test]
    /// vector_component 支持按名称(x/r/b)与按索引访问，越界返回 None。
    fn vector_component_by_name_and_index() {
        // 名称（x/r/b）与数字索引两种访问方式都应命中同一分量。
        let v3 = Value::Cartesian3(DVec3::new(1.0, 2.0, 3.0));
        assert_eq!(
            vector_component(&v3, &Value::String("x".into())),
            Some(Value::Number(1.0))
        );
        assert_eq!(
            vector_component(&v3, &Value::String("r".into())),
            Some(Value::Number(1.0))
        );
        assert_eq!(
            vector_component(&v3, &Value::String("b".into())),
            Some(Value::Number(3.0))
        );
        assert_eq!(
            vector_component(&v3, &Value::Number(1.0)),
            Some(Value::Number(2.0))
        );
        // vec3 没有 .a/.w 分量。
        assert_eq!(vector_component(&v3, &Value::String("a".into())), None);

        let v4 = Value::Cartesian4(DVec4::new(1.0, 2.0, 3.0, 4.0));
        assert_eq!(
            vector_component(&v4, &Value::String("w".into())),
            Some(Value::Number(4.0))
        );

        let v2 = Value::Cartesian2(DVec2::new(7.0, 8.0));
        assert_eq!(
            vector_component(&v2, &Value::String("z".into())),
            None,
            "vec2 has no z"
        );
        assert_eq!(
            vector_component(&v2, &Value::Number(0.0)),
            Some(Value::Number(7.0))
        );
        // 非向量属性 → None。
        assert_eq!(
            vector_component(&Value::Number(1.0), &Value::String("x".into())),
            None
        );
    }

    // --- member_access ---

    #[test]
    /// 数组的 member_access：按索引与数字字符串取值，越界为 Undefined。
    fn member_access_array() {
        // 数组按数字索引与数字字符串索引取值，越界回 Undefined。
        let arr = Value::Array(vec![Value::Number(10.0), Value::Number(20.0)]);
        assert_eq!(member_access(&arr, &Value::Number(1.0)), Value::Number(20.0));
        assert_eq!(member_access(&arr, &Value::Number(5.0)), Value::Undefined);
        assert_eq!(member_access(&arr, &Value::String("0".into())), Value::Number(10.0));
    }

    #[test]
    /// 字符串的 member_access：length 与按索引取字符。
    fn member_access_string() {
        // 字符串支持 length 属性与按索引取字符（均返单字符或长度）。
        let s = Value::String("abc".into());
        assert_eq!(
            member_access(&s, &Value::String("length".into())),
            Value::Number(3.0)
        );
        assert_eq!(
            member_access(&s, &Value::Number(1.0)),
            Value::String("b".into())
        );
        assert_eq!(member_access(&s, &Value::Number(9.0)), Value::Undefined);
    }

    #[test]
    /// 非数组/字符串类型的 member_access 一律返回 Undefined。
    fn member_access_other_is_undefined() {
        // 非数组/字符串的属性一律回 Undefined。
        assert_eq!(
            member_access(&Value::Number(1.0), &Value::Number(0.0)),
            Value::Undefined
        );
    }
}
