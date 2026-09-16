//! Top-level test harness for the M8 e2e network skeleton.
//!
//! Cargo auto-discovers each `tests/*.rs` file as a separate integration-test
//! crate. This file is the crate root that pulls in the `e2e_network` module
//! directory (mirroring the existing `integration_tests.rs` → `mod integration;`
//! convention used elsewhere in this suite).
//!
//! All tests inside are `#[ignore]`d — see `docs/deferred.md` (M8-wiremock
//! skeleton) for the rationale and the unblock conditions. They are
//! skeleton-only until M11.1 wires the real `HttpTileFetcher` against a
//! `wiremock::MockServer`.

mod e2e_network;
