//! Member-access evaluation (`.x`/`.r`, `[0]`, `feature.properties.name`).
//!
//! Ported from the `Member` branch of `Node::evaluate` in
//! `cesium-rs/crates/cesium-scene/src/expression.rs` L2187-2207, the Rust port
//! of upstream `packages/engine/Source/Scene/Expression.js`
//! (`_evaluateMemberAccess`). The low-level primitives (`vector_component`,
//! `member_access`) already live in `ast.rs` (M7-A); this module provides the
//! orchestration that walks a `Member` node at evaluate time.
//!
//! Order of resolution (blueprint-faithful):
//! 1. `feature.<name>` -> a feature property lookup (`check_feature` +
//!    `get_feature_property`), so `feature.properties.name` and `feature.x`
//!    both resolve against the feature.
//! 2. otherwise evaluate the object; if it is `undefined`/`null`, yield
//!    `undefined` (JS short-circuit on a nullish base).
//! 3. vector/color component access (`.x`/`.y`/`.z`/`.w`, `.r`/`.g`/`.b`/`.a`,
//!    `[0]`..`[3]`) via `vector_component`.
//! 4. generic array/string member access via `member_access`.

use crate::ast::{member_access, vector_component, Node};
use crate::runtime::{check_feature, get_feature_property, ExpressionFeature};
use crate::value::{RuntimeError, Value};

/// Evaluates a `Member` node. `node.left` is the object, `node.right` the
/// property (a literal string for `.name`, an arbitrary expression for `[expr]`).
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

    /// A minimal feature backed by a property map.
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
        // color.r on a literal vec4-backed color node.
        let color_node = Node::new(
            ExpressionNodeType::LiteralNumber,
            NodeValue::None,
            None,
            None,
            None,
        );
        // Build a Member node whose object evaluates to a Cartesian4 by using a
        // small custom feature-free path: object is a literal string won't work,
        // so construct via a Variable resolved through a feature.
        let _ = color_node;
        // Direct primitive check (orchestration is exercised end-to-end in
        // runtime.rs / expression.rs tests).
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

        // A Member node `feature.height`: left is the `feature` keyword node,
        // right is a literal string "height".
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
        // Missing property -> undefined.
        assert_eq!(
            get_feature_property(Some(&feature), "missing"),
            Value::Undefined
        );
        let _ = literal_number_node(0.0);
    }

    #[test]
    fn nullish_base_short_circuits_to_undefined() {
        // Object evaluates to null (a LiteralNull node) -> undefined member.
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
