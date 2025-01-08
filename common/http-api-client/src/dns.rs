//! DNS resolver configuration for internal lookups.
//!
//! The resolver itself is the set combination of the google, cloudflare, and quad9 endpoints
//! supporting DoH, DoT, and for google DoH3 as well.
//!
//! This resolver implements a fallback mechanism where, should the DNS-over-TLS resolution fail, a
//! followup resolution will be done using the hosts configured default (e.g. `/etc/resolve.conf` on
//! linux).
//!
//! Requires the `dns-over-https-rustls`, `dns-over-h3`, `webpki-roots` feature for the
//! `hickory-resolver` crate

use crate::ClientBuilder;

use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;

use hickory_resolver::{
    config::{LookupIpStrategy, NameServerConfigGroup, ResolverConfig, ResolverOpts},
    error::ResolveError,
    lookup_ip::LookupIpIntoIter,
    TokioAsyncResolver,
};
use once_cell::sync::OnceCell;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use tracing::warn;

impl ClientBuilder {
    pub fn dns_resolver<R: Resolve + 'static>(mut self, resolver: Arc<R>) -> Self {
        self.reqwest_client_builder = self.reqwest_client_builder.dns_resolver(resolver);
        self
    }
}

struct SocketAddrs {
    iter: LookupIpIntoIter,
}

#[derive(Debug)]
struct HickoryDnsSystemConfError(ResolveError);

/// Wrapper around an `AsyncResolver`, which implements the `Resolve` trait.
#[derive(Debug, Default, Clone)]
pub struct HickoryDnsResolver {
    /// Since we might not have been called in the context of a
    /// Tokio Runtime in initialization, so we must delay the actual
    /// construction of the resolver.
    state: Arc<OnceCell<TokioAsyncResolver>>,
    fallback: Arc<OnceCell<TokioAsyncResolver>>,
}

impl Resolve for HickoryDnsResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let resolver = self.state.clone();
        let fallback = self.fallback.clone();
        Box::pin(async move {
            let resolver = resolver.get_or_try_init(new_resolver)?;

            // try the primary DNS resolver that we set up (DoH or DoT or whatever)
            let lookup = match resolver.lookup_ip(name.as_str()).await {
                Ok(res) => res,
                Err(e) => {
                    // on failure use the fall back system configured DNS resolver
                    warn!("primary DNS failed w/ error {e}: using system fallback");
                    let resolver = fallback.get_or_try_init(new_resolver_system)?;
                    resolver.lookup_ip(name.as_str()).await?
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

/// Create a new resolver with a custom DoT based configuration. The options are overridden to look
/// up for both IPv4 and IPv6 addresses to work with "happy eyeballs" algorithm.
fn new_resolver() -> Result<TokioAsyncResolver, HickoryDnsSystemConfError> {
    let mut name_servers = NameServerConfigGroup::google_tls();
    name_servers.merge(NameServerConfigGroup::google_https());
    name_servers.merge(NameServerConfigGroup::google_h3());
    name_servers.merge(NameServerConfigGroup::quad9_tls());
    name_servers.merge(NameServerConfigGroup::quad9_https());
    name_servers.merge(NameServerConfigGroup::cloudflare_tls());
    name_servers.merge(NameServerConfigGroup::cloudflare_https());

    let config = ResolverConfig::from_parts(None, Vec::new(), name_servers);

    let mut opts = ResolverOpts::default();
    opts.ip_strategy = LookupIpStrategy::Ipv4thenIpv6;
    // Would like to enable this when 0.25 stabilizes
    // opts.server_ordering_strategy = ServerOrderingStrategy::RoundRobin;

    Ok(TokioAsyncResolver::tokio(config, opts))
}

/// Create a new resolver with the default configuration, which reads from `/etc/resolve.conf`. The
/// options are overridden to look up for both IPv4 and IPv6 addresses to work with "happy eyeballs"
/// algorithm.
fn new_resolver_system() -> Result<TokioAsyncResolver, HickoryDnsSystemConfError> {
    let (config, mut opts) =
        hickory_resolver::system_conf::read_system_conf().map_err(HickoryDnsSystemConfError)?;
    opts.ip_strategy = LookupIpStrategy::Ipv4thenIpv6;
    Ok(TokioAsyncResolver::tokio(config, opts))
}

impl fmt::Display for HickoryDnsSystemConfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("error reading DNS system conf for hickory-dns")
    }
}

impl std::error::Error for HickoryDnsSystemConfError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
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
            .get("http://ifconfig.me")
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();

        assert!(!resp.is_empty());
    }
}
