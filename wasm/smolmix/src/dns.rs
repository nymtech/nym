// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! DNS A/AAAA resolution over the tunnel's UDP socket. Defaults are 8.8.8.8
//! primary with 1.1.1.1 fallback, overridable via `TunnelOpts::primary_dns`
//! / `fallback_dns`. Wire format via `hickory-proto`; results cached per session.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use hickory_proto::op::{Message, Query};
use hickory_proto::rr::{Name, RData, RecordType};

use crate::error::FetchError;
use crate::stream::WasmUdpSocket;
use crate::tunnel::WasmTunnel;

/// Maximum number of CNAME hops before giving up.
const MAX_CNAME_HOPS: usize = 8;

pub const DEFAULT_PRIMARY_DNS: SocketAddr =
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 53);
pub const DEFAULT_FALLBACK_DNS: SocketAddr =
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 53);

/// Resolve a hostname to an IP through the mixnet tunnel.
pub async fn resolve(tunnel: &WasmTunnel, hostname: &str) -> Result<IpAddr, FetchError> {
    if let Ok(ip) = hostname.parse::<IpAddr>() {
        return Ok(ip);
    }

    // Serialise DNS lookups so concurrent callers coalesce on the cache.
    let _guard = tunnel.dns_lock().lock().await;

    if let Some(&ip) = tunnel.dns_cache().lock().unwrap().get(hostname) {
        crate::util::debug_log!("[dns] cache hit: '{hostname}' => {ip}");
        return Ok(ip);
    }

    crate::util::debug_log!("[dns] resolving '{hostname}'...");
    let udp = tunnel.udp_socket().await.map_err(FetchError::Io)?;

    let timeout = tunnel.dns_timeout();
    let ip = match resolve_with(&udp, hostname, tunnel.dns_primary(), timeout).await {
        Ok(ip) => ip,
        Err(_) => resolve_with(&udp, hostname, tunnel.dns_fallback(), timeout).await?,
    };

    crate::util::debug_log!("[dns] resolved '{hostname}' => {ip}");
    tunnel
        .dns_cache()
        .lock()
        .unwrap()
        .insert(hostname.to_string(), ip);
    Ok(ip)
}

/// Try A then AAAA against a specific DNS server, following CNAME chains.
async fn resolve_with(
    udp: &WasmUdpSocket,
    hostname: &str,
    server: SocketAddr,
    timeout: Duration,
) -> Result<IpAddr, FetchError> {
    match query_following_cnames(udp, hostname, RecordType::A, server, timeout).await {
        Ok(ip) => Ok(ip),
        Err(_) => query_following_cnames(udp, hostname, RecordType::AAAA, server, timeout).await,
    }
}

/// Send a DNS query and follow any CNAME chain until we get an IP or exhaust hops.
async fn query_following_cnames(
    udp: &WasmUdpSocket,
    hostname: &str,
    record_type: RecordType,
    server: SocketAddr,
    timeout: Duration,
) -> Result<IpAddr, FetchError> {
    let mut current_name = hostname.to_string();

    for _ in 0..MAX_CNAME_HOPS {
        match query_record(udp, &current_name, record_type, server, timeout).await? {
            DnsResult::Ip(ip) => return Ok(ip),
            DnsResult::Cname(target) => current_name = target,
        }
    }

    Err(FetchError::Dns(format!(
        "CNAME chain too long (>{MAX_CNAME_HOPS} hops) for {hostname}"
    )))
}

enum DnsResult {
    Ip(IpAddr),
    Cname(String),
}

/// Send a single DNS query and parse the response.
///
/// The `WasmUdpSocket` is shared across PRIMARY → FALLBACK, A → AAAA, and
/// every CNAME hop in one resolve, so leftover datagrams from a prior query
/// can be sitting in the receive buffer. We loop on `recv_from`, dropping
/// any datagram whose transaction ID doesn't match the query we just sent,
/// until either a match arrives or `timeout` elapses.
async fn query_record(
    udp: &WasmUdpSocket,
    hostname: &str,
    record_type: RecordType,
    server: SocketAddr,
    timeout: Duration,
) -> Result<DnsResult, FetchError> {
    let (query_bytes, query_id) = build_query(hostname, record_type)?;
    udp.send_to(&query_bytes, server)
        .await
        .map_err(FetchError::Io)?;
    crate::util::debug_log!("[dns] query sent to {server} (id={query_id:#06x}), waiting...");

    let start = wasmtimer::std::Instant::now();

    loop {
        let remaining = timeout
            .checked_sub(start.elapsed())
            .ok_or(FetchError::Timeout)?;

        let mut buf = [0u8; 512];
        let (len, src) = wasmtimer::tokio::timeout(remaining, udp.recv_from(&mut buf))
            .await
            .map_err(|_| {
                crate::util::debug_error!("[dns] recv_from TIMED OUT after {timeout:?}");
                FetchError::Timeout
            })?
            .map_err(FetchError::Io)?;

        // Anti-spoof layer 1: source address must match the server we queried.
        // The UDP socket is reused across CNAME hops and primary/fallback
        // retries, so a late reply from an earlier `server` is a real case,
        // not just hypothetical.
        if src != server {
            crate::util::debug_log!(
                "[dns] dropped datagram from unexpected source {src} (expected {server})"
            );
            continue;
        }

        // Anti-spoof layer 2: parse failures and ID mismatches are also
        // "keep reading," not "abort the lookup." A single malformed packet
        // from `server` (or an attacker who can spoof the source) shouldn't
        // turn a live query into a hard failure.
        let response = match Message::from_vec(&buf[..len]) {
            Ok(r) => r,
            Err(e) => {
                crate::util::debug_log!("[dns] dropped malformed datagram from {src}: {e}");
                continue;
            }
        };

        // Anti-spoof layer 3: transaction ID must match. The id was filled
        // from `rand::random()` (CSPRNG via `getrandom`/js), so the guess
        // rate for an off-path attacker is the theoretical 1/65536 per try.
        if response.id != query_id {
            crate::util::debug_log!(
                "[dns] dropped stale datagram (id={:#06x}, expected {query_id:#06x})",
                response.id,
            );
            continue;
        }

        return parse_response(&response, hostname);
    }
}

/// Build a DNS query and return its bytes plus transaction ID.
fn build_query(hostname: &str, record_type: RecordType) -> Result<(Vec<u8>, u16), FetchError> {
    let mut msg = Message::query();
    msg.metadata.recursion_desired = true;
    let id = msg.metadata.id;

    let name = Name::from_ascii(hostname)
        .map_err(|e| FetchError::Dns(format!("invalid hostname '{hostname}': {e}")))?;
    msg.add_query(Query::query(name, record_type));

    let bytes = msg
        .to_vec()
        .map_err(|e| FetchError::Dns(format!("failed to serialise DNS query: {e}")))?;
    Ok((bytes, id))
}

/// Parse a DNS response message, returning an IP or CNAME target.
fn parse_response(msg: &Message, hostname: &str) -> Result<DnsResult, FetchError> {
    let mut cname_target: Option<String> = None;

    for record in &msg.answers {
        match &record.data {
            RData::A(a) => return Ok(DnsResult::Ip(IpAddr::V4(a.0))),
            RData::AAAA(aaaa) => return Ok(DnsResult::Ip(IpAddr::V6(aaaa.0))),
            RData::CNAME(cname) if cname_target.is_none() => {
                cname_target = Some(cname.0.to_string());
            }
            _ => {}
        }
    }

    if let Some(target) = cname_target {
        return Ok(DnsResult::Cname(target));
    }

    Err(FetchError::Dns(format!(
        "no A, AAAA, or CNAME records for {hostname}"
    )))
}
