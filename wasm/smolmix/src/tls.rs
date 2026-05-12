// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>

//! TLS connector using futures-rustls (futures::io traits, NOT tokio).
//!
//! The crypto provider is selected at compile time via feature flags:
//! - `ring-crypto` (default): uses ring 0.17, has experimental wasm32 support
//! - `rustcrypto`: uses rustls-rustcrypto, pure Rust, guaranteed wasm32 compat
//!
//! Both produce identical `ClientConfig`; only the underlying crypto differs.

use std::sync::{Arc, OnceLock};

use futures::io::{AsyncRead, AsyncWrite};
use futures_rustls::TlsConnector;
use rustls::pki_types::ServerName;
use rustls::ClientConfig;

use crate::error::FetchError;

/// Cached TLS client config: built once, reused for all connections.
static TLS_CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();

// Ensure at least one crypto provider is selected at compile time.
#[cfg(not(any(feature = "ring-crypto", feature = "rustcrypto")))]
compile_error!("enable either the 'ring-crypto' or 'rustcrypto' feature for TLS support");

/// Perform a TLS handshake over the given stream.
///
/// Returns a TLS-wrapped stream that implements `futures::io::{AsyncRead, AsyncWrite}`.
/// The stream type is generic; works with `WasmTcpStream` directly.
pub async fn connect<S>(
    stream: S,
    hostname: &str,
) -> Result<futures_rustls::client::TlsStream<S>, FetchError>
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

    let provider = crypto_provider();

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

/// Select the crypto provider based on the enabled feature flag.
///
/// ring-crypto takes priority if both features are somehow enabled.
fn crypto_provider() -> Arc<rustls::crypto::CryptoProvider> {
    #[cfg(feature = "ring-crypto")]
    return Arc::new(rustls::crypto::ring::default_provider());

    #[cfg(all(feature = "rustcrypto", not(feature = "ring-crypto")))]
    return Arc::new(rustls_rustcrypto::provider());
}
