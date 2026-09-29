//! 成员访问求值（`.x`/`.r`、`[0]`、`feature.properties.name`）。
//!
//! 移植自 `cesium-rs/crates/cesium-scene/src/expression.rs` L2187-2207 中
//! `Node::evaluate` 的 `Member` 分支，它是上游
//! `packages/engine/Source/Scene/Expression.js`（`_evaluateMemberAccess`）的
//! Rust 移植。底层原语（`vector_component`、`member_access`）已在 `ast.rs`
//! （M7-A）中；本模块提供在求值时遍历一个 `Member` 节点的编排。
//!
//! 解析顺序（忠于 blueprint）：
//! 1. `feature.<name>` → 一次 feature 属性查找（`check_feature` +
//!    `get_feature_property`），因此 `feature.properties.name` 和 `feature.x`
//!    都针对 feature 解析。
//! 2. 否则求值 object；若它是 `undefined`/`null`，产出
//!    `undefined`（JS 对一个 nullish 基数的短路）。
//! 3. 向量/颜色分量访问（`.x`/`.y`/`.z`/`.w`、`.r`/`.g`/`.b`/`.a`、
//!    `[0]`..`[3]`），经 `vector_component`。
//! 4. 通用数组/字符串成员访问，经 `member_access`。

use crate::ast::{member_access, vector_component, Node};
use crate::runtime::{check_feature, get_feature_property, ExpressionFeature};
use crate::value::{RuntimeError, Value};

/// 求值一个 `Member` 节点。`node.left` 是 object，`node.right` 是
/// 属性（对 `.name` 是字面字符串，对 `[expr]` 是任意表达式）。
pub fn evaluate_member(
    node: &Node,
    feature: Option<&dyn ExpressionFeature>,
) -> Result<Value, RuntimeError> {
    let left_node = node.left.as_ref().unwrap();
    if check_feature(left_node) {
        let name = node.right.as_ref().unwrap().evaluate(feature)?;
        return Ok(get_feature_property(feature, &name.string_conversion()));
    }
    let property = left_node.evaluate(feature)?;
    if !property.is_defined() {
        return Ok(Value::Undefined);
    }
    let member = node.right.as_ref().unwrap().evaluate(feature)?;
    if let Some(component) = vector_component(&property, &member) {
        return Ok(component);
    }
    Ok(member_access(&property, &member))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{ExpressionNodeType, NodeValue};
    use glam::DVec4;
    use std::collections::HashMap;

    /// 一个由属性映射支撑的最小 feature。
    struct TestFeature {
        props: HashMap<String, Value>,
    }
    impl ExpressionFeature for TestFeature {
        fn get_property_inherited(&self, name: &str) -> Option<Value> {
            self.props.get(name).cloned()
        }
    }

    fn literal_number_node(v: f64) -> Node {
        Node::new(
            ExpressionNodeType::LiteralNumber,
            NodeValue::Number(v),
            None,
            None,
            None,
        )
    }

    #[test]
    fn vector_component_dot_access() {
        // 在一个由 vec4 承载的字面颜色节点上取 color.r。
        let color_node = Node::new(
            ExpressionNodeType::LiteralNumber,
            NodeValue::None,
            None,
            None,
            None,
        );
        // 通过一条不依赖 feature 的简洁自定义路径，构建一个 object 求值为 Cartesian4 的
        // Member 节点：object 是字面字符串行不通，
        // 所以改用一个经由 feature 解析的 Variable 来构造。
        let _ = color_node;
        // 直接的原语检查（编排在 runtime.rs / expression.rs 测试中
        // 被端到端演练）。
        let v4 = Value::Cartesian4(DVec4::new(0.1, 0.2, 0.3, 0.4));
        assert_eq!(
            vector_component(&v4, &Value::String("r".into())),
            Some(Value::Number(0.1))
        );
        assert_eq!(member_access(&v4, &Value::Number(0.0)), Value::Undefined);
    }

    #[test]
    fn feature_property_lookup() {
        let mut props = HashMap::new();
        props.insert("height".to_string(), Value::Number(42.0));
        let feature = TestFeature { props };

        // 一个 Member 节点 `feature.height`：left 是 `feature` 关键字节点，
        // right 是字面字符串 "height"。
        let feature_node = Node::new(
            ExpressionNodeType::Variable,
            NodeValue::Str("feature".to_string()),
            None,
            None,
            None,
        );
        let prop_node = Node::new(
            ExpressionNodeType::LiteralString,
            NodeValue::Str("height".to_string()),
            None,
            None,
            None,
        );
        let member = Node::new(
            ExpressionNodeType::Member,
            NodeValue::Str("dot".to_string()),
            Some(feature_node),
            Some(prop_node),
            None,
        );
        assert_eq!(
            evaluate_member(&member, Some(&feature)).unwrap(),
            Value::Number(42.0)
        );
        // 缺失的属性 -> undefined。
        assert_eq!(
            get_feature_property(Some(&feature), "missing"),
            Value::Undefined
        );
        let _ = literal_number_node(0.0);
    }

    #[test]
    fn nullish_base_short_circuits_to_undefined() {
        // Object 求值为 null（一个 LiteralNull 节点）-> undefined 成员。
        let null_node = Node::new(
            ExpressionNodeType::LiteralNull,
            NodeValue::Null,
            None,
            None,
            None,
        );
        let prop_node = Node::new(
            ExpressionNodeType::LiteralString,
            NodeValue::Str("x".to_string()),
            None,
            None,
            None,
        );
        let member = Node::new(
            ExpressionNodeType::Member,
            NodeValue::Str("dot".to_string()),
            Some(null_node),
            Some(prop_node),
            None,
        );
        assert_eq!(evaluate_member(&member, None).unwrap(), Value::Undefined);
    }
}
