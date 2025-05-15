// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Nym HTTP API Client
//!
//! Centralizes and implements the core API client functionality. This crate provides custom,
//! configurable middleware for a re-usable HTTP client that takes advantage of connection pooling
//! and other benefits provided by the [`reqwest`] `Client`.
//!
//! ## Making GET requests
//!
//! Create an HTTP `Client` and use it to make a GET request.
//!
//! ```rust
//! # use url::Url;
//! # use nym_http_api_client::{ApiClient, NO_PARAMS, HttpClientError};
//!
//! # type Err = HttpClientError<String>;
//! # async fn run() -> Result<(), Err> {
//! let url: Url = "https://nymvpn.com".parse()?;
//! let client = nym_http_api_client::Client::new(url, None);
//!
//! // Send a get request to the `/v1/status` path with no query parameters.
//! let resp = client.send_get_request(&["v1", "status"], NO_PARAMS).await?;
//! let body = resp.text().await?;
//!
//! println!("body = {body:?}");
//! # Ok(())
//! # }
//! ```
//!
//! ## JSON
//!
//! There are also json helper methods that assist in executing requests that send or receive json.
//! It can take any value that can be serialized into JSON.
//!
//! ```rust
//! # use std::collections::HashMap;
//! # use std::time::Duration;
//! use nym_http_api_client::{ApiClient, HttpClientError, NO_PARAMS};
//!
//! # use serde::{Serialize, Deserialize};
//! #[derive(Clone, Copy, Debug, Serialize, Deserialize)]
//! pub struct ApiHealthResponse {
//!     pub status: ApiStatus,
//!     pub uptime: u64,
//! }
//!
//! #[derive(Clone, Copy, Debug, Serialize, Deserialize)]
//! pub enum ApiStatus {
//!     Up,
//! }
//!
//! # type Err = HttpClientError<String>;
//! # async fn run() -> Result<(), Err> {
//! // This will POST a body of `{"lang":"rust","body":"json"}`
//! let mut map = HashMap::new();
//! map.insert("lang", "rust");
//! map.insert("body", "json");
//!
//! // Create a client using the ClientBuilder and set a custom timeout.
//! let client = nym_http_api_client::Client::builder("https://nymvpn.com")?
//!     .with_timeout(Duration::from_secs(10))
//!     .build()?;
//!
//! // Send a POST request with our json `map` as the body and attempt to parse the body
//! // of the response as an ApiHealthResponse from json.
//! let res: ApiHealthResponse = client.post_json(&["v1", "status"], NO_PARAMS, &map)
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Creating an ApiClient Wrapper
//!
//! An example API implementation that relies on this crate for managing the HTTP client.
//!
//! ```rust
//! # use async_trait::async_trait;
//! use nym_http_api_client::{ApiClient, HttpClientError, NO_PARAMS};
//!
//! mod routes {
//!     pub const API_VERSION: &str = "v1";
//!     pub const API_STATUS_ROUTES: &str = "api-status";
//!     pub const HEALTH: &str = "health";
//! }
//!
//! mod responses {
//!     # use serde::{Serialize, Deserialize};
//!     #[derive(Clone, Copy, Debug, Serialize, Deserialize)]
//!     pub struct ApiHealthResponse {
//!         pub status: ApiStatus,
//!         pub uptime: u64,
//!     }
//!     
//!     #[derive(Clone, Copy, Debug, Serialize, Deserialize)]
//!     pub enum ApiStatus {
//!         Up,
//!     }
//! }
//!
//! mod error {
//!     # use serde::{Serialize, Deserialize};
//!     # use core::fmt::{Display, Formatter, Result as FmtResult};
//!     #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
//!     pub struct RequestError {
//!         message: String,
//!     }
//!
//!     impl Display for RequestError {
//!         fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
//!             Display::fmt(&self.message, f)
//!         }
//!     }
//! }
//!
//! pub type SpecificAPIError = HttpClientError<error::RequestError>;
//!
//! #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
//! #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
//! pub trait SpecificApi: ApiClient {
//!     async fn health(&self) -> Result<responses::ApiHealthResponse, SpecificAPIError> {
//!         self.get_json(
//!             &[
//!                 routes::API_VERSION,
//!                 routes::API_STATUS_ROUTES,
//!                 routes::HEALTH,
//!             ],
//!             NO_PARAMS,
//!         )
//!         .await
//!     }
//! }
//!
//! impl<T: ApiClient> SpecificApi for T {}
//! ```
#![warn(missing_docs)]

pub use reqwest::{IntoUrl, StatusCode};

use async_trait::async_trait;
use reqwest::header::HeaderValue;
use reqwest::{RequestBuilder, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, instrument, warn};
use url::Url;

use bytes::Bytes;
use http::header::CONTENT_TYPE;
use http::HeaderMap;
use mime::Mime;
#[cfg(not(target_arch = "wasm32"))]
use std::net::SocketAddr;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

mod user_agent;
pub use user_agent::UserAgent;

#[cfg(not(target_arch = "wasm32"))]
mod dns;
#[cfg(not(target_arch = "wasm32"))]
pub use dns::{HickoryDnsError, HickoryDnsResolver};

/// Default HTTP request connection timeout.
///
/// The timeout is relatively high as we are often making requests over the mixnet, where latency is
/// high and chatty protocols take a while to complete.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Collection of URL Path Segments
pub type PathSegments<'a> = &'a [&'a str];
/// Collection of HTTP Request Parameters
pub type Params<'a, K, V> = &'a [(K, V)];

/// Empty collection of HTTP Request Parameters.
pub const NO_PARAMS: Params<'_, &'_ str, &'_ str> = &[];

/// The Errors that may occur when creating or using an HTTP client.
#[derive(Debug, Error)]
#[allow(missing_docs)]
pub enum HttpClientError<E: Display = String> {
    #[error("there was an issue with the REST request: {source}")]
    ReqwestClientError {
        #[from]
        source: reqwest::Error,
    },

    #[error("failed to deserialize received response: {source}")]
    ResponseDeserialisationFailure { source: serde_json::Error },

    #[error("provided url is malformed: {source}")]
    MalformedUrl {
        #[from]
        source: url::ParseError,
    },

    #[error("the requested resource could not be found")]
    NotFound,

    #[error("request failed with error message: {0}")]
    GenericRequestFailure(String),

    #[error("the request failed with status '{status}'. no additional error message provided")]
    RequestFailure { status: StatusCode },

    #[error("the returned response was empty. status: '{status}'")]
    EmptyResponse { status: StatusCode },

    #[error("failed to resolve request. status: '{status}', additional error message: {error}")]
    EndpointFailure { status: StatusCode, error: E },

    #[error("failed to decode response body: {message} from {content}")]
    ResponseDecodeFailure { message: String, content: String },

    #[cfg(target_arch = "wasm32")]
    #[error("the request has timed out")]
    RequestTimeout,
}

impl HttpClientError {
    /// Returns true if the error is a timeout.
    pub fn is_timeout(&self) -> bool {
        match self {
            HttpClientError::ReqwestClientError { source } => source.is_timeout(),
            #[cfg(target_arch = "wasm32")]
            HttpClientError::RequestTimeout => true,
            _ => false,
        }
    }

    /// Returns the HTTP status code if available.
    pub fn status_code(&self) -> Option<StatusCode> {
        match self {
            HttpClientError::RequestFailure { status } => Some(*status),
            HttpClientError::EmptyResponse { status } => Some(*status),
            HttpClientError::EndpointFailure { status, .. } => Some(*status),
            _ => None,
        }
    }
}

/// A `ClientBuilder` can be used to create a [`Client`] with custom configuration applied consistently
/// and state tracked across subsequent requests.
pub struct ClientBuilder {
    url: Url,
    timeout: Option<Duration>,
    custom_user_agent: bool,
    reqwest_client_builder: reqwest::ClientBuilder,
    #[allow(dead_code)] // not dead code, just unused in wasm
    use_secure_dns: bool,
}

impl ClientBuilder {
    /// Constructs a new `ClientBuilder`.
    ///
    /// This is the same as `Client::builder()`.
    pub fn new<U, E>(url: U) -> Result<Self, HttpClientError<E>>
    where
        U: IntoUrl,
        E: Display,
    {
        let str_url = url.as_str();

        // a naive check: if the provided URL does not start with http(s), add that scheme
        if !str_url.starts_with("http") {
            let alt = format!("http://{str_url}");
            warn!("the provided url ('{str_url}') does not contain scheme information. Changing it to '{alt}' ...");
            // TODO: or should we maybe default to https?
            Self::new(alt)
        } else {
            Ok(Self::new_with_url(url.into_url()?))
        }
    }

    /// Constructs a new http `ClientBuilder` from a valid url.
    pub fn new_with_url(url: Url) -> Self {
        if !url.scheme().starts_with("http") {
            warn!("the provided url ('{url}') does not use HTTP / HTTPS scheme");
        }

        #[cfg(target_arch = "wasm32")]
        let reqwest_client_builder = reqwest::ClientBuilder::new();

        #[cfg(not(target_arch = "wasm32"))]
        let reqwest_client_builder = {
            let r = reqwest::ClientBuilder::new();

            // Note this is extra as the `gzip` feature for `reqwest` crate should be enabled which
            // `"Enable[s] auto gzip decompression by checking the Content-Encoding response header."`
            //
            // I am going to leave it here anyways so that gzip decompression is attempted even if
            // that feature is removed.
            r.gzip(true)
        };

        ClientBuilder {
            url,
            timeout: None,
            custom_user_agent: false,
            reqwest_client_builder,
            use_secure_dns: true,
        }
    }

    /// Enables a total request timeout other than the default.
    ///
    /// The timeout is applied from when the request starts connecting until the response body has finished. Also considered a total deadline.
    ///
    /// Default is [`DEFAULT_TIMEOUT`].
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Provide a pre-configured [`reqwest::ClientBuilder`]
    pub fn with_reqwest_builder(mut self, reqwest_builder: reqwest::ClientBuilder) -> Self {
        self.reqwest_client_builder = reqwest_builder;
        self
    }

    /// Sets the `User-Agent` header to be used by this client.
    pub fn with_user_agent<V>(mut self, value: V) -> Self
    where
        V: TryInto<HeaderValue>,
        V::Error: Into<http::Error>,
    {
        self.custom_user_agent = true;
        self.reqwest_client_builder = self.reqwest_client_builder.user_agent(value);
        self
    }

    /// Override DNS resolution for specific domains to particular IP addresses.
    ///
    /// Set the port to `0` to use the conventional port for the given scheme (e.g. 80 for http).
    /// Ports in the URL itself will always be used instead of the port in the overridden addr.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn resolve_to_addrs(mut self, domain: &str, addrs: &[SocketAddr]) -> ClientBuilder {
        self.reqwest_client_builder = self.reqwest_client_builder.resolve_to_addrs(domain, addrs);
        self
    }

    /// Returns a Client that uses this ClientBuilder configuration.
    pub fn build<E>(self) -> Result<Client, HttpClientError<E>>
    where
        E: Display,
    {
        #[cfg(target_arch = "wasm32")]
        let reqwest_client = self.reqwest_client_builder.build()?;

        // TODO: we should probably be propagating the error rather than panicking,
        // but that'd break bunch of things due to type changes
        #[cfg(not(target_arch = "wasm32"))]
        let reqwest_client = {
            let mut builder = self
                .reqwest_client_builder
                .timeout(self.timeout.unwrap_or(DEFAULT_TIMEOUT));

            // if no custom user agent was set, use a default
            if !self.custom_user_agent {
                builder =
                    builder.user_agent(format!("nym-http-api-client/{}", env!("CARGO_PKG_VERSION")))
            }

            // unless explicitly disabled use the DoT/DoH enabled resolver
            if self.use_secure_dns {
                builder = builder.dns_resolver(Arc::new(HickoryDnsResolver::default()));
            }

            builder.build()?
        };

        Ok(Client {
            base_url: self.url,
            reqwest_client,

            #[cfg(target_arch = "wasm32")]
            request_timeout: self.timeout.unwrap_or(DEFAULT_TIMEOUT),
        })
    }
}

/// A simple extendable client wrapper for http request with extra url sanitization.
#[derive(Debug, Clone)]
pub struct Client {
    base_url: Url,
    reqwest_client: reqwest::Client,

    #[cfg(target_arch = "wasm32")]
    request_timeout: Duration,
}

impl Client {
    /// Create a new http `Client`
    // no timeout until https://github.com/seanmonstar/reqwest/issues/1135 is fixed
    //
    // In order to prevent interference in API requests at the DNS phase we default to a resolver
    // that uses DoT and DoH.
    pub fn new(base_url: Url, timeout: Option<Duration>) -> Self {
        Self::new_url::<_, String>(base_url, timeout).expect(
            "we provided valid url and we were unwrapping previous construction errors anyway",
        )
    }

    /// Attempt to create a new http client from a something that can be converted to a URL
    pub fn new_url<U, E>(url: U, timeout: Option<Duration>) -> Result<Self, HttpClientError<E>>
    where
        U: IntoUrl,
        E: Display,
    {
        let builder = Self::builder(url)?;
        match timeout {
            Some(timeout) => builder.with_timeout(timeout).build(),
            None => builder.build(),
        }
    }

    /// Creates a [`ClientBuilder`] to configure a [`Client`].
    ///
    /// This is the same as [`ClientBuilder::new()`].
    pub fn builder<U, E>(url: U) -> Result<ClientBuilder, HttpClientError<E>>
    where
        U: IntoUrl,
        E: Display,
    {
        ClientBuilder::new(url)
    }

    /// Update the host that this client uses when sending API requests.
    pub fn change_base_url(&mut self, new_url: Url) {
        self.base_url = new_url
    }

    /// Get the currently configured host that this client uses when sending API requests.
    pub fn current_url(&self) -> &Url {
        &self.base_url
    }
}

/// Core functionality required for types acting as API clients.
///
/// This trait defines the "skinny waist" of behaviors that are required by an API client. More
/// likely downstream libraries should use functions from the [`ApiClient`] interface which provide
/// a more ergonomic set of functionalities.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait ApiClientCore {
    /// Create an HTTP request using the host configured in this client.
    fn create_request<B, K, V>(
        &self,
        method: reqwest::Method,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
        json_body: Option<&B>,
    ) -> RequestBuilder
    where
        B: Serialize + ?Sized,
        K: AsRef<str>,
        V: AsRef<str>;

    /// Create an HTTP request using the host configured in this client and an API endpoint (i.e.
    /// `"/api/v1/mixnodes?since=12345"`). If the provided endpoint fails to parse as path (and
    /// optionally query parameters).
    ///
    /// Endpoint Examples
    /// - `"/api/v1/mixnodes?since=12345"`
    /// - `"/api/v1/mixnodes"`
    /// - `"/api/v1/mixnodes/img.png"`
    /// - `"/api/v1/mixnodes/img.png?since=12345"`
    /// - `"/"`
    /// - `"/?since=12345"`
    /// - `""`
    /// - `"?since=12345"`
    ///
    /// for more information about URL percent encodings see [`url::Url::set_path()`]
    fn create_request_endpoint<B, S>(
        &self,
        method: reqwest::Method,
        endpoint: S,
        json_body: Option<&B>,
    ) -> RequestBuilder
    where
        B: Serialize + ?Sized,
        S: AsRef<str>,
    {
        // Use a stand-in url to extract the path and queries from the provided endpoint string
        // which could potentially fail.
        //
        // This parse cannot fail
        let mut standin_url: Url = "http://example.com".parse().unwrap();

        match endpoint.as_ref().split_once("?") {
            Some((path, query)) => {
                standin_url.set_path(path);
                standin_url.set_query(Some(query));
            }
            // There is no query in the provided endpoint
            None => standin_url.set_path(endpoint.as_ref()),
        }

        let path: Vec<&str> = match standin_url.path_segments() {
            Some(segments) => segments.collect(),
            None => Vec::new(),
        };
        let params: Vec<(String, String)> = standin_url.query_pairs().into_owned().collect();

        self.create_request(method, &path, &params, json_body)
    }

    /// Send a created HTTP request.
    ///
    /// A [`RequestBuilder`] can be created with [`ApiClientCore::create_request`] or
    /// [`ApiClientCore::create_request_endpoint`] or if absolutely necessary, using reqwest
    /// tooling directly.
    async fn send<E>(&self, request: RequestBuilder) -> Result<Response, HttpClientError<E>>
    where
        E: Display;

    /// Create and send a created HTTP request.
    async fn send_request<B, K, V, E>(
        &self,
        method: reqwest::Method,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
        json_body: Option<&B>,
    ) -> Result<Response, HttpClientError<E>>
    where
        B: Serialize + ?Sized + Sync,
        K: AsRef<str> + Sync,
        V: AsRef<str> + Sync,
        E: Display,
    {
        let req = self.create_request(method, path, params, json_body);
        self.send(req).await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl ApiClientCore for Client {
    #[instrument(level = "debug", skip_all, fields(path=?path))]
    fn create_request<B, K, V>(
        &self,
        method: reqwest::Method,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
        json_body: Option<&B>,
    ) -> RequestBuilder
    where
        B: Serialize + ?Sized,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let url = sanitize_url(&self.base_url, path, params);

        let mut request = self.reqwest_client.request(method.clone(), url);

        // Indicate that compressed responses are preferred, but if not supported other encodings are fine.
        // TODO: Down the road we can be more selective about adding this, but it's inclusion here guarantees
        // that we use compression when available.
        request = request.header(reqwest::header::ACCEPT_ENCODING, "gzip;q=1.0, *;q=0.5");

        if let Some(body) = json_body {
            request = request.json(body);
        }

        request
    }

    async fn send<E>(&self, request: RequestBuilder) -> Result<Response, HttpClientError<E>>
    where
        E: Display,
    {
        #[cfg(target_arch = "wasm32")]
        {
            Ok(
                wasmtimer::tokio::timeout(self.request_timeout, request.send())
                    .await
                    .map_err(|_timeout| HttpClientError::RequestTimeout)??,
            )
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok(request.send().await?)
        }
    }
}

/// Common usage functionality for the http client.
///
/// These functions allow for cleaner downstream usage free of type parameters and unneeded imports.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait ApiClient: ApiClientCore {
    /// Create an HTTP GET Request with the provided path and parameters
    fn create_get_request<K, V>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
    ) -> RequestBuilder
    where
        K: AsRef<str>,
        V: AsRef<str>,
    {
        self.create_request(reqwest::Method::GET, path, params, None::<&()>)
    }

    /// Create an HTTP POST Request with the provided path, parameters, and json body
    fn create_post_request<B, K, V>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
        json_body: &B,
    ) -> RequestBuilder
    where
        B: Serialize + ?Sized,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        self.create_request(reqwest::Method::POST, path, params, Some(json_body))
    }

    /// Create an HTTP DELETE Request with the provided path and parameters
    fn create_delete_request<K, V>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
    ) -> RequestBuilder
    where
        K: AsRef<str>,
        V: AsRef<str>,
    {
        self.create_request(reqwest::Method::DELETE, path, params, None::<&()>)
    }

    /// Create an HTTP PATCH Request with the provided path, parameters, and json body
    fn create_patch_request<B, K, V>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
        json_body: &B,
    ) -> RequestBuilder
    where
        B: Serialize + ?Sized,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        self.create_request(reqwest::Method::PATCH, path, params, Some(json_body))
    }

    /// Create and send an HTTP GET Request with the provided path and parameters
    #[instrument(level = "debug", skip_all, fields(path=?path))]
    async fn send_get_request<K, V, E>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
    ) -> Result<Response, HttpClientError<E>>
    where
        K: AsRef<str> + Sync,
        V: AsRef<str> + Sync,
        E: Display,
    {
        self.send_request(reqwest::Method::GET, path, params, None::<&()>)
            .await
    }

    /// Create and send an HTTP POST Request with the provided path, parameters, and json data
    async fn send_post_request<B, K, V, E>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
        json_body: &B,
    ) -> Result<Response, HttpClientError<E>>
    where
        B: Serialize + ?Sized + Sync,
        K: AsRef<str> + Sync,
        V: AsRef<str> + Sync,
        E: Display,
    {
        self.send_request(reqwest::Method::POST, path, params, Some(json_body))
            .await
    }

    /// Create and send an HTTP DELETE Request with the provided path and parameters
    async fn send_delete_request<K, V, E>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
    ) -> Result<Response, HttpClientError<E>>
    where
        K: AsRef<str> + Sync,
        V: AsRef<str> + Sync,
        E: Display,
    {
        self.send_request(reqwest::Method::DELETE, path, params, None::<&()>)
            .await
    }

    /// Create and send an HTTP PATCH Request with the provided path, parameters, and json data
    async fn send_patch_request<B, K, V, E>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
        json_body: &B,
    ) -> Result<Response, HttpClientError<E>>
    where
        B: Serialize + ?Sized + Sync,
        K: AsRef<str> + Sync,
        V: AsRef<str> + Sync,
        E: Display,
    {
        self.send_request(reqwest::Method::PATCH, path, params, Some(json_body))
            .await
    }

    /// 'get' json data from the segment-defined path, e.g. `["api", "v1", "mixnodes"]`, with tuple
    /// defined key-value parameters, e.g. `[("since", "12345")]`. Attempt to parse the response
    /// into the provided type `T`.
    #[instrument(level = "debug", skip_all)]
    // TODO: deprecate in favour of get_response that works based on mime type in the response
    async fn get_json<T, K, V, E>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
    ) -> Result<T, HttpClientError<E>>
    where
        for<'a> T: Deserialize<'a>,
        K: AsRef<str> + Sync,
        V: AsRef<str> + Sync,
        E: Display + DeserializeOwned,
    {
        self.get_response(path, params).await
    }

    /// 'get' data from the segment-defined path, e.g. `["api", "v1", "mixnodes"]`, with tuple
    /// defined key-value parameters, e.g. `[("since", "12345")]`. Attempt to parse the response
    /// into the provided type `T` based on the content type header
    async fn get_response<T, K, V, E>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
    ) -> Result<T, HttpClientError<E>>
    where
        for<'a> T: Deserialize<'a>,
        K: AsRef<str> + Sync,
        V: AsRef<str> + Sync,
        E: Display + DeserializeOwned,
    {
        let res = self
            .send_request(reqwest::Method::GET, path, params, None::<&()>)
            .await?;
        parse_response(res, false).await
    }

    /// 'post' json data to the segment-defined path, e.g. `["api", "v1", "mixnodes"]`, with tuple
    /// defined key-value parameters, e.g. `[("since", "12345")]`. Attempt to parse the response
    /// into the provided type `T`.
    async fn post_json<B, T, K, V, E>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
        json_body: &B,
    ) -> Result<T, HttpClientError<E>>
    where
        B: Serialize + ?Sized + Sync,
        for<'a> T: Deserialize<'a>,
        K: AsRef<str> + Sync,
        V: AsRef<str> + Sync,
        E: Display + DeserializeOwned,
    {
        let res = self
            .send_request(reqwest::Method::POST, path, params, Some(json_body))
            .await?;
        parse_response(res, false).await
    }

    /// 'delete' json data from the segment-defined path, e.g. `["api", "v1", "mixnodes"]`, with
    /// tuple defined key-value parameters, e.g. `[("since", "12345")]`. Attempt to parse the
    /// response into the provided type `T`.
    async fn delete_json<T, K, V, E>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
    ) -> Result<T, HttpClientError<E>>
    where
        for<'a> T: Deserialize<'a>,
        K: AsRef<str> + Sync,
        V: AsRef<str> + Sync,
        E: Display + DeserializeOwned,
    {
        let res = self
            .send_request(reqwest::Method::DELETE, path, params, None::<&()>)
            .await?;
        parse_response(res, false).await
    }

    /// 'patch' json data at the segment-defined path, e.g. `["api", "v1", "mixnodes"]`, with tuple
    /// defined key-value parameters, e.g. `[("since", "12345")]`. Attempt to parse the response
    /// into the provided type `T`.
    async fn patch_json<B, T, K, V, E>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
        json_body: &B,
    ) -> Result<T, HttpClientError<E>>
    where
        B: Serialize + ?Sized + Sync,
        for<'a> T: Deserialize<'a>,
        K: AsRef<str> + Sync,
        V: AsRef<str> + Sync,
        E: Display + DeserializeOwned,
    {
        let res = self
            .send_request(reqwest::Method::PATCH, path, params, Some(json_body))
            .await?;
        parse_response(res, false).await
    }

    /// `get` json data from the provided absolute endpoint, e.g. `"/api/v1/mixnodes?since=12345"`.
    /// Attempt to parse the response into the provided type `T`.
    async fn get_json_from<T, S, E>(&self, endpoint: S) -> Result<T, HttpClientError<E>>
    where
        for<'a> T: Deserialize<'a>,
        E: Display + DeserializeOwned,
        S: AsRef<str> + Sync + Send,
    {
        let req = self.create_request_endpoint(reqwest::Method::GET, endpoint, None::<&()>);
        let res = self.send(req).await?;
        parse_response(res, false).await
    }

    /// `post` json data to the provided absolute endpoint, e.g. `"/api/v1/mixnodes?since=12345"`.
    /// Attempt to parse the response into the provided type `T`.
    async fn post_json_data_to<B, T, S, E>(
        &self,
        endpoint: S,
        json_body: &B,
    ) -> Result<T, HttpClientError<E>>
    where
        B: Serialize + ?Sized + Sync,
        for<'a> T: Deserialize<'a>,
        E: Display + DeserializeOwned,
        S: AsRef<str> + Sync + Send,
    {
        let req = self.create_request_endpoint(reqwest::Method::POST, endpoint, Some(json_body));
        let res = self.send(req).await?;
        parse_response(res, false).await
    }

    /// `delete` json data from the provided absolute endpoint, e.g.
    /// `"/api/v1/mixnodes?since=12345"`. Attempt to parse the response into the provided type `T`.
    async fn delete_json_from<T, S, E>(&self, endpoint: S) -> Result<T, HttpClientError<E>>
    where
        for<'a> T: Deserialize<'a>,
        E: Display + DeserializeOwned,
        S: AsRef<str> + Sync + Send,
    {
        let req = self.create_request_endpoint(reqwest::Method::DELETE, endpoint, None::<&()>);
        let res = self.send(req).await?;
        parse_response(res, false).await
    }

    /// `patch` json data at the provided absolute endpoint, e.g. `"/api/v1/mixnodes?since=12345"`.
    /// Attempt to parse the response into the provided type `T`.
    async fn patch_json_data_at<B, T, S, E>(
        &self,
        endpoint: S,
        json_body: &B,
    ) -> Result<T, HttpClientError<E>>
    where
        B: Serialize + ?Sized + Sync,
        for<'a> T: Deserialize<'a>,
        E: Display + DeserializeOwned,
        S: AsRef<str> + Sync + Send,
    {
        let req = self.create_request_endpoint(reqwest::Method::PATCH, endpoint, Some(json_body));
        let res = self.send(req).await?;
        parse_response(res, false).await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> ApiClient for C where C: ApiClientCore + Sync {}

/// utility function that should solve the double slash problem in API urls forever.
fn sanitize_url<K: AsRef<str>, V: AsRef<str>>(
    base: &Url,
    segments: PathSegments<'_>,
    params: Params<'_, K, V>,
) -> Url {
    let mut url = base.clone();
    let mut path_segments = url
        .path_segments_mut()
        .expect("provided validator url does not have a base!");

    path_segments.pop_if_empty();

    for segment in segments {
        let segment = segment.strip_prefix('/').unwrap_or(segment);
        let segment = segment.strip_suffix('/').unwrap_or(segment);

        path_segments.push(segment);
    }

    // I don't understand why compiler couldn't figure out that it's no longer used
    // and can be dropped
    drop(path_segments);

    if !params.is_empty() {
        url.query_pairs_mut().extend_pairs(params);
    }

    url
}

fn decode_as_text(bytes: &bytes::Bytes, headers: &HeaderMap) -> String {
    use encoding_rs::{Encoding, UTF_8};

    let content_type = try_get_mime_type(headers);

    let encoding_name = content_type
        .as_ref()
        .and_then(|mime| mime.get_param("charset").map(|charset| charset.as_str()))
        .unwrap_or("utf-8");

    let encoding = Encoding::for_label(encoding_name.as_bytes()).unwrap_or(UTF_8);

    let (text, _, _) = encoding.decode(bytes);
    text.into_owned()
}

/// Attempt to parse a response object from an HTTP response
#[instrument(level = "debug", skip_all)]
pub async fn parse_response<T, E>(res: Response, allow_empty: bool) -> Result<T, HttpClientError<E>>
where
    T: DeserializeOwned,
    E: DeserializeOwned + Display,
{
    let status = res.status();
    tracing::trace!("Status: {} (success: {})", &status, status.is_success());

    if !allow_empty {
        if let Some(0) = res.content_length() {
            return Err(HttpClientError::EmptyResponse { status });
        }
    }
    let headers = res.headers().clone();
    tracing::trace!("headers: {:?}", headers);

    if res.status().is_success() {
        // internally reqwest is first retrieving bytes and then performing parsing via serde_json
        // (and similarly does the same thing for text())
        let full = res.bytes().await?;
        decode_raw_response(&headers, full)
    } else if res.status() == StatusCode::NOT_FOUND {
        Err(HttpClientError::NotFound)
    } else {
        let Ok(plaintext) = res.text().await else {
            return Err(HttpClientError::RequestFailure { status });
        };

        if let Ok(request_error) = serde_json::from_str(&plaintext) {
            Err(HttpClientError::EndpointFailure {
                status,
                error: request_error,
            })
        } else {
            Err(HttpClientError::GenericRequestFailure(plaintext))
        }
    }
}

fn decode_as_json<T, E>(headers: &HeaderMap, content: Bytes) -> Result<T, HttpClientError<E>>
where
    T: DeserializeOwned,
    E: DeserializeOwned + Display,
{
    match serde_json::from_slice(&content) {
        Ok(data) => Ok(data),
        Err(err) => {
            let content = decode_as_text(&content, headers);
            Err(HttpClientError::ResponseDecodeFailure {
                message: err.to_string(),
                content,
            })
        }
    }
}

fn decode_as_bincode<T, E>(headers: &HeaderMap, content: Bytes) -> Result<T, HttpClientError<E>>
where
    T: DeserializeOwned,
    E: DeserializeOwned + Display,
{
    use bincode::Options;

    let opts = nym_http_api_common::make_bincode_serializer();
    match opts.deserialize(&content) {
        Ok(data) => Ok(data),
        Err(err) => {
            let content = decode_as_text(&content, headers);
            Err(HttpClientError::ResponseDecodeFailure {
                message: err.to_string(),
                content,
            })
        }
    }
}

fn decode_raw_response<T, E>(headers: &HeaderMap, content: Bytes) -> Result<T, HttpClientError<E>>
where
    T: DeserializeOwned,
    E: DeserializeOwned + Display,
{
    // if content type header is missing, fallback to our old default, json
    let mime = try_get_mime_type(headers).unwrap_or(mime::APPLICATION_JSON);

    debug!("attempting to parse response as {mime}");

    // unfortunately we can't use stronger typing for subtype as "bincode" is not a defined mime type
    match (mime.type_(), mime.subtype().as_str()) {
        (mime::APPLICATION, "json") => decode_as_json(headers, content),
        (mime::APPLICATION, "bincode") => decode_as_bincode(headers, content),
        (_, _) => {
            debug!("unrecognised mime type {mime}. falling back to json decoding...");
            decode_as_json(headers, content)
        }
    }
}

fn try_get_mime_type(headers: &HeaderMap) -> Option<Mime> {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<Mime>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizing_urls() {
        let base_url: Url = "http://foomp.com".parse().unwrap();

        // works with 1 segment
        assert_eq!(
            "http://foomp.com/foo",
            sanitize_url(&base_url, &["foo"], NO_PARAMS).as_str()
        );

        // works with 2 segments
        assert_eq!(
            "http://foomp.com/foo/bar",
            sanitize_url(&base_url, &["foo", "bar"], NO_PARAMS).as_str()
        );

        // works with leading slash
        assert_eq!(
            "http://foomp.com/foo",
            sanitize_url(&base_url, &["/foo"], NO_PARAMS).as_str()
        );
        assert_eq!(
            "http://foomp.com/foo/bar",
            sanitize_url(&base_url, &["/foo", "bar"], NO_PARAMS).as_str()
        );
        assert_eq!(
            "http://foomp.com/foo/bar",
            sanitize_url(&base_url, &["foo", "/bar"], NO_PARAMS).as_str()
        );

        // works with trailing slash
        assert_eq!(
            "http://foomp.com/foo",
            sanitize_url(&base_url, &["foo/"], NO_PARAMS).as_str()
        );
        assert_eq!(
            "http://foomp.com/foo/bar",
            sanitize_url(&base_url, &["foo/", "bar"], NO_PARAMS).as_str()
        );
        assert_eq!(
            "http://foomp.com/foo/bar",
            sanitize_url(&base_url, &["foo", "bar/"], NO_PARAMS).as_str()
        );

        // works with both leading and trailing slash
        assert_eq!(
            "http://foomp.com/foo",
            sanitize_url(&base_url, &["/foo/"], NO_PARAMS).as_str()
        );
        assert_eq!(
            "http://foomp.com/foo/bar",
            sanitize_url(&base_url, &["/foo/", "/bar/"], NO_PARAMS).as_str()
        );

        // adds params
        assert_eq!(
            "http://foomp.com/foo/bar?foomp=baz",
            sanitize_url(&base_url, &["foo", "bar"], &[("foomp", "baz")]).as_str()
        );
        assert_eq!(
            "http://foomp.com/foo/bar?arg1=val1&arg2=val2",
            sanitize_url(
                &base_url,
                &["/foo/", "/bar/"],
                &[("arg1", "val1"), ("arg2", "val2")]
            )
            .as_str()
        );
    }
}
