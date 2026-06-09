// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

#![doc = include_str!("../README.md")]

//! Shared TLS configuration for smolmix tunneled connections.
//!
//! # Why a separate TLS crate?
//!
//! Every protocol that needs encryption over smolmix (HTTPS, WebSocket, etc.)
//! requires the same setup: build a `ClientConfig` with webpki root
//! certificates, wrap it in a `TlsConnector`. Rather than duplicating this
//! in every crate, `smolmix-tls` provides a single source of truth.
//!
//! The crate is deliberately minimal — 60 lines of pure configuration, no
//! trait impls needed. `tokio-rustls` works directly with anything that
//! implements tokio's `AsyncRead + AsyncWrite`, which `smolmix::TcpStream`
//! does out of the box.
//!
//! # Usage patterns
//!
//! ```no_run
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let tunnel = smolmix::Tunnel::new().await?;
//! // One-shot: creates a fresh connector internally.
//! // Simple but rebuilds the root cert store each time.
//! let tcp = tunnel.tcp_connect("93.184.216.34:443".parse()?).await?;
//! let tls = smolmix_tls::connect(tcp, "example.com").await?;
//!
//! // Reusable: create a connector once, use for many connections.
//! // The TlsConnector wraps an Arc<ClientConfig> — cloning is cheap.
//! let connector = smolmix_tls::connector();
//! let tcp1 = tunnel.tcp_connect("1.1.1.1:443".parse()?).await?;
//! let stream1 = smolmix_tls::connect_with(&connector, tcp1, "one.one.one.one").await?;
//! # Ok(())
//! # }
//! ```
//!
//! # What's inside
//!
//! - [`connector()`] — builds a `TlsConnector` with Mozilla's root CA bundle
//!   ([`webpki-roots`](https://docs.rs/webpki-roots))
//! - [`connect()`] — one-shot TLS handshake (convenience, creates connector internally)
//! - [`connect_with()`] — TLS handshake using a pre-built connector (preferred for repeated use)
//! - Re-exports [`TlsStream`] and [`TlsConnector`] so downstream code doesn't
//!   need `tokio-rustls` in its `Cargo.toml`
//!
//! # Security
//!
//! The connector uses rustls with the standard webpki root certificates and
//! no client authentication. SNI (Server Name Indication) is set from the
//! hostname you pass to `connect`/`connect_with`. There is no way to disable
//! certificate verification — this is intentional.

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
