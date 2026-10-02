//! cesium-styling：3D Tiles 样式与分类。
//!
//! 状态（P2 代码健康审计，2026-09-27）：已实现并被 `cesium-specs` 测试套覆盖，
//! 但**未接入任何生产运行时路径** —— 没有任何 adapter 或 application crate
//! 依赖它（`CESIUM_ENABLE_STYLING_JSEP` 双构建仅从 specs 中被演练）。作为
//! CesiumJS 功能对齐的 domain 模型保留，留待未来的 adapter 桥接；不要把它
//! 当作一项已交付的能力来解读。见 docs/ARCHITECTURE.md "Test-only domain crates"。
//!
//! Domain 层 —— 纯 Rust，f64 精度。
//!
//! 模块职责：
//! - tile_style：3D Tiles 样式与遗留表达式 AST
//! - ast / value / tokenizer / js_math：M7-A jsep 表达式引擎基础层
//! - parser / coerce / literal / member_access / runtime：M7-B 解析与求值核心
//! - classification：分类图元与分类类型

pub mod ast;
pub mod classification;
pub mod coerce;
pub mod expression;
pub mod js_math;
pub mod literal;
pub mod member_access;
pub mod parser;
pub mod regex;
pub mod runtime;
pub mod tile_style;
pub mod tokenizer;
pub mod value;
pub mod variables;

pub use classification::{
    Classification, ClassificationCollection, ClassificationType, FeatureMetadata, MetadataValue,
};
pub use tile_style::{
    ArithmeticOp, CompareOp, PropertyValue, StyleExpression, TileStyle,
};

// M7-A：jsep 1.3.8 表达式引擎基础层。
// tokenizer + value + js_math + ast。M7-B 添加 Pratt 解析器（parser）、
// 预处理（variables）、求值核心（coerce/literal/member_access/
// runtime）、正则支持（regex）与顶层 Expression（expression）。
// `tile_style`（legacy）未改动。
pub use ast::{
    create_runtime_ast, member_access, vector_component, ExpressionNodeType, JsepLiteral, JsepNode,
    Node, NodeValue, BINARY_OPERATORS, UNARY_OPERATORS,
};
pub use js_math::{js_max, js_min, js_round};
pub use tokenizer::{is_digit, is_identifier_char, is_identifier_start, Token, Tokenizer};
pub use value::{js_parse_number, number_to_js_string, runtime_error, RuntimeError, Value};

// M7-B：解析 + 求值核心。
pub use expression::Expression;
pub use parser::{binary_precedence, Parser, UNARY_PRECEDENCE};
pub use regex::RegExpValue;
pub use runtime::ExpressionFeature;
