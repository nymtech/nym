// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Shared rustls crypto provider and client configuration.
//!
//! A binary can end up with more than one rustls crypto provider compiled in, and
//! `rustls::ClientConfig::builder()` panics when it cannot pick one. rustls users without an
//! explicit provider, reqwest among them, fall back to the process-wide default instead, so this
//! module installs the provider selected by the `tls-ring` or `tls-aws-lc-rs` feature as that
//! default and builds explicit configurations from it. ring wins when both features are enabled.

use rustls::crypto::CryptoProvider;
use rustls::{ClientConfig, RootCertStore};
use std::sync::{Arc, LazyLock};

#[cfg(not(any(feature = "tls-ring", feature = "tls-aws-lc-rs")))]
compile_error!(
    "nym-http-api-client needs a rustls crypto provider: enable the `tls-ring` or `tls-aws-lc-rs` feature"
);

static CLIENT_CONFIG: LazyLock<Arc<ClientConfig>> = LazyLock::new(|| {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder_with_provider(crypto_provider())
        .with_safe_default_protocol_versions()
        .expect("the crypto provider supports the default TLS protocol versions")
        .with_root_certificates(roots)
        .with_no_client_auth();
    Arc::new(config)
});

#[cfg(feature = "tls-ring")]
fn selected_provider() -> CryptoProvider {
    rustls::crypto::ring::default_provider()
}

#[cfg(all(feature = "tls-aws-lc-rs", not(feature = "tls-ring")))]
fn selected_provider() -> CryptoProvider {
    rustls::crypto::aws_lc_rs::default_provider()
}

/// Installs the feature-selected provider as the process-wide default unless one is already set.
pub fn install_default_crypto_provider() {
    if CryptoProvider::get_default().is_none() {
        // losing the race to another installer is fine; whichever won is the default
        let _ = selected_provider().install_default();
    }
}

/// The process-wide default crypto provider, installing the feature-selected one if none is set.
pub fn crypto_provider() -> Arc<CryptoProvider> {
    install_default_crypto_provider();
    CryptoProvider::get_default()
        .expect("a default crypto provider has just been installed")
        .clone()
}

/// Client configuration trusting the Mozilla root certificates.
pub fn rustls_client_config() -> Arc<ClientConfig> {
    CLIENT_CONFIG.clone()
}
