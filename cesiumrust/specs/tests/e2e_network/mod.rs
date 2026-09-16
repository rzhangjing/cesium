//! M8 end-to-end network test skeleton.
//!
//! **Status: skeleton-only.** Every test in this module is `#[ignore]`d per
//! plan L206 ("先建骨架标 #[ignore]"). They exist to lock in the intended
//! end-to-end shape of the network path — `HttpTileFetcher` driven against a
//! `wiremock::MockServer`, exercising the domain's `retry_callback` and
//! `priority_function` semantics — before the adapter wiring lands.
//!
//! # Why ignored (see docs/deferred.md: "M8-wiremock skeleton")
//!
//! 1. `wiremock::MockServer` requires an async runtime (`#[tokio::test]`).
//!    The cesium-specs dev-profile does not yet enable tokio's `macros`/`rt`
//!    features for this suite, and the production path is deliberately
//!    tokio-free (ureq blocking pool). Wiring an async test harness is an
//!    M11.1 concern.
//! 2. `adapters/network`'s `HttpTileFetcher::fetch(url, _priority)` currently
//!    *ignores* `_priority` (see `adapters/network/src/lib.rs:186`). The
//!    priority e2e assertion is only meaningful once M8.3 (#66) makes the
//!    fetcher consume it and M11.1 wires the scheduler.
//!
//! # What still runs today
//!
//! The **domain-side** assertions inside each test (RetryPolicy decisions,
//! PriorityFunction ordering, FetchDescriptor construction) are pure and would
//! pass even without a server; they are the parts already implemented in
//! `cesium-resource`. The wiremock `Mock`/`ResponseTemplate` construction is
//! synchronous and compiles now, proving the dev-dependency resolves; only the
//! async `.mount(&server)` + real fetch are deferred.

pub mod test_http_tile_fetcher;
pub mod test_priority;
pub mod test_retry;
