// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

//! DNS resolution through the Nym mixnet.
//!
//! # Why a separate DNS crate?
//!
//! If you resolve hostnames using the OS resolver or a clearnet DNS library,
//! the queries travel over your local network — leaking which domains you're
//! visiting even though the TCP traffic itself goes through the mixnet. This
//! crate routes all DNS queries (both UDP and TCP) through the smolmix
//! [`Tunnel`], so hostname lookups are as private as the rest of your traffic.
//!
//! # How it works
//!
//! [hickory-resolver](https://docs.rs/hickory-resolver)'s extension point is
//! the [`RuntimeProvider`] trait — it controls how the resolver creates TCP
//! connections and UDP sockets. [`SmolmixRuntimeProvider`] implements this
//! trait, routing all I/O through the tunnel:
//!
//! ```text
//! RuntimeProvider::connect_tcp()  →  tunnel.tcp_connect()  →  AsyncIoTokioAsStd<TcpStream>
//! RuntimeProvider::bind_udp()     →  tunnel.udp_socket()   →  SmolmixUdpSocket (newtype)
//! ```
//!
//! hickory expects `futures_io::AsyncRead/Write` for TCP, not tokio's version.
//! `AsyncIoTokioAsStd` (from hickory-proto) adapts between them — and because
//! hickory's `DnsTcpStream` has a blanket impl for any `futures_io::AsyncRead +
//! AsyncWrite`, the wrapped stream satisfies it automatically.
//!
//! For UDP, `SmolmixUdpSocket` is a thin newtype over `tokio_smoltcp::UdpSocket`
//! that implements hickory's [`DnsUdpSocket`](hickory_proto::udp::DnsUdpSocket)
//! — just delegates `poll_recv_from` and `poll_send_to`.
//!
//! # Quick start
//!
//! ```no_run
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
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
//! # Ok(())
//! # }
//! ```
//!
//! # Caching
//!
//! hickory-resolver maintains an internal LRU cache for DNS responses. To
//! benefit from caching, **reuse the [`Resolver`] across requests** rather
//! than creating a new one per lookup. The free function [`resolve()`]
//! constructs a fresh resolver each time and does not cache.
//!
//! # Custom upstream DNS
//!
//! By default, queries go to Quad9 (`9.9.9.9`). Use
//! [`Resolver::with_config()`] for other upstreams:
//!
//! ```no_run
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use smolmix_dns::{Resolver, ResolverConfig};
//!
//! let tunnel = smolmix::Tunnel::new().await?;
//! let resolver = Resolver::with_config(&tunnel, ResolverConfig::cloudflare());
//! # Ok(())
//! # }
//! ```
//!
//! [`RuntimeProvider`]: hickory_proto::runtime::RuntimeProvider

mod provider;

use std::io;
use std::net::SocketAddr;
use std::ops::Deref;

use hickory_resolver::config::NameServerConfigGroup;
use hickory_resolver::name_server::GenericConnector;

use hickory_proto::runtime::TokioHandle;
use smolmix::Tunnel;

/// Re-exported from hickory-resolver. Used with [`Resolver::with_config()`]
/// to select a custom upstream DNS server (Quad9, Cloudflare, Google, etc.).
pub use hickory_resolver::config::ResolverConfig;

/// Re-exported from hickory-resolver. The result of a successful `lookup_ip()`
/// call — iterate with `.iter()` to get `IpAddr` values.
pub use hickory_resolver::lookup_ip::LookupIp;

/// Re-exported from hickory-resolver. The error type for DNS resolution failures.
pub use hickory_resolver::ResolveError;

/// The runtime provider that routes DNS I/O through the tunnel.
///
/// You don't usually need to use this directly — [`Resolver::new()`] wires it
/// up for you. Exposed for advanced use cases (custom resolver configurations
/// beyond what `with_config` covers).
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
    /// Create a resolver using Quad9 (`9.9.9.9`) as upstream DNS.
    pub fn new(tunnel: &Tunnel) -> Self {
        Self::with_config(tunnel, ResolverConfig::quad9())
    }

    /// Create a resolver with a custom upstream DNS configuration.
    ///
    /// IPv6 nameservers are filtered out because the tunnel's smoltcp
    /// interface is IPv4-only (see [`tokio_smoltcp::NetConfig`]). Passing a
    /// config with *only* IPv6 nameservers will cause lookups to fail.
    pub fn with_config(tunnel: &Tunnel, config: ResolverConfig) -> Self {
        // tokio-smoltcp only supports a single IpCidr (IPv4), so contacting
        // an IPv6 nameserver panics in smoltcp's wire layer (IP version
        // mismatch). Strip IPv6 nameservers until the tunnel supports
        // dual-stack.
        let config = ipv4_only_nameservers(config);

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
/// Uses Quad9 (`9.9.9.9`) as the upstream DNS server. Equivalent to
/// [`Resolver::new()`].
pub fn resolver(tunnel: &Tunnel) -> Resolver {
    Resolver::new(tunnel)
}

/// Resolve a hostname through the tunnel (uncached).
///
/// Convenience wrapper for one-shot lookups. Creates a fresh [`Resolver`]
/// internally, so **DNS responses are not cached** across calls. If you're
/// making multiple lookups, create a [`Resolver`] once and reuse it.
pub async fn resolve(tunnel: &Tunnel, host: &str, port: u16) -> io::Result<Vec<SocketAddr>> {
    let r = resolver(tunnel);
    r.resolve(host, port).await
}

/// Strip IPv6 nameservers from a resolver config.
///
/// hickory's preset configs (quad9, cloudflare, google) all include IPv6
/// nameservers. The smolmix tunnel is IPv4-only (tokio-smoltcp's `NetConfig`
/// takes a single `IpCidr`), so sending a UDP packet to an IPv6 nameserver
/// panics in smoltcp's wire layer with "IP version mismatch".
fn ipv4_only_nameservers(config: ResolverConfig) -> ResolverConfig {
    let ipv4_servers: Vec<_> = config
        .name_servers()
        .iter()
        .filter(|ns| ns.socket_addr.is_ipv4())
        .cloned()
        .collect();

    ResolverConfig::from_parts(
        config.domain().cloned(),
        config.search().to_vec(),
        NameServerConfigGroup::from(ipv4_servers),
    )
}
