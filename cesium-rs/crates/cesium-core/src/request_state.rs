//! Ported from `packages/engine/Source/Core/RequestState.js`.

/// State of the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum RequestState {
    /// Initial unissued state.
    Unissued = 0,
    /// Issued but not yet active.
    Issued = 1,
    /// Actual http request has been sent.
    Active = 2,
    /// Request completed successfully.
    Received = 3,
    /// Request was cancelled.
    Cancelled = 4,
    /// Request failed.
    Failed = 5,
}
