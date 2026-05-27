// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! TLS connector using futures-rustls (futures::io traits, NOT tokio).
//!
//! Crypto provider: rustls-rustcrypto (RustCrypto-backed pure-Rust primitives).
//!

use std::io;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};
use std::task::{Context, Poll};

use futures::io::{AsyncRead, AsyncWrite};
use futures_rustls::TlsConnector;
use rustls::pki_types::ServerName;
use rustls::{CipherSuite, ClientConfig};

use crate::error::FetchError;

/// Cached TLS client config: built once, reused for all connections.
static TLS_CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();

/// TLS stream wrapper that tolerates a missing `close_notify` from the peer.
///
/// rustls reports a peer closing the underlying TCP connection without sending
/// the TLS `close_notify` alert as `io::ErrorKind::UnexpectedEof`. Many CDNs
/// (and older servers) do this routinely, and hyper then surfaces it as a body
/// framing error even when the HTTP response was completely received per its
/// Content-Length / chunked terminator.
///
/// This wrapper translates `UnexpectedEof` on `poll_read` to a clean `Ok(0)`
/// (EOF). hyper's body framing is authoritative for whether the message is
/// complete — if it isn't, hyper will report truncation on its own terms. The
/// only attack surface this opens is for HTTP/1.0-style "read to EOF" bodies,
/// which were already truncatable and which modern frameworks don't use.
///
/// Writes pass through unchanged.
pub struct MaybeCloseNotify<S> {
    inner: S,
}

impl<S> MaybeCloseNotify<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for MaybeCloseNotify<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        match Pin::new(&mut self.inner).poll_read(cx, buf) {
            Poll::Ready(Err(e)) if e.kind() == io::ErrorKind::UnexpectedEof => {
                crate::util::debug_log!("[tls] peer closed without close_notify, treating as EOF");
                Poll::Ready(Ok(0))
            }
            other => other,
        }
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for MaybeCloseNotify<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_close(cx)
    }
}

/// Perform a TLS handshake over the given stream.
///
/// Returns a `MaybeCloseNotify`-wrapped TLS stream so that peers omitting
/// the TLS `close_notify` shutdown alert don't cause spurious body-framing
/// errors at the hyper layer.
pub async fn connect<S>(
    stream: S,
    hostname: &str,
) -> Result<MaybeCloseNotify<futures_rustls::client::TlsStream<S>>, FetchError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let config = make_client_config()?;
    let connector = TlsConnector::from(config);

    // ServerName::try_from(String) gives ServerName<'static> (owned),
    // which is what futures-rustls::TlsConnector::connect requires.
    let server_name = ServerName::try_from(hostname.to_string())
        .map_err(|e| FetchError::Dns(format!("invalid TLS server name '{hostname}': {e}")))?;

    let result = connector
        .connect(server_name, stream)
        .await
        .map(MaybeCloseNotify::new)
        .map_err(FetchError::Io);

    if let Err(e) = &result {
        crate::util::debug_error!("[tls] handshake FAILED with '{hostname}': {e}");
    }

    result
}

/// Get or build the cached rustls ClientConfig with the webpki-roots CA bundle.
///
/// The config (crypto provider, root CA store, protocol versions) is identical
/// for every connection, so we build it once and reuse the `Arc<ClientConfig>`.
fn make_client_config() -> Result<Arc<ClientConfig>, FetchError> {
    if let Some(config) = TLS_CONFIG.get() {
        return Ok(config.clone());
    }

    // Restrict cipher suites to only what is explicity implemented as
    // per https://github.com/RustCrypto/rustls-rustcrypto#rustls-rustcrypto.
    let mut provider = rustls_rustcrypto::provider();
    provider.cipher_suites.retain(|s| {
        matches!(
            s.suite(),
            CipherSuite::TLS13_AES_128_GCM_SHA256
                | CipherSuite::TLS13_AES_256_GCM_SHA384
                | CipherSuite::TLS13_CHACHA20_POLY1305_SHA256
                | CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256
                | CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384
                | CipherSuite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256
                | CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256
                | CipherSuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384
                | CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256
        )
    });
    let provider = Arc::new(provider);

    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let mut config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| FetchError::Http(format!("TLS config error: {e}")))?
        .with_root_certificates(root_store)
        .with_no_client_auth();

    // ALPN: advertise HTTP/1.1 so CDNs (GitHub, Cloudflare) that require
    // protocol negotiation don't abort the handshake with an EOF.
    config.alpn_protocols = vec![b"http/1.1".to_vec()];

    // Disable session resumption: TLS session tickets and PSK identities are
    // long-lived correlators a server can use to link separate mixnet circuits
    // back to the same client, defeating per-request unlinkability.
    config.resumption = rustls::client::Resumption::disabled();

    let config = Arc::new(config);
    Ok(TLS_CONFIG.get_or_init(|| config.clone()).clone())
}
