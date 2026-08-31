//! Ported from `packages/engine/Source/Core/Resource.js` (2281 lines).
//!
//! A resource that includes the location and any other parameters we need to
//! retrieve it or create derived resources. It also provides the ability to
//! retry requests.
//!
//! In Rust, HTTP operations are delegated to the [`ResourceBackend`] trait
//! (native `reqwest`, WASM `fetch`, or a mock in tests) instead of XHR/fetch.
//!
//! # Method-level alignment table (JS `Resource` -> Rust)
//!
//! | CesiumJS (Resource.js)                       | Rust                                                     |
//! | -------------------------------------------- | -------------------------------------------------------- |
//! | `constructor(options)`                       | [`Resource::with_options`] / [`Resource::new`]           |
//! | `Resource.createIfNeeded`                    | [`Resource::create_if_needed`] (string branch)           |
//! | `get url` / `set url`                        | [`Resource::url`] / [`Resource::set_url`]                |
//! | `get queryParameters` / `get templateValues` | [`Resource::query_parameters`] / [`Resource::template_values`] |
//! | `get extension` / `get isDataUri` / `get isBlobUri` | [`Resource::extension`] / [`Resource::is_data_uri`] / [`Resource::is_blob_uri`] |
//! | `get hasHeaders`                             | [`Resource::has_headers`]                                |
//! | `toString()`                                 | `Display for Resource`                                   |
//! | `parseUrl(url, merge, preserveQuery, baseUrl)` | [`Resource::parse_url`]                                |
//! | `getUrlComponent(query, proxy)`              | [`Resource::get_url_component`]                          |
//! | `setQueryParameters(params, useAsDefault)`   | [`Resource::set_query_parameters`]                       |
//! | `appendQueryParameters(params)`              | [`Resource::append_query_parameters`]                    |
//! | `setTemplateValues(template, useAsDefault)`  | [`Resource::set_template_values`]                        |
//! | `appendTemplateValues(template)`             | [`Resource::append_template_values`]                     |
//! | `getDerivedResource(options)`                | [`Resource::get_derived_resource_with_options`]          |
//! | `getDerivedResource({ path })` (Rust legacy) | [`Resource::get_derived_resource`]                       |
//! | `retryOnError(error)`                        | [`Resource::retry_on_error`]                             |
//! | `clone(result)`                              | [`Resource::clone_resource`]                             |
//! | `getBaseUri(includeQuery)`                   | [`Resource::get_base_uri`]                               |
//! | `appendForwardSlash()`                       | [`Resource::append_forward_slash`]                       |
//! | `fetchArrayBuffer/fetchBlob/fetchText/fetchJson` | [`Resource::fetch_array_buffer`] / [`Resource::fetch_blob`] / [`Resource::fetch_text`] / [`Resource::fetch_json`] |
//! | `fetch(options)` / `_makeRequest(options)`   | [`Resource::fetch`]                                      |
//! | `post/put/patch/delete/head/options`         | [`Resource::post`] / [`Resource::put`] / [`Resource::patch`] / [`Resource::delete`] / [`Resource::head`] / [`Resource::options_request`] |
//! | `fetchXML` / `fetchJsonp` / `fetchImage`     | DEVIATION: browser-only (DOM/script injection) — not ported |
//! | `Resource._Implementations.*`                | DEVIATION: replaced by [`ResourceBackend`] trait          |
//! | `Resource.DEFAULT`                           | [`Resource::DEFAULT`]                                    |
//!
//! DEVIATION: query parameters are stored in an insertion-ordered list
//! (mirroring JS object key order used for stringification).
//! DEVIATION: `proxy` is a [`DefaultProxy`] instance rather than a
//! duck-typed `{ getURL }` object.
//! DEVIATION: `retryCallback` is a synchronous `Fn(&ResourceError) -> bool`;
//! JS allows async callbacks returning `Promise<boolean>`.
//! DEVIATION: when the request scheduler rejects a fetch (no capacity /
//! bumped from the priority heap) JS `fetch()` returns `undefined`; the Rust
//! port returns `Err(ResourceError::RequestThrottled)` instead.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::check;
use crate::default_proxy::DefaultProxy;
use crate::get_absolute_uri::get_absolute_uri;
use crate::get_base_uri::get_base_uri;
use crate::get_extension_from_uri::get_extension_from_uri;
use crate::is_blob_uri::is_blob_uri;
use crate::is_data_uri::is_data_uri;
use crate::request::Request;
use crate::request_scheduler::RequestScheduler;
use crate::request_state::RequestState;

/// Trait abstracting HTTP backend for Resource.
///
/// Allows swapping reqwest (native), web-sys fetch (WASM), or mock (tests).
pub trait ResourceBackend: Send + Sync {
    /// Fetches the URL and returns the response body as bytes.
    fn fetch_bytes(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
    ) -> impl std::future::Future<Output = Result<Vec<u8>, ResourceError>> + Send;

    /// Fetches the URL and returns the response body as text.
    fn fetch_text(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
    ) -> impl std::future::Future<Output = Result<String, ResourceError>> + Send;
}

/// Error type for Resource operations.
#[derive(Debug, Clone)]
pub enum ResourceError {
    /// HTTP request failed.
    RequestFailed(String),
    /// HTTP status code indicates an error.
    HttpError { status: u16, message: String },
    /// JSON parsing failed.
    JsonParseError(String),
    /// Retry limit exceeded.
    RetryExceeded { attempts: u32 },
    /// URL construction error.
    InvalidUrl(String),
    /// Mirrors JS `RuntimeError("The Resource is already being fetched.")`.
    AlreadyFetching(String),
    /// The request scheduler rejected the request (no capacity / bumped off
    /// the priority heap).
    ///
    /// DEVIATION: JS `fetch()` returns `undefined` in this case.
    RequestThrottled,
    /// The request was cancelled while queued/active.
    ///
    /// Mirrors JS deferred rejection with
    /// `RuntimeError('Request cancelled: "<url>"')`.
    RequestCancelled(String),
}

impl std::fmt::Display for ResourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RequestFailed(msg) => write!(f, "Request failed: {msg}"),
            Self::HttpError { status, message } => {
                write!(f, "HTTP {status}: {message}")
            }
            Self::JsonParseError(msg) => write!(f, "JSON parse error: {msg}"),
            Self::RetryExceeded { attempts } => {
                write!(f, "Retry limit exceeded after {attempts} attempts")
            }
            Self::InvalidUrl(msg) => write!(f, "Invalid URL: {msg}"),
            Self::AlreadyFetching(msg) => write!(f, "RuntimeError: {msg}"),
            Self::RequestThrottled => {
                write!(f, "Request throttled by the request scheduler")
            }
            Self::RequestCancelled(msg) => write!(f, "RuntimeError: {msg}"),
        }
    }
}

impl std::error::Error for ResourceError {}

/// A value stored for a query parameter.
///
/// Mirrors JS query parameter values: `undefined` (bare `?key`), a single
/// string, or an array of strings (repeated keys).
#[derive(Debug, Clone, PartialEq)]
pub enum QueryValue {
    /// The key appears without `=` (JS `undefined` value).
    None,
    /// A single value.
    Single(String),
    /// Multiple values (repeated keys).
    Multiple(Vec<String>),
}

/// Insertion-ordered list of query parameters (mirrors JS object key order).
type QueryParams = Vec<(String, QueryValue)>;

/// Retry callback: called when a request fails; returning `true` retries.
///
/// DEVIATION: JS signature is `(resource, error) => boolean|Promise<boolean>`.
pub type RetryCallback = Box<dyn Fn(&ResourceError) -> bool + Send + Sync>;

/// Initialization options for the Resource constructor.
///
/// Mirrors `Resource.ConstructorOptions`.
#[derive(Default)]
pub struct ResourceOptions {
    /// The url of the resource.
    pub url: Option<String>,
    /// Query parameters that will be sent when retrieving the resource.
    pub query_parameters: Option<HashMap<String, String>>,
    /// Key/Value pairs used to replace template values (eg. `{x}`).
    pub template_values: Option<HashMap<String, String>>,
    /// Additional HTTP headers that will be sent.
    pub headers: Option<HashMap<String, String>>,
    /// A proxy to be used when loading the resource.
    pub proxy: Option<DefaultProxy>,
    /// The function to call when a request for this resource fails.
    pub retry_callback: Option<RetryCallback>,
    /// The number of times the retryCallback should be called (JS default 0).
    pub retry_attempts: Option<u32>,
    /// If true (default), parse the url for query parameters.
    pub parse_url: Option<bool>,
    /// A [`Request`] used by the request scheduler when fetching.
    ///
    /// Mirrors `options.request` (JS `this.request = options.request ?? new
    /// Request()`).
    pub scheduler_request: Option<Request>,
}

/// Options for [`Resource::get_derived_resource_with_options`].
///
/// Mirrors the options object of `Resource.prototype.getDerivedResource`.
#[derive(Default)]
pub struct DerivedResourceOptions<'a> {
    /// URL resolved relative to the url of the current instance.
    pub url: Option<&'a str>,
    /// Query parameters combined with those of the current instance.
    pub query_parameters: Option<&'a HashMap<String, String>>,
    /// Template values combined with those of the current instance.
    pub template_values: Option<&'a HashMap<String, String>>,
    /// Additional HTTP headers that will be sent.
    pub headers: Option<&'a HashMap<String, String>>,
    /// A proxy to be used when loading the resource.
    pub proxy: Option<DefaultProxy>,
    /// The function to call when loading the resource fails.
    pub retry_callback: Option<RetryCallback>,
    /// The number of times the retryCallback should be called.
    pub retry_attempts: Option<u32>,
    /// If true, keep all query parameters from both resources; otherwise
    /// derived parameters replace those of the current resource.
    pub preserve_query_parameters: bool,
}

/// Per-call request options for [`Resource::fetch`] (mirrors the `options`
/// object passed to `_makeRequest`).
#[derive(Default, Clone)]
pub struct FetchParams {
    /// The type of response (controls the type of item returned).
    pub response_type: Option<ResponseType>,
    /// Additional HTTP headers to send with the request.
    pub headers: HashMap<String, String>,
    /// Overrides the MIME type returned by the server.
    pub override_mime_type: Option<String>,
    /// HTTP method (defaults to "GET").
    pub method: String,
    /// Data posted with the request (POST/PUT/PATCH).
    pub data: Option<Vec<u8>>,
}

/// The response produced by [`Resource::fetch`].
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    /// Text body (responseType "" / "text").
    Text(String),
    /// Binary body (responseType "arraybuffer" / "blob"; in JS a blob carries
    /// the mime type — DEVIATION: mime type is not modeled).
    Bytes(Vec<u8>),
    /// Parsed JSON body (responseType "json").
    Json(serde_json::Value),
    /// No content (mirrors JS `undefined`, e.g. HEAD/OPTIONS/204).
    None,
}

/// Options for a specific request.
pub struct RequestOptions {
    /// HTTP method (GET, POST, etc.).
    pub method: String,
    /// Request body data.
    pub data: Option<Vec<u8>>,
    /// Content type header.
    pub content_type: Option<String>,
    /// Response type hint.
    pub response_type: ResponseType,
}

/// The expected response type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseType {
    Arraybuffer,
    Blob,
    Json,
    Text,
    Document,
}

impl Default for RequestOptions {
    fn default() -> Self {
        Self {
            method: "GET".to_string(),
            data: None,
            content_type: None,
            response_type: ResponseType::Json,
        }
    }
}

/// A resource that includes the location and any other parameters we need to
/// retrieve it or create derived resources. It also provides the ability to
/// retry requests.
///
/// Mirrors CesiumJS `Resource` (Resource.js, 2281 lines).
pub struct Resource {
    url: String,
    query_parameters: QueryParams,
    template_values: HashMap<String, String>,
    /// Additional HTTP headers that will be sent with the request.
    pub headers: HashMap<String, String>,
    /// A Request object that will be used. Intended for internal use only.
    pub request: Option<RequestOptions>,
    /// A proxy to be used when loading the resource.
    proxy: Option<DefaultProxy>,
    /// Function to call when a request for this resource fails.
    retry_callback: Option<RetryCallback>,
    /// The number of times the retryCallback should be called before giving up.
    retry_attempts: u32,
    retry_count: u32,
    /// Internal request state used by `checkAndResetRequest`.
    request_state: RequestState,
    /// The scheduler request used when fetching.
    ///
    /// Mirrors JS `this.request` (a `Request` instance passed to
    /// `RequestScheduler.request`).
    scheduler_request: Request,
}

impl Resource {
    /// A resource instance initialized to an empty url.
    ///
    /// Mirrors `Resource.DEFAULT` (browser: current document location;
    /// DEVIATION: native builds have no document, so the url is the JS
    /// `typeof document === "undefined"` branch: the empty string).
    pub fn default_resource() -> Self {
        Self::with_options(ResourceOptions {
            url: Some(String::new()),
            ..Default::default()
        })
    }

    /// Creates a new Resource from a URL string.
    ///
    /// Mirrors `new Resource(url)`; the url is parsed for query parameters
    /// and `retryAttempts` defaults to 0 (JS semantics).
    pub fn new(url: String) -> Self {
        Self::with_options(ResourceOptions {
            url: Some(url),
            ..Default::default()
        })
    }

    /// Creates a Resource from a URL string with query parameters.
    pub fn from_url_with_params(
        url: String,
        query_parameters: HashMap<String, String>,
    ) -> Self {
        let mut resource = Self::new(url);
        for (k, v) in query_parameters {
            set_param(&mut resource.query_parameters, k, QueryValue::Single(v));
        }
        resource
    }

    /// Creates a Resource from full constructor options.
    ///
    /// Mirrors `new Resource(options)`.
    ///
    /// # Panics
    /// In debug builds, panics with `DeveloperError` when `options.url` is
    /// missing (JS `Check.typeOf.string("options.url", options.url)`).
    /// DEVIATION: JS `new Resource()` (no url) always throws; the Rust
    /// `Default` impl permits an empty url for legacy compatibility.
    pub fn with_options(options: ResourceOptions) -> Self {
        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            check::type_of::string(
                "options.url",
                options.url.as_deref(),
            );
        }
        //>>includeEnd('debug');

        let mut resource = Self {
            url: String::new(),
            query_parameters: options
                .query_parameters
                .map(|params| {
                    params
                        .into_iter()
                        .map(|(k, v)| (k, QueryValue::Single(v)))
                        .collect()
                })
                .unwrap_or_default(),
            template_values: options.template_values.unwrap_or_default(),
            headers: options.headers.unwrap_or_default(),
            request: None,
            proxy: options.proxy,
            retry_callback: options.retry_callback,
            retry_attempts: options.retry_attempts.unwrap_or(0),
            retry_count: 0,
            request_state: RequestState::Unissued,
            scheduler_request: options.scheduler_request.unwrap_or_default(),
        };

        let parse_url = options.parse_url.unwrap_or(true);
        if parse_url {
            if let Some(url) = options.url {
                resource.parse_url(&url, true, true, None);
            }
        } else if let Some(url) = options.url {
            resource.url = url;
        }

        resource
    }

    /// A helper function to create a resource depending on whether we have a
    /// String or a Resource.
    ///
    /// Mirrors the string branch of `Resource.createIfNeeded`; when you
    /// already hold a `Resource`, use `get_derived_resource_with_options`
    /// with the same request instead (the Resource branch of the JS API).
    pub fn create_if_needed(url: &str) -> Self {
        Self::new(url.to_string())
    }

    // ── URL properties ───────────────────────────────────────────────

    /// The url to the resource with template values replaced, query string
    /// appended and encoded by proxy if one was set.
    ///
    /// Mirrors `get url()`.
    pub fn url(&self) -> String {
        self.get_url_component(true, true)
    }

    /// Sets the url, parsing it for query parameters.
    ///
    /// Mirrors `set url(value)` (`parseUrl(value, false, false)`).
    pub fn set_url(&mut self, url: String) {
        self.parse_url(&url, false, false, None);
    }

    /// The raw stored url (without query string/template substitution).
    ///
    /// DEVIATION: exposes JS private `_url` for internal/test convenience.
    pub fn raw_url(&self) -> &str {
        &self.url
    }

    /// Query parameters appended to the url (read-only view).
    pub fn query_parameters(&self) -> &QueryParams {
        &self.query_parameters
    }

    /// The key/value pairs used to replace template parameters in the url.
    pub fn template_values(&self) -> &HashMap<String, String> {
        &self.template_values
    }

    /// The file extension of the resource.
    pub fn extension(&self) -> String {
        get_extension_from_uri(Some(&self.url))
    }

    /// True if the Resource refers to a data URI.
    pub fn is_data_uri(&self) -> bool {
        is_data_uri(Some(&self.url))
    }

    /// True if the Resource refers to a blob URI.
    pub fn is_blob_uri(&self) -> bool {
        is_blob_uri(Some(&self.url))
    }

    /// True if the Resource has request headers.
    pub fn has_headers(&self) -> bool {
        !self.headers.is_empty()
    }

    /// Gets the URL with query parameters appended (ordered, JS fidelity).
    ///
    /// DEVIATION: retained for legacy compatibility; equivalent to
    /// `get_url_component(true, false)`.
    pub fn get_url_with_query_parameters(&self) -> String {
        self.get_url_component(true, false)
    }

    /// Returns the url, optional with the query string and processed by a proxy.
    ///
    /// Mirrors `getUrlComponent(query, proxy)`.
    pub fn get_url_component(&self, query: bool, proxy: bool) -> String {
        if self.is_data_uri() {
            return self.url.clone();
        }

        let mut url = self.url.clone();
        if query {
            url.push_str(&stringify_query(&self.query_parameters));
        }

        // Restore the placeholders, which may have been escaped in
        // objectToQuery or elsewhere
        let url = url.replace("%7B", "{").replace("%7D", "}");

        let mut url = if !self.template_values.is_empty() {
            replace_template_values(&url, &self.template_values)
        } else {
            url
        };

        if proxy {
            if let Some(proxy) = &self.proxy {
                url = proxy.get_url(&url);
            }
        }

        url
    }

    /// Parse a url string, and store its info.
    ///
    /// Mirrors `parseUrl(url, merge, preserveQuery, baseUrl)`.
    pub fn parse_url(&mut self, url: &str, merge: bool, preserve_query: bool, base_url: Option<&str>) {
        if is_data_uri(Some(url)) {
            // DEVIATION: data URIs are stored verbatim; JS urijs would mangle
            // the query-like portion of a data URI.
            if !merge {
                self.query_parameters.clear();
            }
            self.url = url.to_string();
            return;
        }

        // Split off fragment and query (port of `new Uri(url)` usage).
        let without_fragment = url.split('#').next().unwrap_or(url);
        let (base, query_string) = match without_fragment.find('?') {
            Some(i) => (&without_fragment[..i], &without_fragment[i + 1..]),
            None => (without_fragment, ""),
        };

        let query = parse_query_string(query_string);

        self.query_parameters = if merge {
            combine_query_parameters(&query, &self.query_parameters, preserve_query)
        } else {
            query
        };

        // Remove unneeded info from the Uri
        let mut new_url = base.to_string();

        if let Some(base_url) = base_url {
            if !has_scheme(&new_url) {
                new_url = get_absolute_uri(Some(&new_url), Some(&get_absolute_uri(Some(base_url), None)));
            }
        }

        self.url = new_url;
    }

    // ── Query parameters ─────────────────────────────────────────────

    /// Combines the specified parameters and the existing query parameters.
    /// If a value is already set, it will be replaced with the new value
    /// (unless `use_as_default`).
    ///
    /// Mirrors `setQueryParameters(params, useAsDefault)`.
    pub fn set_query_parameters(&mut self, params: &HashMap<String, String>, use_as_default: bool) {
        let new_params: QueryParams = params
            .iter()
            .map(|(k, v)| (k.clone(), QueryValue::Single(v.clone())))
            .collect();
        self.query_parameters = if use_as_default {
            combine_query_parameters(&self.query_parameters, &new_params, false)
        } else {
            combine_query_parameters(&new_params, &self.query_parameters, false)
        };
    }

    /// Combines the specified parameters and the existing query parameters,
    /// concatenating duplicate keys into arrays.
    ///
    /// Mirrors `appendQueryParameters(params)`.
    pub fn append_query_parameters(&mut self, params: &HashMap<String, String>) {
        let new_params: QueryParams = params
            .iter()
            .map(|(k, v)| (k.clone(), QueryValue::Single(v.clone())))
            .collect();
        self.query_parameters =
            combine_query_parameters(&new_params, &self.query_parameters, true);
    }

    /// Sets a single query parameter (legacy convenience API).
    pub fn set_query_parameter(&mut self, key: String, value: String) {
        set_param(&mut self.query_parameters, key, QueryValue::Single(value));
    }

    /// Adds multiple query parameters (legacy convenience API; new values
    /// take precedence, matching `setQueryParameters(params, false)`).
    pub fn add_query_parameters(&mut self, params: &HashMap<String, String>) {
        self.set_query_parameters(params, false);
    }

    /// Gets a query parameter value (single values only).
    pub fn get_query_parameter(&self, key: &str) -> Option<&str> {
        match get_param(&self.query_parameters, key)? {
            QueryValue::Single(v) => Some(v.as_str()),
            _ => None,
        }
    }

    /// Removes a query parameter entirely.
    ///
    /// DEVIATION: no single JS analogue; supports the JS
    /// `delete queryParameters[key]` pattern (e.g.
    /// `IonGeocoderService.geocodeProviderType` setter).
    pub fn remove_query_parameter(&mut self, key: &str) {
        self.query_parameters.retain(|(k, _)| k != key);
    }

    // ── Template values ──────────────────────────────────────────────

    /// Combines the specified template values and the existing ones. If a
    /// value is already set, it will be replaced with the new value (unless
    /// `use_as_default`).
    ///
    /// Mirrors `setTemplateValues(template, useAsDefault)`.
    pub fn set_template_values(&mut self, template: &HashMap<String, String>, use_as_default: bool) {
        self.template_values = if use_as_default {
            combine_string_maps(&self.template_values, template)
        } else {
            combine_string_maps(template, &self.template_values)
        };
    }

    /// Combines the specified template values and the existing ones; new
    /// values take precedence.
    ///
    /// Mirrors `appendTemplateValues(template)`.
    pub fn append_template_values(&mut self, template: &HashMap<String, String>) {
        self.template_values = combine_string_maps(template, &self.template_values);
    }

    // ── Derived resources ────────────────────────────────────────────

    /// Returns a new Resource with the path appended to this resource's URL.
    ///
    /// DEVIATION: legacy Rust convenience API; JS-faithful behavior is
    /// [`Resource::get_derived_resource_with_options`] with `url`.
    pub fn get_derived_resource(&self, path: &str) -> Self {
        let mut url = self.url.clone();
        if !url.ends_with('/') && !path.starts_with('/') {
            url.push('/');
        }
        url.push_str(path);
        let mut derived = self.clone_resource();
        derived.url = url;
        derived
    }

    /// Returns a resource relative to the current instance. All properties
    /// remain the same as the current instance unless overridden in options.
    ///
    /// Mirrors `getDerivedResource(options)`.
    pub fn get_derived_resource_with_options(&self, options: DerivedResourceOptions<'_>) -> Self {
        let mut resource = self.clone_resource();
        resource.retry_count = 0;

        if let Some(url) = options.url {
            resource.parse_url(url, true, options.preserve_query_parameters, Some(&self.url));
        }

        if let Some(query_parameters) = options.query_parameters {
            let new_params: QueryParams = query_parameters
                .iter()
                .map(|(k, v)| (k.clone(), QueryValue::Single(v.clone())))
                .collect();
            resource.query_parameters =
                combine_query_parameters(&new_params, &resource.query_parameters, false);
        }
        if let Some(template_values) = options.template_values {
            resource.template_values =
                combine_string_maps(template_values, &resource.template_values);
        }
        if let Some(headers) = options.headers {
            resource.headers = combine_string_maps(headers, &resource.headers);
        }
        if let Some(proxy) = options.proxy {
            resource.proxy = Some(proxy);
        }
        if let Some(retry_callback) = options.retry_callback {
            resource.retry_callback = Some(retry_callback);
        }
        if let Some(retry_attempts) = options.retry_attempts {
            resource.retry_attempts = retry_attempts;
        }

        resource
    }

    // ── Retry ────────────────────────────────────────────────────────

    /// Called when a resource fails to load. This will call the retryCallback
    /// function if defined until retryAttempts is reached.
    ///
    /// Mirrors `retryOnError(error)` (synchronous; see module DEVIATION).
    pub fn retry_on_error(&mut self, error: &ResourceError) -> bool {
        let Some(retry_callback) = &self.retry_callback else {
            return false;
        };
        if self.retry_count >= self.retry_attempts {
            return false;
        }

        let result = retry_callback(error);
        self.retry_count += 1;
        result
    }

    /// Gets the retry count.
    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }

    /// Sets the number of retry attempts.
    pub fn set_retry_attempts(&mut self, attempts: u32) {
        self.retry_attempts = attempts;
    }

    /// Gets the retry attempts setting.
    pub fn retry_attempts(&self) -> u32 {
        self.retry_attempts
    }

    /// Sets the retry callback.
    pub fn set_retry_callback(&mut self, callback: RetryCallback) {
        self.retry_callback = Some(callback);
    }

    /// Whether a retry callback is registered.
    pub fn has_retry_callback(&self) -> bool {
        self.retry_callback.is_some()
    }

    // ── Clone ────────────────────────────────────────────────────────

    /// Duplicates a Resource instance.
    ///
    /// Mirrors `clone()` (the no-result branch). DEVIATION: `_retryCount`
    /// resets to 0 as in the JS result-parameter branch.
    pub fn clone_resource(&self) -> Self {
        Self {
            url: self.url.clone(),
            query_parameters: self.query_parameters.clone(),
            template_values: self.template_values.clone(),
            headers: self.headers.clone(),
            request: None,
            proxy: self.proxy.clone(),
            // DEVIATION: the callback is an Rc-less Box in Rust; clones share
            // nothing — the callback is not copied. Callers that need it on
            // derived resources must re-set it.
            retry_callback: None,
            retry_attempts: self.retry_attempts,
            retry_count: 0,
            request_state: RequestState::Unissued,
            scheduler_request: self.scheduler_request.clone_request(),
        }
    }

    // ── Base URI / path helpers ──────────────────────────────────────

    /// Returns the base path of the Resource.
    ///
    /// Mirrors `getBaseUri(includeQuery)`.
    pub fn get_base_uri(&self, include_query: bool) -> String {
        get_base_uri(
            Some(&self.get_url_component(include_query, false)),
            Some(include_query),
        )
    }

    /// Appends a forward slash to the URL.
    ///
    /// Mirrors `appendForwardSlash()`.
    pub fn append_forward_slash(&mut self) {
        if !self.url.ends_with('/') {
            self.url.push('/');
        }
    }

    // ── Headers ──────────────────────────────────────────────────────

    /// Sets a header value.
    pub fn set_header(&mut self, key: String, value: String) {
        self.headers.insert(key, value);
    }

    /// Returns whether this resource has the given header.
    pub fn has_header(&self, name: &str) -> bool {
        self.headers.contains_key(name)
    }

    /// Removes a header.
    pub fn delete_header(&mut self, name: &str) {
        self.headers.remove(name);
    }

    /// Gets a header value.
    pub fn get_header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(|s| s.as_str())
    }

    // ── Proxy ────────────────────────────────────────────────────────

    /// Sets the proxy.
    pub fn set_proxy(&mut self, proxy: DefaultProxy) {
        self.proxy = Some(proxy);
    }

    /// Gets the proxy.
    pub fn proxy(&self) -> Option<&DefaultProxy> {
        self.proxy.as_ref()
    }

    // ── Request options ──────────────────────────────────────────────

    /// Sets the request options for the next fetch.
    pub fn set_request_options(&mut self, options: RequestOptions) {
        self.request = Some(options);
    }

    /// The scheduler request used when fetching.
    ///
    /// Mirrors the public JS `resource.request` (the `Request` instance
    /// scheduled by `RequestScheduler`; distinct from the Rust
    /// [`RequestOptions`] HTTP options).
    pub fn scheduler_request(&self) -> &Request {
        &self.scheduler_request
    }

    // ── Fetch methods ────────────────────────────────────────────────

    /// Asynchronously loads the given resource.
    ///
    /// Mirrors `fetch(options)` -> `_makeRequest(options)`:
    /// - `checkAndResetRequest` guard ("The Resource is already being fetched.")
    /// - data URI decoding short-circuit (JS `loadWithXhr` dataUriRegex branch)
    /// - headers combine (options headers take precedence over resource headers)
    /// - `retryOnError` -> reset to UNISSUED and re-fetch (loop here)
    /// - the request is scheduled through [`RequestScheduler::request`];
    ///   throttled requests wait for promotion by
    ///   [`RequestScheduler::update`] before the backend fetch runs, and
    ///   completion/failure releases the scheduler slot
    ///   (JS `requestFunction`/`getRequestReceivedFunction` /
    ///   `getRequestFailedFunction` flow).
    ///
    /// DEVIATION: when the scheduler rejects the request (JS returns
    /// `undefined`), `Err(ResourceError::RequestThrottled)` is returned.
    pub async fn fetch(
        &mut self,
        backend: &(impl ResourceBackend + ?Sized),
        params: Option<FetchParams>,
    ) -> Result<Response, ResourceError> {
        let params = params.unwrap_or_default();

        // checkAndResetRequest(this.request)
        check_and_reset_request(&mut self.request_state)?;

        let mut params = params;
        if params.method.is_empty() {
            params.method = "GET".to_string();
        }

        loop {
            // JS `_makeRequest`: request.url = resource.url, then
            // RequestScheduler.request(request).
            self.scheduler_request.url = Some(self.url());
            self.scheduler_request.state = RequestState::Unissued;
            self.scheduler_request.cancelled = false;
            if RequestScheduler::request(&mut self.scheduler_request).is_none() {
                // The request did not have high enough priority to be
                // issued (JS: fetch returns undefined).
                self.request_state = RequestState::Unissued;
                return Err(ResourceError::RequestThrottled);
            }

            let request_id = self.scheduler_request.id();
            // data/blob uris bypass the scheduler (state == Received).
            let bypassed_scheduler =
                self.scheduler_request.state == RequestState::Received;

            if !bypassed_scheduler && self.scheduler_request.state == RequestState::Issued {
                // Throttled: wait until the scheduler promotes the request
                // to ACTIVE (JS: the deferred promise resolves when
                // startRequest runs; Scene drives update() every frame, so
                // promotion never happens synchronously at request time —
                // yield first to mirror that frame boundary).
                loop {
                    yield_now().await;
                    RequestScheduler::update();
                    match RequestScheduler::tracked_request_state(request_id) {
                        Some(RequestState::Active) => {
                            self.scheduler_request.state = RequestState::Active;
                            break;
                        }
                        Some(RequestState::Cancelled) => {
                            self.request_state = RequestState::Unissued;
                            return Err(ResourceError::RequestCancelled(format!(
                                "Request cancelled: \"{}\"",
                                self.url()
                            )));
                        }
                        _ => yield_now().await,
                    }
                }
            }

            self.request_state = RequestState::Active;
            let result = self.fetch_once(backend, &params).await;
            match result {
                Ok(response) => {
                    if !bypassed_scheduler {
                        RequestScheduler::complete_request_with_id(request_id);
                    }
                    self.request_state = RequestState::Received;
                    // Reset so the resource can be fetched again.
                    self.request_state = RequestState::Unissued;
                    return Ok(response);
                }
                Err(error) => {
                    if !bypassed_scheduler {
                        // JS: only retry when request.state === RequestState.FAILED
                        RequestScheduler::fail_request_with_id(
                            request_id,
                            &error.to_string(),
                        );
                    }
                    self.request_state = RequestState::Failed;
                    if self.retry_on_error(&error) {
                        // Reset request so it can try again
                        self.request_state = RequestState::Unissued;
                        continue;
                    }
                    self.request_state = RequestState::Unissued;
                    return Err(error);
                }
            }
        }
    }

    async fn fetch_once(
        &self,
        backend: &(impl ResourceBackend + ?Sized),
        params: &FetchParams,
    ) -> Result<Response, ResourceError> {
        let url = self.url();

        // data URI short-circuit (JS: dataUriRegex branch in loadWithXhr).
        if let Some(decoded) = decode_data_uri(&url, params.response_type)? {
            return Ok(decoded);
        }

        // headers = combine(options.headers, resource.headers)
        let mut headers = self.headers.clone();
        for (k, v) in &params.headers {
            headers.insert(k.clone(), v.clone());
        }

        let response_type = params.response_type;
        let result = match response_type {
            Some(ResponseType::Arraybuffer) | Some(ResponseType::Blob) => {
                backend.fetch_bytes(&url, &headers).await.map(Response::Bytes)
            }
            Some(ResponseType::Json) => {
                let text = backend.fetch_text(&url, &headers).await?;
                serde_json::from_str(&text)
                    .map(Response::Json)
                    .map_err(|e| ResourceError::JsonParseError(e.to_string()))
            }
            // "" / "text" / "document" -> text (DEVIATION: no DOM document)
            _ => backend.fetch_text(&url, &headers).await.map(Response::Text),
        };
        result
    }

    /// Asynchronously loads the resource as raw binary data.
    ///
    /// Mirrors `fetchArrayBuffer()`.
    pub async fn fetch_array_buffer(
        &mut self,
        backend: &(impl ResourceBackend + ?Sized),
    ) -> Result<Option<Vec<u8>>, ResourceError> {
        match self
            .fetch(
                backend,
                Some(FetchParams {
                    response_type: Some(ResponseType::Arraybuffer),
                    ..Default::default()
                }),
            )
            .await?
        {
            Response::Bytes(bytes) => Ok(Some(bytes)),
            Response::None => Ok(None),
            _ => Ok(None),
        }
    }

    /// Asynchronously loads the given resource as a blob (bytes in Rust).
    ///
    /// Mirrors `fetchBlob()`.
    pub async fn fetch_blob(
        &mut self,
        backend: &(impl ResourceBackend + ?Sized),
    ) -> Result<Option<Vec<u8>>, ResourceError> {
        match self
            .fetch(
                backend,
                Some(FetchParams {
                    response_type: Some(ResponseType::Blob),
                    ..Default::default()
                }),
            )
            .await?
        {
            Response::Bytes(bytes) => Ok(Some(bytes)),
            _ => Ok(None),
        }
    }

    /// Asynchronously loads the given resource as text.
    ///
    /// Mirrors `fetchText()`.
    pub async fn fetch_text(
        &mut self,
        backend: &(impl ResourceBackend + ?Sized),
    ) -> Result<Option<String>, ResourceError> {
        match self
            .fetch(
                backend,
                Some(FetchParams {
                    response_type: Some(ResponseType::Text),
                    ..Default::default()
                }),
            )
            .await?
        {
            Response::Text(text) => Ok(Some(text)),
            _ => Ok(None),
        }
    }

    /// Asynchronously loads the given resource as JSON. Adds
    /// `Accept: application/json,*/*;q=0.01` to the request headers.
    ///
    /// Mirrors `fetchJson()`.
    pub async fn fetch_json(
        &mut self,
        backend: &(impl ResourceBackend + ?Sized),
    ) -> Result<Option<serde_json::Value>, ResourceError> {
        let mut headers = HashMap::new();
        headers.insert(
            "Accept".to_string(),
            "application/json,*/*;q=0.01".to_string(),
        );
        let params = FetchParams {
            response_type: Some(ResponseType::Text),
            headers,
            ..Default::default()
        };
        match self.fetch(backend, Some(params)).await? {
            Response::Text(text) => {
                let value = serde_json::from_str(&text)
                    .map_err(|e| ResourceError::JsonParseError(e.to_string()))?;
                Ok(Some(value))
            }
            Response::Json(value) => Ok(Some(value)),
            _ => Ok(None),
        }
    }

    /// Posts data to the given resource.
    ///
    /// Mirrors `post(data, options)`.
    pub async fn post(
        &mut self,
        backend: &(impl ResourceBackend + ?Sized),
        data: Vec<u8>,
        options: Option<FetchParams>,
    ) -> Result<Response, ResourceError> {
        let mut params = options.unwrap_or_default();
        params.method = "POST".to_string();
        params.data = Some(data);
        self.fetch(backend, Some(params)).await
    }

    /// Puts data to the given resource.
    ///
    /// Mirrors `put(data, options)`.
    pub async fn put(
        &mut self,
        backend: &(impl ResourceBackend + ?Sized),
        data: Vec<u8>,
        options: Option<FetchParams>,
    ) -> Result<Response, ResourceError> {
        let mut params = options.unwrap_or_default();
        params.method = "PUT".to_string();
        params.data = Some(data);
        self.fetch(backend, Some(params)).await
    }

    /// Patches data to the given resource.
    ///
    /// Mirrors `patch(data, options)`.
    pub async fn patch(
        &mut self,
        backend: &(impl ResourceBackend + ?Sized),
        data: Vec<u8>,
        options: Option<FetchParams>,
    ) -> Result<Response, ResourceError> {
        let mut params = options.unwrap_or_default();
        params.method = "PATCH".to_string();
        params.data = Some(data);
        self.fetch(backend, Some(params)).await
    }

    /// Asynchronously deletes the given resource.
    ///
    /// Mirrors `delete(options)`.
    pub async fn delete(
        &mut self,
        backend: &(impl ResourceBackend + ?Sized),
        options: Option<FetchParams>,
    ) -> Result<Response, ResourceError> {
        let mut params = options.unwrap_or_default();
        params.method = "DELETE".to_string();
        self.fetch(backend, Some(params)).await
    }

    /// Asynchronously gets headers of the given resource.
    ///
    /// Mirrors `head(options)`. DEVIATION: backends return bodies, not
    /// header maps; resolves to [`Response::None`].
    pub async fn head(
        &mut self,
        backend: &(impl ResourceBackend + ?Sized),
        options: Option<FetchParams>,
    ) -> Result<Response, ResourceError> {
        let mut params = options.unwrap_or_default();
        params.method = "HEAD".to_string();
        self.fetch(backend, Some(params)).await
    }

    /// Asynchronously performs an OPTIONS request.
    ///
    /// Mirrors `options(options)`. DEVIATION: see [`Resource::head`].
    pub async fn options_request(
        &mut self,
        backend: &(impl ResourceBackend + ?Sized),
        options: Option<FetchParams>,
    ) -> Result<Response, ResourceError> {
        let mut params = options.unwrap_or_default();
        params.method = "OPTIONS".to_string();
        self.fetch(backend, Some(params)).await
    }

    // ── Static fetch helpers ─────────────────────────────────────────

    /// Creates a Resource and calls fetchArrayBuffer() on it.
    pub async fn fetch_array_buffer_with_options(
        backend: &(impl ResourceBackend + ?Sized),
        options: ResourceOptions,
    ) -> Result<Option<Vec<u8>>, ResourceError> {
        Self::with_options(options).fetch_array_buffer(backend).await
    }

    /// Creates a Resource and calls fetchText() on it.
    pub async fn fetch_text_with_options(
        backend: &(impl ResourceBackend + ?Sized),
        options: ResourceOptions,
    ) -> Result<Option<String>, ResourceError> {
        Self::with_options(options).fetch_text(backend).await
    }

    /// Creates a Resource and calls fetchJson() on it.
    pub async fn fetch_json_with_options(
        backend: &(impl ResourceBackend + ?Sized),
        options: ResourceOptions,
    ) -> Result<Option<serde_json::Value>, ResourceError> {
        Self::with_options(options).fetch_json(backend).await
    }

    // ── Ion endpoint helpers ─────────────────────────────────────────

    /// Creates a Resource for a Cesium ion asset endpoint.
    ///
    /// DEVIATION: In CesiumJS, this calls `IonResource.fromAssetId()` which
    /// contacts the ion API to resolve the asset URL. In Rust, this returns
    /// a Resource pointing to the ion REST API URL pattern. Actual resolution
    /// requires an ion access token and HTTP request.
    pub fn from_ion_asset_id(asset_id: u64, access_token: &str) -> Self {
        let url = format!(
            "https://api.cesium.com/v1/assets/{asset_id}/endpoint?access_token={access_token}"
        );
        let mut resource = Self::new(url);
        resource.set_header(
            "Authorization".to_string(),
            format!("Bearer {access_token}"),
        );
        resource
    }
}

impl Default for Resource {
    fn default() -> Self {
        // DEVIATION: JS `new Resource()` throws (options.url required); the
        // Rust Default impl keeps an empty url for legacy compatibility.
        Self {
            url: String::new(),
            query_parameters: Vec::new(),
            template_values: HashMap::new(),
            headers: HashMap::new(),
            request: None,
            proxy: None,
            retry_callback: None,
            retry_attempts: 0,
            retry_count: 0,
            request_state: RequestState::Unissued,
            scheduler_request: Request::default(),
        }
    }
}

impl std::fmt::Display for Resource {
    /// Mirrors `Resource.prototype.toString`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_url_component(true, true))
    }
}

/// Checks to make sure the Resource isn't already being requested.
///
/// Mirrors `checkAndResetRequest(request)`.
fn check_and_reset_request(state: &mut RequestState) -> Result<(), ResourceError> {
    if matches!(state, RequestState::Issued | RequestState::Active) {
        return Err(ResourceError::AlreadyFetching(
            "The Resource is already being fetched.".to_string(),
        ));
    }
    *state = RequestState::Unissued;
    Ok(())
}

/// A cooperative yield for the throttled-fetch wait loop.
///
/// DEVIATION: JS awaits the scheduler's deferred promise; the Rust wait
/// loop polls `RequestScheduler::update()` and yields to the executor so
/// other fetches can complete and free slots (tokio is dev-only, so this is
/// a hand-rolled one-shot yield).
struct YieldNow(bool);

impl std::future::Future for YieldNow {
    type Output = ();

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<()> {
        if self.0 {
            std::task::Poll::Ready(())
        } else {
            self.0 = true;
            cx.waker().wake_by_ref();
            std::task::Poll::Pending
        }
    }
}

async fn yield_now() {
    YieldNow(false).await
}

// ── Query string helpers (mirror private functions in Resource.js) ───

/// Mirrors `parseQueryString(queryString)`.
fn parse_query_string(query_string: &str) -> QueryParams {
    if query_string.is_empty() {
        return Vec::new();
    }

    // Special case where the querystring is just a string, not key/value pairs
    if !query_string.contains('=') {
        return vec![(query_string.to_string(), QueryValue::None)];
    }

    let mut result: QueryParams = Vec::new();
    let replaced = query_string.replace('+', "%20");
    for part in replaced.split(['&', ';']) {
        if part.is_empty() {
            continue;
        }
        let mut subparts = part.splitn(2, '=');
        let name = decode_uri_component(subparts.next().unwrap_or(""));
        let value = match subparts.next() {
            Some(v) => decode_uri_component(v),
            None => String::new(),
        };

        match find_param(&result, &name) {
            Some(index) => match &mut result[index].1 {
                QueryValue::None => result[index].1 = QueryValue::Single(value),
                QueryValue::Single(existing) => {
                    let old = std::mem::take(existing);
                    result[index].1 = QueryValue::Multiple(vec![old, value]);
                }
                QueryValue::Multiple(arr) => arr.push(value),
            },
            None => result.push((name, QueryValue::Single(value))),
        }
    }
    result
}

/// Mirrors `stringifyQuery(queryObject)`.
fn stringify_query(params: &QueryParams) -> String {
    if params.is_empty() {
        return String::new();
    }
    if params.len() == 1 && matches!(params[0].1, QueryValue::None) {
        // We have 1 key with an undefined value, so this is just a string,
        // not key/value pairs
        return format!("?{}", params[0].0);
    }

    let mut result = String::from("?");
    let mut first = true;
    for (key, value) in params {
        let encoded_key = url_encode_component(key);
        match value {
            QueryValue::None => {
                // DEVIATION: JS objectToQuery throws for undefined values in
                // multi-key maps; serialize as a bare key for robustness.
                if !first {
                    result.push('&');
                }
                result.push_str(&encoded_key);
            }
            QueryValue::Single(v) => {
                if !first {
                    result.push('&');
                }
                result.push_str(&encoded_key);
                result.push('=');
                result.push_str(&url_encode_component(v));
            }
            QueryValue::Multiple(arr) => {
                for v in arr {
                    if !first {
                        result.push('&');
                    }
                    result.push_str(&encoded_key);
                    result.push('=');
                    result.push_str(&url_encode_component(v));
                    first = false;
                }
                continue;
            }
        }
        first = false;
    }
    result
}

/// Mirrors `combineQueryParameters(q1, q2, preserveQueryParameters)`.
fn combine_query_parameters(q1: &QueryParams, q2: &QueryParams, preserve: bool) -> QueryParams {
    if !preserve {
        // combine(q1, q2): q1 values take precedence, q2 fills gaps.
        let mut result = q1.clone();
        for (key, value) in q2 {
            if find_param(&result, key).is_none() {
                result.push((key.clone(), value.clone()));
            }
        }
        return result;
    }

    let mut result = q1.clone();
    for (key, q2_value) in q2 {
        match find_param_mut(&mut result, key) {
            Some(existing) => {
                let mut values = match std::mem::replace(existing, QueryValue::None) {
                    QueryValue::None => Vec::new(),
                    QueryValue::Single(v) => vec![v],
                    QueryValue::Multiple(arr) => arr,
                };
                match q2_value {
                    QueryValue::None => {}
                    QueryValue::Single(v) => values.push(v.clone()),
                    QueryValue::Multiple(arr) => values.extend(arr.iter().cloned()),
                }
                *existing = QueryValue::Multiple(values);
            }
            None => {
                let value = match q2_value {
                    QueryValue::Multiple(arr) => QueryValue::Multiple(arr.clone()),
                    other => other.clone(),
                };
                result.push((key.clone(), value));
            }
        }
    }
    result
}

/// Mirrors `combine(a, b)` for string maps: keys of `a` take precedence.
fn combine_string_maps(
    a: &HashMap<String, String>,
    b: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut result = a.clone();
    for (k, v) in b {
        result.entry(k.clone()).or_insert_with(|| v.clone());
    }
    result
}

fn find_param(params: &QueryParams, key: &str) -> Option<usize> {
    params.iter().position(|(k, _)| k == key)
}

fn find_param_mut<'a>(params: &'a mut QueryParams, key: &str) -> Option<&'a mut QueryValue> {
    params
        .iter_mut()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v)
}

fn set_param(params: &mut QueryParams, key: String, value: QueryValue) {
    match find_param(params, &key) {
        Some(index) => params[index].1 = value,
        None => params.push((key, value)),
    }
}

fn get_param<'a>(params: &'a QueryParams, key: &str) -> Option<&'a QueryValue> {
    params
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v)
}

fn has_scheme(url: &str) -> bool {
    url::Url::parse(url).is_ok_and(|u| !u.scheme().is_empty())
}

/// Mirrors the `{(.*?)}` template replacement in `getUrlComponent`.
fn replace_template_values(url: &str, template_values: &HashMap<String, String>) -> String {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| regex::Regex::new(r"\{(.*?)\}").unwrap());
    re.replace_all(url, |caps: &regex::Captures| {
        let key = caps.get(1).map_or("", |m| m.as_str());
        match template_values.get(key) {
            // use the replacement value from templateValues if there is one...
            Some(replacement) => url_encode_component(replacement),
            // otherwise leave it unchanged
            None => caps.get(0).map_or("", |m| m.as_str()).to_string(),
        }
    })
    .into_owned()
}

// ── Data URI decoding (mirrors decodeDataUri & friends) ──────────────

const DATA_URI_REGEX: &str = r"^data:(.*?)(;base64)?,(.*)$";

/// Mirrors `decodeDataUri(dataUriRegexResult, responseType)`; returns `None`
/// when `url` is not a data URI.
fn decode_data_uri(url: &str, response_type: Option<ResponseType>) -> Result<Option<Response>, ResourceError> {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| regex::Regex::new(DATA_URI_REGEX).unwrap());
    let Some(caps) = re.captures(url) else {
        return Ok(None);
    };

    let is_base64 = caps.get(2).is_some();
    let data = caps.get(3).map_or("", |m| m.as_str());

    let response = match response_type {
        None | Some(ResponseType::Text) => {
            Response::Text(decode_data_uri_text(is_base64, data)?)
        }
        Some(ResponseType::Arraybuffer) | Some(ResponseType::Blob) => {
            Response::Bytes(decode_data_uri_array_buffer(is_base64, data)?)
        }
        Some(ResponseType::Json) => {
            let text = decode_data_uri_text(is_base64, data)?;
            let value = serde_json::from_str(&text)
                .map_err(|e| ResourceError::JsonParseError(e.to_string()))?;
            Response::Json(value)
        }
        // DEVIATION: "document" responseType requires a DOM parser; fall back
        // to text.
        Some(ResponseType::Document) => {
            Response::Text(decode_data_uri_text(is_base64, data)?)
        }
    };
    Ok(Some(response))
}

/// Mirrors `decodeDataUriText(isBase64, data)`.
fn decode_data_uri_text(is_base64: bool, data: &str) -> Result<String, ResourceError> {
    let result = decode_uri_component(data);
    if is_base64 {
        let bytes = base64_decode(&result)?;
        // JS atob produces a binary string (char codes); treat as UTF-8 where
        // possible for text semantics.
        return Ok(bytes.into_iter().map(|b| b as char).collect());
    }
    Ok(result)
}

/// Mirrors `decodeDataUriArrayBuffer(isBase64, data)`.
fn decode_data_uri_array_buffer(is_base64: bool, data: &str) -> Result<Vec<u8>, ResourceError> {
    if is_base64 {
        let decoded = decode_uri_component(data);
        return base64_decode(&decoded);
    }
    Ok(decode_uri_component(data).bytes().collect())
}

/// Minimal base64 decoder (mirrors `atob`).
fn base64_decode(input: &str) -> Result<Vec<u8>, ResourceError> {
    fn value(b: u8) -> Option<u8> {
        match b {
            b'A'..=b'Z' => Some(b - b'A'),
            b'a'..=b'z' => Some(b - b'a' + 26),
            b'0'..=b'9' => Some(b - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    let bytes: Vec<u8> = input
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        if chunk.len() < 4 {
            return Err(ResourceError::InvalidUrl(
                "Invalid base64 data URI".to_string(),
            ));
        }
        let padding = chunk.iter().filter(|&&b| b == b'=').count();
        let mut buf: u32 = 0;
        for (i, &b) in chunk.iter().enumerate() {
            let v = if b == b'=' {
                0
            } else {
                match value(b) {
                    Some(v) => v,
                    None => {
                        return Err(ResourceError::InvalidUrl(
                            "Invalid base64 data URI".to_string(),
                        ))
                    }
                }
            };
            buf |= (v as u32) << (18 - 6 * i);
        }
        out.push((buf >> 16) as u8);
        if padding < 2 {
            out.push((buf >> 8) as u8);
        }
        if padding < 1 {
            out.push(buf as u8);
        }
    }
    Ok(out)
}

/// Mirrors `decodeURIComponent` (percent-decoding; `+` is preserved).
fn decode_uri_component(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                result.push(byte as char);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

/// Mirrors `encodeURIComponent` (unreserved: A-Z a-z 0-9 - _ . ! ~ * ' ( )).
fn url_encode_component(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~'
            | b'*' | b'\'' | b'(' | b')' => result.push(b as char),
            _ => result.push_str(&format!("%{b:02X}")),
        }
    }
    result
}

/// A mock ResourceBackend for testing.
///
/// Returns pre-configured responses without making HTTP requests.
pub struct MockResourceBackend {
    responses: HashMap<String, Vec<u8>>,
}

impl MockResourceBackend {
    /// Creates a new mock backend.
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
        }
    }

    /// Registers a mock response for a URL.
    pub fn register_response(&mut self, url: &str, body: Vec<u8>) {
        self.responses.insert(url.to_string(), body);
    }

    /// Registers a mock JSON response for a URL.
    pub fn register_json_response(&mut self, url: &str, json: &str) {
        self.register_response(url, json.as_bytes().to_vec());
    }
}

impl Default for MockResourceBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceBackend for MockResourceBackend {
    async fn fetch_bytes(
        &self,
        url: &str,
        _headers: &HashMap<String, String>,
    ) -> Result<Vec<u8>, ResourceError> {
        self.responses
            .get(url)
            .cloned()
            .ok_or_else(|| ResourceError::RequestFailed(format!("No mock response for: {url}")))
    }

    async fn fetch_text(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
    ) -> Result<String, ResourceError> {
        let bytes = self.fetch_bytes(url, headers).await?;
        String::from_utf8(bytes)
            .map_err(|e| ResourceError::RequestFailed(format!("Invalid UTF-8: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_resource_with_url() {
        let r = Resource::new("https://example.com/data".to_string());
        assert_eq!(r.url(), "https://example.com/data");
    }

    #[test]
    fn stringify_query_orders_values() {
        let r = Resource::new("http://test.com/x?a=1&a=2&b=3&a=4".to_string());
        assert_eq!(r.url(), "http://test.com/x?a=1&a=2&a=4&b=3");
    }

    #[test]
    fn template_replacement_encodes() {
        let mut template_values = HashMap::new();
        template_values.insert("foo".to_string(), "a/b".to_string());
        let r = Resource::with_options(ResourceOptions {
            url: Some("http://test.com/{foo}".to_string()),
            template_values: Some(template_values),
            ..Default::default()
        });
        assert_eq!(r.url(), "http://test.com/a%2Fb");
    }

    #[test]
    fn decode_data_uri_text_plain() {
        let decoded = decode_data_uri(
            "data:,A%20brief%20note",
            Some(ResponseType::Text),
        )
        .unwrap();
        assert_eq!(decoded, Some(Response::Text("A brief note".to_string())));
    }

    #[test]
    fn decode_data_uri_base64_arraybuffer() {
        // base64("abc") = "YWJj"
        let decoded = decode_data_uri(
            "data:application/octet-stream;base64,YWJj",
            Some(ResponseType::Arraybuffer),
        )
        .unwrap();
        assert_eq!(decoded, Some(Response::Bytes(vec![97, 98, 99])));
    }
}
