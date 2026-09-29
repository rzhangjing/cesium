//! 3D Tiles Styling 表达式引擎的运行时 AST。
//!
//! 移植自 `cesium-rs/crates/cesium-scene/src/expression.rs`：
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
// 节点类型（镜像 ExpressionNodeType.js）
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
    Null,
    Undefined,
    Bool(bool),
    Number(f64),
    Str(String),
    /// 子表达式（ARRAY 节点类型）。
    Nodes(Vec<Node>),
    /// 一个预编译的正则表达式（LITERAL_REGEX）。M7-A：从不
    /// 构造（正则编译推迟到 M7-B）；见模块文档。
    Regex(RegExpValue),
}

/// 一个运行时 AST 节点，镜像 Expression.js 的 `Node` 构造器。
/// `_left` 既可以是单个节点，也可以是参数节点数组
/// （LITERAL_COLOR / LITERAL_VECTOR）；`left_children` 持有后者。
#[derive(Debug, Clone)]
pub struct Node {
    pub node_type: ExpressionNodeType,
    pub value: NodeValue,
    pub left: Option<Box<Node>>,
    pub left_children: Option<Vec<Node>>,
    pub right: Option<Box<Node>>,
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
    Literal(JsepLiteral),
    Identifier(String),
    ThisExpression,
    Unary {
        operator: String,
        argument: Box<JsepNode>,
    },
    Binary {
        operator: String,
        left: Box<JsepNode>,
        right: Box<JsepNode>,
    },
    Conditional {
        test: Box<JsepNode>,
        consequent: Box<JsepNode>,
        alternate: Box<JsepNode>,
    },
    Member {
        object: Box<JsepNode>,
        property: Box<JsepNode>,
        computed: bool,
    },
    Call {
        callee: Box<JsepNode>,
        arguments: Vec<JsepNode>,
    },
    Array(Vec<JsepNode>),
}

/// 一个 jsep 字面值。
#[derive(Debug, Clone)]
pub enum JsepLiteral {
    Null,
    Boolean(bool),
    Number(f64),
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
    expression.replace(BACKSLASH_REPLACEMENT, "\\")
}

/// 镜像 `parseLiteral`。
fn parse_literal(literal: &JsepLiteral) -> Node {
    match literal {
        JsepLiteral::Null => Node::new(
            ExpressionNodeType::LiteralNull,
            NodeValue::Null,
            None,
            None,
            None,
        ),
        JsepLiteral::Boolean(value) => Node::new(
            ExpressionNodeType::LiteralBoolean,
            NodeValue::Bool(*value),
            None,
            None,
            None,
        ),
        JsepLiteral::Number(value) => Node::new(
            ExpressionNodeType::LiteralNumber,
            NodeValue::Number(*value),
            None,
            None,
            None,
        ),
        JsepLiteral::Str(value) => {
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

fn is_variable(name: &str) -> bool {
    name.starts_with("czm_")
}

fn get_property_name(variable: &str) -> &str {
    &variable[4..]
}

/// 镜像 `parseKeywordsAndVariables`。
fn parse_keywords_and_variables(name: &str) -> Result<Node, RuntimeError> {
    if is_variable(name) {
        let property = get_property_name(name);
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
        return Ok(Node::new(
            ExpressionNodeType::LiteralNumber,
            NodeValue::Number(f64::NAN),
            None,
            None,
            None,
        ));
    } else if name == "Infinity" {
        return Ok(Node::new(
            ExpressionNodeType::LiteralNumber,
            NodeValue::Number(f64::INFINITY),
            None,
            None,
            None,
        ));
    } else if name == "undefined" {
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
/// 原函数返回 undefined）。
fn parse_math_constant(name: &str) -> Option<Node> {
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

fn is_unary_function(call: &str) -> bool {
    matches!(
        call,
        "abs" | "sqrt" | "cos" | "sin" | "tan" | "acos" | "asin" | "atan" | "radians"
            | "degrees" | "sign" | "floor" | "ceil" | "round" | "exp" | "exp2" | "log"
            | "log2" | "fract" | "length" | "normalize"
    )
}

pub(crate) fn is_binary_function(call: &str) -> bool {
    matches!(call, "atan2" | "pow" | "min" | "max" | "distance" | "dot" | "cross")
}

fn is_ternary_function(call: &str) -> bool {
    matches!(call, "clamp" | "mix")
}

/// 镜像 `parseMemberExpression`。
fn parse_member_expression(
    object: &JsepNode,
    property: &JsepNode,
    computed: bool,
) -> Result<Node, RuntimeError> {
    let object_name = match object {
        JsepNode::Identifier(name) => Some(name.as_str()),
        _ => None,
    };
    let property_name = match property {
        JsepNode::Identifier(name) => Some(name.as_str()),
        _ => None,
    };
    if object_name == Some("Math") {
        if let Some(name) = property_name {
            if let Some(node) = parse_math_constant(name) {
                return Ok(node);
            }
        }
        return Err(runtime_error("Cannot parse expression."));
    } else if object_name == Some("Number") {
        if let Some(name) = property_name {
            if let Some(node) = parse_number_constant(name) {
                return Ok(node);
            }
        }
        return Err(runtime_error("Cannot parse expression."));
    }

    let obj = create_runtime_ast(object)?;
    if computed {
        let val = create_runtime_ast(property)?;
        return Ok(Node::new(
            ExpressionNodeType::Member,
            NodeValue::Str("brackets".to_string()),
            Some(obj),
            Some(val),
            None,
        ));
    }

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
    let args_length = arguments.len();

    // 成员函数调用
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
            let is_reg_exp = matches!(
                object.as_ref(),
                JsepNode::Call { callee, .. }
                    if matches!(callee.as_ref(), JsepNode::Identifier(name) if name == "regExp")
            );
            if !is_reg_exp {
                return Err(runtime_error(&format!("{call} is not a function.")));
            }
            if args_length == 0 {
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
            let right = create_runtime_ast(&arguments[0])?;
            return Ok(Node::new(
                ExpressionNodeType::FunctionCall,
                NodeValue::Str(call.to_string()),
                Some(left),
                Some(right),
                None,
            ));
        } else if call == "toString" {
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
    let call = match callee {
        JsepNode::Identifier(name) => name.as_str(),
        _ => return Err(runtime_error("Unexpected function call.")),
    };
    if call == "color" {
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

/// 镜像 `createRuntimeAst`。
pub fn create_runtime_ast(ast: &JsepNode) -> Result<Node, RuntimeError> {
    match ast {
        JsepNode::Literal(literal) => Ok(parse_literal(literal)),
        JsepNode::Call { callee, arguments } => parse_call(callee, arguments),
        JsepNode::Identifier(name) => parse_keywords_and_variables(name),
        JsepNode::ThisExpression => parse_keywords_and_variables("this"),
        JsepNode::Unary { operator, argument } => {
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

    fn num(v: f64) -> JsepNode {
        JsepNode::Literal(JsepLiteral::Number(v))
    }
    fn ident(name: &str) -> JsepNode {
        JsepNode::Identifier(name.to_string())
    }
    fn boxed(n: JsepNode) -> Box<JsepNode> {
        Box::new(n)
    }

    // --- ExpressionNodeType：19 个变体 + 判别值 + is_literal_type ---

    #[test]
    fn node_type_discriminants_and_count() {
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
    fn is_literal_type_ordering() {
        // >= LiteralNull(10) 是一个字面类型。
        assert!(!ExpressionNodeType::Variable.is_literal_type());
        assert!(!ExpressionNodeType::VariableInString.is_literal_type());
        assert!(ExpressionNodeType::LiteralNull.is_literal_type());
        assert!(ExpressionNodeType::LiteralNumber.is_literal_type());
        assert!(ExpressionNodeType::BuiltinVariable.is_literal_type());
    }

    // --- create_runtime_ast：字面量 ---

    #[test]
    fn literal_number_node() {
        let node = create_runtime_ast(&num(5.0)).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::LiteralNumber);
        assert!(matches!(node.value, NodeValue::Number(n) if n == 5.0));
    }

    #[test]
    fn literal_string_node() {
        let node =
            create_runtime_ast(&JsepNode::Literal(JsepLiteral::Str("hello".into()))).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::LiteralString);
        assert!(matches!(&node.value, NodeValue::Str(s) if s == "hello"));
    }

    #[test]
    fn string_with_placeholder_is_variable_in_string() {
        let node =
            create_runtime_ast(&JsepNode::Literal(JsepLiteral::Str("${x}".into()))).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::VariableInString);
    }

    #[test]
    fn literal_null_and_boolean_nodes() {
        let n = create_runtime_ast(&JsepNode::Literal(JsepLiteral::Null)).unwrap();
        assert_eq!(n.node_type, ExpressionNodeType::LiteralNull);
        assert!(matches!(n.value, NodeValue::Null));

        let b = create_runtime_ast(&JsepNode::Literal(JsepLiteral::Boolean(true))).unwrap();
        assert_eq!(b.node_type, ExpressionNodeType::LiteralBoolean);
        assert!(matches!(b.value, NodeValue::Bool(true)));
    }

    // --- create_runtime_ast：关键字与变量 ---

    #[test]
    fn variable_and_builtin_nodes() {
        let v = create_runtime_ast(&ident("czm_height")).unwrap();
        assert_eq!(v.node_type, ExpressionNodeType::Variable);
        assert!(matches!(&v.value, NodeValue::Str(s) if s == "height"));

        let b = create_runtime_ast(&ident("czm_tiles3d_tileset_time")).unwrap();
        assert_eq!(b.node_type, ExpressionNodeType::BuiltinVariable);
        assert!(matches!(&b.value, NodeValue::Str(s) if s == "tiles3d_tileset_time"));
    }

    #[test]
    fn keyword_literals() {
        let u = create_runtime_ast(&ident("undefined")).unwrap();
        assert_eq!(u.node_type, ExpressionNodeType::LiteralUndefined);

        let n = create_runtime_ast(&ident("NaN")).unwrap();
        assert!(matches!(n.value, NodeValue::Number(x) if x.is_nan()));

        let i = create_runtime_ast(&ident("Infinity")).unwrap();
        assert!(matches!(i.value, NodeValue::Number(x) if x == f64::INFINITY));
    }

    #[test]
    fn undefined_identifier_errors() {
        let err = create_runtime_ast(&ident("foo")).unwrap_err();
        assert!(err.message().contains("is not defined"));
    }

    // --- create_runtime_ast：运算符 ---

    #[test]
    fn binary_node() {
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
    fn binary_bad_operator_errors() {
        let b = JsepNode::Binary {
            operator: "**".into(),
            left: boxed(num(1.0)),
            right: boxed(num(2.0)),
        };
        let err = create_runtime_ast(&b).unwrap_err();
        assert!(err.message().contains("Unexpected operator"));
    }

    #[test]
    fn unary_node_and_bad_operator() {
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
    fn conditional_node() {
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
    fn array_node() {
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
    fn member_dot_and_brackets() {
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
    fn math_and_number_constants() {
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
    fn call_vector_constructor() {
        let c = JsepNode::Call {
            callee: boxed(ident("vec3")),
            arguments: vec![num(1.0), num(2.0), num(3.0)],
        };
        let node = create_runtime_ast(&c).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::LiteralVector);
        assert_eq!(node.left_children.as_ref().unwrap().len(), 3);
    }

    #[test]
    fn call_unary_binary_ternary_functions() {
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
    fn call_color_no_args() {
        let c = JsepNode::Call {
            callee: boxed(ident("color")),
            arguments: vec![],
        };
        let node = create_runtime_ast(&c).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::LiteralColor);
    }

    #[test]
    fn call_boolean_no_args_is_false_literal() {
        let c = JsepNode::Call {
            callee: boxed(ident("Boolean")),
            arguments: vec![],
        };
        let node = create_runtime_ast(&c).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::LiteralBoolean);
        assert!(matches!(node.value, NodeValue::Bool(false)));
    }

    #[test]
    fn call_regexp_builds_literal_regex_node() {
        let c = JsepNode::Call {
            callee: boxed(ident("regExp")),
            arguments: vec![JsepNode::Literal(JsepLiteral::Str("ab".into()))],
        };
        let node = create_runtime_ast(&c).unwrap();
        assert_eq!(node.node_type, ExpressionNodeType::LiteralRegex);
        assert!(matches!(&node.value, NodeValue::Regex(re) if re.source == "ab"));
    }

    #[test]
    fn call_unknown_function_errors() {
        let c = JsepNode::Call {
            callee: boxed(ident("nope")),
            arguments: vec![],
        };
        let err = create_runtime_ast(&c).unwrap_err();
        assert!(err.message().contains("Unexpected function call"));
    }

    #[test]
    fn this_expression_is_not_defined() {
        // `this` 既不是 czm_ 变量也不是关键字 → "this is not defined."
        let err = create_runtime_ast(&JsepNode::ThisExpression).unwrap_err();
        assert!(err.message().contains("is not defined"));
    }

    // --- vector_component ---

    #[test]
    fn vector_component_by_name_and_index() {
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
    fn member_access_array() {
        let arr = Value::Array(vec![Value::Number(10.0), Value::Number(20.0)]);
        assert_eq!(member_access(&arr, &Value::Number(1.0)), Value::Number(20.0));
        assert_eq!(member_access(&arr, &Value::Number(5.0)), Value::Undefined);
        assert_eq!(member_access(&arr, &Value::String("0".into())), Value::Number(10.0));
    }

    #[test]
    fn member_access_string() {
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
    fn member_access_other_is_undefined() {
        assert_eq!(
            member_access(&Value::Number(1.0), &Value::Number(0.0)),
            Value::Undefined
        );
    }
}
