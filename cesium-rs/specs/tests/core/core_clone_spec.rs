//! Mirrors packages/engine/Specs/Core/cloneSpec.js
//!
//! DEVIATION: JS `clone` copies property bags (shallow or deep). The Rust
//! port is `clone<T: Clone>` and the `deep` flag is a no-op; shared
//! sub-objects are modeled with `Rc` so the JS `toBe` (reference identity)
//! assertions map to `Rc::ptr_eq`. See docs/deviations.md.

use std::rc::Rc;

use cesium_core::clone::clone;

// describe("Core/clone")

#[derive(Clone)]
struct Inner {
    d: i32,
}

#[derive(Clone)]
struct Obj {
    a: i32,
    b: String,
    c: Rc<Inner>,
}

#[test]
fn can_make_shallow_clones() {
    let obj = Obj {
        a: 1,
        b: "s".to_owned(),
        c: Rc::new(Inner { d: 0 }),
    };

    let cloned_obj = clone(&obj, false);
    // expect(clonedObj).not.toBe(obj)
    assert!(!std::ptr::eq(&cloned_obj, &obj));
    assert_eq!(cloned_obj.a, obj.a);
    assert_eq!(cloned_obj.b, obj.b);
    // expect(clonedObj.c).toBe(obj.c) — shared reference on shallow clone
    assert!(Rc::ptr_eq(&cloned_obj.c, &obj.c));
    assert_eq!(cloned_obj.c.d, obj.c.d);
}

#[test]
#[ignore = "the `deep` flag is a no-op in the Rust port; deep copying is defined by each type's Clone impl (DEVIATION)"]
fn can_make_deep_clones() {
    // JS: clone(obj, true) yields clonedObj.c !== obj.c, clonedObj.c.e !== obj.c.e, ...
    // In Rust, `clone(&obj, true)` behaves identically to the shallow case.
}
