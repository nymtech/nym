// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

//! HTTP client routing all traffic through the Nym mixnet.
//!
//! This crate wraps [hyper-util]'s `Client` with a newtype [`Client`] that
//! routes DNS, TCP, and TLS through a smolmix [`Tunnel`]. It feels like a
//! drop-in replacement for `hyper_util::client::legacy::Client`.
//!
//! # Quick start
//!
//! ```ignore
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
//! ```

mod connector;
pub mod tls_stream;

use std::ops::Deref;

use bytes::Bytes;
use http_body_util::Empty;
use hyper_util::client::legacy;
use hyper_util::rt::TokioExecutor;

use smolmix::Tunnel;

// Re-exports so users don't need hyper/http-body-util/bytes in their Cargo.toml
pub use bytes;
pub use http_body_util::{BodyExt, Empty as EmptyBody};
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
