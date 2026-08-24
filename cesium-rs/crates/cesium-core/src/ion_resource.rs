//! Ported from `packages/engine/Source/Core/IonResource.js` (322 lines).
//!
//! A [`Resource`] instance that encapsulates Cesium ion asset access.
//! This object is normally not instantiated directly, use
//! [`IonResource::from_asset_id`].
//!
//! # Method-level alignment table (JS `IonResource` -> Rust)
//!
//! | CesiumJS (IonResource.js)              | Rust                                          |
//! | --------------------------------------- | --------------------------------------------- |
//! | `constructor(endpoint, endpointResource)` | [`IonResource::from_endpoint`]             |
//! | `IonResource.fromAssetId`               | [`IonResource::from_asset_id`]                |
//! | `IonResource._createEndpointResource`   | [`IonResource::create_endpoint_resource`]     |
//! | `clone` / `_makeRequest` token header   | DEVIATION: composition instead of inheritance |
//! | `credits` / `getCreditsFromEndpoint`    | DEVIATION: Credit pipeline not ported         |
//! | `retryCallback` token refresh flow      | DEVIATION: requires async endpoint re-fetch   |

use crate::check;
use crate::resource::{Resource, ResourceBackend, ResourceOptions};
use crate::runtime_error::RuntimeError;

/// Options for [`IonResource::from_asset_id`].
///
/// Mirrors the `options` parameter of `IonResource.fromAssetId`.
#[derive(Default)]
pub struct IonAssetOptions {
    /// The access token to use (defaults to `Ion.defaultAccessToken`).
    pub access_token: Option<String>,
    /// The url of the Cesium ion API server (defaults to `Ion.defaultServer`).
    pub server: Option<String>,
    /// Additional query parameters for the endpoint request.
    pub query_parameters: Option<std::collections::HashMap<String, String>>,
}

/// The asset endpoint data returned from the Cesium ion endpoint service.
///
/// DEVIATION: modeled as a struct where JS uses a plain object.
#[derive(Debug, Clone)]
pub struct IonEndpoint {
    /// The url of the asset content.
    pub url: String,
    /// The external asset type (`"3DTILES"`, `"STK_TERRAIN_SERVER"`, ...),
    /// if this is an external asset.
    pub external_type: Option<String>,
    /// The access token to use for requests against the endpoint.
    pub access_token: Option<String>,
    /// `endpoint.options.url` for external assets.
    pub options_url: Option<String>,
}

/// A Resource that encapsulates Cesium ion asset access.
///
/// DEVIATION: JS uses prototype inheritance (`IonResource extends Resource`);
/// Rust composes a [`Resource`] and re-exposes the needed accessors.
pub struct IonResource {
    /// The underlying resource (JS `this` Resource portion).
    pub resource: Resource,
    /// The asset endpoint data returned from ion (JS `_ionEndpoint`).
    ion_endpoint: IonEndpoint,
    /// The authority of the endpoint url (JS `_ionEndpointDomain`).
    ion_endpoint_domain: Option<String>,
    /// Whether this asset is external (JS `_isExternal`).
    is_external: bool,
}

impl IonResource {
    /// Creates an IonResource from endpoint data.
    ///
    /// Mirrors `new IonResource(endpoint, endpointResource)`.
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] for external imagery assets (JS:
    /// "Ion.createResource does not support external imagery assets; use
    /// IonImageryProvider instead.").
    pub fn from_endpoint(endpoint: IonEndpoint) -> Result<Self, RuntimeError> {
        let external_type = endpoint.external_type.clone();
        let is_external = external_type.is_some();

        let options = if !is_external {
            ResourceOptions {
                url: Some(endpoint.url.clone()),
                retry_attempts: Some(1),
                // DEVIATION: JS retryCallback re-fetches the endpoint on 401;
                // that async token-refresh flow is not ported yet.
                ..Default::default()
            }
        } else if matches!(
            external_type.as_deref(),
            Some("3DTILES") | Some("STK_TERRAIN_SERVER")
        ) {
            // 3D Tiles and STK Terrain Server external assets can still be
            // represented as an IonResource
            ResourceOptions {
                url: Some(endpoint.options_url.clone().unwrap_or_default()),
                ..Default::default()
            }
        } else {
            // External imagery assets have additional configuration that
            // can't be represented as a Resource
            return Err(RuntimeError::new(Some(
                "Ion.createResource does not support external imagery assets; use IonImageryProvider instead.",
            )));
        };

        let ion_endpoint_domain = if is_external {
            None
        } else {
            authority_of(&endpoint.url)
        };

        Ok(Self {
            resource: Resource::with_options(options),
            ion_endpoint: endpoint,
            ion_endpoint_domain,
            is_external,
        })
    }

    /// Asynchronously creates an instance from a Cesium ion asset id.
    ///
    /// Mirrors `IonResource.fromAssetId(assetId, options)`:
    /// fetches the endpoint JSON then constructs the resource.
    pub async fn from_asset_id(
        asset_id: u64,
        options: Option<IonAssetOptions>,
        backend: &(impl ResourceBackend + ?Sized),
    ) -> Result<Self, crate::resource::ResourceError> {
        let mut endpoint_resource = Self::create_endpoint_resource(asset_id, options);
        let endpoint_json = endpoint_resource.fetch_json(backend).await?;
        let endpoint_json = endpoint_json.ok_or_else(|| {
            crate::resource::ResourceError::RequestFailed(
                "empty ion endpoint response".to_string(),
            )
        })?;

        let endpoint = IonEndpoint {
            url: endpoint_json
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            external_type: endpoint_json
                .get("externalType")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            access_token: endpoint_json
                .get("accessToken")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            options_url: endpoint_json
                .get("options")
                .and_then(|v| v.get("url"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };

        Self::from_endpoint(endpoint).map_err(|e| {
            crate::resource::ResourceError::RequestFailed(e.message.clone())
        })
    }

    /// Creates the resource used to fetch the ion endpoint for an asset.
    ///
    /// Mirrors `IonResource._createEndpointResource(assetId, options)`.
    pub fn create_endpoint_resource(asset_id: u64, options: Option<IonAssetOptions>) -> Resource {
        //>>includeStart('debug', pragmas.debug);
        // assetId is typed in Rust (JS `Check.defined("assetId", assetId)`).
        //>>includeEnd('debug');

        let options = options.unwrap_or_default();
        let server = options
            .server
            .unwrap_or_else(crate::ion::default_server);
        let access_token = options
            .access_token
            .unwrap_or_else(crate::ion::default_access_token);
        let server = crate::resource::Resource::create_if_needed(&server);

        let mut query_parameters = std::collections::HashMap::new();
        if !access_token.is_empty() {
            query_parameters.insert("access_token".to_string(), access_token);
        }
        if let Some(extra) = options.query_parameters {
            query_parameters.extend(extra);
        }

        let mut headers = std::collections::HashMap::new();
        add_client_headers(&mut headers);

        server.get_derived_resource_with_options(crate::resource::DerivedResourceOptions {
            url: Some(&format!("v1/assets/{asset_id}/endpoint")),
            query_parameters: Some(&query_parameters),
            headers: Some(&headers),
            ..Default::default()
        })
    }

    /// The asset endpoint data returned from ion.
    pub fn ion_endpoint(&self) -> &IonEndpoint {
        &self.ion_endpoint
    }

    /// Whether this asset is external.
    pub fn is_external(&self) -> bool {
        self.is_external
    }

    /// The authority of the endpoint url (None for external assets).
    pub fn ion_endpoint_domain(&self) -> Option<&str> {
        self.ion_endpoint_domain.as_deref()
    }
}

/// Adds CesiumJS client headers to the provided headers object.
///
/// Mirrors `addClientHeaders(headers)`.
fn add_client_headers(headers: &mut std::collections::HashMap<String, String>) {
    headers.insert("X-Cesium-Client".to_string(), "CesiumJS".to_string());
    // DEVIATION: JS adds "X-Cesium-Client-Version" from the CESIUM_VERSION
    // build-time global; the Rust port does not embed a version yet.
}

/// Extracts the authority (host[:port]) of a url (port of `new Uri(url)
/// .authority()` used for `_ionEndpointDomain`).
fn authority_of(url: &str) -> Option<String> {
    url::Url::parse(url).ok().map(|u| {
        let host = u.host_str().unwrap_or_default().to_string();
        match u.port() {
            Some(port) => format!("{host}:{port}"),
            None => host,
        }
    })
}

/// Debug check helper mirroring JS `Check.defined` (debug builds only).
#[allow(dead_code)]
fn debug_check_defined<T>(name: &str, value: &Option<T>) {
    if cfg!(debug_assertions) {
        check::defined(name, value.as_ref());
    }
}
