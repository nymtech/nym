// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! DNS A/AAAA resolution over DoH (RFC 8484) through the tunnel. Queries are
//! sent as GET requests to the configured DoH endpoints (Cloudflare, Quad9,
//! Google by default) over the same TLS/HTTP stack `mixFetch` uses, so a
//! rate-limited resolver surfaces as an HTTP 429 error instead of a silent
//! hang. Endpoints are tried in order; results cached per session.

use std::net::IpAddr;
use std::time::Duration;

use hickory_proto::op::{Message, Query};
use hickory_proto::rr::{Name, RData, RecordType};
use url::Url;

use crate::error::FetchError;
use crate::tunnel::WasmTunnel;

/// Maximum number of CNAME hops before giving up.
const MAX_CNAME_HOPS: usize = 8;

/// Default DoH endpoints, tried in order. IP-literal HTTPS URLs so the resolver
/// itself needs no bootstrap DNS; each resolver serves a certificate carrying
/// the IP in its SANs. Quad9 first: a Swiss non-profit that does not log queries
/// and filters known-malicious names, the best privacy fit for a privacy project.
/// Cloudflare next (also no query logging). Google last, as a reliability
/// fallback only; it retains query data, so it is the least private of the three.
/// Quad9 needs HTTP/2 (it retired HTTP/1.1 DoH), which the `doh-h2` feature
/// provides; without that feature a 505 just rotates to Cloudflare.
pub fn default_doh_endpoints() -> Vec<Url> {
    [
        "https://9.9.9.9/dns-query",
        "https://1.1.1.1/dns-query",
        "https://8.8.8.8/dns-query",
    ]
    .iter()
    .map(|s| Url::parse(s).expect("hardcoded DoH URL is valid"))
    .collect()
}

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

    crate::util::debug_log!("[dns] resolving '{hostname}' over DoH...");
    let timeout = tunnel.dns_timeout();

    let mut last_err: Option<FetchError> = None;
    for endpoint in tunnel.doh_endpoints() {
        match resolve_with(tunnel, hostname, endpoint, timeout).await {
            Ok(ip) => {
                crate::util::debug_log!("[dns] resolved '{hostname}' => {ip} via {endpoint}");
                tunnel
                    .dns_cache()
                    .lock()
                    .unwrap()
                    .insert(hostname.to_string(), ip);
                return Ok(ip);
            }
            Err(e) => {
                crate::util::debug_log!(
                    "[dns] endpoint {endpoint} failed for '{hostname}': {e}; rotating"
                );
                last_err = Some(e);
            }
        }
    }

    Err(last_err.unwrap_or_else(|| FetchError::Dns("no DoH endpoints configured".into())))
}

/// Try A then AAAA against a specific DoH endpoint, following CNAME chains.
async fn resolve_with(
    tunnel: &WasmTunnel,
    hostname: &str,
    endpoint: &Url,
    timeout: Duration,
) -> Result<IpAddr, FetchError> {
    match query_following_cnames(tunnel, hostname, RecordType::A, endpoint, timeout).await {
        Ok(ip) => Ok(ip),
        // Only a genuine no-records answer (a `Dns` error from `parse_response`)
        // is worth an AAAA retry. A rate-limit, server, or transport error means
        // this endpoint is unusable, so surface it unchanged and let `resolve`
        // rotate to the next endpoint rather than masking it with an AAAA attempt.
        Err(FetchError::Dns(_)) => {
            query_following_cnames(tunnel, hostname, RecordType::AAAA, endpoint, timeout).await
        }
        Err(e) => Err(e),
    }
}

/// Send a DNS query and follow any CNAME chain until we get an IP or exhaust hops.
async fn query_following_cnames(
    tunnel: &WasmTunnel,
    hostname: &str,
    record_type: RecordType,
    endpoint: &Url,
    timeout: Duration,
) -> Result<IpAddr, FetchError> {
    let mut current_name = hostname.to_string();

    for _ in 0..MAX_CNAME_HOPS {
        match query_record(tunnel, &current_name, record_type, endpoint, timeout).await? {
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

/// Send a single DNS query over DoH and parse the response. The HTTP status maps
/// resolver health: 200 parses the wire-format body; 429 is rate-limiting,
/// surfaced as a distinct error instead of a silent stall; any other status is a
/// server error. The DoH response body is the same wire-format DNS message the
/// UDP path returned, so `parse_response` is unchanged. HTTP request/response
/// pairing over TLS supersedes the UDP anti-spoof checks (source address and
/// transaction id), so those are not needed here.
async fn query_record(
    tunnel: &WasmTunnel,
    hostname: &str,
    record_type: RecordType,
    endpoint: &Url,
    timeout: Duration,
) -> Result<DnsResult, FetchError> {
    let (query_bytes, _id) = build_query(hostname, record_type)?;
    let response = match crate::fetch::doh_query(tunnel, endpoint, &query_bytes, timeout).await {
        Ok(response) => response,
        Err(e) => {
            // Log the full cause here, where we still hold the typed error. Once
            // it crosses to JS it is flattened to the terse `Display` string.
            crate::util::debug_error!(
                "[dns] resolver {endpoint} request failed: {}",
                crate::error::detail(&e)
            );
            return Err(e);
        }
    };

    // Log the status of every DoH response, not only the error arms below, so the
    // connection probe can compare each resolver's behaviour, 200s included. A
    // resolver should answer 200; a 3xx is not normal, so name its redirect target.
    crate::util::debug_log!("[dns] resolver {endpoint} => HTTP {}", response.status);
    if (300..400).contains(&response.status) {
        let location = response
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("location"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("(none)");
        crate::util::debug_log!("[dns] resolver {endpoint} redirect → Location: {location}");
    }

    match response.status {
        200 => {
            let msg = Message::from_vec(&response.body).map_err(|e| {
                FetchError::Dns(format!("malformed DoH response from {endpoint}: {e}"))
            })?;
            parse_response(&msg, hostname)
        }
        // Always-on (not debug-gated) so rate-limiting is visible in production,
        // which is the whole reason for moving off silent UDP.
        429 => {
            nym_wasm_utils::console_warn!(
                "[dns] resolver {endpoint} rate-limited us (HTTP 429); rotating to next endpoint"
            );
            Err(FetchError::DnsRateLimited {
                endpoint: endpoint.to_string(),
            })
        }
        status => {
            nym_wasm_utils::console_warn!(
                "[dns] resolver {endpoint} returned HTTP {status}; rotating to next endpoint"
            );
            Err(FetchError::DnsServerError {
                endpoint: endpoint.to_string(),
                status,
            })
        }
    }
}

/// Build a DNS query and return its bytes plus transaction ID. The id is unused
/// over DoH (RFC 8484 recommends 0 and HTTP pairs the response for us), but
/// `build_query` is shared, so it is returned and ignored by the caller.
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
