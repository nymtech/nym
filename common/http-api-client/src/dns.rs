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
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::{Arc, LazyLock},
    time::Duration,
};

use hickory_resolver::{
    TokioResolver,
    config::{LookupIpStrategy, NameServerConfigGroup, ResolverConfig},
    lookup_ip::LookupIpIntoIter,
    name_server::TokioConnectionProvider,
};
use once_cell::sync::OnceCell;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use tracing::*;

mod constants;
mod static_resolver;
pub use static_resolver::*;

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

// n.b. static items do not call [`Drop`] on program termination, so this won't be deallocated.
// this is fine, as the OS can deallocate the terminated program faster than we can free memory
// but tools like valgrind might report "memory leaks" as it isn't obvious this is intentional.
static SHARED_RESOLVER: LazyLock<HickoryDnsResolver> = LazyLock::new(|| {
    tracing::debug!("Initializing shared DNS resolver");
    HickoryDnsResolver::default()
});

#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
/// Error occurring while resolving a hostname into an IP address.
pub enum ResolveError {
    #[error("invalid name: {0}")]
    InvalidNameError(String),
    #[error("hickory-dns resolver error: {0}")]
    ResolveError(#[from] hickory_resolver::ResolveError),
    #[error("high level lookup timed out")]
    Timeout,
    #[error("hostname not found in static lookup table")]
    StaticLookupMiss,
}

/// Wrapper around an `AsyncResolver`, which implements the `Resolve` trait.
///
/// Typical use involves instantiating using the `Default` implementation and then resolving using
/// methods or trait implementations.
///
/// The default initialization uses a shared underlying `AsyncResolver`. If a thread local resolver
/// is required use `thread_resolver()` to build a resolver with an independently instantiated
/// internal `AsyncResolver`.
#[derive(Debug, Clone)]
pub struct HickoryDnsResolver {
    // Since we might not have been called in the context of a
    // Tokio Runtime in initialization, so we must delay the actual
    // construction of the resolver.
    state: Arc<OnceCell<TokioResolver>>,
    fallback: Option<Arc<OnceCell<TokioResolver>>>,
    static_base: Option<Arc<OnceCell<StaticResolver>>>,
    dont_use_shared: bool,
    /// Overall timeout for dns lookup associated with any individual host resolution. For example,
    /// use of retries, server_ordering_strategy, etc. ends absolutely if this timeout is reached.
    overall_dns_timeout: Duration,
}

impl Default for HickoryDnsResolver {
    fn default() -> Self {
        Self {
            state: Default::default(),
            fallback: Default::default(),
            static_base: Default::default(),
            dont_use_shared: Default::default(),
            overall_dns_timeout: Duration::from_secs(10),
        }
    }
}

impl Resolve for HickoryDnsResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let resolver = self.state.clone();
        let maybe_fallback = self.fallback.clone();
        let maybe_static = self.static_base.clone();
        let independent = self.dont_use_shared;
        let overall_dns_timeout = self.overall_dns_timeout;
        Box::pin(async move {
            resolve(
                name,
                resolver,
                maybe_fallback,
                maybe_static,
                independent,
                overall_dns_timeout,
            )
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        })
    }
}

async fn resolve(
    name: Name,
    resolver: Arc<OnceCell<TokioResolver>>,
    maybe_fallback: Option<Arc<OnceCell<TokioResolver>>>,
    maybe_static: Option<Arc<OnceCell<StaticResolver>>>,
    independent: bool,
    overall_dns_timeout: Duration,
) -> Result<Addrs, ResolveError> {
    let resolver = resolver.get_or_try_init(|| HickoryDnsResolver::new_resolver(independent))?;

    // Attempt a lookup using the primary resolver
    let resolve_fut = tokio::time::timeout(overall_dns_timeout, resolver.lookup_ip(name.as_str()));
    let primary_err = match resolve_fut.await {
        Err(_) => ResolveError::Timeout,
        Ok(Ok(lookup)) => {
            let addrs: Addrs = Box::new(SocketAddrs {
                iter: lookup.into_iter(),
            });
            return Ok(addrs);
        }
        Ok(Err(e)) => {
            // on failure use the fall back system configured DNS resolver
            if !e.is_no_records_found() {
                warn!("primary DNS failed w/ error: {e}");
            }
            e.into()
        }
    };

    // If the primary resolver encountered an error, attempt a lookup using the fallback
    // resolver if one is configured.
    if let Some(ref fallback) = maybe_fallback {
        let resolver =
            fallback.get_or_try_init(|| HickoryDnsResolver::new_resolver_system(independent))?;

        let resolve_fut =
            tokio::time::timeout(overall_dns_timeout, resolver.lookup_ip(name.as_str()));
        if let Ok(Ok(lookup)) = resolve_fut.await {
            let addrs: Addrs = Box::new(SocketAddrs {
                iter: lookup.into_iter(),
            });
            return Ok(addrs);
        }
    }

    // If no record has been found and a static map of fallback addresses is configured
    // check the table for our entry
    if let Some(ref static_resolver) = maybe_static {
        debug!("checking static");
        let resolver =
            static_resolver.get_or_init(|| HickoryDnsResolver::new_static_fallback(independent));

        if let Ok(addrs) = resolver.resolve(name).await {
            return Ok(addrs);
        }
    }

    Err(primary_err)
}

struct SocketAddrs {
    iter: LookupIpIntoIter,
}

impl Iterator for SocketAddrs {
    type Item = SocketAddr;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|ip_addr| SocketAddr::new(ip_addr, 0))
    }
}

impl HickoryDnsResolver {
    /// Attempt to resolve a domain name to a set of ['IpAddr']s
    pub async fn resolve_str(
        &self,
        name: &str,
    ) -> Result<impl Iterator<Item = IpAddr> + use<>, ResolveError> {
        let n =
            Name::from_str(name).map_err(|_| ResolveError::InvalidNameError(name.to_string()))?;
        resolve(
            n,
            self.state.clone(),
            self.fallback.clone(),
            self.static_base.clone(),
            self.dont_use_shared,
            self.overall_dns_timeout,
        )
        .await
        .map(|addrs| addrs.map(|socket_addr| socket_addr.ip()))
    }

    /// Create a (lazy-initialized) resolver that is not shared across threads.
    pub fn thread_resolver() -> Self {
        Self {
            dont_use_shared: true,
            ..Default::default()
        }
    }

    fn new_resolver(dont_use_shared: bool) -> Result<TokioResolver, ResolveError> {
        // using a closure here is slightly gross, but this makes sure that if the
        // lazy-init returns an error it can be handled by the client
        if dont_use_shared {
            new_resolver()
        } else {
            Ok(SHARED_RESOLVER.state.get_or_try_init(new_resolver)?.clone())
        }
    }

    fn new_resolver_system(dont_use_shared: bool) -> Result<TokioResolver, ResolveError> {
        // using a closure here is slightly gross, but this makes sure that if the
        // lazy-init returns an error it can be handled by the client
        if dont_use_shared || SHARED_RESOLVER.fallback.is_none() {
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

    fn new_static_fallback(dont_use_shared: bool) -> StaticResolver {
        if !dont_use_shared && let Some(ref shared_resolver) = SHARED_RESOLVER.static_base {
            shared_resolver
                .get_or_init(new_default_static_fallback)
                .clone()
        } else {
            new_default_static_fallback()
        }
    }

    /// Enable fallback to the system default resolver if the primary (DoX) resolver fails
    pub fn enable_system_fallback(&mut self) -> Result<(), ResolveError> {
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

    /// Get the current map of hostname to address in use by the fallback static lookup if one
    /// exists.
    pub fn get_static_fallbacks(&self) -> Option<HashMap<String, Vec<IpAddr>>> {
        Some(self.static_base.as_ref()?.get()?.get_addrs())
    }

    /// Set (or overwrite) the map of addresses used in the fallback static hostname lookup
    pub fn set_static_fallbacks(&mut self, addrs: HashMap<String, Vec<IpAddr>>) {
        let cell = OnceCell::new();
        cell.set(StaticResolver::new(addrs))
            .expect("infallible assign");
        self.static_base = Some(Arc::new(cell));
    }
}

/// Create a new resolver with a custom DoT based configuration. The options are overridden to look
/// up for both IPv4 and IPv6 addresses to work with "happy eyeballs" algorithm.
///
/// Timeout Defaults to 5 seconds
/// Number of retries after lookup failure before giving up Defaults to 2
///
/// Caches successfully resolved addresses for 30 minutes to prevent continual use of remote lookup.
/// This resolver is intended to be used for OUR API endpoints that do not rapidly rotate IPs.
fn new_resolver() -> Result<TokioResolver, ResolveError> {
    info!("building new configured resolver");

    let mut name_servers = NameServerConfigGroup::quad9_tls();
    name_servers.merge(NameServerConfigGroup::quad9_https());
    name_servers.merge(NameServerConfigGroup::cloudflare_tls());
    name_servers.merge(NameServerConfigGroup::cloudflare_https());

    configure_and_build_resolver(name_servers)
}

fn configure_and_build_resolver(
    name_servers: NameServerConfigGroup,
) -> Result<TokioResolver, ResolveError> {
    let config = ResolverConfig::from_parts(None, Vec::new(), name_servers);
    let mut resolver_builder =
        TokioResolver::builder_with_config(config, TokioConnectionProvider::default());

    resolver_builder.options_mut().ip_strategy = LookupIpStrategy::Ipv4thenIpv6;
    // Cache successful responses for queries received by this resolver for 30 min minimum.
    resolver_builder.options_mut().positive_min_ttl = Some(Duration::from_secs(1800));

    Ok(resolver_builder.build())
}

/// Create a new resolver with the default configuration, which reads from the system DNS config
/// (i.e. `/etc/resolve.conf` in unix). The options are overridden to look up for both IPv4 and IPv6
/// addresses to work with "happy eyeballs" algorithm.
fn new_resolver_system() -> Result<TokioResolver, ResolveError> {
    let mut resolver_builder = TokioResolver::builder_tokio()?;
    resolver_builder.options_mut().ip_strategy = LookupIpStrategy::Ipv4AndIpv6;

    Ok(resolver_builder.build())
}

fn new_default_static_fallback() -> StaticResolver {
    StaticResolver::new(constants::default_static_addrs())
}

#[cfg(test)]
mod test {
    use super::*;
    use itertools::Itertools;
    use std::collections::HashMap;

    #[tokio::test]
    async fn reqwest_with_custom_dns() {
        let var_name = HickoryDnsResolver::default();
        let resolver = var_name;
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
    async fn dns_lookup() -> Result<(), ResolveError> {
        let resolver = HickoryDnsResolver::default();

        let domain = "ifconfig.me";
        let addrs = resolver.resolve_str(domain).await?;

        assert!(addrs.into_iter().next().is_some());

        Ok(())
    }

    #[tokio::test]
    async fn static_resolver_as_fallback() -> Result<(), ResolveError> {
        let example_domain = "non-existent.nymvpn.com";
        let mut resolver = HickoryDnsResolver {
            ..Default::default()
        };

        let result = resolver.resolve_str(example_domain).await;
        assert!(result.is_err()); // should be NXDomain

        resolver.static_base = Some(Default::default());

        let mut addr_map = HashMap::new();
        let example_ip4: IpAddr = "10.10.10.10".parse().unwrap();
        let example_ip6: IpAddr = "dead::beef".parse().unwrap();
        addr_map.insert(example_domain.to_string(), vec![example_ip4, example_ip6]);

        resolver.set_static_fallbacks(addr_map);

        let mut addrs = resolver.resolve_str(example_domain).await?;
        assert!(addrs.contains(&example_ip4));
        assert!(addrs.contains(&example_ip6));
        Ok(())
    }
}

#[cfg(test)]
mod failure_test {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    /// IP addresses guaranteed to fail attempts to resolve
    ///
    /// Addresses drawn from blocks set off by RFC5737 (ipv4) and RFC3849 (ipv6)
    const GUARANTEED_BROKEN_IPS_1: &[IpAddr] = &[
        IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)),
        IpAddr::V4(Ipv4Addr::new(198, 51, 100, 1)),
        IpAddr::V6(Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 0x1111)),
        IpAddr::V6(Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 0x1001)),
    ];

    // Create a resolver that behaves the same as the custom configured router, except for the fact
    // that it is guaranteed to fail.
    fn build_broken_resolver() -> Result<TokioResolver, ResolveError> {
        info!("building new faulty resolver");

        let mut broken_ns_group = NameServerConfigGroup::from_ips_tls(
            GUARANTEED_BROKEN_IPS_1,
            853,
            "cloudflare-dns.com".to_string(),
            true,
        );
        let broken_ns_https = NameServerConfigGroup::from_ips_https(
            GUARANTEED_BROKEN_IPS_1,
            443,
            "cloudflare-dns.com".to_string(),
            true,
        );
        broken_ns_group.merge(broken_ns_https);

        configure_and_build_resolver(broken_ns_group)
    }

    #[tokio::test]
    async fn dns_lookup_failures() -> Result<(), ResolveError> {
        let time_start = std::time::Instant::now();

        let r = OnceCell::new();
        r.set(build_broken_resolver().expect("failed to build resolver"))
            .expect("broken resolver init error");

        // create a new resolver that won't mess with the shared resolver used by other tests
        let resolver = HickoryDnsResolver {
            dont_use_shared: true,
            state: Arc::new(r),
            overall_dns_timeout: Duration::from_secs(5),
            ..Default::default()
        };
        build_broken_resolver()?;
        let domain = "ifconfig.me";
        let result = resolver.resolve_str(domain).await;
        assert!(result.is_err_and(|e| matches!(e, ResolveError::Timeout)));

        let duration = time_start.elapsed();
        assert!(duration < resolver.overall_dns_timeout + Duration::from_secs(1));

        Ok(())
    }

    #[tokio::test]
    async fn fallback_to_static() -> Result<(), ResolveError> {
        let r = OnceCell::new();
        r.set(build_broken_resolver().expect("failed to build resolver"))
            .expect("broken resolver init error");

        // create a new resolver that won't mess with the shared resolver used by other tests
        let resolver = HickoryDnsResolver {
            dont_use_shared: true,
            state: Arc::new(r),
            static_base: Some(Default::default()),
            overall_dns_timeout: Duration::from_secs(5),
            ..Default::default()
        };
        build_broken_resolver()?;

        // successful lookup using fallback to static resolver
        let domain = "nymvpn.com";
        let _ = resolver
            .resolve_str(domain)
            .await
            .expect("failed to resolve address in static lookup");

        // unsuccessful lookup - primary times out, and not in
        let domain = "non-existent.nymtech.net";
        let result = resolver.resolve_str(domain).await;
        assert!(result.is_err_and(|e| matches!(e, ResolveError::Timeout)));

        Ok(())
    }
}
