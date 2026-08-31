//! Ported from `packages/engine/Source/Core/Request.js` (96 lines).
//!
//! Stores information for making a request. Requests are used by
//! [`crate::request_scheduler::RequestScheduler`] to control the order and
//! number of active requests.
//!
//! # Method-level alignment table (JS `Request` -> Rust)
//!
//! | CesiumJS (Request.js)        | Rust                                   |
//! | ---------------------------- | -------------------------------------- |
//! | `constructor(options)`       | [`Request::new`] / [`Request::default`]|
//! | `cancel()`                   | [`Request::cancel`]                    |
//! | `priorityFunction`           | [`Request::set_priority_function`]     |
//! | `deferred` / `requestFunction` / `cancelFunction` | DEVIATION: promise flow not ported |
//!
//! DEVIATION: JS takes a single `options` object; the Rust port uses
//! positional `Option` parameters with the same defaults (priority 0.0,
//! throttle false, throttleByServer false, requestType OTHER,
//! state UNISSUED).
//! DEVIATION: each Rust `Request` carries a unique [`Request::id`] standing
//! in for JS object identity, which the scheduler uses to track requests
//! that were copied into its heap.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::request_state::RequestState;
use crate::request_type::RequestType;

/// The function that is called to update the request's priority, which
/// occurs once per frame.
///
/// Port of `Request.PriorityCallback`. Shared via `Arc` so that clones of
/// the request keep referencing the same function (JS reference semantics).
pub type PriorityFunction = Arc<Mutex<dyn FnMut() -> f64 + Send>>;

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

/// Stores information for making a request.
pub struct Request {
    /// The URL to request.
    pub url: Option<String>,
    /// The priority of the request (lower = higher priority).
    pub priority: f64,
    /// Whether to throttle and prioritize the request.
    pub throttle: bool,
    /// Whether to throttle the request by server.
    pub throttle_by_server: bool,
    /// Type of request.
    pub request_type: RequestType,
    /// A key identifying the target server.
    pub server_key: Option<String>,
    /// The current state of the request.
    pub state: RequestState,
    /// Whether the request was explicitly cancelled.
    pub cancelled: bool,
    /// Unique identifier standing in for JS object identity (used by the
    /// request scheduler to track copies of this request).
    id: u64,
    /// The function that is called to update the request's priority.
    priority_function: Option<PriorityFunction>,
}

impl Request {
    /// Creates a new Request.
    pub fn new(
        url: Option<String>,
        priority: Option<f64>,
        throttle: Option<bool>,
        throttle_by_server: Option<bool>,
        request_type: Option<RequestType>,
        server_key: Option<String>,
    ) -> Self {
        Self {
            url,
            priority: priority.unwrap_or(0.0),
            throttle: throttle.unwrap_or(false),
            throttle_by_server: throttle_by_server.unwrap_or(false),
            request_type: request_type.unwrap_or(RequestType::Other),
            server_key,
            state: RequestState::Unissued,
            cancelled: false,
            id: NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed),
            priority_function: None,
        }
    }

    /// The unique scheduler-tracking identifier of this request.
    #[must_use]
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Sets the function called to update the request's priority.
    ///
    /// Mirrors assigning `options.priorityFunction`.
    pub fn set_priority_function(&mut self, priority_function: PriorityFunction) {
        self.priority_function = Some(priority_function);
    }

    /// The priority function, if one was set.
    #[must_use]
    pub fn priority_function(&self) -> Option<&PriorityFunction> {
        self.priority_function.as_ref()
    }

    /// Marks the request as cancelled.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    /// Duplicates a Request instance.
    ///
    /// DEVIATION: the clone receives a fresh [`Request::id`] (JS `clone`
    /// produces a new object with its own identity).
    pub fn clone_request(&self) -> Self {
        Self {
            url: self.url.clone(),
            priority: self.priority,
            throttle: self.throttle,
            throttle_by_server: self.throttle_by_server,
            request_type: self.request_type,
            server_key: self.server_key.clone(),
            state: RequestState::Unissued,
            cancelled: false,
            id: NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed),
            priority_function: self.priority_function.clone(),
        }
    }
}

impl Default for Request {
    /// Mirrors `new Request()` in JS: every option left at its default.
    fn default() -> Self {
        Self::new(None, None, None, None, None, None)
    }
}
