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
//! **Static Table --**  There is also a second optional fallback mechanism that allows a static map to
//! be used as a last resort. This can help when DNS encounters errors due to blocked resolvers or
//! unknown conditions. This is enabled by default, and can be customized if building a new resolver.
//!
//! ## IPv4 / IPv6
//!
//! The resolver can be modified to control the behavior with respect to IPv4 and IPv6. If using the
//! shared resolver setting these options will apply to future lookups done using the shared
//! resolver as well.
//!
//! Be default the resolver uses both IPv4 and IPv6 nameservers, and is configured to do `A` lookups
//! first, and only do `AAAA` if no `A` record is available.
//!
//! ```rust
//! # use nym_http_api_client::{HickoryDnsResolver, dns::NameServerIpVersionPolicy};
//! let mut resolver = HickoryDnsResolver::default();
//!
//! // Set the resolver to only use IPv4 nameservers and
//! // only do lookups for A records.
//! resolver.set_ipv4_only();
//!
//! // Set the resolver to use only IPv6 nameservers
//! resolver.set_nameserver_ip_version_strategy(NameServerIpVersionPolicy::Ipv6Only);
//! ```
//!
//! ---
//!
//! Requires the `dns-over-https-rustls`, `webpki-roots` feature for the `hickory-resolver` crate
#![deny(missing_docs)]

use crate::ClientBuilder;

use std::{
    collections::HashMap,
    fmt::Display,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::{Arc, LazyLock, RwLock},
    time::{Duration, Instant},
};

use hickory_resolver::{
    TokioResolver,
    config::{
        LookupIpStrategy, NameServerConfig, NameServerConfigGroup, ResolverConfig, ResolverOpts,
    },
    lookup_ip::LookupIpIntoIter,
    name_server::TokioConnectionProvider,
};
use once_cell::sync::OnceCell;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use tokio::task::JoinSet;
use tracing::*;

mod constants;
mod static_resolver;
pub(crate) use static_resolver::*;

pub(crate) const DEFAULT_POSITIVE_LOOKUP_CACHE_TTL: Duration = Duration::from_secs(1800);
pub(crate) const DEFAULT_OVERALL_LOOKUP_TIMEOUT: Duration = Duration::from_secs(6);
pub(crate) const DEFAULT_QUERY_TIMEOUT: Duration = Duration::from_secs(3);

const RECONFIGURE_ERROR_MSG: &str = "attempted to reconfigure with no working nameservers";

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

// n.b. static items do not call [`Drop`] on program termination, so this won't be deallocated. this
// is fine, as the OS can deallocate the terminated program faster than we can free memory but tools
// like valgrind might report "memory leaks" as it isn't obvious this is intentional. Using RwLock
// for interior mutability to allow safe modification of the shared resolver -- i.e. rebuilding on
// change to desired IPv4/IPv6 config.
static SHARED_RESOLVER: LazyLock<Arc<RwLock<HickoryDnsResolver>>> = LazyLock::new(|| {
    tracing::debug!("Initializing shared DNS resolver with interior mutability");
    Arc::new(RwLock::new(HickoryDnsResolver {
        dont_use_shared: true, // prevent infinite recursion
        ..Default::default()
    }))
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
    #[error("configuration error: {0}")]
    ConfigError(String),
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
    dont_use_shared: bool,
    /// Toggle to indicate whether `self.static_base` should be checked before the attempting to go
    /// to network using the inner resolver (state).
    static_base_first: bool,
    /// Overall timeout for dns lookup associated with any individual host resolution. For example,
    /// use of retries, server_ordering_strategy, etc. ends absolutely if this timeout is reached.
    overall_dns_timeout: Duration,
    /// Policy for which Ip version should be used for name_servers.
    ns_ip_ver_policy: NameServerIpVersionPolicy,
    /// Current options used by the resolver and used for rebuilding on preference change.
    current_options: Option<ResolverOpts>,

    /// Set of nameservers used for this resolver before any filtering is applied.
    // Used internally and for testing.
    default_nameserver_config_group: NsConfigGroupWithStatus,
}

#[derive(Debug, Clone)]
struct NsConfigGroupWithStatus {
    inner: Vec<(NameServerConfig, NsStatus)>,
}

impl NsConfigGroupWithStatus {
    fn into_ns_group(self) -> NameServerConfigGroup {
        self.inner
            .iter()
            .map(|entry| entry.0.clone())
            .collect::<Vec<NameServerConfig>>()
            .into()
    }

    fn nameserver_configs(&self) -> Vec<NameServerConfig> {
        self.inner.iter().map(|(cfg, _)| cfg.clone()).collect()
    }

    fn active_nameserver_configs(&self) -> Vec<NameServerConfig> {
        self.inner
            .iter()
            .filter(|(_, status)| matches!(status, NsStatus::Untested | NsStatus::Working(_)))
            .map(|(cfg, _)| cfg.clone())
            .collect()
    }
}

impl From<&[NameServerConfig]> for NsConfigGroupWithStatus {
    fn from(value: &[NameServerConfig]) -> Self {
        let inner = value
            .iter()
            .map(|cfg| (cfg.clone(), NsStatus::Untested))
            .collect();

        Self { inner }
    }
}

impl From<Vec<NameServerConfig>> for NsConfigGroupWithStatus {
    fn from(value: Vec<NameServerConfig>) -> Self {
        let inner = value
            .iter()
            .map(|cfg| (cfg.clone(), NsStatus::Untested))
            .collect();

        Self { inner }
    }
}

impl From<NameServerConfigGroup> for NsConfigGroupWithStatus {
    fn from(value: NameServerConfigGroup) -> Self {
        Self::from(&value as &[NameServerConfig])
    }
}

#[derive(Debug, Clone, PartialEq)]
enum NsStatus {
    Untested,
    Working(Instant),
    Failed(Instant),
}

impl Default for HickoryDnsResolver {
    fn default() -> Self {
        Self {
            state: Default::default(),
            fallback: Default::default(),
            dont_use_shared: Default::default(),
            ns_ip_ver_policy: Default::default(),
            current_options: Default::default(),
            static_base: Some(Default::default()),
            static_base_first: false,
            overall_dns_timeout: DEFAULT_OVERALL_LOOKUP_TIMEOUT,
            default_nameserver_config_group: default_nameserver_group(),
        }
    }
}

impl Resolve for HickoryDnsResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let resolver = self.state.clone();
        let maybe_fallback = self.fallback.clone();
        let maybe_static = self.static_base.clone();
        let independent = self.dont_use_shared;
        let static_base_first = self.static_base_first;
        let ns_strategy = self.ns_ip_ver_policy;
        let overall_dns_timeout = self.overall_dns_timeout;
        let options = self.current_options.clone();
        Box::pin(async move {
            resolve(
                name,
                resolver,
                maybe_fallback,
                maybe_static,
                independent,
                static_base_first,
                ns_strategy,
                options,
                overall_dns_timeout,
            )
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        })
    }
}

#[allow(clippy::too_many_arguments)]
async fn resolve(
    name: Name,
    resolver: Arc<OnceCell<TokioResolver>>,
    maybe_fallback: Option<Arc<OnceCell<TokioResolver>>>,
    maybe_static: Option<Arc<OnceCell<StaticResolver>>>,
    independent: bool,
    static_base_first: bool,
    ns_strategy: NameServerIpVersionPolicy,
    options: Option<ResolverOpts>,
    overall_dns_timeout: Duration,
) -> Result<Addrs, ResolveError> {
    let name_str = name.as_str().to_string();

    debug!(
        "looking up {name_str} - {} {}",
        maybe_static.is_some(),
        maybe_fallback.is_some()
    );

    // If we are configured to check the static map first, and a static map of addresses is
    // configured -- check the table for our entry.
    if static_base_first && let Some(ref static_resolver) = maybe_static {
        debug!("checking static");
        let qname = Name::from_str(&name_str).unwrap();
        let resolver =
            static_resolver.get_or_init(|| HickoryDnsResolver::new_static_fallback(independent));
        if let Ok(addrs) = resolver.resolve(qname).await {
            let _addrs: Vec<SocketAddr> = addrs.into_iter().collect();
            debug!("internal static table found {} -> {_addrs:?}", name_str);
            return Ok(Box::new(_addrs.into_iter()));
        }
    }

    let resolver = resolver
        .get_or_try_init(|| HickoryDnsResolver::new_resolver(independent, ns_strategy, options))?;

    // Attempt a lookup using the primary resolver
    let resolve_fut = tokio::time::timeout(overall_dns_timeout, resolver.lookup_ip(&name_str));
    let primary_err = match resolve_fut.await {
        Err(_) => ResolveError::Timeout,
        Ok(Ok(lookup)) => {
            debug!("internal primary resolver found {name_str} -> {lookup:?}",);
            let addrs: Addrs = Box::new(SocketAddrs {
                iter: lookup.into_iter(),
            });
            return Ok(addrs);
        }
        Ok(Err(e)) => e.into(),
    };
    warn!("primary DNS failed w/ error: {primary_err}");

    // If the primary resolver encountered an error, attempt a lookup using the fallback
    // resolver if one is configured.
    if let Some(ref fallback) = maybe_fallback {
        let resolver =
            fallback.get_or_try_init(|| HickoryDnsResolver::new_resolver_system(independent))?;

        let resolve_fut = tokio::time::timeout(overall_dns_timeout, resolver.lookup_ip(&name_str));
        if let Ok(Ok(lookup)) = resolve_fut.await {
            debug!("internal fallback resolver found {name_str} -> {lookup:?}",);
            let addrs: Addrs = Box::new(SocketAddrs {
                iter: lookup.into_iter(),
            });
            return Ok(addrs);
        }
    }

    // If no record has been found, we are configured to check the static table as a fallback, and
    // static map of fallback addresses is configured -- check the table for our entry.
    if !static_base_first && let Some(ref static_resolver) = maybe_static {
        debug!("checking static");
        // this unwrap cannot fail as we serialize name_str from a valid Name
        let qname = Name::from_str(&name_str).unwrap();
        let resolver =
            static_resolver.get_or_init(|| HickoryDnsResolver::new_static_fallback(independent));
        if let Ok(addrs) = resolver.resolve(qname).await {
            let _addrs: Vec<SocketAddr> = addrs.into_iter().collect();
            debug!("internal static table found {} -> {_addrs:?}", name_str);
            return Ok(Box::new(_addrs.into_iter()));
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
            self.static_base_first,
            self.ns_ip_ver_policy,
            self.current_options.clone(),
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

    fn new_resolver(
        dont_use_shared: bool,
        ns_strategy: NameServerIpVersionPolicy,
        options: Option<ResolverOpts>,
    ) -> Result<TokioResolver, ResolveError> {
        // using a closure here is slightly gross, but this makes sure that if the
        // lazy-init returns an error it can be handled by the client
        if dont_use_shared {
            new_resolver(ns_strategy, options)
        } else {
            Ok(SHARED_RESOLVER
                .read()
                .unwrap()
                .state
                .get_or_try_init(|| new_resolver(ns_strategy, options))?
                .clone())
        }
    }

    fn new_resolver_system(dont_use_shared: bool) -> Result<TokioResolver, ResolveError> {
        // using a closure here is slightly gross, but this makes sure that if the
        // lazy-init returns an error it can be handled by the client
        if dont_use_shared || SHARED_RESOLVER.read().unwrap().fallback.is_none() {
            new_resolver_system()
        } else {
            Ok(SHARED_RESOLVER
                .read()
                .unwrap()
                .fallback
                .as_ref()
                .unwrap()
                .get_or_try_init(new_resolver_system)?
                .clone())
        }
    }

    fn new_static_fallback(dont_use_shared: bool) -> StaticResolver {
        if !dont_use_shared
            && let Some(ref shared_resolver) = SHARED_RESOLVER.read().unwrap().static_base
        {
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
        if !self.dont_use_shared {
            return Some(
                SHARED_RESOLVER
                    .read()
                    .unwrap()
                    .static_base
                    .as_ref()?
                    .get()?
                    .get_addrs(),
            );
        }
        Some(self.static_base.as_ref()?.get()?.get_addrs())
    }

    /// Set (or overwrite) the map of addresses used in the fallback static hostname lookup
    pub fn set_static_fallbacks(&mut self, addrs: HashMap<String, Vec<IpAddr>>) {
        let resolver_table = Arc::new(OnceCell::with_value(StaticResolver::new(addrs)));
        self.static_base = Some(resolver_table.clone());

        if !self.dont_use_shared {
            SHARED_RESOLVER.write().unwrap().static_base = Some(resolver_table);
        }
    }

    /// Change whether the resolver checks the static table of fallback addresses first or last in
    /// the flow (i.e.`true` = before using the internal resolver, `false` = as a fallback after the
    /// internal resolver). If the resolver has no table of static addressed configured, this
    /// setting has no impact on the lookup flow.
    pub fn set_check_static_fallback_first(&mut self, setting: bool) {
        self.static_base_first = setting;

        if !self.dont_use_shared {
            SHARED_RESOLVER.write().unwrap().static_base_first = setting
        }
    }

    /// Configure the resolver to use only Ipv4 nameservers and to resolve addresses to Ipv4 (A)
    /// records only.
    ///
    /// NOTE: Calling this function will rebuild the inner resolver which means that the
    /// cached dns lookups will not carry over and will go to network to resolve again.
    pub fn set_ipv4_only(&mut self) {
        self.set_hostname_ip_version_lookup_strategy(HostnameIpLookupStrategy::A_only);
        self.set_nameserver_ip_version_strategy(NameServerIpVersionPolicy::Ipv4Only);
    }

    /// Set the policy relating to nameserver IP version.
    ///
    /// NOTE: Calling this function will rebuild the inner resolver which means that the
    /// cached dns lookups will not carry over and will go to network to resolve again.
    pub fn set_nameserver_ip_version_strategy(&mut self, strategy: NameServerIpVersionPolicy) {
        if strategy == self.ns_ip_ver_policy {
            // correct strategy is already set. avoid rebuilding and clearing cache.
            return;
        }
        self.ns_ip_ver_policy = strategy;

        self.force_primary_rebuild();
    }

    /// Get the current policy in use by this resolver for nameserver IP version.
    pub fn get_nameserver_ip_version_strategy(&self) -> NameServerIpVersionPolicy {
        self.ns_ip_ver_policy
    }

    /// Set the policy for the record type queried when looking up hostnames
    ///
    /// NOTE: Calling this function will rebuild the inner resolver which means that the
    /// cached dns lookups will not carry over and will go to network to resolve again.
    pub fn set_hostname_ip_version_lookup_strategy(&mut self, strategy: HostnameIpLookupStrategy) {
        if let Some(opts) = &self.current_options
            && opts.ip_strategy == strategy.into()
        {
            // correct strategy is already set. avoid rebuilding and clearing cache.
            return;
        }

        let mut options = self
            .current_options
            .clone()
            .unwrap_or(Self::default_options());
        options.ip_strategy = strategy.into();

        self.current_options = Some(options);
        self.force_primary_rebuild();
    }

    /// Get the list of currently available nameserver configs. This includes nameservers that are
    /// available, but unused because of a filter function -- for example [`Self::set_ipv4_only`]
    /// would cause available IPv6 nameservers to be unused, but this function WOULD
    /// include them in the returned value anyways.
    pub fn all_configured_name_servers(&self) -> Vec<NameServerConfig> {
        self.default_nameserver_config_group.nameserver_configs()
    }

    /// Get the list of currently used nameserver configs. This excludes nameservers that are
    /// available, but unused because of a filter function -- for example [`Self::set_ipv4_only`]
    /// would cause available IPv6 nameservers to be unused meaning that this function would NOT
    /// include them in the returned value.
    pub fn active_name_servers(&self) -> Vec<NameServerConfig> {
        if !self.dont_use_shared {
            return SHARED_RESOLVER.read().unwrap().active_name_servers();
        }
        if let Some(r) = self.state.get() {
            return r.config().name_servers().to_vec();
        }
        self.default_nameserver_config_group
            .active_nameserver_configs()
    }

    /// Sets the available nameservers for use by this resolver to the provided list
    ///
    /// NOTE: Calling this function will rebuild the inner resolver which means that the
    /// cached dns lookups will not carry over and will go to network to resolve again.
    pub fn set_name_servers(&mut self, name_servers: impl AsRef<[NameServerConfig]>) {
        self.default_nameserver_config_group =
            NsConfigGroupWithStatus::from(name_servers.as_ref().to_vec());
        self.force_primary_rebuild();

        if !self.dont_use_shared {
            SHARED_RESOLVER
                .write()
                .unwrap()
                .set_name_servers(name_servers);
        }
    }

    fn default_options() -> ResolverOpts {
        let mut opts = ResolverOpts::default();
        // Always cache successful responses for queries received by this resolver for 30 min minimum.
        opts.positive_min_ttl = Some(DEFAULT_POSITIVE_LOOKUP_CACHE_TTL);
        opts.timeout = DEFAULT_QUERY_TIMEOUT;

        opts
    }

    fn force_primary_rebuild(&mut self) {
        if !self.dont_use_shared {
            let mut resolver = SHARED_RESOLVER.write().unwrap();
            // *resolver = HickoryDnsResolver {
            //     dont_use_shared: true,
            //     current_options: resolver.,
            //     ..Default::default()
            // }
            (*resolver).state = Arc::new(OnceCell::new());
        }
        self.state = Arc::new(OnceCell::new());
    }

    /// Do a trial resolution using each nameserver individually to test which are working and which
    /// fail to complete a lookup. This will always try the full set of default configured resolvers.
    pub async fn trial_nameservers(&self) -> Result<(), ResolveError> {
        let nameservers = self.default_nameserver_config_group.nameserver_configs();
        for (ns, result) in trial_nameservers_inner(&nameservers).await {
            if let Err(e) = result {
                warn!("trial {ns:?} errored: {e}");
            } else {
                info!("trial {ns:?} succeeded");
            }
        }
        Ok(())
    }

    /// Do a trial resolution using each nameserver individually to test which are working and which
    /// fail to complete a lookup. If one or more of the resolutions succeeds, rebuild the resolver
    /// using only the nameservers that successfully completed the lookup.
    ///
    /// This will always try the full set of default configured resolvers.
    ///
    /// If no nameservers successfully complete the lookup return an error and leave the current
    /// configured resolver set as is.
    pub async fn trial_nameservers_and_reconfigure(&mut self) -> Result<(), ResolveError> {
        let nameservers = self.default_nameserver_config_group.nameserver_configs();

        let mut working_nameservers = Vec::new();
        for (ns, result) in trial_nameservers_inner(&nameservers).await {
            if let Err(e) = result {
                warn!("trial {ns:?} errored: {e}");
            } else {
                info!("trial {ns:?} succeeded");
                working_nameservers.push(ns);
            }
        }

        if working_nameservers.is_empty() {
            return Err(ResolveError::ConfigError(RECONFIGURE_ERROR_MSG.to_string()));
        }

        let new_resolver =
            configure_and_build_resolver(working_nameservers, self.current_options.clone())?;

        self.state = Arc::new(OnceCell::with_value(new_resolver.clone()));
        if !self.dont_use_shared {
            // take a write lock on the shared resolver only once we are ready to make changes
            SHARED_RESOLVER.write().unwrap().state = Arc::new(OnceCell::with_value(new_resolver));
        }

        Ok(())
    }
}

/// Do a trial resolution using each nameserver individually to test which are working and which
/// fail to complete a lookup.
async fn trial_nameservers_inner(
    name_servers: &[NameServerConfig],
) -> Vec<(NameServerConfig, Result<(), ResolveError>)> {
    let mut trial_lookups = JoinSet::new();

    for name_server in name_servers {
        let ns = name_server.clone();
        trial_lookups.spawn(async { (ns.clone(), trial_lookup(ns, "example.com").await) });
    }

    trial_lookups.join_all().await
}

/// Create an independent resolver that has only the provided nameserver and do one lookup for the
/// provided query target.
async fn trial_lookup(name_server: NameServerConfig, query: &str) -> Result<(), ResolveError> {
    info!("running ns trial {name_server:?} query={query}");

    let resolver = configure_and_build_resolver(vec![name_server], None)?;

    match tokio::time::timeout(DEFAULT_OVERALL_LOOKUP_TIMEOUT, resolver.ipv4_lookup(query)).await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(e.into()),
        Err(_) => Err(ResolveError::Timeout),
    }
}

/// Policy options for nameserver IP versions to use when sending DNS queries.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum NameServerIpVersionPolicy {
    /// Only send queries to Ipv4 nameservers
    #[default]
    Ipv4Only,
    /// Only send queries to Ipv6 nameserver
    Ipv6Only,
    /// Send queries to Ipv4 AND Ipv6 nameservers in parallel
    Ipv4AndIpv6,
}

impl Display for NameServerIpVersionPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ipv4AndIpv6 => write!(f, "ipv4 & ipv6"),
            Self::Ipv6Only => write!(f, "ipv6 only"),
            Self::Ipv4Only => write!(f, "ipv4 only"),
        }
    }
}

/// Policy options for query types sent when lookup up a hostname.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum HostnameIpLookupStrategy {
    /// Only query for A (Ipv4) records
    A_only,
    /// Only query for AAAA (Ipv6) records
    AAAA_only,
    /// Query for A and AAAA in parallel
    A_and_AAAA,
    /// Query for Ipv6 if that fails, query for Ipv4
    AAAA_then_A,
    #[default]
    /// Query for Ipv4 if that fails, query for Ipv6 (default)
    A_then_AAAA,
}

impl From<LookupIpStrategy> for HostnameIpLookupStrategy {
    fn from(value: LookupIpStrategy) -> Self {
        match value {
            LookupIpStrategy::Ipv4AndIpv6 => HostnameIpLookupStrategy::A_and_AAAA,
            LookupIpStrategy::Ipv4Only => HostnameIpLookupStrategy::A_only,
            LookupIpStrategy::Ipv6Only => HostnameIpLookupStrategy::AAAA_only,
            LookupIpStrategy::Ipv4thenIpv6 => HostnameIpLookupStrategy::A_then_AAAA,
            LookupIpStrategy::Ipv6thenIpv4 => HostnameIpLookupStrategy::AAAA_then_A,
        }
    }
}

impl From<&LookupIpStrategy> for HostnameIpLookupStrategy {
    fn from(value: &LookupIpStrategy) -> Self {
        match value {
            LookupIpStrategy::Ipv4AndIpv6 => HostnameIpLookupStrategy::A_and_AAAA,
            LookupIpStrategy::Ipv4Only => HostnameIpLookupStrategy::A_only,
            LookupIpStrategy::Ipv6Only => HostnameIpLookupStrategy::AAAA_only,
            LookupIpStrategy::Ipv4thenIpv6 => HostnameIpLookupStrategy::A_then_AAAA,
            LookupIpStrategy::Ipv6thenIpv4 => HostnameIpLookupStrategy::AAAA_then_A,
        }
    }
}

impl From<HostnameIpLookupStrategy> for LookupIpStrategy {
    fn from(value: HostnameIpLookupStrategy) -> LookupIpStrategy {
        match value {
            HostnameIpLookupStrategy::A_and_AAAA => LookupIpStrategy::Ipv4AndIpv6,
            HostnameIpLookupStrategy::A_only => LookupIpStrategy::Ipv4Only,
            HostnameIpLookupStrategy::AAAA_only => LookupIpStrategy::Ipv6Only,
            HostnameIpLookupStrategy::A_then_AAAA => LookupIpStrategy::Ipv4thenIpv6,
            HostnameIpLookupStrategy::AAAA_then_A => LookupIpStrategy::Ipv6thenIpv4,
        }
    }
}

impl From<&HostnameIpLookupStrategy> for LookupIpStrategy {
    fn from(value: &HostnameIpLookupStrategy) -> LookupIpStrategy {
        match value {
            HostnameIpLookupStrategy::A_and_AAAA => LookupIpStrategy::Ipv4AndIpv6,
            HostnameIpLookupStrategy::A_only => LookupIpStrategy::Ipv4Only,
            HostnameIpLookupStrategy::AAAA_only => LookupIpStrategy::Ipv6Only,
            HostnameIpLookupStrategy::A_then_AAAA => LookupIpStrategy::Ipv4thenIpv6,
            HostnameIpLookupStrategy::AAAA_then_A => LookupIpStrategy::Ipv6thenIpv4,
        }
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
fn new_resolver(
    ns_ip_version_policy: NameServerIpVersionPolicy,
    options: Option<ResolverOpts>,
) -> Result<TokioResolver, ResolveError> {
    let name_servers: NameServerConfigGroup = match ns_ip_version_policy {
        NameServerIpVersionPolicy::Ipv4AndIpv6 => default_nameserver_group(),
        NameServerIpVersionPolicy::Ipv4Only => default_nameserver_group_ipv4_only(),
        NameServerIpVersionPolicy::Ipv6Only => default_nameserver_group_ipv6_only(),
    }
    .into_ns_group();

    info!("building new configured {ns_ip_version_policy} resolver");
    debug!("configuring resolver to use nameserver set: {name_servers:?}");

    configure_and_build_resolver(name_servers, options)
}

fn default_nameserver_group() -> NsConfigGroupWithStatus {
    let mut name_servers = NameServerConfigGroup::quad9_tls();
    name_servers.merge(NameServerConfigGroup::quad9_https());
    name_servers.merge(NameServerConfigGroup::cloudflare_tls());
    name_servers.merge(NameServerConfigGroup::cloudflare_https());
    name_servers.into()
}

fn configure_and_build_resolver<G>(
    name_servers: G,
    options: Option<ResolverOpts>,
) -> Result<TokioResolver, ResolveError>
where
    G: Into<NameServerConfigGroup>,
{
    let config = ResolverConfig::from_parts(None, Vec::new(), name_servers);
    let mut resolver_builder =
        TokioResolver::builder_with_config(config, TokioConnectionProvider::default());

    let options = options.unwrap_or(HickoryDnsResolver::default_options());
    resolver_builder = resolver_builder.with_options(options);

    Ok(resolver_builder.build())
}

fn filter_ipv4(nameservers: impl AsRef<[NameServerConfig]>) -> Vec<NameServerConfig> {
    nameservers
        .as_ref()
        .iter()
        .filter(|ns| ns.socket_addr.is_ipv4())
        .cloned()
        .collect()
}

fn filter_ipv6(nameservers: impl AsRef<[NameServerConfig]>) -> Vec<NameServerConfig> {
    nameservers
        .as_ref()
        .iter()
        .filter(|ns| ns.socket_addr.is_ipv6())
        .cloned()
        .collect()
}

fn default_nameserver_group_ipv4_only() -> NsConfigGroupWithStatus {
    filter_ipv4(default_nameserver_group().nameserver_configs()).into()
}

fn default_nameserver_group_ipv6_only() -> NsConfigGroupWithStatus {
    filter_ipv6(default_nameserver_group().nameserver_configs()).into()
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
    use std::{collections::HashMap, net::Ipv4Addr};

    #[tokio::test]
    async fn reqwest_with_custom_dns() {
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

    #[test]
    fn address_filter_check() {
        // Make sure the nameserver group for Ipv4 is really IPv4 only
        let ns_group = default_nameserver_group_ipv4_only();
        let addrs: Vec<IpAddr> = ns_group
            .into_ns_group()
            .iter()
            .map(|cfg| cfg.socket_addr.ip())
            .collect();
        assert!(addrs.iter().all(|addr| addr.is_ipv4()));

        // Make sure the nameserver group for Ipv6 is really IPv6 only
        let ns_group = default_nameserver_group_ipv6_only();
        let addrs: Vec<IpAddr> = ns_group
            .into_ns_group()
            .iter()
            .map(|cfg| cfg.socket_addr.ip())
            .collect();
        assert!(addrs.iter().all(|addr| addr.is_ipv6()));
    }

    #[tokio::test]
    async fn setting_ns_ip_version_works() {
        let resolver = HickoryDnsResolver {
            dont_use_shared: true,
            ..Default::default()
        };

        resolver_ns_ip_version_test(resolver).await
    }

    #[ignore]
    // This messes with the settings for the shared resolver which can interleave in negative ways with other tests.
    #[tokio::test]
    async fn setting_ns_ip_version_for_shared_resolver() {
        let resolver = HickoryDnsResolver::default();
        resolver_ns_ip_version_test(resolver).await
    }

    async fn resolver_ns_ip_version_test(mut resolver: HickoryDnsResolver) {
        let _ = resolver.resolve_str("example.com").await;

        // Make sure that setting IPv4Only changes the resolver set to only use IPv4 nameservers and
        // only do lookups for A records.
        resolver.set_ipv4_only();

        // setting opts resets resolver initialization
        assert!(resolver.state.get().is_none());

        let _ = resolver.resolve_str("example.com").await;

        // after rebuilding with new options it should have Ipv4 / A only
        let lookup_strategy = resolver.state.get().unwrap().options().ip_strategy;
        assert_eq!(lookup_strategy, HostnameIpLookupStrategy::A_only.into());

        let nameservers = resolver.state.get().unwrap().config().name_servers();
        assert!(nameservers.iter().all(|cfg| cfg.socket_addr.is_ipv4()));

        resolver.set_nameserver_ip_version_strategy(NameServerIpVersionPolicy::Ipv6Only);

        // setting opts resets resolver initialization
        assert!(resolver.state.get().is_none());

        let _ = resolver.resolve_str("example.com").await;

        // Make sure that setting the resolver to use only Ipv6 nameservers changes the set of
        // nameservers to only IPv6.
        let nameservers = resolver.state.get().unwrap().config().name_servers();
        assert!(nameservers.iter().all(|cfg| cfg.socket_addr.is_ipv6()));

        // reset to default
        resolver.set_hostname_ip_version_lookup_strategy(HostnameIpLookupStrategy::A_then_AAAA);
        resolver.set_nameserver_ip_version_strategy(NameServerIpVersionPolicy::Ipv4AndIpv6);
    }

    #[test]
    // ignore this test as changes to the shared resolver can cause unexpected behavior when
    // interleaved with other tests
    #[ignore]
    fn set_nameservers_using_public_fn_for_shared() {
        let mut resolver = HickoryDnsResolver::default();

        // Try setting the set of nameservers to be cloudflare only
        let new_ns_set = NameServerConfigGroup::cloudflare();
        resolver.set_name_servers(new_ns_set.into_inner());

        // check that our resolver instance contains a cloudflare address, and does not contain a
        // quad9 address that it would contain by default if the assignment had failed.
        assert!(
            resolver
                .all_configured_name_servers()
                .iter()
                .any(|cfg| cfg.socket_addr.ip() == IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)))
        );
        assert!(
            resolver
                .all_configured_name_servers()
                .iter()
                .all(|cfg| cfg.socket_addr.ip() != IpAddr::V4(Ipv4Addr::new(9, 9, 9, 9)))
        );

        // check that the shared resolver instance contains a cloudflare address, and does not
        // contain a quad9 address that it would contain by default if the assignment had failed.
        assert!(
            SHARED_RESOLVER
                .read()
                .unwrap()
                .all_configured_name_servers()
                .iter()
                .any(|cfg| cfg.socket_addr.ip() == IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)))
        );
        assert!(
            SHARED_RESOLVER
                .read()
                .unwrap()
                .all_configured_name_servers()
                .iter()
                .all(|cfg| cfg.socket_addr.ip() != IpAddr::V4(Ipv4Addr::new(9, 9, 9, 9)))
        );
    }

    // /// Triple check that ip_strategy in ResolverOpts does NOT impact the IP version of the
    // /// selected nameservers.
    // ///
    // /// => Looking at logs, yes it still uses IPv6 nameservers.
    // #[tokio::test]
    // async fn resolver_ipv6_triple_check() {
    //     // tracing_subscriber::fmt()
    //     //     .with_max_level(tracing::Level::DEBUG)
    //     //     .init();

    //     let ns_group = default_nameserver_group();
    //     let mut options = ResolverOpts::default();
    //     options.ip_strategy = LookupIpStrategy::Ipv4Only;
    //     options.num_concurrent_reqs= 4;

    //     let resolver = configure_and_build_resolver(ns_group, Some(options)).unwrap();

    //     let _ = resolver.lookup_ip("example.com").await.unwrap();
    // }
}

#[cfg(test)]
mod failure_test {
    use super::*;
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

        configure_and_build_resolver(broken_ns_group, None)
    }

    #[tokio::test]
    async fn dns_lookup_failures() -> Result<(), ResolveError> {
        let time_start = std::time::Instant::now();

        // create a new resolver that won't mess with the shared resolver used by other tests
        let resolver = HickoryDnsResolver {
            dont_use_shared: true,
            state: Arc::new(OnceCell::with_value(build_broken_resolver().unwrap())),
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
        // create a new resolver that won't mess with the shared resolver used by other tests
        let mut resolver = HickoryDnsResolver {
            dont_use_shared: true,
            state: Arc::new(OnceCell::with_value(build_broken_resolver().unwrap())),
            static_base: Some(Default::default()),
            overall_dns_timeout: Duration::from_secs(5),
            ..Default::default()
        };

        // do a lookup where we check the static table as a fallback
        let start = Instant::now();
        // successful lookup using fallback to static resolver
        let domain = "nymvpn.com";
        let _ = resolver
            .resolve_str(domain)
            .await
            .expect("failed to resolve address in static lookup");

        let lookup_duration = Instant::now() - start;
        assert!(lookup_duration > Duration::from_secs(5));

        // do a lookup where we check the static table first
        resolver.set_check_static_fallback_first(true);
        let start = Instant::now();
        // successful lookup using fallback to static resolver
        let domain = "nymvpn.com";
        let _ = resolver
            .resolve_str(domain)
            .await
            .expect("failed to resolve address in static lookup");

        let lookup_duration = Instant::now() - start;
        assert!(lookup_duration < Duration::from_millis(50));

        // unsuccessful lookup - primary times out, and not in static table
        let domain = "non-existent.nymtech.net";
        let result = resolver.resolve_str(domain).await;
        assert!(result.is_err_and(|e| matches!(e, ResolveError::Timeout)));

        Ok(())
    }

    /// This test is meant to check if shifting the lookup in our configured static table forward
    /// means that the http request will succeed, in the situation where the DNS timeout and HTTP
    /// request timeout would align IF we had to wait for the DNS lookup to reach its timeout.
    #[tokio::test]
    async fn reqwest_using_static_fallback() -> Result<(), ResolveError> {
        // create a new resolver that won't mess with the shared resolver used by other tests
        let resolver = HickoryDnsResolver {
            dont_use_shared: true,
            state: Arc::new(OnceCell::with_value(build_broken_resolver().unwrap())),
            static_base: Some(Default::default()),
            ..Default::default()
        };

        let client = reqwest::ClientBuilder::new()
            .dns_resolver(resolver.clone().into())
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        // Because the static table is checked first there is no delay and the lookup succeeds
        // immediately. This means that (for hosts with an entry in the static lookup table) there
        // should no longer be an issue with timeouts or filling the hickory resolvers query buffer.
        let resp = client
            .get("https://nymvpn.com/api/public/v1/health")
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();

        assert!(!resp.is_empty());
        Ok(())
    }

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

        let inner = configure_and_build_resolver(broken_ns_https, None).unwrap();

        // create a new resolver that won't mess with the shared resolver used by other tests
        let resolver = HickoryDnsResolver {
            dont_use_shared: true,
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

    #[tokio::test]
    async fn trial_nameservers_reconfigure_none_working() {
        let broken_ns_group = NameServerConfigGroup::from_ips_https(
            GUARANTEED_BROKEN_IPS_1,
            443,
            "cloudflare-dns.com".to_string(),
            true,
        );

        let inner = configure_and_build_resolver(broken_ns_group.clone(), None).unwrap();

        // create a new resolver that won't mess with the shared resolver used by other tests
        let mut resolver = HickoryDnsResolver {
            dont_use_shared: true,
            state: Arc::new(OnceCell::with_value(inner)),
            static_base: Some(Default::default()),
            overall_dns_timeout: Duration::from_secs(5),
            default_nameserver_config_group: broken_ns_group.into(),
            ..Default::default()
        };

        let res = resolver.trial_nameservers_and_reconfigure().await;
        assert!(res.is_err_and(
            |e| matches!(e, ResolveError::ConfigError(msg) if msg == RECONFIGURE_ERROR_MSG)
        ));
    }

    #[tokio::test]
    async fn trial_nameservers_independent_reconfigure() {
        let good_cf_ip = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));

        let mut ns_ips = GUARANTEED_BROKEN_IPS_1.to_vec();
        ns_ips.push(good_cf_ip);

        let broken_ns_https = NameServerConfigGroup::from_ips_https(
            &ns_ips,
            443,
            "cloudflare-dns.com".to_string(),
            true,
        );

        let inner = configure_and_build_resolver(broken_ns_https.clone(), None).unwrap();

        // create a new resolver that won't mess with the shared resolver used by other tests
        let mut resolver = HickoryDnsResolver {
            dont_use_shared: true,
            state: Arc::new(OnceCell::with_value(inner)),
            static_base: Some(Default::default()),
            default_nameserver_config_group: broken_ns_https.into(),
            ..Default::default()
        };

        resolver.trial_nameservers_and_reconfigure().await.unwrap();

        let ns_set = resolver.state.get().unwrap().config().name_servers();
        let addrs: Vec<IpAddr> = ns_set.iter().map(|cfg| cfg.socket_addr.ip()).collect();
        assert_eq!(addrs.len(), 1);
        assert!(addrs.contains(&good_cf_ip));
    }

    /// This test ensures that calling `trial_nameservers_and_reconfigure` on a resolver using the
    /// shared resolver results in the shared resolver updating its nameserver set to use only the
    /// working nameservers. From the caller perspective this should have the same result.
    #[tokio::test]
    // ignore this test as changes to the shared resolver can cause unexpected behavior when
    // interleaved with other tests
    #[ignore]
    async fn trial_nameservers_shared_reconfigure() {
        let good_cf_ip = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));

        let mut ns_ips = GUARANTEED_BROKEN_IPS_1.to_vec();
        ns_ips.push(good_cf_ip);

        let broken_ns_https = NameServerConfigGroup::from_ips_https(
            &ns_ips,
            443,
            "cloudflare-dns.com".to_string(),
            true,
        );

        let inner = configure_and_build_resolver(broken_ns_https, None).unwrap();
        SHARED_RESOLVER.write().unwrap().state = Arc::new(OnceCell::with_value(inner));

        let mut resolver = HickoryDnsResolver::default();
        resolver.trial_nameservers_and_reconfigure().await.unwrap();

        let binding = SHARED_RESOLVER.read().unwrap();
        let ns_set = binding.state.get().unwrap().config().name_servers();
        let addrs: Vec<IpAddr> = ns_set.iter().map(|cfg| cfg.socket_addr.ip()).collect();
        assert_eq!(addrs.len(), 1);
        assert!(addrs.contains(&good_cf_ip));
    }
}
