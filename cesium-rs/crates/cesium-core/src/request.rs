//! Ported from `packages/engine/Source/Core/Request.js`.

use crate::request_state::RequestState;
use crate::request_type::RequestType;

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
        }
    }

    /// Marks the request as cancelled.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    /// Duplicates a Request instance.
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
        }
    }
}
