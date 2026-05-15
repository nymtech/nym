// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

#![doc = include_str!("../README.md")]

//! HTTP client routing all traffic through the Nym mixnet.
//!
//! # What this crate does
//!
//! Wraps [hyper-util]'s legacy `Client` with a [`SmolmixConnector`] that
//! routes DNS resolution, TCP connections, and TLS handshakes through a
//! smolmix [`Tunnel`]. From the outside it behaves like a normal HTTP client,
//! but all traffic travels through the mixnet.
//!
//! # How it works
//!
//! hyper-util's `Client` uses the [`tower::Service<Uri>`] trait to open
//! connections. [`SmolmixConnector`] implements this: given a URI, it resolves
//! the hostname via [`smolmix_dns`], connects TCP via the tunnel, and wraps
//! in TLS for `https://` URIs:
//!
//! ```text
//! SmolmixConnector::call(uri)
//!   → resolver.resolve(host, port)        DNS through tunnel (cached)
//!   → tunnel.tcp_connect(addr)            TCP through mixnet
//!   → smolmix_tls::connect_with(tls, tcp, host)   TLS if https
//!   → MaybeTlsStream::Plain { TcpStream }
//!     or MaybeTlsStream::Tls { TlsStream<TcpStream> }
//!   → TokioIo<MaybeTlsStream>   (implements hyper's Read/Write/Connection)
//! ```
//!
//! [`MaybeTlsStream`] is a two-variant enum with [`pin_project_lite`] for safe
//! pin projection through `AsyncRead`/`AsyncWrite`. hyper-util's `TokioIo`
//! wraps it to satisfy hyper's own I/O traits.
//!
//! # Quick start (GET)
//!
//! ```no_run
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use smolmix_hyper::{Client, Request, EmptyBody, BodyExt};
//! use bytes::Bytes;
//!
//! let tunnel = smolmix::Tunnel::new().await?;
//! let client = Client::new(&tunnel);
//!
//! let req = Request::get("https://example.com")
//!     .header("Host", "example.com")
//!     .body(EmptyBody::<Bytes>::new())?;
//! let resp = client.request(req).await?;
//! let body = resp.into_body().collect().await?.to_bytes();
//! println!("{}", String::from_utf8_lossy(&body));
//! # Ok(())
//! # }
//! ```
//!
//! # Sending request bodies (POST, PUT, etc.)
//!
//! The convenience [`Client`] wrapper uses `Empty<Bytes>` as its body type,
//! which is suitable for GET/HEAD/DELETE. For requests that carry a body,
//! construct a hyper-util client directly with [`SmolmixConnector`]:
//!
//! ```no_run
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use http_body_util::Full;
//! use hyper_util::{client::legacy, rt::TokioExecutor};
//! use bytes::Bytes;
//! use smolmix_hyper::SmolmixConnector;
//!
//! let tunnel = smolmix::Tunnel::new().await?;
//! let connector = SmolmixConnector::new(&tunnel);
//! let client = legacy::Client::builder(TokioExecutor::new())
//!     .build::<_, Full<Bytes>>(connector);
//!
//! let body = Full::new(Bytes::from(r#"{"key": "value"}"#));
//! let req = hyper::Request::post("https://httpbin.org/post")
//!     .header("Host", "httpbin.org")
//!     .header("Content-Type", "application/json")
//!     .body(body)?;
//! let resp = client.request(req).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Re-exports
//!
//! This crate re-exports the most commonly needed types so you don't need
//! `hyper`, `http-body-util`, or `bytes` in your `Cargo.toml` for basic use:
//!
//! - [`Request`], [`Response`], [`StatusCode`], [`Uri`] — from hyper
//! - [`BodyExt`], [`EmptyBody`] — from http-body-util
//! - [`bytes`] — the bytes crate
//!
//! [hyper-util]: https://docs.rs/hyper-util
//! [`pin_project_lite`]: https://docs.rs/pin-project-lite

mod connector;
mod tls_stream;

use std::ops::Deref;

use bytes::Bytes;
use http_body_util::Empty;
use hyper_util::client::legacy;
use hyper_util::rt::TokioExecutor;

use smolmix::Tunnel;

/// Re-exported [`bytes`](https://docs.rs/bytes) crate for constructing request bodies.
pub use bytes;

/// Extension trait for consuming HTTP response bodies. Provides `.collect()`,
/// `.frame()`, etc.
pub use http_body_util::BodyExt;

/// An empty HTTP body. Use `EmptyBody::<Bytes>::new()` for GET/HEAD requests.
pub use http_body_util::Empty as EmptyBody;

/// Re-exported hyper types for building requests without depending on hyper directly.
pub use hyper::{Request, Response, StatusCode, Uri};

pub use connector::SmolmixConnector;
pub use tls_stream::MaybeTlsStream;

/// Inner hyper-util client type alias for readability.
type HyperClient = legacy::Client<SmolmixConnector, Empty<Bytes>>;

/// An HTTP client that routes all traffic through a smolmix [`Tunnel`].
///
/// Wraps a hyper-util `Client` and exposes its full API via [`Deref`]. DNS
/// resolution, TCP connections, and TLS all travel through the mixnet.
///
/// The body type is [`Empty<Bytes>`], suitable for GET requests. For requests
/// that carry a body, construct a [`SmolmixConnector`] directly and pass it
/// to [`hyper_util::client::legacy::Client::builder`].
pub struct Client {
    inner: HyperClient,
}

impl Client {
    /// Create a new HTTP client for the given tunnel.
    pub fn new(tunnel: &Tunnel) -> Self {
        let connector = SmolmixConnector::new(tunnel);
        Self {
            inner: legacy::Client::builder(TokioExecutor::new()).build(connector),
        }
    }
}

impl Deref for Client {
    type Target = HyperClient;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Create a hyper-util [`Client`] that routes all traffic through the tunnel.
///
/// Equivalent to [`Client::new()`].
pub fn client(tunnel: &Tunnel) -> Client {
    Client::new(tunnel)
}
