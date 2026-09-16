//! e2e skeleton: `retry_callback` semantics driven by mock server failures.
//!
//! The pure `RetryPolicy` / `RetryContext` / `default_retry_callback` decisions
//! are already implemented in `cesium-resource` and asserted here. The
//! end-to-end loop (mock returns 5xx/429 → `HttpTileFetcher` retries with
//! backoff) is deferred to M11.1.

use cesium_resource::{
    classify_status, default_retry_callback, BackoffStrategy, RequestErrorClass, RetryContext,
    RetryDecision, RetryPolicy,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

/// Skeleton: a 503 then 200 sequence drives one retry and succeeds.
#[test]
#[ignore = "M8-wiremock skeleton: needs MockServer async runtime + HttpTileFetcher retry wiring (M11.1). See docs/deferred.md."]
fn transient_failure_retries_then_succeeds() {
    // First call 503, second 200 — wiremock `respond_with` sequences land in M11.1.
    let _mock = Mock::given(method("GET"))
        .and(path("/flaky/tile.b3dm"))
        .respond_with(ResponseTemplate::new(503));
    // TODO(M11.1): mount + drive HttpTileFetcher with a RetryPolicy::exponential.

    // ── domain-side retry decision (pure) ──
    let policy = RetryPolicy::exponential(3, 100);
    let ctx = RetryContext::from_status("https://x.com/flaky/tile.b3dm", 0, 503, policy.clone());
    assert_eq!(ctx.error_class, RequestErrorClass::Transient);
    // First failure with a 100ms exponential base → retry after 100ms.
    assert_eq!(ctx.default_decision(), RetryDecision::RetryAfterMillis(100));
}

/// Skeleton: a 404 never retries (permanent client error).
#[test]
#[ignore = "M8-wiremock skeleton: needs MockServer async runtime + HttpTileFetcher retry wiring (M11.1). See docs/deferred.md."]
fn permanent_failure_does_not_retry() {
    let _mock = Mock::given(method("GET"))
        .and(path("/missing/tile.b3dm"))
        .respond_with(ResponseTemplate::new(404));

    assert_eq!(classify_status(404), RequestErrorClass::Permanent);
    let policy = RetryPolicy::exponential(5, 100);
    assert_eq!(
        policy.decide(0, RequestErrorClass::Permanent),
        RetryDecision::GiveUp
    );
}

/// Skeleton: a 429 is classified throttled and honours `retry_on_throttled`.
#[test]
#[ignore = "M8-wiremock skeleton: needs MockServer async runtime + HttpTileFetcher retry wiring (M11.1). See docs/deferred.md."]
fn throttled_429_honours_retry_flag() {
    let _mock = Mock::given(method("GET"))
        .and(path("/throttled/tile.b3dm"))
        .respond_with(ResponseTemplate::new(429));

    assert_eq!(classify_status(429), RequestErrorClass::Throttled);

    let mut policy = RetryPolicy::default();
    policy.backoff = BackoffStrategy::Fixed;
    assert_eq!(
        policy.decide(0, RequestErrorClass::Throttled),
        RetryDecision::RetryNow
    );

    policy.retry_on_throttled = false;
    assert_eq!(
        policy.decide(0, RequestErrorClass::Throttled),
        RetryDecision::GiveUp
    );
}

/// Skeleton: the pluggable `retry_callback` closure is what the adapter invokes.
#[test]
#[ignore = "M8-wiremock skeleton: needs MockServer async runtime + HttpTileFetcher retry wiring (M11.1). See docs/deferred.md."]
fn default_retry_callback_applies_policy() {
    let callback = default_retry_callback();
    let ctx = RetryContext::from_status("https://x.com/a", 0, 500, RetryPolicy::default());
    // Default policy: max_attempts=1, no delay → retry immediately on first 5xx.
    assert_eq!(callback(&ctx), RetryDecision::RetryNow);

    // After exhausting attempts, give up.
    let ctx2 = RetryContext::from_status("https://x.com/a", 1, 500, RetryPolicy::default());
    assert_eq!(callback(&ctx2), RetryDecision::GiveUp);
}
