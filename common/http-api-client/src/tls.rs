// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Shared rustls crypto provider and client configuration.
//!
//! A binary can end up with more than one rustls crypto provider compiled in, and
//! `rustls::ClientConfig::builder()` panics when it cannot pick one. rustls users without an
//! explicit provider, reqwest among them, fall back to the process-wide default instead, so this
//! module installs ring as that default and builds explicit configurations from it.

use rustls::crypto::CryptoProvider;
use rustls::{ClientConfig, RootCertStore};
use std::sync::{Arc, LazyLock};

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

/// Installs ring as the process-wide default crypto provider unless one is already set.
pub fn install_default_crypto_provider() {
    if CryptoProvider::get_default().is_none() {
        // losing the race to another installer is fine; whichever won is the default
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
}

/// The process-wide default crypto provider, installing ring if none is set.
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
