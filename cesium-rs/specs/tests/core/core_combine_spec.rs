//! Mirrors packages/engine/Specs/Core/combineSpec.js

use cesium_core::combine::combine;
use serde_json::json;

// describe("Core/combine")

#[test]
fn can_combine_shallow_references() {
    let obj1 = json!({
        "x": 1,
        "y": 2,
        "other": {
            "value1": 0,
        },
    });
    let obj2 = json!({
        "x": -1,
        "z": 3,
        "other": {
            "value2": 1,
        },
    });
    let composite = combine(Some(&obj1), Some(&obj2), None);
    assert_eq!(
        composite,
        json!({
            "x": 1,
            "y": 2,
            "z": 3,
            "other": {
                "value1": 0,
            },
        })
    );
}

#[test]
fn can_combine_deep_references() {
    let object1 = json!({
        "one": 1,
        "deep": {
            "value1": 10,
        },
    });
    let object2 = json!({
        "two": 2,
        "deep": {
            "value1": 5,
            "value2": 11,
            "sub": {
                "val": "a",
            },
        },
    });

    let composite = combine(Some(&object1), Some(&object2), Some(true));
    assert_eq!(
        composite,
        json!({
            "one": 1,
            "two": 2,
            "deep": {
                "value1": 10,
                "value2": 11,
                "sub": {
                    "val": "a",
                },
            },
        })
    );
}

#[test]
fn can_accept_undefined_as_either_object() {
    let object = json!({
        "one": 1,
        "deep": {
            "value1": 10,
        },
    });

    assert_eq!(combine(None, Some(&object), None), object);
    assert_eq!(combine(None, Some(&object), Some(true)), object);
    assert_eq!(combine(Some(&object), None, None), object);
    assert_eq!(combine(Some(&object), None, Some(true)), object);

    assert_eq!(combine(None, None, None), json!({}));
    assert_eq!(combine(None, None, Some(true)), json!({}));
}
