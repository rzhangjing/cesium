//! e2e skeleton: `HttpTileFetcher` against a `wiremock::MockServer`.
//!
//! Verifies the domain-side `FetchDescriptor` construction today; the actual
//! HTTP round-trip through `HttpTileFetcher` is deferred to M11.1 (see
//! docs/deferred.md "M8-wiremock skeleton").
//!
//! **M8.3 (#66) wiring proof**: [`m83_backend_types_are_wired`] below is a
//! non-ignored compile-time assertion that the real `HttpTileFetcher` +
//! `NetworkResourceBackend` + `resource_fetch_backend_enabled` types are
//! reachable from the specs crate. This makes `cargo test --no-run` a hard
//! gate on the M8.3 adapter surface existing, without needing the async
//! harness that M11.1 will provide.

use cesium_network::{
    resource_fetch_backend_enabled, HttpTileFetcher, NetworkResourceBackend,
    ENV_ENABLE_RESOURCE_FETCH_BACKEND,
};
use cesium_ports_driven::{ResourceBackend, TileFetcher};
use cesium_resource::{FetchDescriptor, HttpMethod, RequestType, Resource, ResponseType};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

/// M8.3 (#66) compile-time wiring proof — **not ignored**.
///
/// Asserts the real network-adapter surface is reachable from `cesium-specs`:
/// * [`HttpTileFetcher`] can be constructed and holds a
///   [`NetworkResourceBackend`] (the M8.3 delegate target).
/// * [`NetworkResourceBackend`] implements
///   [`cesium_ports_driven::ResourceBackend<u64>`] (dyn-compatible).
/// * [`resource_fetch_backend_enabled`] defaults OFF when the env var is unset
///   (the golden-path invariant that keeps v0 byte-identical).
/// * [`ENV_ENABLE_RESOURCE_FETCH_BACKEND`] is the exact gate name (guards
///   against accidental rename / conflation with the M12
///   `ENV_ENABLE_RESOURCE_BACKEND`).
///
/// No network calls, no wiremock mount, no async runtime required.
#[test]
fn m83_backend_types_are_wired() {
    // (a) The M8.3 gate name is exactly the isolated M8.3 identifier, not the
    //     M12 `ENV_ENABLE_RESOURCE_BACKEND`.
    assert_eq!(
        ENV_ENABLE_RESOURCE_FETCH_BACKEND,
        "CESIUM_ENABLE_RESOURCE_FETCH_BACKEND"
    );
    assert_ne!(
        ENV_ENABLE_RESOURCE_FETCH_BACKEND,
        "CESIUM_ENABLE_RESOURCE_BACKEND",
        "M8.3 gate must NOT be conflated with the M12 Resource-backend flag"
    );

    // (b) The gate defaults OFF when the env var is unset (golden-path
    //     invariant). We only assert this when the ambient env is clean, so
    //     parallel tests that legitimately flip the gate are not disturbed.
    if std::env::var(ENV_ENABLE_RESOURCE_FETCH_BACKEND).is_err() {
        assert!(!resource_fetch_backend_enabled());
    }

    // (c) `HttpTileFetcher` still implements `TileFetcher` (the port the
    //     golden path consumes) and can be constructed with the default ureq
    //     agent.
    let fetcher = HttpTileFetcher::new("https://tiles.example.com");
    let _: &dyn TileFetcher = &fetcher;

    // (d) `NetworkResourceBackend` is dyn-compatible as
    //     `ResourceBackend<u64>` — this is the M8.3 delegate target that
    //     `HttpTileFetcher::fetch` consults when the gate is ON.
    let backend = NetworkResourceBackend::new();
    let boxed: Box<dyn ResourceBackend<u64>> = Box::new(backend);
    assert_eq!(boxed.name(), "cesium-network-resource");
    assert!(boxed.is_available());
}

/// M8.4 (#67) wiring proof — **not ignored**, runs the real adapter execution.
///
/// Proves the M8.4 convergence gate (`Resource::fetch` 全量切换收敛): the
/// IO-free domain descriptors produced by `Resource::fetch_*`/`post` are
/// executed by the adapter's [`HttpTileFetcher::fetch_descriptor_blocking`] —
/// the single gate-guarded entry point — not by ad-hoc HTTP calls scattered in
/// loaders. Two branches are exercised here with zero network / no async
/// runtime:
/// * a `data:` URI descriptor short-circuits through the pure domain decoder,
/// * a non-GET (`post`) descriptor surfaces a `PortError::Network` because the
///   `NetworkBackend::fetch(url)` trait executes GET only (docs/deferred.md).
///
/// The gate-ON (backend) and gate-OFF (direct) live HTTP round-trip branches
/// are covered by the `adapters/network/src/lib.rs` unit tests against a local
/// ephemeral server; the wiremock-driven end-to-end assertions below stay
/// `#[ignore]`d until M11.1 provides the async harness.
#[test]
fn m84_fetch_descriptor_executes_through_adapter() {
    use cesium_ports_driven::PortError;

    let fetcher = HttpTileFetcher::new("https://tiles.example.com");

    // (a) data: URI descriptor -> short-circuit decode, zero network.
    //     "QUJDRA==" is base64 for "ABCD".
    let data_resource = Resource::new("data:application/octet-stream;base64,QUJDRA==");
    let data_descriptor = data_resource.fetch_array_buffer(None);
    assert!(data_descriptor.is_data_uri);
    let decoded = fetcher
        .fetch_descriptor_blocking(&data_descriptor)
        .expect("data URI decodes without network");
    assert_eq!(decoded, b"ABCD");

    // (b) non-GET descriptor -> the backend trait boundary surfaces an error
    //     rather than silently downgrading to GET.
    let post_resource = Resource::new("https://tiles.example.com/api");
    let post_descriptor = post_resource.post(vec![1, 2, 3], None);
    assert!(matches!(post_descriptor.method, HttpMethod::Post));
    let err = fetcher
        .fetch_descriptor_blocking(&post_descriptor)
        .unwrap_err();
    assert!(matches!(err, PortError::Network(_)));

    // (c) a GET descriptor for a non-data URL is routed by the gate: with the
    //     ambient env unset (golden path) it takes the byte-identical direct
    //     path. Assert the gate default; the live round-trip is the
    //     local-server unit test in adapters/network.
    let get_resource = Resource::new("https://tiles.example.com/tiles/0/0/0.b3dm");
    let get_descriptor = get_resource.fetch_array_buffer(None);
    assert!(matches!(get_descriptor.method, HttpMethod::Get));
    assert!(!get_descriptor.is_data_uri);
    if std::env::var(ENV_ENABLE_RESOURCE_FETCH_BACKEND).is_err() {
        assert!(!resource_fetch_backend_enabled());
    }
}

/// Skeleton: a 200 tile fetch through the real `HttpTileFetcher`.
///
/// The `Mock`/`ResponseTemplate` below are constructed synchronously to prove
/// the wiremock dev-dependency resolves offline. Mounting on a `MockServer`
/// and driving `HttpTileFetcher::fetch(url, priority)` requires an async
/// runtime (M11.1).
#[test]
#[ignore = "M8-wiremock skeleton: needs MockServer async runtime + HttpTileFetcher wiring (M11.1). See docs/deferred.md."]
fn http_tile_fetcher_returns_200_bytes() {
    // ── wiremock skeleton (sync construction; async mount deferred) ──
    let _mock = Mock::given(method("GET"))
        .and(path("/tiles/0/0/0.b3dm"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(b"glb-payload", "application/octet-stream"),
        );
    // TODO(M11.1): `_mock.mount(&mock_server).await;` then drive
    // `HttpTileFetcher::fetch(&url, 0.0)` and assert the returned bytes.

    // ── domain-side assertion (pure; already implemented) ──
    let resource = Resource::new("https://tiles.example.com/tiles/0/0/0.b3dm");
    let descriptor: FetchDescriptor = resource
        .fetch_array_buffer(None)
        .with_request_type(RequestType::Tiles3D);

    assert_eq!(descriptor.method, HttpMethod::Get);
    assert_eq!(descriptor.response_type, ResponseType::ArrayBuffer);
    assert_eq!(descriptor.request_type, RequestType::Tiles3D);
    assert_eq!(descriptor.server_key, "tiles.example.com:443");
    assert!(!descriptor.is_data_uri);
}

/// Skeleton: proxy rewrite is applied for untrusted servers before the fetch.
#[test]
#[ignore = "M8-wiremock skeleton: needs MockServer async runtime + HttpTileFetcher wiring (M11.1). See docs/deferred.md."]
fn http_tile_fetcher_applies_proxy_for_untrusted_server() {
    use cesium_resource::proxy::{DefaultProxy, ProxyPolicy};

    let policy =
        ProxyPolicy::with_proxy(DefaultProxy::new("https://proxy.example.com/".to_string()));

    let _mock = Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(b"proxied", "application/octet-stream"));

    let resource = Resource::new("https://untrusted.example.com/tile.b3dm");
    let descriptor = resource.fetch_array_buffer(Some(&policy));

    // The proxy prefix is applied and the original URL is preserved as a query.
    assert!(descriptor.url.starts_with("https://proxy.example.com/"));
    assert!(descriptor.url.contains("untrusted.example.com"));
}

/// Skeleton: a data: URI short-circuits the network entirely.
#[test]
#[ignore = "M8-wiremock skeleton: needs MockServer async runtime + HttpTileFetcher wiring (M11.1). See docs/deferred.md."]
fn data_uri_never_hits_mock_server() {
    let resource = Resource::new("data:application/octet-stream;base64,QUJDRA==");
    let descriptor = resource.fetch_array_buffer(None);

    // No mock would ever be matched: the descriptor is flagged inline.
    assert!(descriptor.is_data_uri);
    assert!(descriptor.server_key.is_empty());
    assert!(descriptor.url.starts_with("data:"));
}
