// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>

//! DNS A/AAAA resolution over the WASM tunnel's UDP transport.
//!
//! Uses `hickory-proto` for wire-format construction and parsing (no tokio
//! dep with `default-features = false`, just the protocol types). Queries
//! go via the tunnel's UDP socket to a public resolver (8.8.8.8 primary,
//! 1.1.1.1 fallback). Results are cached for the session.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use hickory_proto::op::{Message, Query};
use hickory_proto::rr::{Name, RData, RecordType};

use crate::error::FetchError;
use crate::stream::WasmUdpSocket;
use crate::tunnel::WasmTunnel;

/// Maximum number of CNAME hops before giving up.
const MAX_CNAME_HOPS: usize = 8;

const PRIMARY_DNS: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 53);
const FALLBACK_DNS: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 53);
const DNS_TIMEOUT: Duration = Duration::from_secs(30);

/// Resolve a hostname to an IP address via DNS over the mixnet tunnel.
///
/// - Cached results are returned immediately (no TTL, session-lived).
/// - Literal IP addresses (e.g. "1.2.3.4") are returned without a query.
/// - Tries A records first, then AAAA.
/// - Falls back to 1.1.1.1 if 8.8.8.8 times out.
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

    let ip = match resolve_with(&udp, hostname, PRIMARY_DNS).await {
        Ok(ip) => ip,
        Err(_) => resolve_with(&udp, hostname, FALLBACK_DNS).await?,
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
) -> Result<IpAddr, FetchError> {
    match query_following_cnames(udp, hostname, RecordType::A, server).await {
        Ok(ip) => Ok(ip),
        Err(_) => query_following_cnames(udp, hostname, RecordType::AAAA, server).await,
    }
}

/// Send a DNS query and follow any CNAME chain until we get an IP or exhaust hops.
async fn query_following_cnames(
    udp: &WasmUdpSocket,
    hostname: &str,
    record_type: RecordType,
    server: SocketAddr,
) -> Result<IpAddr, FetchError> {
    let mut current_name = hostname.to_string();

    for _ in 0..MAX_CNAME_HOPS {
        match query_record(udp, &current_name, record_type, server).await? {
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
async fn query_record(
    udp: &WasmUdpSocket,
    hostname: &str,
    record_type: RecordType,
    server: SocketAddr,
) -> Result<DnsResult, FetchError> {
    let (query_bytes, query_id) = build_query(hostname, record_type)?;
    udp.send_to(&query_bytes, server)
        .await
        .map_err(FetchError::Io)?;
    crate::util::debug_log!("[dns] query sent to {server} (id={query_id:#06x}), waiting...");

    let mut buf = [0u8; 512];
    let (len, _) = wasmtimer::tokio::timeout(DNS_TIMEOUT, udp.recv_from(&mut buf))
        .await
        .map_err(|_| {
            crate::util::debug_error!("[dns] recv_from TIMED OUT after {DNS_TIMEOUT:?}");
            FetchError::Timeout
        })?
        .map_err(FetchError::Io)?;

    let response = Message::from_vec(&buf[..len])
        .map_err(|e| FetchError::Dns(format!("failed to parse DNS response: {e}")))?;

    // Anti-spoof: drop responses whose transaction ID doesn't match the
    // query we sent. Without this an attacker who guesses our open queries
    // can inject forged A records.
    if response.id != query_id {
        return Err(FetchError::Dns(format!(
            "DNS response ID mismatch: expected {query_id:#06x}, got {:#06x}",
            response.id
        )));
    }

    parse_response(&response, hostname)
}

/// Build a DNS query packet and return its wire bytes plus the transaction
/// ID we set, so the caller can verify the response.
///
/// hickory-proto's `Message::query()` auto-generates an ID, but we override
/// it with `rand::random()` (wasm32 backend = `crypto.getRandomValues()`
/// via `getrandom/js`) so we control the entropy source and keep the
/// security-audit guarantee from item #5 of the audit.
fn build_query(hostname: &str, record_type: RecordType) -> Result<(Vec<u8>, u16), FetchError> {
    let mut msg = Message::query();
    msg.metadata.recursion_desired = true;
    let id: u16 = rand::random();
    msg.metadata.id = id;

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
            RData::CNAME(cname) => {
                if cname_target.is_none() {
                    cname_target = Some(cname.0.to_string());
                }
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
