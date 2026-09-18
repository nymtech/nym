// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Tunnel-scoped DNS resolver.
//!
//! Performs A/AAAA lookups over a stack [`UdpSocket`], so name resolution
//! travels through the same transport as the rest of the tunnel traffic rather
//! than leaking to the host's system resolver. This is the `smol-core`
//! equivalent of `smolmix`'s in-mixnet DNS approach.

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use hickory_proto::op::{Message, MessageType, OpCode, Query, ResponseCode};
use hickory_proto::rr::{Name, RData, RecordType};
use tokio::time::Instant;
use tokio_smoltcp::Net;

use crate::error::{Result, SmolCoreError};

/// Default upstream DNS server used inside the tunnel (Cloudflare).
pub const DEFAULT_DNS_SERVER: SocketAddr =
    SocketAddr::new(IpAddr::V4(std::net::Ipv4Addr::new(1, 1, 1, 1)), 53);

/// Default per-query timeout.
pub const DEFAULT_QUERY_TIMEOUT: Duration = Duration::from_secs(5);

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

/// Resolve `host` to a list of IP addresses over the given stack.
///
/// Queries A, and AAAA only when `want_ipv6` is set — the stack binds a single IPv4 `/32` today, so
/// returning an AAAA answer would hand callers an unroutable address. (When dual-stack lands, the
/// two queries can be issued concurrently with `join!`.)
pub(crate) async fn resolve(
    net: &Net,
    cfg: &DnsConfig,
    host: &str,
    want_ipv6: bool,
) -> Result<Vec<IpAddr>> {
    let name = Name::from_utf8(host).map_err(|e| SmolCoreError::DnsProto(e.to_string()))?;

    let mut rtypes = vec![RecordType::A];
    if want_ipv6 {
        rtypes.push(RecordType::AAAA);
    }

    let mut addrs = Vec::new();
    let mut last_err = None;
    for rtype in rtypes {
        match query_one(net, cfg, &name, rtype).await {
            Ok(mut a) => addrs.append(&mut a),
            // A missing record of one type is normal.
            Err(SmolCoreError::DnsNoRecords { .. }) => {}
            // Remember a server failure / protocol / timeout error, but don't let a failing query
            // for one record type discard addresses another type already resolved (e.g. AAAA
            // SERVFAILs while A succeeded). Only surface it if nothing resolves at all.
            Err(e) => last_err = Some(e),
        }
    }

    if !addrs.is_empty() {
        return Ok(addrs);
    }
    if let Some(e) = last_err {
        return Err(e);
    }
    Err(SmolCoreError::DnsNoRecords {
        name: host.to_string(),
    })
}

async fn query_one(
    net: &Net,
    cfg: &DnsConfig,
    name: &Name,
    rtype: RecordType,
) -> Result<Vec<IpAddr>> {
    // A random transaction id gives off-path spoof resistance; it is validated on the response
    // below (together with the source address). Do not replace with a predictable counter.
    let id: u16 = rand::random();

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

    // Read until a datagram matches our transaction id AND comes from the configured server, or the
    // timeout elapses. A single `recv` would let any stray/spurious datagram delivered to this
    // ephemeral port defeat the lookup with no chance to recover.
    let deadline = Instant::now() + cfg.timeout;
    let mut buf = vec![0u8; 1500];
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or_else(|| SmolCoreError::DnsTimeout {
                name: name.to_utf8(),
            })?;

        let (len, src) = match tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await {
            Ok(res) => res?,
            Err(_) => {
                return Err(SmolCoreError::DnsTimeout {
                    name: name.to_utf8(),
                })
            }
        };

        // Discard datagrams that did not come from the configured server.
        if src != cfg.server {
            tracing::debug!("DNS: ignoring datagram from unexpected source {src}");
            continue;
        }

        let response = match Message::from_vec(&buf[..len]) {
            Ok(m) => m,
            Err(e) => {
                tracing::debug!("DNS: ignoring undecodable datagram: {e}");
                continue;
            }
        };

        // Discard responses whose id does not match our query.
        if response.metadata.id != id {
            tracing::debug!("DNS: ignoring response with mismatched id");
            continue;
        }

        // A truncated (TC-bit) response is incomplete — RFC 1035 requires retrying over TCP, which
        // this datagram-only resolver doesn't do. Reject it rather than return whatever partial
        // answers happened to fit, so callers never treat a partial set as authoritative.
        if response.metadata.truncation {
            return Err(SmolCoreError::DnsTruncated {
                name: name.to_utf8(),
            });
        }

        // Distinguish a server-side failure from a genuinely empty answer.
        match response.metadata.response_code {
            ResponseCode::NoError => {}
            ResponseCode::NXDomain => {
                return Err(SmolCoreError::DnsNoRecords {
                    name: name.to_utf8(),
                })
            }
            other => {
                return Err(SmolCoreError::DnsServerFailure {
                    name: name.to_utf8(),
                    rcode: other.to_string(),
                })
            }
        }

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
        return Ok(addrs);
    }
}
