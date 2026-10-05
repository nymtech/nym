// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Shared rustls client configuration.
//!
//! A binary can end up with more than one rustls crypto provider compiled in, and
//! `rustls::ClientConfig::builder()` panics when it cannot pick one. TLS clients in the
//! workspace therefore build their configuration from the explicit provider exposed here.

use rustls::crypto::CryptoProvider;
use rustls::{ClientConfig, RootCertStore};
use std::sync::{Arc, LazyLock};

static PROVIDER: LazyLock<Arc<CryptoProvider>> =
    LazyLock::new(|| Arc::new(rustls::crypto::ring::default_provider()));

static CLIENT_CONFIG: LazyLock<Arc<ClientConfig>> = LazyLock::new(|| {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder_with_provider(crypto_provider())
        .with_safe_default_protocol_versions()
        .expect("the ring provider supports the default TLS protocol versions")
        .with_root_certificates(roots)
        .with_no_client_auth();
    Arc::new(config)
});

/// The crypto provider backing every TLS client in the workspace.
pub fn crypto_provider() -> Arc<CryptoProvider> {
    PROVIDER.clone()
}

/// Client configuration trusting the Mozilla root certificates.
pub fn rustls_client_config() -> Arc<ClientConfig> {
    CLIENT_CONFIG.clone()
}
