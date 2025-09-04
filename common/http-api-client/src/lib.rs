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
//! # type Err = HttpClientError;
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
//! pub type SpecificAPIError = HttpClientError;
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

pub use reqwest::ClientBuilder as ReqwestClientBuilder;
pub use reqwest::StatusCode;

use crate::path::RequestPath;
use async_trait::async_trait;
use bytes::Bytes;
use http::header::{ACCEPT, CONTENT_TYPE};
use http::HeaderMap;
use itertools::Itertools;
use mime::Mime;
use reqwest::header::HeaderValue;
use reqwest::{RequestBuilder, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::net::IpAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, instrument, warn};

#[cfg(not(target_arch = "wasm32"))]
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

#[cfg(feature = "tunneling")]
mod fronted;
#[cfg(feature = "tunneling")]
pub use fronted::FrontPolicy;
mod url;
pub use url::{IntoUrl, Url};
mod user_agent;
pub use user_agent::UserAgent;

#[cfg(not(target_arch = "wasm32"))]
mod dns;
mod path;

#[cfg(not(target_arch = "wasm32"))]
pub use dns::{HickoryDnsError, HickoryDnsResolver};

// helper for generating user agent based on binary information
#[doc(hidden)]
pub use nym_bin_common::bin_info;

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

/// Serialization format for API requests and responses
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializationFormat {
    /// Use JSON serialization (default, always works)
    Json,
    /// Use bincode serialization (must be explicitly opted into)
    Bincode,
    /// Use YAML serialization
    Yaml,
    /// Use Text serialization
    Text,
}

impl Display for SerializationFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializationFormat::Json => write!(f, "json"),
            SerializationFormat::Bincode => write!(f, "bincode"),
            SerializationFormat::Yaml => write!(f, "yaml"),
            SerializationFormat::Text => write!(f, "text"),
        }
    }
}

impl SerializationFormat {
    #[allow(missing_docs)]
    pub fn content_type(&self) -> String {
        match self {
            SerializationFormat::Json => "application/json".to_string(),
            SerializationFormat::Bincode => "application/bincode".to_string(),
            SerializationFormat::Yaml => "application/yaml".to_string(),
            SerializationFormat::Text => "text/plain".to_string(),
        }
    }
}

/// The Errors that may occur when creating or using an HTTP client.
#[derive(Debug, Error)]
#[allow(missing_docs)]
pub enum HttpClientError {
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
    EndpointFailure { status: StatusCode, error: String },

    #[error("failed to decode response body: {message} from {content}")]
    ResponseDecodeFailure { message: String, content: String },

    #[error("Failed to encode bincode: {0}")]
    Bincode(#[from] bincode::Error),

    #[error("Failed to json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Failed to yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Failed to plain: {0}")]
    Plain(#[from] serde_plain::Error),

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

/// Core functionality required for types acting as API clients.
///
/// This trait defines the "skinny waist" of behaviors that are required by an API client. More
/// likely downstream libraries should use functions from the [`ApiClient`] interface which provide
/// a more ergonomic set of functionalities.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait ApiClientCore {
    /// Create an HTTP request using the host configured in this client.
    fn create_request<P, B, K, V>(
        &self,
        method: reqwest::Method,
        path: P,
        params: Params<'_, K, V>,
        body: Option<&B>,
    ) -> Result<RequestBuilder, HttpClientError>
    where
        P: RequestPath,
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
        body: Option<&B>,
    ) -> Result<RequestBuilder, HttpClientError>
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

        self.create_request(method, path.as_slice(), &params, body)
    }

    /// Send a created HTTP request.
    ///
    /// A [`RequestBuilder`] can be created with [`ApiClientCore::create_request`] or
    /// [`ApiClientCore::create_request_endpoint`] or if absolutely necessary, using reqwest
    /// tooling directly.
    async fn send(&self, request: RequestBuilder) -> Result<Response, HttpClientError>;

    /// Create and send a created HTTP request.
    async fn send_request<P, B, K, V>(
        &self,
        method: reqwest::Method,
        path: P,
        params: Params<'_, K, V>,
        json_body: Option<&B>,
    ) -> Result<Response, HttpClientError>
    where
        P: RequestPath + Send + Sync,
        B: Serialize + ?Sized + Sync,
        K: AsRef<str> + Sync,
        V: AsRef<str> + Sync,
    {
        let req = self.create_request(method, path, params, json_body)?;
        self.send(req).await
    }
}

/// A `ClientBuilder` can be used to create a [`Client`] with custom configuration applied consistently
/// and state tracked across subsequent requests.
pub struct ClientBuilder {
    urls: Vec<Url>,

    timeout: Option<Duration>,
    custom_user_agent: bool,
    reqwest_client_builder: reqwest::ClientBuilder,
    #[allow(dead_code)] // not dead code, just unused in wasm
    use_secure_dns: bool,

    #[cfg(feature = "tunneling")]
    front: Option<fronted::Front>,

    retry_limit: usize,
    serialization: SerializationFormat,
}

impl ClientBuilder {
    /// Constructs a new `ClientBuilder`.
    ///
    /// This is the same as `Client::builder()`.
    pub fn new<U>(url: U) -> Result<Self, HttpClientError>
    where
        U: IntoUrl,
    {
        let str_url = url.as_str();

        // a naive check: if the provided URL does not start with http(s), add that scheme
        if !str_url.starts_with("http") {
            let alt = format!("http://{str_url}");
            warn!("the provided url ('{str_url}') does not contain scheme information. Changing it to '{alt}' ...");
            // TODO: or should we maybe default to https?
            Self::new(alt)
        } else {
            let url = url.to_url()?;
            Ok(Self::new_with_urls(vec![url]))
        }
    }

    /// Create a client builder from network details with sensible defaults
    #[cfg(feature = "network-defaults")]
    pub fn from_network(
        network: &nym_network_defaults::NymNetworkDetails,
    ) -> Result<Self, HttpClientError> {
        let urls = network
            .nym_api_urls
            .as_ref()
            .ok_or_else(|| {
                HttpClientError::GenericRequestFailure(
                    "No API URLs configured in network details".to_string(),
                )
            })?
            .iter()
            .map(|api_url| {
                // Convert ApiUrl to our Url type with fronting support
                let mut url = Url::parse(&api_url.url)?;

                // Add fronting domains if available
                #[cfg(feature = "tunneling")]
                if let Some(ref front_hosts) = api_url.front_hosts {
                    let fronts: Vec<String> = front_hosts
                        .iter()
                        .map(|host| format!("https://{}", host))
                        .collect();
                    url = Url::new(api_url.url.clone(), Some(fronts))
                        .map_err(|e| HttpClientError::GenericRequestFailure(e.to_string()))?;
                }

                Ok(url)
            })
            .collect::<Result<Vec<_>, HttpClientError>>()?;

        let mut builder = Self::new_with_urls(urls);

        // Enable domain fronting by default (on retry)
        #[cfg(feature = "tunneling")]
        {
            builder = builder.with_fronting(FrontPolicy::OnRetry);
        }

        Ok(builder)
    }

    /// Constructs a new http `ClientBuilder` from a valid url.
    pub fn new_with_urls(urls: Vec<Url>) -> Self {
        let urls = Self::check_urls(urls);

        #[cfg(target_arch = "wasm32")]
        let reqwest_client_builder = reqwest::ClientBuilder::new();

        warn!("FORCING IPv4 CONNECTIONS");

        #[cfg(not(target_arch = "wasm32"))]
        let reqwest_client_builder = {
            // Note: I believe the manual enable calls for the compression methods are extra
            // as the various compression features for `reqwest` crate should be enabled
            // just by including the feature which:
            // `"Enable[s] auto decompression by checking the Content-Encoding response header."`
            //
            // I am going to leave these here anyways so that removing a decompression method
            // from the features list will throw an error if it is not also removed here.
            reqwest::ClientBuilder::new()
                .gzip(true)
                .deflate(true)
                .brotli(true)
                .zstd(true)
        };

        #[cfg(feature = "force-ipv4")]
        let reqwest_client_builder = reqwest_client_builder.local_address(IpAddr::from_str("0.0.0.0").unwrap());

        ClientBuilder {
            urls,
            timeout: None,
            custom_user_agent: false,
            reqwest_client_builder,
            use_secure_dns: true,
            #[cfg(feature = "tunneling")]
            front: None,

            retry_limit: 0,
            serialization: SerializationFormat::Json,
        }
    }

    /// Add an additional URL to the set usable by this constructed `Client`
    pub fn add_url(mut self, url: Url) -> Self {
        self.urls.push(url);
        self
    }

    fn check_urls(mut urls: Vec<Url>) -> Vec<Url> {
        // remove any duplicate URLs
        urls = urls.into_iter().unique().collect();

        // warn about any invalid URLs
        urls.iter()
            .filter(|url| !url.scheme().contains("http") && !url.scheme().contains("https"))
            .for_each(|url| {
                warn!("the provided url ('{url}') does not use HTTP / HTTPS scheme");
            });

        urls
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

    /// Sets the maximum number of retries for a request. This defaults to 0, indicating no retries.
    ///
    /// Note that setting a retry limit of 3 (for example) will result in 4 attempts to send the
    /// request in the case that all are unsuccessful.
    ///
    /// If multiple urls (or fronting configurations if enabled) are available, retried requests
    /// will be sent to the next URL in the list.
    pub fn with_retries(mut self, retry_limit: usize) -> Self {
        self.retry_limit = retry_limit;
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

    /// Set the serialization format for API requests and responses
    pub fn with_serialization(mut self, format: SerializationFormat) -> Self {
        self.serialization = format;
        self
    }

    /// Configure the client to use bincode serialization
    pub fn with_bincode(self) -> Self {
        self.with_serialization(SerializationFormat::Bincode)
    }

    /// Returns a Client that uses this ClientBuilder configuration.
    pub fn build(self) -> Result<Client, HttpClientError> {
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

        let client = Client {
            base_urls: self.urls,
            current_idx: Arc::new(AtomicUsize::new(0)),
            reqwest_client,

            #[cfg(feature = "tunneling")]
            front: self.front,

            #[cfg(target_arch = "wasm32")]
            request_timeout: self.timeout.unwrap_or(DEFAULT_TIMEOUT),
            retry_limit: self.retry_limit,
            serialization: self.serialization,
        };

        Ok(client)
    }
}

/// A simple extendable client wrapper for http request with extra url sanitization.
#[derive(Debug, Clone)]
pub struct Client {
    base_urls: Vec<Url>,
    current_idx: Arc<AtomicUsize>,
    reqwest_client: reqwest::Client,

    #[cfg(feature = "tunneling")]
    front: Option<fronted::Front>,

    #[cfg(target_arch = "wasm32")]
    request_timeout: Duration,

    retry_limit: usize,
    serialization: SerializationFormat,
}

impl Client {
    /// Create a new http `Client`
    // no timeout until https://github.com/seanmonstar/reqwest/issues/1135 is fixed
    //
    // In order to prevent interference in API requests at the DNS phase we default to a resolver
    // that uses DoT and DoH.
    pub fn new(base_url: ::url::Url, timeout: Option<Duration>) -> Self {
        Self::new_url(base_url, timeout).expect(
            "we provided valid url and we were unwrapping previous construction errors anyway",
        )
    }

    /// Attempt to create a new http client from a something that can be converted to a URL
    pub fn new_url<U>(url: U, timeout: Option<Duration>) -> Result<Self, HttpClientError>
    where
        U: IntoUrl,
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
    pub fn builder<U>(url: U) -> Result<ClientBuilder, HttpClientError>
    where
        U: IntoUrl,
    {
        ClientBuilder::new(url)
    }

    /// Update the set of hosts that this client uses when sending API requests.
    pub fn change_base_urls(&mut self, new_urls: Vec<Url>) {
        self.current_idx.store(0, Ordering::Relaxed);
        self.base_urls = new_urls
    }

    /// Create new instance of `Client` using the provided base url and existing client config
    pub fn clone_with_new_url(&self, new_url: Url) -> Self {
        Client {
            base_urls: vec![new_url],
            current_idx: Arc::new(Default::default()),
            reqwest_client: self.reqwest_client.clone(),

            #[cfg(feature = "tunneling")]
            front: self.front.clone(),
            retry_limit: self.retry_limit,

            #[cfg(target_arch = "wasm32")]
            request_timeout: self.request_timeout,
            serialization: self.serialization,
        }
    }

    /// Get the currently configured host that this client uses when sending API requests.
    pub fn current_url(&self) -> &Url {
        &self.base_urls[self.current_idx.load(std::sync::atomic::Ordering::Relaxed)]
    }

    /// Get the currently configured host that this client uses when sending API requests.
    pub fn base_urls(&self) -> &[Url] {
        &self.base_urls
    }

    /// Get a mutable reference to the hosts that this client uses when sending API requests.
    pub fn base_urls_mut(&mut self) -> &mut [Url] {
        &mut self.base_urls
    }

    /// Change the currently configured limit on the number of retries for a request.
    pub fn change_retry_limit(&mut self, limit: usize) {
        self.retry_limit = limit;
    }

    /// If multiple base urls are available rotate to next (e.g. when the current one resulted in an error)
    fn update_host(&self) {
        #[cfg(feature = "tunneling")]
        if let Some(ref front) = self.front {
            if front.is_enabled() {
                // if we are using fronting, try updating to the next front
                let url = self.current_url();

                // try to update the current host to use a next front, if one is available, otherwise
                // we move on and try the next base url (if one is available)
                if url.has_front() && !url.update() {
                    // we swapped to the next front for the current host
                    return;
                }
            }
        }

        if self.base_urls.len() > 1 {
            let orig = self.current_idx.load(Ordering::Relaxed);
            let mut next = (orig + 1) % self.base_urls.len();

            // if fronting is enabled we want to update to a host that has fronts configured
            #[cfg(feature = "tunneling")]
            if let Some(ref front) = self.front {
                if front.is_enabled() {
                    while next != orig {
                        if self.base_urls[next].has_front() {
                            // we have a front for the next host, so we can use it
                            break;
                        }

                        next = (next + 1) % self.base_urls.len();
                    }
                }
            }

            self.current_idx.store(next, Ordering::Relaxed);
        }
    }

    /// Make modifications to the request to apply the current state of this client i.e. the
    /// currently configured host. This is required as a caller may use this client to create a
    /// request, but then have the state of the client change before the caller uses the client to
    /// send their request.
    ///
    /// This enures that the outgoing requests benefit from the configured fallback mechanisms, even
    /// for requests that were created before the state of the client changed.
    ///
    /// This method assumes that any updates to the state of the client are made before the call to
    /// this method. For example, if the client is configured to rotate hosts after each error, this
    /// method should be called after the host has been updated -- i.e. as part of the subsequent
    /// send.
    fn apply_hosts_to_req(&self, r: &mut reqwest::Request) -> (&str, Option<&str>) {
        let url = self.current_url();
        r.url_mut().set_host(url.host_str()).unwrap();

        #[cfg(feature = "tunneling")]
        if let Some(ref front) = self.front {
            if front.is_enabled() {
                if let Some(front_host) = url.front_str() {
                    if let Some(actual_host) = url.host_str() {
                        tracing::debug!(
                            "Domain fronting enabled: routing via CDN {} to actual host {}",
                            front_host,
                            actual_host
                        );

                        // this should never fail as we are transplanting the host from one url to another
                        r.url_mut().set_host(Some(front_host)).unwrap();

                        let actual_host_header: HeaderValue =
                            actual_host.parse().unwrap_or(HeaderValue::from_static(""));
                        // If the map did have this key present, the new value is associated with the key
                        // and all previous values are removed. (reqwest HeaderMap docs)
                        _ = r
                            .headers_mut()
                            .insert(reqwest::header::HOST, actual_host_header);

                        return (url.as_str(), url.front_str());
                    } else {
                        warn!("Domain fronting is enabled, but no host_url is defined! Domain fronting WILL NOT WORK")
                    }
                } else {
                    warn!("Domain fronting is enabled, but no front_url is defined! Domain fronting WILL NOT WORK")
                }
            }
        }
        (url.as_str(), None)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl ApiClientCore for Client {
    #[instrument(level = "debug", skip_all, fields(path=?path))]
    fn create_request<P, B, K, V>(
        &self,
        method: reqwest::Method,
        path: P,
        params: Params<'_, K, V>,
        body: Option<&B>,
    ) -> Result<RequestBuilder, HttpClientError>
    where
        P: RequestPath,
        B: Serialize + ?Sized,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let url = self.current_url();
        let url = sanitize_url(url, path, params);

        let mut req = reqwest::Request::new(method, url.into());

        self.apply_hosts_to_req(&mut req);

        let mut rb = RequestBuilder::from_parts(self.reqwest_client.clone(), req);

        rb = rb
            .header(ACCEPT, self.serialization.content_type())
            .header(CONTENT_TYPE, self.serialization.content_type());

        if let Some(body) = body {
            match self.serialization {
                SerializationFormat::Json => {
                    rb = rb.json(body);
                }
                SerializationFormat::Bincode => {
                    let body = bincode::serialize(body)?;
                    rb = rb.body(body);
                }
                SerializationFormat::Yaml => {
                    let mut body_bytes = Vec::new();
                    serde_yaml::to_writer(&mut body_bytes, &body)?;
                    rb = rb.body(body_bytes);
                }
                SerializationFormat::Text => {
                    let body = serde_plain::to_string(&body)?.as_bytes().to_vec();
                    rb = rb.body(body);
                }
            }
        }

        Ok(rb)
    }

    async fn send(&self, request: RequestBuilder) -> Result<Response, HttpClientError> {
        let mut attempts = 0;
        loop {
            // try_clone may fail if the body is a stream in which case using retries is not advised.
            let r = request
                .try_clone()
                .ok_or(HttpClientError::GenericRequestFailure(
                    "failed to send request".to_string(),
                ))?;

            // apply any changes based on the current state of the client wrt. hosts,
            // fronting domains, etc.
            let mut req = r.build()?;
            self.apply_hosts_to_req(&mut req);

            #[cfg(target_arch = "wasm32")]
            let response: Result<Response, HttpClientError> = {
                Ok(wasmtimer::tokio::timeout(
                    self.request_timeout,
                    self.reqwest_client.execute(req),
                )
                .await
                .map_err(|_timeout| HttpClientError::RequestTimeout)??)
            };

            #[cfg(not(target_arch = "wasm32"))]
            let response = self.reqwest_client.execute(req).await;

            match response {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    // if we have multiple urls, update to the next
                    self.update_host();

                    #[cfg(feature = "tunneling")]
                    if let Some(ref front) = self.front {
                        // If fronting is set to be enabled on error, enable domain fronting as we
                        // have encountered an error.
                        let was_enabled = front.is_enabled();
                        front.retry_enable();
                        if !was_enabled && front.is_enabled() {
                            tracing::info!(
                                "Domain fronting activated after connection failure: {}",
                                e
                            );
                        }
                    }

                    if attempts < self.retry_limit {
                        warn!("Retrying request due to http error: {}", e);
                        attempts += 1;
                        continue;
                    }

                    // if we have exhausted our attempts, return the error
                    #[allow(clippy::useless_conversion)] // conversion considered useless in wasm
                    return Err(e.into());
                }
            }
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
    fn create_get_request<P, K, V>(
        &self,
        path: P,
        params: Params<'_, K, V>,
    ) -> Result<RequestBuilder, HttpClientError>
    where
        P: RequestPath,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        self.create_request(reqwest::Method::GET, path, params, None::<&()>)
    }

    /// Create an HTTP POST Request with the provided path, parameters, and json body
    fn create_post_request<P, B, K, V>(
        &self,
        path: P,
        params: Params<'_, K, V>,
        json_body: &B,
    ) -> Result<RequestBuilder, HttpClientError>
    where
        P: RequestPath,
        B: Serialize + ?Sized,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        self.create_request(reqwest::Method::POST, path, params, Some(json_body))
    }

    /// Create an HTTP DELETE Request with the provided path and parameters
    fn create_delete_request<P, K, V>(
        &self,
        path: P,
        params: Params<'_, K, V>,
    ) -> Result<RequestBuilder, HttpClientError>
    where
        P: RequestPath,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        self.create_request(reqwest::Method::DELETE, path, params, None::<&()>)
    }

    /// Create an HTTP PATCH Request with the provided path, parameters, and json body
    fn create_patch_request<P, B, K, V>(
        &self,
        path: P,
        params: Params<'_, K, V>,
        json_body: &B,
    ) -> Result<RequestBuilder, HttpClientError>
    where
        P: RequestPath,
        B: Serialize + ?Sized,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        self.create_request(reqwest::Method::PATCH, path, params, Some(json_body))
    }

    /// Create and send an HTTP GET Request with the provided path and parameters
    #[instrument(level = "debug", skip_all, fields(path=?path))]
    async fn send_get_request<P, K, V>(
        &self,
        path: P,
        params: Params<'_, K, V>,
    ) -> Result<Response, HttpClientError>
    where
        P: RequestPath + Send + Sync,
        K: AsRef<str> + Sync,
        V: AsRef<str> + Sync,
    {
        self.send_request(reqwest::Method::GET, path, params, None::<&()>)
            .await
    }

    /// Create and send an HTTP POST Request with the provided path, parameters, and json data
    async fn send_post_request<P, B, K, V>(
        &self,
        path: P,
        params: Params<'_, K, V>,
        json_body: &B,
    ) -> Result<Response, HttpClientError>
    where
        P: RequestPath + Send + Sync,
        B: Serialize + ?Sized + Sync,
        K: AsRef<str> + Sync,
        V: AsRef<str> + Sync,
    {
        self.send_request(reqwest::Method::POST, path, params, Some(json_body))
            .await
    }

    /// Create and send an HTTP DELETE Request with the provided path and parameters
    async fn send_delete_request<P, K, V>(
        &self,
        path: P,
        params: Params<'_, K, V>,
    ) -> Result<Response, HttpClientError>
    where
        P: RequestPath + Send + Sync,
        K: AsRef<str> + Sync,
        V: AsRef<str> + Sync,
    {
        self.send_request(reqwest::Method::DELETE, path, params, None::<&()>)
            .await
    }

    /// Create and send an HTTP PATCH Request with the provided path, parameters, and json data
    async fn send_patch_request<P, B, K, V>(
        &self,
        path: P,
        params: Params<'_, K, V>,
        json_body: &B,
    ) -> Result<Response, HttpClientError>
    where
        P: RequestPath + Send + Sync,
        B: Serialize + ?Sized + Sync,
        K: AsRef<str> + Sync,
        V: AsRef<str> + Sync,
    {
        self.send_request(reqwest::Method::PATCH, path, params, Some(json_body))
            .await
    }

    /// 'get' json data from the segment-defined path, e.g. `["api", "v1", "mixnodes"]`, with tuple
    /// defined key-value parameters, e.g. `[("since", "12345")]`. Attempt to parse the response
    /// into the provided type `T`.
    #[instrument(level = "debug", skip_all, fields(path=?path))]
    // TODO: deprecate in favour of get_response that works based on mime type in the response
    async fn get_json<P, T, K, V>(
        &self,
        path: P,
        params: Params<'_, K, V>,
    ) -> Result<T, HttpClientError>
    where
        P: RequestPath + Send + Sync,
        for<'a> T: Deserialize<'a>,
        K: AsRef<str> + Sync,
        V: AsRef<str> + Sync,
    {
        self.get_response(path, params).await
    }

    /// 'get' data from the segment-defined path, e.g. `["api", "v1", "mixnodes"]`, with tuple
    /// defined key-value parameters, e.g. `[("since", "12345")]`. Attempt to parse the response
    /// into the provided type `T` based on the content type header
    async fn get_response<P, T, K, V>(
        &self,
        path: P,
        params: Params<'_, K, V>,
    ) -> Result<T, HttpClientError>
    where
        P: RequestPath + Send + Sync,
        for<'a> T: Deserialize<'a>,
        K: AsRef<str> + Sync,
        V: AsRef<str> + Sync,
    {
        let res = self
            .send_request(reqwest::Method::GET, path, params, None::<&()>)
            .await?;
        parse_response(res, false).await
    }

    /// 'post' json data to the segment-defined path, e.g. `["api", "v1", "mixnodes"]`, with tuple
    /// defined key-value parameters, e.g. `[("since", "12345")]`. Attempt to parse the response
    /// into the provided type `T`.
    async fn post_json<P, B, T, K, V>(
        &self,
        path: P,
        params: Params<'_, K, V>,
        json_body: &B,
    ) -> Result<T, HttpClientError>
    where
        P: RequestPath + Send + Sync,
        B: Serialize + ?Sized + Sync,
        for<'a> T: Deserialize<'a>,
        K: AsRef<str> + Sync,
        V: AsRef<str> + Sync,
    {
        let res = self
            .send_request(reqwest::Method::POST, path, params, Some(json_body))
            .await?;
        parse_response(res, false).await
    }

    /// 'delete' json data from the segment-defined path, e.g. `["api", "v1", "mixnodes"]`, with
    /// tuple defined key-value parameters, e.g. `[("since", "12345")]`. Attempt to parse the
    /// response into the provided type `T`.
    async fn delete_json<P, T, K, V>(
        &self,
        path: P,
        params: Params<'_, K, V>,
    ) -> Result<T, HttpClientError>
    where
        P: RequestPath + Send + Sync,
        for<'a> T: Deserialize<'a>,
        K: AsRef<str> + Sync,
        V: AsRef<str> + Sync,
    {
        let res = self
            .send_request(reqwest::Method::DELETE, path, params, None::<&()>)
            .await?;
        parse_response(res, false).await
    }

    /// 'patch' json data at the segment-defined path, e.g. `["api", "v1", "mixnodes"]`, with tuple
    /// defined key-value parameters, e.g. `[("since", "12345")]`. Attempt to parse the response
    /// into the provided type `T`.
    async fn patch_json<P, B, T, K, V>(
        &self,
        path: P,
        params: Params<'_, K, V>,
        json_body: &B,
    ) -> Result<T, HttpClientError>
    where
        P: RequestPath + Send + Sync,
        B: Serialize + ?Sized + Sync,
        for<'a> T: Deserialize<'a>,
        K: AsRef<str> + Sync,
        V: AsRef<str> + Sync,
    {
        let res = self
            .send_request(reqwest::Method::PATCH, path, params, Some(json_body))
            .await?;
        parse_response(res, false).await
    }

    /// `get` json data from the provided absolute endpoint, e.g. `"/api/v1/mixnodes?since=12345"`.
    /// Attempt to parse the response into the provided type `T`.
    async fn get_json_from<T, S>(&self, endpoint: S) -> Result<T, HttpClientError>
    where
        for<'a> T: Deserialize<'a>,
        S: AsRef<str> + Sync + Send,
    {
        let req = self.create_request_endpoint(reqwest::Method::GET, endpoint, None::<&()>)?;
        let res = self.send(req).await?;
        parse_response(res, false).await
    }

    /// `post` json data to the provided absolute endpoint, e.g. `"/api/v1/mixnodes?since=12345"`.
    /// Attempt to parse the response into the provided type `T`.
    async fn post_json_data_to<B, T, S>(
        &self,
        endpoint: S,
        json_body: &B,
    ) -> Result<T, HttpClientError>
    where
        B: Serialize + ?Sized + Sync,
        for<'a> T: Deserialize<'a>,
        S: AsRef<str> + Sync + Send,
    {
        let req = self.create_request_endpoint(reqwest::Method::POST, endpoint, Some(json_body))?;
        let res = self.send(req).await?;
        parse_response(res, false).await
    }

    /// `delete` json data from the provided absolute endpoint, e.g.
    /// `"/api/v1/mixnodes?since=12345"`. Attempt to parse the response into the provided type `T`.
    async fn delete_json_from<T, S>(&self, endpoint: S) -> Result<T, HttpClientError>
    where
        for<'a> T: Deserialize<'a>,
        S: AsRef<str> + Sync + Send,
    {
        let req = self.create_request_endpoint(reqwest::Method::DELETE, endpoint, None::<&()>)?;
        let res = self.send(req).await?;
        parse_response(res, false).await
    }

    /// `patch` json data at the provided absolute endpoint, e.g. `"/api/v1/mixnodes?since=12345"`.
    /// Attempt to parse the response into the provided type `T`.
    async fn patch_json_data_at<B, T, S>(
        &self,
        endpoint: S,
        json_body: &B,
    ) -> Result<T, HttpClientError>
    where
        B: Serialize + ?Sized + Sync,
        for<'a> T: Deserialize<'a>,
        S: AsRef<str> + Sync + Send,
    {
        let req =
            self.create_request_endpoint(reqwest::Method::PATCH, endpoint, Some(json_body))?;
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
    request_path: impl RequestPath,
    params: Params<'_, K, V>,
) -> Url {
    let mut url = base.clone();
    let mut path_segments = url
        .path_segments_mut()
        .expect("provided validator url does not have a base!");

    path_segments.pop_if_empty();

    for segment in request_path.to_sanitized_segments() {
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
pub async fn parse_response<T>(res: Response, allow_empty: bool) -> Result<T, HttpClientError>
where
    T: DeserializeOwned,
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

fn decode_as_json<T>(headers: &HeaderMap, content: Bytes) -> Result<T, HttpClientError>
where
    T: DeserializeOwned,
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

fn decode_as_bincode<T>(headers: &HeaderMap, content: Bytes) -> Result<T, HttpClientError>
where
    T: DeserializeOwned,
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

fn decode_raw_response<T>(headers: &HeaderMap, content: Bytes) -> Result<T, HttpClientError>
where
    T: DeserializeOwned,
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
mod tests;
