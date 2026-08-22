//! Ported from `packages/engine/Source/Core/Defer.js`.
//!
//! Creates a deferred object containing a promise, and functions to resolve
//! or reject the promise.

/// A deferred object wrapping a promise with resolve/reject handles.
pub struct Defer<T> {
    /// Resolves the promise.
    pub resolve: std::sync::mpsc::Sender<T>,
    /// The promise receiver.
    pub promise: std::sync::mpsc::Receiver<T>,
}

impl<T> Defer<T> {
    /// Creates a new deferred.
    pub fn new() -> Self {
        let (resolve, promise) = std::sync::mpsc::channel();
        Self { resolve, promise }
    }
}

impl<T> Default for Defer<T> {
    fn default() -> Self {
        Self::new()
    }
}
