//! cesium-styling: 3D Tiles styling and classification.
//!
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping:
//! - `Scene/Cesium3DTileStyle.js` → tile_style
//! - `Scene/Expression.js` → tile_style (legacy AST-only) + ast/value/tokenizer/js_math (M7-A jsep engine base layer)
//! - `Scene/ClassificationPrimitive.js` → classification
//! - `Scene/ClassificationType.js` → classification

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

// M7-A: jsep 1.3.8 expression-engine base layer (Scene/Expression.js port).
// tokenizer + value + js_math + ast. M7-B adds the Pratt parser (parser),
// preprocessing (variables), evaluation core (coerce/literal/member_access/
// runtime), regex support (regex) and the top-level Expression (expression).
// `tile_style` (legacy) is untouched.
pub use ast::{
    create_runtime_ast, member_access, vector_component, ExpressionNodeType, JsepLiteral, JsepNode,
    Node, NodeValue, BINARY_OPERATORS, UNARY_OPERATORS,
};
pub use js_math::{js_max, js_min, js_round};
pub use tokenizer::{is_digit, is_identifier_char, is_identifier_start, Token, Tokenizer};
pub use value::{js_parse_number, number_to_js_string, runtime_error, RuntimeError, Value};

// M7-B: parsing + evaluation core.
pub use expression::Expression;
pub use parser::{binary_precedence, Parser, UNARY_PRECEDENCE};
pub use regex::RegExpValue;
pub use runtime::ExpressionFeature;
