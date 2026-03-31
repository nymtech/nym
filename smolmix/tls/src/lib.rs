// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

//! Shared TLS setup for smolmix tunneled connections.
//!
//! Provides a pre-configured [`TlsConnector`] with webpki root certificates
//! and convenience functions for TLS over [`TcpStream`].
//!
//! # Quick start
//!
//! ```ignore
//! // One-shot: TLS handshake over an existing TCP stream.
//! let tls_stream = smolmix_tls::connect(tcp, "example.com").await?;
//!
//! // Reusable: create a connector once, use it for many connections.
//! let tls = smolmix_tls::connector();
//! let stream1 = smolmix_tls::connect_with(&tls, tcp1, "a.com").await?;
//! let stream2 = smolmix_tls::connect_with(&tls, tcp2, "b.com").await?;
//! ```

use std::io;
use std::sync::Arc;

use rustls::pki_types::ServerName;
use tokio_smoltcp::TcpStream;

pub use tokio_rustls::client::TlsStream;
pub use tokio_rustls::TlsConnector;

/// Create a [`TlsConnector`] configured with the standard webpki root certificates.
///
/// The returned connector can be cloned cheaply (it wraps an `Arc<ClientConfig>`)
/// and reused across many connections.
pub fn connector() -> TlsConnector {
    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    TlsConnector::from(Arc::new(config))
}

/// Perform a TLS handshake over an existing TCP stream.
///
/// Creates a fresh [`TlsConnector`] with webpki roots. For repeated connections,
/// prefer [`connect_with()`] to avoid rebuilding the root store each time.
pub async fn connect(tcp: TcpStream, hostname: &str) -> io::Result<TlsStream<TcpStream>> {
    connect_with(&connector(), tcp, hostname).await
}

/// Perform a TLS handshake using a pre-built connector.
///
/// Extracts the SNI hostname from `hostname` and connects.
pub async fn connect_with(
    tls: &TlsConnector,
    tcp: TcpStream,
    hostname: &str,
) -> io::Result<TlsStream<TcpStream>> {
    let domain = ServerName::try_from(hostname.to_owned())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    tls.connect(domain, tcp).await
}
