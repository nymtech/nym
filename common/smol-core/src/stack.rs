// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The userspace TCP/IP stack.
//!
//! [`Stack`] wraps a tokio-smoltcp [`Net`] driven by any [`AsyncDevice`] — most
//! commonly a [`ChannelDevice`](crate::ChannelDevice) fed by an abstract
//! IP-packet transport. It exposes tokio-native [`TcpStream`] / [`UdpSocket`]
//! sockets and a tunnel-scoped DNS resolver, with no OS `tun` device and no
//! elevated privileges.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use smoltcp::iface::Config;
use smoltcp::wire::{HardwareAddress, IpAddress, IpCidr};
use tokio_smoltcp::device::AsyncDevice;
use tokio_smoltcp::{BufferSize, Net, NetConfig};

use crate::dns::{self, DnsConfig};
use crate::error::Result;

pub use tokio_smoltcp::{TcpStream, UdpSocket};

/// Default per-socket TCP buffer size (bytes). The receive buffer *is* the TCP
/// receive window, so this caps throughput to `buffer / RTT`. tokio-smoltcp's own
/// default is only 8 KiB, which throttles bulk transfers over a higher-RTT tunnel
/// (e.g. two-hop); 512 KiB gives a window sized to a realistic path BDP.
pub const DEFAULT_TCP_BUFFER: usize = 512 * 1024;

/// Address configuration for the virtual interface.
///
/// The stack is configured with the tunnel's assigned IPv4 address as a `/32`;
/// outbound packets carry it as source. An optional IPv6 address is accepted
/// for forward-compatibility, but `tokio-smoltcp` 0.5 binds a single interface
/// address, so dual-stack is not yet wired end-to-end.
///
/// The interface MTU is not configured here — it is fixed by the caller-supplied
/// [`AsyncDevice`] (e.g. [`ChannelDevice`](crate::ChannelDevice)), which is the single source of
/// truth for MTU.
#[derive(Clone, Copy, Debug)]
pub struct StackConfig {
    /// Assigned tunnel IPv4 address.
    pub ipv4: Ipv4Addr,
    /// Assigned tunnel IPv6 address (accepted; dual-stack deferred).
    pub ipv6: Option<Ipv6Addr>,
    /// Per-socket TCP rx/tx buffer size (bytes); the rx side is the TCP receive
    /// window and the dominant throughput lever on a higher-RTT tunnel.
    pub tcp_buffer: usize,
}

impl StackConfig {
    /// Configure the stack with an assigned IPv4 address.
    pub fn new(ipv4: Ipv4Addr) -> Self {
        Self {
            ipv4,
            ipv6: None,
            tcp_buffer: DEFAULT_TCP_BUFFER,
        }
    }

    /// Set the assigned IPv6 address.
    ///
    /// **Warning:** dual-stack is not yet wired end-to-end — `tokio-smoltcp` 0.5 binds a single
    /// interface address, so today the stack operates as IPv4-only and this address has no effect on
    /// routing. Accepted for forward-compatibility only.
    #[must_use]
    pub fn with_ipv6(mut self, ipv6: Ipv6Addr) -> Self {
        self.ipv6 = Some(ipv6);
        self
    }

    /// Set the per-socket TCP buffer size (the TCP window; see [`DEFAULT_TCP_BUFFER`]).
    #[must_use]
    pub fn with_tcp_buffer(mut self, tcp_buffer: usize) -> Self {
        self.tcp_buffer = tcp_buffer;
        self
    }
}

/// A pure-Rust userspace TCP/IP stack exposing tokio sockets over an abstract
/// IP-packet transport.
pub struct Stack {
    net: Net,
    dns: DnsConfig,
    config: StackConfig,
}

impl Stack {
    /// Build a stack over `device`, configuring the smoltcp interface with the
    /// assigned tunnel address(es) from `config`.
    ///
    /// `Net::new` spawns the smoltcp reactor as a background task; after this,
    /// `tcp_connect` / `udp_bind` create sockets managed by that reactor.
    pub fn new<D: AsyncDevice + 'static>(device: D, config: StackConfig) -> Self {
        let iface_config = Config::new(HardwareAddress::Ip);
        let mut net_config = NetConfig::new(
            iface_config,
            IpCidr::new(IpAddress::Ipv4(config.ipv4), 32),
            // Default route via the unspecified address; the transport does the
            // actual routing (the tunnel exit gateway, mixnet IPR, etc.).
            vec![IpAddress::Ipv4(Ipv4Addr::UNSPECIFIED)],
        );
        // Widen the TCP window beyond tokio-smoltcp's 8 KiB default (the dominant
        // throughput cap on a higher-RTT tunnel). UDP/raw keep their defaults.
        net_config.buffer_size = BufferSize {
            tcp_rx_size: config.tcp_buffer,
            tcp_tx_size: config.tcp_buffer,
            ..Default::default()
        };

        Self {
            net: Net::new(device, net_config),
            dns: DnsConfig::default(),
            config,
        }
    }

    /// Override the tunnel DNS configuration (upstream server, timeout).
    #[must_use]
    pub fn with_dns_config(mut self, dns: DnsConfig) -> Self {
        self.dns = dns;
        self
    }

    /// The assigned tunnel IPv4 address.
    pub fn ipv4(&self) -> Ipv4Addr {
        self.config.ipv4
    }

    /// Open a TCP connection to `addr` through the transport.
    ///
    /// The returned [`TcpStream`] implements `tokio::io::AsyncRead + AsyncWrite`.
    ///
    /// # Errors
    /// Returns an I/O error if the handshake fails (refused/timeout) or the
    /// stack has shut down.
    pub async fn tcp_connect(&self, addr: SocketAddr) -> Result<TcpStream> {
        Ok(self.net.tcp_connect(addr).await?)
    }

    /// Bind a UDP socket to an ephemeral port.
    pub async fn udp_socket(&self) -> Result<UdpSocket> {
        Ok(self.net.udp_bind(([0, 0, 0, 0], 0).into()).await?)
    }

    /// Bind a UDP socket to a specific local port.
    pub async fn udp_socket_on(&self, port: u16) -> Result<UdpSocket> {
        Ok(self.net.udp_bind(([0, 0, 0, 0], port).into()).await?)
    }

    /// Resolve `host` to IP addresses over the tunnel, using the configured
    /// upstream DNS server. Each query travels through a stack UDP socket, not
    /// the host resolver.
    ///
    /// A host that is already an IP literal is returned as-is with no DNS query.
    pub async fn resolve(&self, host: &str) -> Result<Vec<IpAddr>> {
        if let Ok(ip) = host.parse::<IpAddr>() {
            return Ok(vec![ip]);
        }
        // AAAA is only queried once the stack has an IPv6 address; otherwise an AAAA answer would be
        // unroutable on this v4-only interface.
        let want_ipv6 = self.config.ipv6.is_some();
        dns::resolve(&self.net, &self.dns, host, want_ipv6).await
    }

    /// Resolve `host` and open a TCP connection to the first address on `port`.
    /// An IP-literal `host` connects directly, without a DNS query.
    pub async fn tcp_connect_host(&self, host: &str, port: u16) -> Result<TcpStream> {
        let addrs = self.resolve(host).await?;
        let addr = SocketAddr::new(addrs[0], port);
        self.tcp_connect(addr).await
    }

    /// Access the underlying tokio-smoltcp [`Net`] for advanced use.
    pub fn net(&self) -> &Net {
        &self.net
    }
}
