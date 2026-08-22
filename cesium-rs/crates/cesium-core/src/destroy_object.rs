//! Ported from packages/engine/Source/Core/destroyObject.js
//!
//! DEVIATION: JS `destroyObject` swaps every method of the object with a
//! function throwing `DeveloperError` and marks `isDestroyed() === true`.
//! Rust expresses this lifecycle via a trait implemented by each resource
//! holder; calling methods on a destroyed object is either a compile-time
//! impossibility (ownership consumed) or a `DeveloperError` panic guarded by
//! the implementor. See docs/deviations.md.

use crate::developer_error::throw_developer_error;

/// Objects that hold native resources (e.g. GPU resources) which need to be
/// explicitly released implement this trait. Client code calls
/// [`Destroyable::destroy`], which releases the native resource and puts the
/// object in a destroyed state.
pub trait Destroyable {
    /// Returns `true` if this object was destroyed, i.e.,
    /// [`Destroyable::destroy`] was called.
    ///
    /// Port of `isDestroyed()`.
    fn is_destroyed(&self) -> bool;

    /// Releases the native resources held by this object.
    ///
    /// Port of `destroy()`.
    fn destroy(self);
}

/// The default message thrown when a destroyed object is used.
pub const DESTROYED_MESSAGE: &str = "This object was destroyed, i.e., destroy() was called.";

/// Helper for `Destroyable` implementors: panics with a `DeveloperError`
/// when an operation is attempted on a destroyed object.
///
/// Port of the `throwOnDestroyed` closure installed by CesiumJS
/// `destroyObject(object, message)`.
///
/// # Panics
/// Always panics with a `DeveloperError` containing `message` (or
/// [`DESTROYED_MESSAGE`] when `message` is `None`).
pub fn throw_on_destroyed(message: Option<&str>) -> ! {
    // >>includeStart('debug', pragmas.debug) — the JS throw itself lives in
    // the debug pragma block; release builds keep the panic because using a
    // destroyed object is undefined behavior for the resource.
    throw_developer_error(message.unwrap_or(DESTROYED_MESSAGE));
}
