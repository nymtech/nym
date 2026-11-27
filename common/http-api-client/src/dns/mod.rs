// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! DNS resolver configuration for internal lookups.
//!
//! The resolver itself is the combination of the Cloudflare and Quad9 endpoints supporting DoH
//! and DoT.
//!
//! ```rust
//! use nym_http_api_client::HickoryDnsResolver;
//! # use nym_http_api_client::ResolveError;
//! # type Err = ResolveError;
//! # async fn run() -> Result<(), Err> {
//! let resolver = HickoryDnsResolver::new();
//! resolver.resolve_str("example.com").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Fallbacks
//!
//! **System Resolver --** This resolver supports an optional fallback mechanism where, should the
//! DNS-over-TLS resolution fail, a followup resolution will be done using the hosts configured
//! default (e.g. `/etc/resolv.conf` on linux).
//!
//! This is disabled by default and can be enabled using [`HickoryDnsResolver::use_system_resolver`].
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
//! ## Connection provider
//!
//! [`HickoryDnsResolver`] is generic over the [`ConnectionProvider`] used by the underlying
//! `hickory-resolver` crate, defaulting to [`TokioRuntimeProvider`] so that it behaves exactly
//! like a `TokioResolver` wrapper out of the box. A different connection provider can be supplied
//! by naming it explicitly, e.g. `HickoryDnsResolver::<MyProvider>::default()`, though process-wide
//! sharing (see [`HickoryDnsResolver::shared`]) is only available for the default provider.
//!
//! ---
//!
//! Requires the `https-ring`, `tls-ring`, `webpki-roots` features for the `hickory-resolver` crate
#![deny(missing_docs)]

use crate::ClientBuilder;

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicBool, Ordering::Relaxed},
    },
    time::Duration,
};

use arc_swap::ArcSwap;
use hickory_resolver::{
    ConnectionProvider, Resolver,
    config::{CLOUDFLARE, NameServerConfig, QUAD9, ResolverConfig, ResolverOpts},
    net::{NetError, runtime::TokioRuntimeProvider},
};
use once_cell::sync::OnceCell;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use tracing::*;

use nym_network_defaults::mainnet::dns as constants;
mod static_resolver;
mod trial;
pub(crate) use static_resolver::*;

pub(crate) const DEFAULT_POSITIVE_LOOKUP_CACHE_TTL: Duration = Duration::from_secs(1800);
pub(crate) const DEFAULT_OVERALL_LOOKUP_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const DEFAULT_QUERY_TIMEOUT: Duration = Duration::from_secs(5);

impl ClientBuilder {
    /// Override the DNS resolver implementation used by the underlying http client.
    /// This forces the use of an independent request executor (via [`Self::non_shared`]).
    pub fn dns_resolver<R: Resolve + 'static>(mut self, resolver: Arc<R>) -> Self {
        self = self.non_shared();
        // because of the call to non-shared this conditional should always run.
        if let Some(rb) = self.reqwest_client_builder {
            self.reqwest_client_builder = Some(rb.dns_resolver(resolver));
        }
        self.use_secure_dns = false;
        self
    }

    /// Disables the hickory-dns async resolver in favor of the `reqwest` default threadpool using
    /// `getaddrinfo`.
    ///
    /// If [`Self::dns_resolver`] is called, there is no need to call this as well.
    ///
    /// This forces the use of an independent request executor (via [`Self::non_shared`]).
    pub fn no_hickory_dns(mut self) -> Self {
        self = self.non_shared();
        self.use_secure_dns = false;
        self
    }
}

// n.b. static items do not call [`Drop`] on program termination, so this won't be deallocated.
// this is fine, as the OS can deallocate the terminated program faster than we can free memory
// but tools like valgrind might report "memory leaks" as it isn't obvious this is intentional.
static SHARED_RESOLVER: LazyLock<HickoryDnsResolver> = LazyLock::new(|| {
    tracing::debug!("Initializing shared DNS resolver");
    HickoryDnsResolver::thread_resolver()
});

/// Associates a [`ConnectionProvider`] with the process-wide shared resolver instance backing
/// [`HickoryDnsResolver::shared`], if one exists for that provider.
///
/// [`TokioRuntimeProvider`] is the only connection provider with shared state; every other
/// connection provider always builds an independent resolver.
pub trait SharedResolverState: ConnectionProvider + Default {
    /// The process-wide shared resolver instance for this connection provider, if one exists.
    fn shared_resolver() -> Option<&'static HickoryDnsResolver<Self>> {
        None
    }
}

impl SharedResolverState for TokioRuntimeProvider {
    fn shared_resolver() -> Option<&'static HickoryDnsResolver<Self>> {
        Some(&SHARED_RESOLVER)
    }
}

#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
/// Error occurring while resolving a hostname into an IP address.
pub enum ResolveError {
    #[error("invalid name: {0}")]
    InvalidNameError(String),
    #[error("hickory-dns resolver error: {0}")]
    ResolveError(#[from] NetError),
    #[error("high level lookup timed out")]
    Timeout,
    #[error("hostname not found in static lookup table")]
    StaticLookupMiss,
}

impl ResolveError {
    /// Returns true if the error is a timeout.
    pub fn is_timeout(&self) -> bool {
        matches!(
            self,
            ResolveError::Timeout | ResolveError::ResolveError(NetError::Timeout)
        )
    }
}

/// Wrapper around a `hickory-resolver` [`Resolver`], which implements the `Resolve` trait.
///
/// Generic over the [`ConnectionProvider`] used by the underlying resolver, defaulting to
/// [`TokioRuntimeProvider`] (i.e. a `TokioResolver`) so existing callers are unaffected. See the
/// [module docs](self#connection-provider) for details on using a different provider.
///
/// Typical use involves instantiating using the `Default` implementation and then resolving using
/// methods or trait implementations.
///
/// The default initialization is a handle to the process-wide resolver state, so a change made
/// through one handle is seen by all of them. If independent state is required use
/// `thread_resolver()`.
#[derive(Debug, Clone)]
pub struct HickoryDnsResolver<C: ConnectionProvider = TokioRuntimeProvider> {
    inner: Arc<Inner<C>>,
    /// Overall timeout for dns lookup associated with any individual host resolution. For example,
    /// use of retries, server_ordering_strategy, etc. ends absolutely if this timeout is reached.
    overall_dns_timeout: Duration,
}

#[derive(Debug)]
struct Inner<C: ConnectionProvider> {
    // Since we might not have been called in the context of a
    // Tokio Runtime during initialization, we must delay the actual
    // construction of the resolver. This is swappable (rather than a plain `OnceCell`) so that
    // [`HickoryDnsResolver::set_name_servers`] can invalidate it and force a rebuild against the
    // new nameserver group on the next lookup.
    state: ArcSwap<OnceCell<Resolver<C>>>,
    use_system: AtomicBool,
    system_resolver: OnceCell<Resolver<C>>,
    static_base: OnceCell<StaticResolver>,
    /// Nameserver group used to build `state` when it needs (re)constructing.
    name_servers: ArcSwap<Vec<NameServerConfig>>,
}

impl<C: ConnectionProvider> Default for Inner<C> {
    fn default() -> Self {
        Self {
            state: Default::default(),
            use_system: AtomicBool::new(false),
            system_resolver: Default::default(),
            static_base: Default::default(),
            name_servers: ArcSwap::from_pointee(default_nameserver_group_ipv4_only()),
        }
    }
}

impl<C: SharedResolverState> Default for HickoryDnsResolver<C> {
    fn default() -> Self {
        Self::shared()
    }
}

impl HickoryDnsResolver<TokioRuntimeProvider> {
    /// Construct the default, Tokio-backed resolver.
    ///
    /// Prefer this over `Default::default()` / `HickoryDnsResolver::default()`: since
    /// [`HickoryDnsResolver`] is generic over its connection provider, `Default` is implemented for
    /// every valid provider. Building a `HickoryDnsResolver` through the `Default` trait therefore
    /// requires a type annotation, e.g. `let resolver: HickoryDnsResolver<TokioRuntimeProvider> =
    /// HickoryDnsResolver::default();`. `new` is inherent to [`TokioRuntimeProvider`] specifically,
    /// so it resolves without an explicit type annotation.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<C: SharedResolverState> Resolve for HickoryDnsResolver<C> {
    fn resolve(&self, name: Name) -> Resolving {
        let resolver = match self.resolver() {
            Ok(r) => r,
            Err(err) => return Box::pin(return_err(err)),
        };

        let static_resolver = self.static_resolver().clone();
        let overall_dns_timeout = self.overall_dns_timeout;
        Box::pin(async move {
            resolve(name, resolver, static_resolver, overall_dns_timeout)
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        })
    }
}

async fn return_err(e: ResolveError) -> Result<Addrs, Box<dyn std::error::Error + Send + Sync>> {
    Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

async fn resolve<C: ConnectionProvider>(
    name: Name,
    resolver: Resolver<C>,
    static_resolver: StaticResolver,
    overall_dns_timeout: Duration,
) -> Result<Addrs, ResolveError> {
    // try checking the static table to see if any of the addresses in the table have been
    // looked up previously within the timeout to where we are not yet ready to try the
    // default resolver yet again.
    if let Some(addrs) = static_resolver.pre_resolve(name.as_str()) {
        let addrs: Addrs = Box::new(addrs.into_iter().map(|ip_addr| SocketAddr::new(ip_addr, 0)));
        return Ok(addrs);
    }

    // Attempt a lookup using the primary resolver
    let resolve_fut = tokio::time::timeout(overall_dns_timeout, resolver.lookup_ip(name.as_str()));
    let primary_err = match resolve_fut.await {
        Err(_) => ResolveError::Timeout,
        Ok(Ok(lookup)) => {
            // Shuffle so that successive connection attempts cycle through all
            // returned IPs rather than always hitting the same first address.
            let mut ips = Vec::from_iter(lookup.iter());
            fastrand::shuffle(&mut ips);
            let addrs: Addrs = Box::new(ips.into_iter().map(|ip| SocketAddr::new(ip, 0)));
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

    // If no record has been found check the static table of fallback addresses for our entry
    debug!("checking static");
    if let Ok(addrs) = static_resolver.resolve(name).await {
        return Ok(addrs);
    }

    Err(primary_err)
}

impl<C: SharedResolverState> HickoryDnsResolver<C> {
    /// Returns a handle to the process-wide shared resolver state for this connection provider, if
    /// one exists (see [`SharedResolverState`]). Falls back to independent state (as if built via
    /// [`Self::thread_resolver`]) for connection providers without shared state.
    pub fn shared() -> Self {
        C::shared_resolver()
            .cloned()
            .unwrap_or_else(Self::thread_resolver)
    }

    /// Attempt to resolve a domain name to a set of [`IpAddr`]s
    pub async fn resolve_str(
        &self,
        name: &str,
    ) -> Result<impl Iterator<Item = IpAddr> + use<C>, ResolveError> {
        let n =
            Name::from_str(name).map_err(|_| ResolveError::InvalidNameError(name.to_string()))?;

        resolve(
            n,
            self.resolver()?,
            self.static_resolver().clone(),
            self.overall_dns_timeout,
        )
        .await
        .map(|addrs| addrs.map(|socket_addr| socket_addr.ip()))
    }

    /// Create a (lazy-initialized) resolver with its own state, independent of the shared one.
    pub fn thread_resolver() -> Self {
        Self {
            inner: Default::default(),
            overall_dns_timeout: DEFAULT_OVERALL_LOOKUP_TIMEOUT,
        }
    }

    /// The system resolver when it is selected, otherwise the (built or already cached) resolver
    /// for the currently configured nameserver group.
    fn resolver(&self) -> Result<Resolver<C>, ResolveError> {
        if self.inner.use_system.load(Relaxed) {
            self.inner
                .system_resolver
                .get_or_try_init(new_resolver_system::<C>)
                .cloned()
        } else {
            self.inner
                .state
                .load()
                .get_or_try_init(|| {
                    configure_and_build_resolver::<C>(
                        self.inner.name_servers.load_full().as_ref().clone(),
                    )
                })
                .cloned()
        }
    }

    fn static_resolver(&self) -> &StaticResolver {
        self.inner
            .static_base
            .get_or_init(new_default_static_fallback)
    }

    /// Swap the primary internal resolver to the system resolver rather than the
    /// configured custom resolver.
    pub fn use_system_resolver(&self) {
        self.inner.use_system.store(true, Relaxed);
    }

    /// Swap the primary internal resolver to the configured custom resolver rather than the
    /// system resolver.
    pub fn use_configured_resolver(&self) {
        self.inner.use_system.store(false, Relaxed);
    }

    /// Clear entries from the static table that would return entries during the pre-resolve stage.
    /// This means that all lookups will attempt to use the network resolver again before the static
    /// table is consulted.
    ///
    /// Entries elevated to pre-resolve from fallback (added from default or using
    /// [`Self::set_fallback_addrs`]) will have their cache timeout cleared. Entries added directly
    /// to pre-resolve (using [`Self::set_static_preresolve`]) will be removed.
    pub fn clear_preresolve(&self) {
        debug!("clearing pre-resolve table");
        if let Some(static_base) = self.inner.static_base.get() {
            static_base.clear_preresolve()
        }
    }

    /// Get the current map of hostnames to addresses used in the fallback static lookup stage if one
    /// exists.
    pub fn get_static_fallbacks(&self) -> Option<HashMap<String, Vec<IpAddr>>> {
        Some(self.inner.static_base.get()?.get_fallback_addrs())
    }

    /// Set (or overwrite) the map of addresses used in the fallback static hostname lookup.
    pub fn set_fallback_addrs(&self, addrs: HashMap<String, Vec<IpAddr>>) {
        debug!("setting fallback entries for {:?}", addrs.keys());
        self.static_resolver().set_fallback(addrs);
    }

    /// Get the current map of hostnames to addresses used in the preresolve static lookup stage
    /// if one exists.
    pub fn get_static_preresolve(&self) -> Option<HashMap<String, Vec<IpAddr>>> {
        Some(self.inner.static_base.get()?.get_preresolve_addrs())
    }

    /// Set (or overwrite) the map of addresses used in the preresolve static hostname lookup.
    pub fn set_static_preresolve(&self, addrs: HashMap<String, Vec<IpAddr>>) {
        debug!("setting pre-resolve entries for {:?}", addrs.keys());
        self.static_resolver().set_preresolve(addrs);
    }

    /// Get the full default set of known nameserver configs (Cloudflare and Quad9, DoT and DoH,
    /// IPv4 and IPv6). Unlike [`Self::get_name_servers`], this is independent of any override
    /// applied via [`Self::set_name_servers`].
    pub fn default_name_servers(&self) -> Vec<NameServerConfig> {
        default_nameserver_group()
    }

    /// Get the nameserver group currently configured for this resolver, reflecting any change
    /// made via [`Self::set_name_servers`].
    pub fn get_name_servers(&self) -> Vec<NameServerConfig> {
        self.inner.name_servers.load_full().as_ref().clone()
    }

    /// Set (or overwrite) the nameserver group used by this resolver. Since the underlying
    /// resolver is immutable once built, this invalidates the internal (cached, lazily-built)
    /// resolver: the next lookup made with this resolver rebuilds it using the new nameservers.
    ///
    /// Every handle to the same state, e.g. all shared ones, sees the change.
    pub fn set_name_servers(&self, name_servers: Vec<NameServerConfig>) {
        debug!("setting nameserver group to {name_servers:?}");
        self.inner.name_servers.store(Arc::new(name_servers));
        self.inner.state.store(Arc::new(OnceCell::new()));
    }
}

/// Successfully resolved addresses are cached for a minimum of 30 minutes Individual lookup
/// timeouts are set to `DEFAULT_QUERY_TIMEOUT` (5 seconds) Retries after lookup failure are
/// disabled (`attempts = 0`) Lookup order is set to (default) A then AAAA Number or parallel lookup
/// is set to (default) 2 Nameserver selection uses the (default) EWMA statistics / performance
/// based strategy
fn default_options() -> ResolverOpts {
    let mut opts = ResolverOpts::default();
    // Always cache successful responses for queries received by this resolver for 30 min minimum.
    opts.positive_min_ttl = Some(DEFAULT_POSITIVE_LOOKUP_CACHE_TTL);
    opts.timeout = DEFAULT_QUERY_TIMEOUT;
    opts.attempts = 0;

    opts
}

fn configure_and_build_resolver<C: ConnectionProvider + Default>(
    name_servers: Vec<NameServerConfig>,
) -> Result<Resolver<C>, ResolveError> {
    let options = default_options();
    info!("building new configured resolver");
    debug!("configuring resolver with {options:?}, {name_servers:?}");

    let config = ResolverConfig::from_parts(None, Vec::new(), name_servers);
    let mut resolver_builder = Resolver::<C>::builder_with_config(config, C::default());

    resolver_builder = resolver_builder.with_options(options);

    Ok(resolver_builder.build()?)
}

fn filter_ipv4(nameservers: impl IntoIterator<Item = NameServerConfig>) -> Vec<NameServerConfig> {
    nameservers
        .into_iter()
        .filter(|ns| ns.ip.is_ipv4())
        .collect()
}

#[allow(unused)]
fn filter_ipv6(nameservers: impl IntoIterator<Item = NameServerConfig>) -> Vec<NameServerConfig> {
    nameservers
        .into_iter()
        .filter(|ns| ns.ip.is_ipv6())
        .collect()
}

fn default_nameserver_group() -> Vec<NameServerConfig> {
    QUAD9
        .tls()
        .chain(QUAD9.https())
        .chain(CLOUDFLARE.tls())
        .chain(CLOUDFLARE.https())
        .collect()
}

fn default_nameserver_group_ipv4_only() -> Vec<NameServerConfig> {
    filter_ipv4(default_nameserver_group())
}

#[allow(unused)]
fn default_nameserver_group_ipv6_only() -> Vec<NameServerConfig> {
    filter_ipv6(default_nameserver_group())
}

/// Create a new resolver with the default configuration, which reads from the system DNS config
/// (i.e. `/etc/resolv.conf` in unix). The options are overridden to look up for both IPv4 and IPv6
/// addresses to work with "happy eyeballs" algorithm.
fn new_resolver_system<C: ConnectionProvider + Default>() -> Result<Resolver<C>, ResolveError> {
    let mut resolver_builder = Resolver::<C>::builder(C::default())?;

    let options = default_options();
    info!("building new fallback system resolver");
    debug!("fallback system resolver with {options:?}");

    resolver_builder = resolver_builder.with_options(options);

    Ok(resolver_builder.build()?)
}

fn new_default_static_fallback() -> StaticResolver {
    StaticResolver::new().with_fallback(constants::default_static_addrs())
}

#[cfg(test)]
mod test;
