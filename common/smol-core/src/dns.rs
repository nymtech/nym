// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Tunnel-scoped DNS resolver.
//!
//! Performs A/AAAA lookups over a stack [`UdpSocket`], so name resolution
//! travels through the same transport as the rest of the tunnel traffic rather
//! than leaking to the host's system resolver. This is the `smol-core`
//! equivalent of `smolmix`'s in-mixnet DNS approach.

use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;

use hickory_proto::op::{Message, MessageType, OpCode, Query};
use hickory_proto::rr::{Name, RData, RecordType};
use tokio_smoltcp::Net;

use crate::error::{Result, SmolCoreError};

/// Default upstream DNS server used inside the tunnel (Cloudflare).
pub const DEFAULT_DNS_SERVER: SocketAddr =
    SocketAddr::new(IpAddr::V4(std::net::Ipv4Addr::new(1, 1, 1, 1)), 53);

/// Default per-query timeout.
pub const DEFAULT_QUERY_TIMEOUT: Duration = Duration::from_secs(5);

// Monotonic-ish DNS transaction id source (single in-flight query per socket,
// but distinct ids avoid confusing a resolver that reuses a socket).
static TXN_ID: AtomicU16 = AtomicU16::new(1);

/// Configuration for the tunnel DNS resolver.
#[derive(Clone, Copy, Debug)]
pub struct DnsConfig {
    /// Upstream server to query (reached through the tunnel).
    pub server: SocketAddr,
    /// Per-query timeout.
    pub timeout: Duration,
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            server: DEFAULT_DNS_SERVER,
            timeout: DEFAULT_QUERY_TIMEOUT,
        }
    }
}

/// Resolve `host` to a list of IP addresses over the given stack, querying both
/// A and AAAA records. Every DNS packet is sent through a stack UDP socket.
pub(crate) async fn resolve(net: &Net, cfg: &DnsConfig, host: &str) -> Result<Vec<IpAddr>> {
    let name = Name::from_utf8(host).map_err(|e| SmolCoreError::DnsProto(e.to_string()))?;

    let mut addrs = Vec::new();
    // A and AAAA are independent queries; collect whatever resolves.
    for rtype in [RecordType::A, RecordType::AAAA] {
        match query_one(net, cfg, &name, rtype).await {
            Ok(mut a) => addrs.append(&mut a),
            // A missing AAAA (or vice versa) is normal; only propagate if both
            // fail, which surfaces as an empty set below.
            Err(SmolCoreError::DnsNoRecords { .. }) => {}
            Err(e) => tracing::debug!("DNS {rtype} query for {host} failed: {e}"),
        }
    }

    if addrs.is_empty() {
        return Err(SmolCoreError::DnsNoRecords {
            name: host.to_string(),
        });
    }
    Ok(addrs)
}

async fn query_one(
    net: &Net,
    cfg: &DnsConfig,
    name: &Name,
    rtype: RecordType,
) -> Result<Vec<IpAddr>> {
    let id = TXN_ID.fetch_add(1, Ordering::Relaxed);

    let mut msg = Message::new(id, MessageType::Query, OpCode::Query);
    msg.metadata.recursion_desired = true;
    msg.add_query(Query::query(name.clone(), rtype));
    let query_bytes = msg
        .to_vec()
        .map_err(|e| SmolCoreError::DnsProto(e.to_string()))?;

    let socket = net
        .udp_bind("0.0.0.0:0".parse().expect("valid bind addr"))
        .await?;
    socket.send_to(&query_bytes, cfg.server).await?;

    let mut buf = vec![0u8; 1500];
    let (len, _src) = tokio::time::timeout(cfg.timeout, socket.recv_from(&mut buf))
        .await
        .map_err(|_| SmolCoreError::DnsTimeout {
            name: name.to_utf8(),
        })??;

    let response =
        Message::from_vec(&buf[..len]).map_err(|e| SmolCoreError::DnsProto(e.to_string()))?;

    let addrs: Vec<IpAddr> = response
        .answers
        .iter()
        .filter_map(|record| match &record.data {
            RData::A(a) => Some(IpAddr::V4(a.0)),
            RData::AAAA(aaaa) => Some(IpAddr::V6(aaaa.0)),
            _ => None,
        })
        .collect();

    if addrs.is_empty() {
        return Err(SmolCoreError::DnsNoRecords {
            name: name.to_utf8(),
        });
    }
    Ok(addrs)
}
