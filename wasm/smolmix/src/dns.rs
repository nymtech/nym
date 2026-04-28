// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

//! DNS A/AAAA resolution over the WASM tunnel's UDP transport.
//!
//! Uses `simple-dns` (no_std, pure Rust) for packet construction and parsing.
//! Queries are sent via the tunnel's UDP socket to a public resolver (8.8.8.8),
//! falling back to 1.1.1.1 on timeout. Results are cached for the session.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use simple_dns::rdata::RData;
use simple_dns::{Packet, PacketFlag, Question, CLASS, QCLASS, QTYPE, TYPE};

use crate::error::FetchError;
use crate::tunnel::{WasmTunnel, WasmUdpSocket};

/// Maximum number of CNAME hops before giving up.
const MAX_CNAME_HOPS: usize = 8;

const PRIMARY_DNS: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 53);
const FALLBACK_DNS: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 53);
const DNS_TIMEOUT: Duration = Duration::from_secs(30);

/// Resolve a hostname to an IP address via DNS over the mixnet tunnel.
///
/// - Cached results are returned immediately (no TTL — session-lived).
/// - Literal IP addresses (e.g. "1.2.3.4") are returned without a query.
/// - Tries A records first, then AAAA.
/// - Falls back to 1.1.1.1 if 8.8.8.8 times out.
pub async fn resolve(tunnel: &WasmTunnel, hostname: &str) -> Result<IpAddr, FetchError> {
    // Skip DNS for literal IP addresses
    if let Ok(ip) = hostname.parse::<IpAddr>() {
        return Ok(ip);
    }

    // Serialise DNS lookups: the first caller for a given hostname does
    // the actual query and populates the cache. Concurrent callers wait
    // on this lock, then hit the cache immediately.
    let _guard = tunnel.dns_lock().lock().await;

    // Check cache (inside the lock so concurrent callers see the result)
    if let Some(&ip) = tunnel.dns_cache().lock().unwrap().get(hostname) {
        nym_wasm_utils::console_log!("[dns] cache hit: '{hostname}' => {ip}");
        return Ok(ip);
    }

    nym_wasm_utils::console_log!("[dns] resolving '{hostname}'...");
    let udp = tunnel.udp_socket().await.map_err(FetchError::Io)?;

    let ip = match resolve_with(&udp, hostname, PRIMARY_DNS).await {
        Ok(ip) => ip,
        Err(_) => resolve_with(&udp, hostname, FALLBACK_DNS).await?,
    };

    nym_wasm_utils::console_log!("[dns] resolved '{hostname}' => {ip}");
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
) -> Result<IpAddr, FetchError> {
    // Try A first, then AAAA. Each follows CNAME chains internally.
    match query_following_cnames(udp, hostname, TYPE::A, server).await {
        Ok(ip) => Ok(ip),
        Err(_) => query_following_cnames(udp, hostname, TYPE::AAAA, server).await,
    }
}

/// Send a DNS query and follow any CNAME chain until we get an IP or exhaust hops.
async fn query_following_cnames(
    udp: &WasmUdpSocket,
    hostname: &str,
    record_type: TYPE,
    server: SocketAddr,
) -> Result<IpAddr, FetchError> {
    let mut current_name = hostname.to_string();

    for _ in 0..MAX_CNAME_HOPS {
        match query_record(udp, &current_name, record_type, server).await? {
            DnsResult::Ip(ip) => return Ok(ip),
            DnsResult::Cname(target) => {
                current_name = target;
            }
        }
    }

    Err(FetchError::Dns(format!(
        "CNAME chain too long (>{MAX_CNAME_HOPS} hops) for {hostname}"
    )))
}

/// Intermediate parse result: either a resolved IP or a CNAME to follow.
enum DnsResult {
    Ip(IpAddr),
    Cname(String),
}

/// Send a single DNS query and parse the response.
async fn query_record(
    udp: &WasmUdpSocket,
    hostname: &str,
    record_type: TYPE,
    server: SocketAddr,
) -> Result<DnsResult, FetchError> {
    let (query, query_id) = build_query(hostname, record_type)?;
    udp.send_to(&query, server).await.map_err(FetchError::Io)?;
    nym_wasm_utils::console_log!("[dns] query sent to {server} (id={query_id:#06x}), waiting...");

    let mut buf = [0u8; 512];
    let (len, _) = wasmtimer::tokio::timeout(DNS_TIMEOUT, udp.recv_from(&mut buf))
        .await
        .map_err(|_| {
            nym_wasm_utils::console_error!("[dns] recv_from TIMED OUT after {DNS_TIMEOUT:?}");
            FetchError::Timeout
        })?
        .map_err(FetchError::Io)?;

    // Verify the response ID matches our query to avoid consuming stale
    // responses from previous timed-out queries.
    let data = &buf[..len];
    if len >= 2 {
        let resp_id = u16::from_be_bytes([data[0], data[1]]);
        if resp_id != query_id {
            return Err(FetchError::Dns(format!(
                "DNS response ID mismatch: expected {query_id:#06x}, got {resp_id:#06x}"
            )));
        }
    }

    parse_response(data, hostname)
}

/// Build a DNS query packet for the given hostname and record type.
/// Returns (wire bytes, query ID) so the caller can verify the response.
fn build_query(hostname: &str, record_type: TYPE) -> Result<(Vec<u8>, u16), FetchError> {
    let id = random_query_id();
    let mut packet = Packet::new_query(id);
    packet.set_flags(PacketFlag::RECURSION_DESIRED);
    let name = simple_dns::Name::new_unchecked(hostname);
    packet.questions.push(Question::new(
        name,
        QTYPE::TYPE(record_type),
        QCLASS::CLASS(CLASS::IN),
        false,
    ));
    let bytes = packet
        .build_bytes_vec()
        .map_err(|e| FetchError::Dns(format!("failed to build DNS query: {e}")))?;
    Ok((bytes, id))
}

/// Parse a DNS response, returning an IP or CNAME target for following.
fn parse_response(data: &[u8], hostname: &str) -> Result<DnsResult, FetchError> {
    let packet = Packet::parse(data)
        .map_err(|e| FetchError::Dns(format!("failed to parse DNS response: {e}")))?;

    let rcode = packet.rcode();
    let truncated = packet.has_flags(PacketFlag::TRUNCATION);

    let mut cname_target: Option<String> = None;
    for answer in &packet.answers {
        match &answer.rdata {
            RData::A(a) => {
                return Ok(DnsResult::Ip(IpAddr::V4(Ipv4Addr::from(a.address))));
            }
            RData::AAAA(aaaa) => {
                return Ok(DnsResult::Ip(IpAddr::V6(Ipv6Addr::from(aaaa.address))));
            }
            RData::CNAME(cname) => {
                if cname_target.is_none() {
                    cname_target = Some(cname.0.to_string());
                }
            }
            _ => {}
        }
    }

    // No A/AAAA found — return CNAME if we have one.
    if let Some(target) = cname_target {
        return Ok(DnsResult::Cname(target));
    }

    if truncated {
        return Err(FetchError::Dns(format!(
            "DNS response truncated for {hostname} (TC bit set, need TCP fallback)"
        )));
    }

    Err(FetchError::Dns(format!(
        "no A, AAAA, or CNAME records for {hostname} (rcode={rcode:?}, {} answers)",
        packet.answers.len()
    )))
}

/// Generate a random 16-bit DNS query ID using the browser's Math.random().
fn random_query_id() -> u16 {
    (js_sys::Math::random() * f64::from(u16::MAX)) as u16
}
