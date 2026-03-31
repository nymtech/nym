// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

//! DNS resolution through the Nym mixnet.
//!
//! This crate wraps [hickory-resolver] with a newtype [`Resolver`] that routes
//! all DNS traffic through a smolmix [`Tunnel`], ensuring hostnames are resolved
//! via the mixnet rather than leaking queries to the local network.
//!
//! # Quick start
//!
//! ```ignore
//! use smolmix_dns::Resolver;
//!
//! let tunnel = smolmix::Tunnel::new().await?;
//! let resolver = Resolver::new(&tunnel);
//!
//! // Full hickory API via Deref:
//! let lookup = resolver.lookup_ip("example.com").await?;
//! for ip in lookup.iter() { println!("{ip}"); }
//!
//! // Convenience one-shot:
//! let addrs = resolver.resolve("example.com", 443).await?;
//! ```

mod provider;

use std::io;
use std::net::SocketAddr;
use std::ops::Deref;

use hickory_resolver::name_server::GenericConnector;

use hickory_proto::runtime::TokioHandle;
use smolmix::Tunnel;

// Re-exports so users don't need hickory-resolver in their Cargo.toml
pub use hickory_resolver::config::ResolverConfig;
pub use hickory_resolver::lookup_ip::LookupIp;
pub use hickory_resolver::ResolveError;
pub use provider::SmolmixRuntimeProvider;

/// Inner resolver type alias for readability.
type HickoryResolver = hickory_resolver::Resolver<GenericConnector<SmolmixRuntimeProvider>>;

/// A DNS resolver that routes all queries through a smolmix [`Tunnel`].
///
/// Wraps a hickory-resolver `Resolver` and exposes its full API via [`Deref`].
/// All DNS traffic (both TCP and UDP) travels through the mixnet.
pub struct Resolver {
    inner: HickoryResolver,
}

impl Resolver {
    /// Create a resolver using Cloudflare (`1.1.1.1`) as upstream DNS.
    pub fn new(tunnel: &Tunnel) -> Self {
        Self::with_config(tunnel, ResolverConfig::cloudflare())
    }

    /// Create a resolver with a custom upstream DNS configuration.
    pub fn with_config(tunnel: &Tunnel, config: ResolverConfig) -> Self {
        let provider = SmolmixRuntimeProvider {
            tunnel: tunnel.clone(),
            handle: TokioHandle::default(),
        };
        let connector = GenericConnector::new(provider);
        Self {
            inner: hickory_resolver::Resolver::builder_with_config(config, connector).build(),
        }
    }

    /// Resolve a hostname to socket addresses through the tunnel.
    ///
    /// Convenience method for one-shot lookups. Returns all resolved addresses
    /// paired with the given `port`.
    pub async fn resolve(&self, host: &str, port: u16) -> io::Result<Vec<SocketAddr>> {
        let lookup = self
            .inner
            .lookup_ip(host)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(lookup.iter().map(|ip| SocketAddr::new(ip, port)).collect())
    }
}

impl Deref for Resolver {
    type Target = HickoryResolver;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Create a hickory [`Resolver`] that routes all DNS through the tunnel.
///
/// Uses Cloudflare (`1.1.1.1`) as the upstream DNS server. Equivalent to
/// [`Resolver::new()`].
pub fn resolver(tunnel: &Tunnel) -> Resolver {
    Resolver::new(tunnel)
}

/// Resolve a hostname through the tunnel.
///
/// Convenience wrapper for one-shot lookups. Equivalent to
/// `Resolver::new(tunnel).resolve(host, port)`.
pub async fn resolve(tunnel: &Tunnel, host: &str, port: u16) -> io::Result<Vec<SocketAddr>> {
    let r = resolver(tunnel);
    r.resolve(host, port).await
}
