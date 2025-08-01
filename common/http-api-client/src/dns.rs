// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! DNS resolver configuration for internal lookups.
//!
//! The resolver itself is the set combination of the google, cloudflare, and quad9 endpoints
//! supporting DoH and DoT.
//!
//! This resolver supports a fallback mechanism where, should the DNS-over-TLS resolution fail, a
//! followup resolution will be done using the hosts configured default (e.g. `/etc/resolve.conf` on
//! linux). This is disabled by default and can be enabled using [`enable_system_fallback`].
//!
//! Requires the `dns-over-https-rustls`, `webpki-roots` feature for the
//! `hickory-resolver` crate
//!
//!
//! Note: The hickory DoH resolver can cause warning logs about H2 connection failure. This
//! indicates that the long lived https connection was closed by the remote peer and the resolver
//! will have to reconnect. It should not impact actual functionality.
//!
//! code ref: https://github.com/hickory-dns/hickory-dns/blob/06a8b1ce9bd9322d8e6accf857d30257e1274427/crates/proto/src/h2/h2_client_stream.rs#L534
//!
//! example log:
//!
//! ```txt
//!   WARN /home/ubuntu/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hickory-proto-0.24.3/src/h2/h2_client_stream.rs:493: h2 connection failed: unexpected end of file
//! ```
#![deny(missing_docs)]

use crate::ClientBuilder;

use std::{
    net::SocketAddr,
    sync::{Arc, LazyLock},
};

use hickory_resolver::{
    config::{LookupIpStrategy, NameServerConfigGroup, ResolverConfig, ServerOrderingStrategy},
    lookup_ip::{LookupIp, LookupIpIntoIter},
    name_server::TokioConnectionProvider,
    ResolveError, TokioResolver,
};
use once_cell::sync::OnceCell;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use tracing::warn;

impl ClientBuilder {
    /// Override the DNS resolver implementation used by the underlying http client.
    pub fn dns_resolver<R: Resolve + 'static>(mut self, resolver: Arc<R>) -> Self {
        self.reqwest_client_builder = self.reqwest_client_builder.dns_resolver(resolver);
        self.use_secure_dns = false;
        self
    }

    /// Override the DNS resolver implementation used by the underlying http client.
    pub fn no_hickory_dns(mut self) -> Self {
        self.use_secure_dns = false;
        self
    }
}

struct SocketAddrs {
    iter: LookupIpIntoIter,
}

// n.b. static items do not call [`Drop`] on program termination, so this won't be deallocated.
// this is fine, as the OS can deallocate the terminated program faster than we can free memory
// but tools like valgrind might report "memory leaks" as it isn't obvious this is intentional.
static SHARED_RESOLVER: LazyLock<HickoryDnsResolver> = LazyLock::new(|| {
    tracing::debug!("Initializing shared DNS resolver");
    HickoryDnsResolver::default()
});

#[derive(Debug, thiserror::Error)]
#[error("hickory-dns resolver error: {hickory_error}")]
/// Error occurring while resolving a hostname into an IP address.
pub struct HickoryDnsError {
    #[from]
    hickory_error: ResolveError,
}

/// Wrapper around an `AsyncResolver`, which implements the `Resolve` trait.
///
/// Typical use involves instantiating using the `Default` implementation and then resolving using
/// methods or trait implementations.
///
/// The default initialization uses a shared underlying `AsyncResolver`. If a thread local resolver
/// is required use `thread_resolver()` to build a resolver with an independently instantiated
/// internal `AsyncResolver`.
#[derive(Debug, Default, Clone)]
pub struct HickoryDnsResolver {
    // Since we might not have been called in the context of a
    // Tokio Runtime in initialization, so we must delay the actual
    // construction of the resolver.
    state: Arc<OnceCell<TokioResolver>>,
    fallback: Option<Arc<OnceCell<TokioResolver>>>,
    dont_use_shared: bool,
}

impl Resolve for HickoryDnsResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let resolver = self.state.clone();
        let maybe_fallback = self.fallback.clone();
        let independent = self.dont_use_shared;
        Box::pin(async move {
            let resolver = resolver.get_or_try_init(|| {
                // using a closure here is slightly gross, but this makes sure that if the
                // lazy-init returns an error it can be handled by the client
                if independent {
                    new_resolver()
                } else {
                    Ok(SHARED_RESOLVER.state.get_or_try_init(new_resolver)?.clone())
                }
            })?;

            // try the primary DNS resolver that we set up (DoH or DoT or whatever)
            let lookup = match resolver.lookup_ip(name.as_str()).await {
                Ok(res) => res,
                Err(e) => {
                    if let Some(ref fallback) = maybe_fallback {
                        // on failure use the fall back system configured DNS resolver
                        if !e.is_no_records_found() {
                            warn!("primary DNS failed w/ error {e}: using system fallback");
                        }
                        let resolver = fallback.get_or_try_init(|| {
                            // using a closure here is slightly gross, but this makes sure that if the
                            // lazy-init returns an error it can be handled by the client
                            if independent {
                                new_resolver_system()
                            } else {
                                Ok(SHARED_RESOLVER
                                    .fallback
                                    .as_ref()
                                    .ok_or(e)? // if the shared resolver has no fallback return the original error
                                    .get_or_try_init(new_resolver_system)?
                                    .clone())
                            }
                        })?;

                        resolver.lookup_ip(name.as_str()).await?
                    } else {
                        return Err(e.into());
                    }
                }
            };

            let addrs: Addrs = Box::new(SocketAddrs {
                iter: lookup.into_iter(),
            });
            Ok(addrs)
        })
    }
}

impl Iterator for SocketAddrs {
    type Item = SocketAddr;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|ip_addr| SocketAddr::new(ip_addr, 0))
    }
}

impl HickoryDnsResolver {
    /// Attempt to resolve a domain name to a set of ['IpAddr']s
    pub async fn resolve_str(&self, name: &str) -> Result<LookupIp, HickoryDnsError> {
        let resolver = self.state.get_or_try_init(|| self.new_resolver())?;

        // try the primary DNS resolver that we set up (DoH or DoT or whatever)
        let lookup = match resolver.lookup_ip(name).await {
            Ok(res) => res,
            Err(e) => {
                if let Some(ref fallback) = self.fallback {
                    // on failure use the fall back system configured DNS resolver
                    if !e.is_no_records_found() {
                        warn!("primary DNS failed w/ error {e}: using system fallback");
                    }

                    let resolver = fallback.get_or_try_init(|| self.new_resolver_system())?;
                    resolver.lookup_ip(name).await?
                } else {
                    return Err(e.into());
                }
            }
        };

        Ok(lookup)
    }

    /// Create a (lazy-initialized) resolver that is not shared across threads.
    pub fn thread_resolver() -> Self {
        Self {
            dont_use_shared: true,
            ..Default::default()
        }
    }

    fn new_resolver(&self) -> Result<TokioResolver, HickoryDnsError> {
        if self.dont_use_shared {
            new_resolver()
        } else {
            Ok(SHARED_RESOLVER.state.get_or_try_init(new_resolver)?.clone())
        }
    }

    fn new_resolver_system(&self) -> Result<TokioResolver, HickoryDnsError> {
        if self.dont_use_shared || SHARED_RESOLVER.fallback.is_none() {
            new_resolver_system()
        } else {
            Ok(SHARED_RESOLVER
                .fallback
                .as_ref()
                .unwrap()
                .get_or_try_init(new_resolver_system)?
                .clone())
        }
    }

    /// Enable fallback to the system default resolver if the primary (DoX) resolver fails
    pub fn enable_system_fallback(&mut self) -> Result<(), HickoryDnsError> {
        self.fallback = Some(Default::default());
        let _ = self
            .fallback
            .as_ref()
            .unwrap()
            .get_or_try_init(new_resolver_system)?;
        Ok(())
    }

    /// Disable fallback resolution. If the primary resolver fails the error is
    /// returned immediately
    pub fn disable_system_fallback(&mut self) {
        self.fallback = None;
    }
}

/// Create a new resolver with a custom DoT based configuration. The options are overridden to look
/// up for both IPv4 and IPv6 addresses to work with "happy eyeballs" algorithm.
fn new_resolver() -> Result<TokioResolver, HickoryDnsError> {
    let mut name_servers = NameServerConfigGroup::quad9_tls();
    name_servers.merge(NameServerConfigGroup::quad9_https());
    name_servers.merge(NameServerConfigGroup::cloudflare_tls());
    name_servers.merge(NameServerConfigGroup::cloudflare_https());

    let config = ResolverConfig::from_parts(None, Vec::new(), name_servers);
    let mut resolver_builder =
        TokioResolver::builder_with_config(config, TokioConnectionProvider::default());

    resolver_builder.options_mut().ip_strategy = LookupIpStrategy::Ipv4AndIpv6;
    resolver_builder.options_mut().server_ordering_strategy = ServerOrderingStrategy::RoundRobin;

    Ok(resolver_builder.build())
}

/// Create a new resolver with the default configuration, which reads from the system DNS config
/// (i.e. `/etc/resolve.conf` in unix). The options are overridden to look up for both IPv4 and IPv6
/// addresses to work with "happy eyeballs" algorithm.
fn new_resolver_system() -> Result<TokioResolver, HickoryDnsError> {
    let mut resolver_builder = TokioResolver::builder_tokio()?;
    resolver_builder.options_mut().ip_strategy = LookupIpStrategy::Ipv4AndIpv6;

    Ok(resolver_builder.build())
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn reqwest_hickory_doh() {
        let resolver = HickoryDnsResolver::default();
        let client = reqwest::ClientBuilder::new()
            .dns_resolver(resolver.into())
            .build()
            .unwrap();

        let resp = client
            .get("http://ifconfig.me:80")
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();

        assert!(!resp.is_empty());
    }

    #[tokio::test]
    async fn dns_lookup() -> Result<(), HickoryDnsError> {
        let resolver = HickoryDnsResolver::default();

        let domain = "ifconfig.me";
        let addrs = resolver.resolve_str(domain).await?;

        assert!(addrs.into_iter().next().is_some());

        Ok(())
    }
}
