//! Ported from `packages/engine/Source/Core/wrapFunction.js`.
//!
//! Wraps a function so that a new function is called immediately before the old one.

/// Wraps `old_fn` so that `new_fn` is called first, then `old_fn`.
/// Both receive the same arguments.
pub fn wrap_function<A, F1, F2>(new_fn: F2, old_fn: F1) -> impl Fn(&A)
where
    F1: Fn(&A),
    F2: Fn(&A),
{
    move |args: &A| {
        new_fn(args);
        old_fn(args);
    }
}
