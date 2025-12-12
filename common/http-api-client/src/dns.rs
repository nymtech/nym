// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! DNS resolver configuration for internal lookups.
//!
//! The resolver itself is the set combination of the cloudflare, and quad9 endpoints supporting DoH
//! and DoT.
//!
//! ```rust
//! use nym_http_api_client::HickoryDnsResolver;
//! # use nym_http_api_client::ResolveError;
//! # type Err = ResolveError;
//! # async fn run() -> Result<(), Err> {
//! let resolver = HickoryDnsResolver::default();
//! resolver.resolve_str("example.com").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Fallbacks
//!
//! **System Resolver --** This resolver supports an optional fallback mechanism where, should the
//! DNS-over-TLS resolution fail, a followup resolution will be done using the hosts configured
//! default (e.g. `/etc/resolve.conf` on linux).
//!
//! This is disabled by default and can be enabled using `enable_system_fallback`.
//!
//! **Static Table --**  There is also a second optional fallback mechanism that allows a static map
//! to be used as a last resort. This can help when DNS encounters errors due to blocked resolvers
//! or unknown conditions. This is enabled by default, and can be customized if building a new
//! resolver.
//!
//! ## IPv4 / IPv6
//!
//! By default the resolver uses only IPv4 nameservers, and is configured to do `A` lookups first,
//! and only do `AAAA` if no `A` record is available.
//!
//! ---
//!
//! Requires the `dns-over-https-rustls`, `webpki-roots` feature for the `hickory-resolver` crate
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
    config::{NameServerConfig, NameServerConfigGroup, ResolverConfig, ResolverOpts},
    lookup_ip::LookupIpIntoIter,
    name_server::TokioConnectionProvider,
};
use once_cell::sync::OnceCell;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use tracing::*;

mod constants;
mod static_resolver;
pub(crate) use static_resolver::*;

pub(crate) const DEFAULT_POSITIVE_LOOKUP_CACHE_TTL: Duration = Duration::from_secs(1800);
pub(crate) const DEFAULT_OVERALL_LOOKUP_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const DEFAULT_QUERY_TIMEOUT: Duration = Duration::from_secs(5);

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
    HickoryDnsResolver {
        use_shared: false, // prevent infinite recursion
        ..Default::default()
    }
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

impl ResolveError {
    /// Returns true if the error is a timeout.
    pub fn is_timeout(&self) -> bool {
        matches!(self, ResolveError::Timeout)
    }
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
    use_shared: bool,
    /// Overall timeout for dns lookup associated with any individual host resolution. For example,
    /// use of retries, server_ordering_strategy, etc. ends absolutely if this timeout is reached.
    overall_dns_timeout: Duration,
}

impl Default for HickoryDnsResolver {
    fn default() -> Self {
        Self {
            state: Default::default(),
            fallback: Default::default(),
            static_base: Some(Default::default()),
            use_shared: true,
            overall_dns_timeout: DEFAULT_OVERALL_LOOKUP_TIMEOUT,
        }
    }
}

impl Resolve for HickoryDnsResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let resolver = self.state.clone();
        let maybe_fallback = self.fallback.clone();
        let maybe_static = self.static_base.clone();
        let use_shared = self.use_shared;
        let overall_dns_timeout = self.overall_dns_timeout;
        Box::pin(async move {
            resolve(
                name,
                resolver,
                maybe_fallback,
                maybe_static,
                use_shared,
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
            self.use_shared,
            self.overall_dns_timeout,
        )
        .await
        .map(|addrs| addrs.map(|socket_addr| socket_addr.ip()))
    }

    /// Create a (lazy-initialized) resolver that is not shared across threads.
    pub fn thread_resolver() -> Self {
        Self {
            use_shared: false,
            ..Default::default()
        }
    }

    fn new_resolver(use_shared: bool) -> Result<TokioResolver, ResolveError> {
        // using a closure here is slightly gross, but this makes sure that if the
        // lazy-init returns an error it can be handled by the client
        if !use_shared {
            new_resolver()
        } else {
            Ok(SHARED_RESOLVER.state.get_or_try_init(new_resolver)?.clone())
        }
    }

    fn new_resolver_system(use_shared: bool) -> Result<TokioResolver, ResolveError> {
        // using a closure here is slightly gross, but this makes sure that if the
        // lazy-init returns an error it can be handled by the client
        if !use_shared || SHARED_RESOLVER.fallback.is_none() {
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

    fn new_static_fallback(use_shared: bool) -> StaticResolver {
        if use_shared && let Some(ref shared_resolver) = SHARED_RESOLVER.static_base {
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

        // IF THIS INSTANCE IS A FRONT FOR THE SHARED RESOLVER SHOULDN'T THIS FN ENABLE THE SYSTEM FALLBACK FOR THE SHARED RESOLVER TOO?
        // if self.use_shared {
        //     SHARED_RESOLVER.enable_system_fallback()?;
        // }
        Ok(())
    }

    /// Disable fallback resolution. If the primary resolver fails the error is
    /// returned immediately
    pub fn disable_system_fallback(&mut self) {
        self.fallback = None;

        // // IF THIS INSTANCE IS A FRONT FOR THE SHARED RESOLVER SHOULDN'T THIS FN ENABLE THE SYSTEM FALLBACK FOR THE SHARED RESOLVER TOO?
        // if self.use_shared {
        //     SHARED_RESOLVER.fallback = None;
        // }
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

    /// Successfully resolved addresses are cached for a minimum of 30 minutes
    /// Individual lookup Timeouts are set to 3 seconds
    /// Number of retries after lookup failure before giving up is set to (default) to 2
    /// Lookup order is set to (default) A then AAAA
    /// Number or parallel lookup is set to (default) 2
    /// Nameserver selection uses the (default) EWMA statistics / performance based strategy
    fn default_options() -> ResolverOpts {
        let mut opts = ResolverOpts::default();
        // Always cache successful responses for queries received by this resolver for 30 min minimum.
        opts.positive_min_ttl = Some(DEFAULT_POSITIVE_LOOKUP_CACHE_TTL);
        opts.timeout = DEFAULT_QUERY_TIMEOUT;

        opts
    }

    /// Get the list of currently available nameserver configs.
    pub fn all_configured_name_servers(&self) -> Vec<NameServerConfig> {
        default_nameserver_group().to_vec()
    }

    /// Get the list of currently used nameserver configs.
    pub fn active_name_servers(&self) -> Vec<NameServerConfig> {
        if !self.use_shared {
            return self
                .state
                .get()
                .map(|r| r.config().name_servers().to_vec())
                .unwrap_or(self.all_configured_name_servers());
        }

        SHARED_RESOLVER.active_name_servers()
    }

    /// Do a trial resolution using each nameserver individually to test which are working and which
    /// fail to complete a lookup. This will always try the full set of default configured resolvers.
    pub async fn trial_nameservers(&self) {
        let nameservers = default_nameserver_group();
        for (ns, result) in trial_nameservers_inner(&nameservers).await {
            if let Err(e) = result {
                warn!("trial {ns:?} errored: {e}");
            } else {
                info!("trial {ns:?} succeeded");
            }
        }
    }
}

/// Create a new resolver with a custom DoT based configuration. The options are overridden to look
/// up for both IPv4 and IPv6 addresses to work with "happy eyeballs" algorithm.
///
/// Individual lookup Timeouts are set to 3 seconds
/// Number of retries after lookup failure before giving up Defaults to 2
/// Lookup order is set to (default) A then AAAA
///
/// Caches successfully resolved addresses for 30 minutes to prevent continual use of remote lookup.
/// This resolver is intended to be used for OUR API endpoints that do not rapidly rotate IPs.
fn new_resolver() -> Result<TokioResolver, ResolveError> {
    let name_servers = default_nameserver_group_ipv4_only();

    Ok(configure_and_build_resolver(name_servers))
}

fn configure_and_build_resolver<G>(name_servers: G) -> TokioResolver
where
    G: Into<NameServerConfigGroup>,
{
    let options = HickoryDnsResolver::default_options();
    let name_servers: NameServerConfigGroup = name_servers.into();
    info!("building new configured resolver");
    debug!("configuring resolver with {options:?}, {name_servers:?}");

    let config = ResolverConfig::from_parts(None, Vec::new(), name_servers);
    let mut resolver_builder =
        TokioResolver::builder_with_config(config, TokioConnectionProvider::default());

    resolver_builder = resolver_builder.with_options(options);

    resolver_builder.build()
}

fn filter_ipv4(nameservers: impl AsRef<[NameServerConfig]>) -> Vec<NameServerConfig> {
    nameservers
        .as_ref()
        .iter()
        .filter(|ns| ns.socket_addr.is_ipv4())
        .cloned()
        .collect()
}

#[allow(unused)]
fn filter_ipv6(nameservers: impl AsRef<[NameServerConfig]>) -> Vec<NameServerConfig> {
    nameservers
        .as_ref()
        .iter()
        .filter(|ns| ns.socket_addr.is_ipv6())
        .cloned()
        .collect()
}

#[allow(unused)]
fn default_nameserver_group() -> NameServerConfigGroup {
    let mut name_servers = NameServerConfigGroup::quad9_tls();
    name_servers.merge(NameServerConfigGroup::quad9_https());
    name_servers.merge(NameServerConfigGroup::cloudflare_tls());
    name_servers.merge(NameServerConfigGroup::cloudflare_https());
    name_servers
}

fn default_nameserver_group_ipv4_only() -> NameServerConfigGroup {
    filter_ipv4(&default_nameserver_group() as &[NameServerConfig]).into()
}

#[allow(unused)]
fn default_nameserver_group_ipv6_only() -> NameServerConfigGroup {
    filter_ipv6(&default_nameserver_group() as &[NameServerConfig]).into()
}

/// Create a new resolver with the default configuration, which reads from the system DNS config
/// (i.e. `/etc/resolve.conf` in unix). The options are overridden to look up for both IPv4 and IPv6
/// addresses to work with "happy eyeballs" algorithm.
fn new_resolver_system() -> Result<TokioResolver, ResolveError> {
    let mut resolver_builder = TokioResolver::builder_tokio()?;

    let options = HickoryDnsResolver::default_options();
    info!("building new fallback system resolver");
    debug!("fallback system resolver with {options:?}");

    resolver_builder = resolver_builder.with_options(options);

    Ok(resolver_builder.build())
}

fn new_default_static_fallback() -> StaticResolver {
    StaticResolver::new(constants::default_static_addrs())
}

/// Do a trial resolution using each nameserver individually to test which are working and which
/// fail to complete a lookup.
async fn trial_nameservers_inner(
    name_servers: &[NameServerConfig],
) -> Vec<(NameServerConfig, Result<(), ResolveError>)> {
    let mut trial_lookups = tokio::task::JoinSet::new();

    for name_server in name_servers {
        let ns = name_server.clone();
        trial_lookups.spawn(async { (ns.clone(), trial_lookup(ns, "example.com").await) });
    }

    trial_lookups.join_all().await
}

/// Create an independent resolver that has only the provided nameserver and do one lookup for the
/// provided query target.
async fn trial_lookup(name_server: NameServerConfig, query: &str) -> Result<(), ResolveError> {
    debug!("running ns trial {name_server:?} query={query}");

    let resolver = configure_and_build_resolver(vec![name_server]);

    match tokio::time::timeout(DEFAULT_OVERALL_LOOKUP_TIMEOUT, resolver.ipv4_lookup(query)).await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(e.into()),
        Err(_) => Err(ResolveError::Timeout),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use itertools::Itertools;
    use std::collections::HashMap;
    use std::{
        net::{IpAddr, Ipv4Addr, Ipv6Addr},
        time::Instant,
    };

    /// IP addresses guaranteed to fail attempts to resolve
    ///
    /// Addresses drawn from blocks set off by RFC5737 (ipv4) and RFC3849 (ipv6)
    const GUARANTEED_BROKEN_IPS_1: &[IpAddr] = &[
        IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)),
        IpAddr::V4(Ipv4Addr::new(198, 51, 100, 1)),
        IpAddr::V6(Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 0x1111)),
        IpAddr::V6(Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 0x1001)),
    ];

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

    // Test the nameserver trial functionality with mostly nameservers guaranteed to be broken and
    // one that should work.
    #[tokio::test]
    async fn trial_nameservers() {
        let good_cf_ip = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));

        let mut ns_ips = GUARANTEED_BROKEN_IPS_1.to_vec();
        ns_ips.push(good_cf_ip);

        let broken_ns_https = NameServerConfigGroup::from_ips_https(
            &ns_ips,
            443,
            "cloudflare-dns.com".to_string(),
            true,
        );

        let inner = configure_and_build_resolver(broken_ns_https);

        // create a new resolver that won't mess with the shared resolver used by other tests
        let resolver = HickoryDnsResolver {
            use_shared: false,
            state: Arc::new(OnceCell::with_value(inner)),
            static_base: Some(Default::default()),
            ..Default::default()
        };

        let name_servers = resolver.state.get().unwrap().config().name_servers();
        for (ns, result) in trial_nameservers_inner(name_servers).await {
            if ns.socket_addr.ip() == good_cf_ip {
                assert!(result.is_ok())
            } else {
                assert!(result.is_err())
            }
        }
    }

    mod failure_test {
        use super::*;

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

            Ok(configure_and_build_resolver(broken_ns_group))
        }

        #[tokio::test]
        async fn dns_lookup_failures() -> Result<(), ResolveError> {
            let time_start = std::time::Instant::now();

            let r = OnceCell::new();
            r.set(build_broken_resolver().expect("failed to build resolver"))
                .expect("broken resolver init error");

            // create a new resolver that won't mess with the shared resolver used by other tests
            let resolver = HickoryDnsResolver {
                use_shared: false,
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
                use_shared: false,
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

            // unsuccessful lookup - primary times out, and not in static table
            let domain = "non-existent.nymtech.net";
            let result = resolver.resolve_str(domain).await;
            assert!(result.is_err_and(|e| matches!(e, ResolveError::Timeout)));

            Ok(())
        }

        #[test]
        fn default_resolver_uses_ipv4_only_nameservers() {
            let resolver = HickoryDnsResolver::thread_resolver();
            resolver
                .active_name_servers()
                .iter()
                .all(|cfg| cfg.socket_addr.is_ipv4());

            SHARED_RESOLVER
                .active_name_servers()
                .iter()
                .all(|cfg| cfg.socket_addr.is_ipv4());
        }

        #[tokio::test]
        #[ignore]
        // this test is dependent of external network setup -- i.e. blocking all traffic to the default
        // resolvers. Otherwise the default resolvers will succeed without using the static fallback,
        // making the test pointless
        async fn dns_lookup_failure_on_shared() -> Result<(), ResolveError> {
            let time_start = Instant::now();
            let r = OnceCell::new();
            r.set(build_broken_resolver().expect("failed to build resolver"))
                .expect("broken resolver init error");

            // create a new resolver that won't mess with the shared resolver used by other tests
            let resolver = HickoryDnsResolver::default();

            // successful lookup using fallback to static resolver
            let domain = "rpc.nymtech.net";
            let _ = resolver
                .resolve_str(domain)
                .await
                .expect("failed to resolve address in static lookup");

            println!(
                "{}ms resolved {domain}",
                (Instant::now() - time_start).as_millis()
            );

            // unsuccessful lookup - primary times out, and not in static table
            let domain = "non-existent.nymtech.net";
            let result = resolver.resolve_str(domain).await;
            assert!(result.is_err());
            // assert!(result.is_err_and(|e| matches!(e, ResolveError::Timeout)));
            // assert!(result.is_err_and(|e| matches!(e, ResolveError::ResolveError(e) if e.is_nx_domain())));
            Ok(())
        }
    }
}
