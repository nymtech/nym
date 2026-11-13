// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Probe the bridge transports a gateway publishes.
//!
//! # Presence vs failure
//!
//! - No client-params endpoint (or the node has no address) → `None`. The
//!   field stays unset on the probe outcome; a gateway without `nym-bridge` is
//!   still a normal gateway.
//! - A body that does not parse as `PersistedClientConfig` → `Some` with
//!   `error` set and an empty transport list.
//! - A published transport → one entry; IPv4 dial / forward results when that
//!   endpoint is advertised and usable.
//!
//! # What each field means
//!
//! For every advertised transport we dial the IPv4 endpoint with
//! [`BridgeConn::try_connect`] (`can_connect_v4`). When a WG peer is available
//! we also launch [`UdpForwarder`] and run WireGuard through it
//! (`can_handshake_v4` + ping) — the Amnezia-shaped forward signal.
//!
//! Public IPv6 dials are deferred until NS-Agents confirm host IPv6 to the
//! node's published bridge address is a valid test (WG `can_handshake_v6` is
//! *in-tunnel* traffic and does not prove that path).
//!
//! If authenticator registration failed, dials still run so transport
//! connectivity is reported; `can_handshake_v4` stays `None` and the top-level
//! `error` carries the registration reason.
//!
//! Informational only: nothing here feeds the directory score, and a failing
//! transport is reported rather than used to drop the node.

use std::error::Error as _;
use std::net::IpAddr;
use std::sync::Once;
use std::time::Duration;

use nym_bridges::connection::BridgeConn;
use nym_bridges::error::TransportError;
use nym_bridges::forward::UdpForwarder;
use nym_bridges::types::{ClientConfig, PersistedClientConfig, Sufficiency, TransportAssociation};
use nym_network_defaults::DEFAULT_NYM_NODE_HTTP_PORT;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::common::nodes::TestedNodeDetails;
use crate::common::types::{BridgeProbeResults, BridgeTransportProbe};
use crate::common::wireguard::{BridgeTunnelOutcome, WgTunnelConfig, run_bridge_tunnel_tests};
use crate::config::NetstackArgs;

/// Matches `nym-bridges` `DEFAULT_CONNECT_TIMEOUT` for `BridgeConn::try_connect`.
const BRIDGE_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// `UdpForwarder::launch_initiator` wait for the first local UDP datagram
/// (typically the WG handshake initiation) before giving up.
const BRIDGE_FWD_TIMEOUT: Duration = Duration::from_secs(15);
const CLIENT_PARAMS_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_CLIENT_PARAMS_BYTES: usize = 1024 * 1024;
const MAX_BRIDGE_TRANSPORTS: usize = 8;
const CLIENT_PARAMS_PATH: &str = "/api/v1/bridges/client-params";

/// WG peer for the forward path, or why it is missing.
pub(crate) enum BridgePeer<'a> {
    Registered(&'a WgTunnelConfig),
    Unavailable { reason: String },
}

struct BridgeForward<'a> {
    tunnel: Option<&'a WgTunnelConfig>,
    netstack: &'a NetstackArgs,
    awg: &'a str,
}

/// `Some` when client-params were fetched. `None` when no contact URL answered.
pub(crate) async fn bridge_probe(
    node: &TestedNodeDetails,
    peer: BridgePeer<'_>,
    netstack_args: &NetstackArgs,
    awg_args: &str,
) -> Option<BridgeProbeResults> {
    let body = fetch_client_params(node).await?;
    Some(probe_descriptor(&body, peer, netstack_args, awg_args).await)
}

async fn probe_descriptor(
    body: &str,
    peer: BridgePeer<'_>,
    netstack_args: &NetstackArgs,
    awg_args: &str,
) -> BridgeProbeResults {
    let cfg = match serde_json::from_str::<PersistedClientConfig>(body) {
        Ok(cfg) => cfg,
        Err(err) => {
            warn!("bridge client params did not parse: {err}");
            return BridgeProbeResults {
                transports: Vec::new(),
                error: Some(format!("descriptor did not parse: {err}")),
            };
        }
    };

    info!(
        "bridge descriptor: {} advertised transport(s)",
        cfg.advertised().len()
    );

    let (tunnel, registration_error) = match peer {
        BridgePeer::Registered(tunnel) => (Some(tunnel), None),
        BridgePeer::Unavailable { reason } => {
            warn!("bridge forward path unavailable: {reason}");
            (None, Some(reason))
        }
    };

    install_crypto_provider();

    // Sequential: each forward pass owns a netstack/CGo call.
    let advertised = cfg.advertised();
    if advertised.len() > MAX_BRIDGE_TRANSPORTS {
        warn!(
            "bridge descriptor advertises {} transports; probing first {MAX_BRIDGE_TRANSPORTS}",
            advertised.len()
        );
    }
    let forward = BridgeForward {
        tunnel,
        netstack: netstack_args,
        awg: awg_args,
    };
    let mut transports = Vec::with_capacity(advertised.len().min(MAX_BRIDGE_TRANSPORTS));
    for transport in advertised.iter().cloned().take(MAX_BRIDGE_TRANSPORTS) {
        transports.push(probe_transport(transport, &forward).await);
    }

    BridgeProbeResults {
        transports,
        error: registration_error,
    }
}

async fn probe_transport(
    transport: ClientConfig,
    forward: &BridgeForward<'_>,
) -> BridgeTransportProbe {
    let mut reported = advertised_only(&transport);

    if !transport.is_sufficient() {
        reported.error = Some("not usable".to_string());
        return reported;
    }

    let Some(v4) = with_family(&transport, true) else {
        reported.error = Some("no ipv4 endpoint advertised".to_string());
        return reported;
    };

    let mut errors = Vec::new();
    let v4 = fold_family(
        &mut errors,
        "ipv4",
        dial_family(&reported.transport, "ipv4", Some(v4), forward).await,
    );

    reported.can_connect_v4 = v4.can_connect;
    reported.can_handshake_v4 = v4.can_handshake;
    reported.ping_ips_performance_v4 = v4.ping;
    // Dial vs forward failures both land here; the bool fields keep the distinction.
    reported.error = (!errors.is_empty()).then(|| errors.join("; "));
    reported
}

/// Transport name only — base record before dial results are filled in.
fn advertised_only(transport: &ClientConfig) -> BridgeTransportProbe {
    BridgeTransportProbe {
        transport: transport.transport_name(),
        ..Default::default()
    }
}

struct FamilyFields {
    can_connect: Option<bool>,
    can_handshake: Option<bool>,
    ping: Option<f32>,
}

fn fold_family(
    errors: &mut Vec<String>,
    family: &str,
    result: Option<FamilyProbe>,
) -> FamilyFields {
    match result {
        None => FamilyFields {
            can_connect: None,
            can_handshake: None,
            ping: None,
        },
        Some(FamilyProbe::Connected) => FamilyFields {
            can_connect: Some(true),
            can_handshake: None,
            ping: None,
        },
        Some(FamilyProbe::Forwarded(outcome)) => {
            if !outcome.can_handshake {
                errors.push(format!("{family}: wg handshake through bridge failed"));
            }
            FamilyFields {
                can_connect: Some(true),
                can_handshake: Some(outcome.can_handshake),
                ping: Some(outcome.ping_ips_performance),
            }
        }
        // Dial succeeded; keep connect true so this is not read as unreachable.
        Some(FamilyProbe::ForwardFailed(err)) => {
            errors.push(format!("{family}: {err}"));
            FamilyFields {
                can_connect: Some(true),
                can_handshake: Some(false),
                ping: None,
            }
        }
        Some(FamilyProbe::DialFailed(err)) => {
            errors.push(format!("{family}: {err}"));
            FamilyFields {
                can_connect: Some(false),
                can_handshake: None,
                ping: None,
            }
        }
    }
}

enum FamilyProbe {
    /// Dial succeeded; no WG peer so the forward path was not tested.
    Connected,
    /// Dial + forwarder + WG attempt completed (handshake may still be false).
    Forwarded(BridgeTunnelOutcome),
    /// Dial succeeded; forwarder setup or netstack pass failed.
    ForwardFailed(String),
    /// `BridgeConn::try_connect` failed.
    DialFailed(String),
}

/// `None` when this transport does not advertise the family.
async fn dial_family(
    name: &str,
    family: &str,
    params: Option<ClientConfig>,
    forward: &BridgeForward<'_>,
) -> Option<FamilyProbe> {
    let params = params?;
    let outcome = probe_path(params, forward).await;
    match &outcome {
        FamilyProbe::Connected => {
            info!("{name} bridge connected over {family} (forward not tested)");
        }
        FamilyProbe::Forwarded(fwd) => {
            info!(
                "{name} bridge over {family}: handshake={} ping_ips={:.2}",
                fwd.can_handshake, fwd.ping_ips_performance
            );
        }
        FamilyProbe::ForwardFailed(err) => {
            warn!("{name} bridge over {family}: connected, forward failed: {err}");
        }
        FamilyProbe::DialFailed(err) => {
            warn!("{name} bridge over {family} dial failed: {err}");
        }
    }
    Some(outcome)
}

async fn probe_path(params: ClientConfig, forward: &BridgeForward<'_>) -> FamilyProbe {
    let conn = match dial(params).await {
        Ok(conn) => conn,
        Err(err) => return FamilyProbe::DialFailed(format_transport_error(&err)),
    };

    let Some(tunnel) = forward.tunnel else {
        let (_reader, _writer, closer) = conn.into_parts();
        closer.close().await;
        return FamilyProbe::Connected;
    };

    let cancel = CancellationToken::new();
    let (listen_addr, fwd_handle) = match UdpForwarder::launch_initiator(
        conn,
        None,
        None,
        cancel.child_token(),
        Some(BRIDGE_FWD_TIMEOUT),
    )
    .await
    {
        Ok(launched) => launched,
        Err(err) => return FamilyProbe::ForwardFailed(format_transport_error(&err)),
    };

    let mut through_bridge = tunnel.clone();
    through_bridge.endpoint = listen_addr.to_string();
    let netstack_args = forward.netstack.clone();
    let awg_args = forward.awg.to_string();

    let result = tokio::task::spawn_blocking(move || {
        run_bridge_tunnel_tests(&through_bridge, &netstack_args, &awg_args)
    })
    .await;
    cancel.cancel();
    let _ = fwd_handle.await;

    match result {
        Ok(Ok(outcome)) => FamilyProbe::Forwarded(outcome),
        Ok(Err(err)) => FamilyProbe::ForwardFailed(err),
        Err(err) => FamilyProbe::ForwardFailed(format!("bridge netstack task panicked: {err}")),
    }
}

/// `BridgeConn::try_connect` takes an `on_socket_open` callback on Linux and
/// Android. The probe is not inside a tunnel, so the callback is the no-op.
#[cfg(any(target_os = "linux", target_os = "android"))]
async fn dial(params: ClientConfig) -> Result<BridgeConn, TransportError> {
    BridgeConn::try_connect(
        params,
        CancellationToken::new(),
        nym_bridges::connection::SOCKET_OPEN_NOP,
        Some(BRIDGE_CONNECT_TIMEOUT),
    )
    .await
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
async fn dial(params: ClientConfig) -> Result<BridgeConn, TransportError> {
    BridgeConn::try_connect(
        params,
        CancellationToken::new(),
        Some(BRIDGE_CONNECT_TIMEOUT),
    )
    .await
}

fn install_crypto_provider() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        // nym-bridges builds its rustls config from the process-default provider.
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Keep the OS error under `SocketIo` / `Io`. The Display impl alone is
/// `"transport socket io error"` and drops the source — that is what made
/// `No route to host` invisible in earlier probe output.
fn format_transport_error(err: &TransportError) -> String {
    match err.source() {
        Some(source) => format!("{err}: {source}"),
        None => err.to_string(),
    }
}

/// GET `/api/v1/bridges/client-params` from each contact URL until one succeeds.
async fn fetch_client_params(node: &TestedNodeDetails) -> Option<String> {
    let urls = contact_urls(node);
    if urls.is_empty() {
        return None;
    }

    let client = reqwest::Client::builder()
        .timeout(CLIENT_PARAMS_TIMEOUT)
        .build()
        .inspect_err(|err| warn!("bridge params client: {err}"))
        .ok()?;

    for base in urls {
        let url = format!("{}{CLIENT_PARAMS_PATH}", base.trim_end_matches('/'));
        info!("fetching bridge client params from {url}");
        match client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                if let Some(body) = read_client_params_body(response, &url).await {
                    return Some(body);
                }
            }
            Ok(response) => {
                info!(
                    "no bridge client params at {url}: HTTP {}",
                    response.status()
                );
            }
            Err(err) => {
                info!("no bridge client params at {url}: {err}");
            }
        }
    }
    None
}

async fn read_client_params_body(response: reqwest::Response, url: &str) -> Option<String> {
    if response
        .content_length()
        .is_some_and(|n| n as usize > MAX_CLIENT_PARAMS_BYTES)
    {
        warn!("bridge client params at {url}: body too large");
        return None;
    }

    let bytes = response
        .bytes()
        .await
        .inspect_err(|err| warn!("bridge client params body at {url}: {err}"))
        .ok()?;
    if bytes.len() > MAX_CLIENT_PARAMS_BYTES {
        warn!("bridge client params at {url}: body too large");
        return None;
    }

    String::from_utf8(bytes.to_vec())
        .inspect_err(|err| warn!("bridge client params body at {url}: {err}"))
        .ok()
}

/// HTTP bases for the node API: host IPs / hostname with the scraper default
/// port ladder. Bond custom HTTP ports are not available from the
/// described-nodes directory today.
fn contact_urls(node: &TestedNodeDetails) -> Vec<String> {
    let mut hosts = Vec::new();
    for ip in &node.host_ips {
        hosts.push(host_for_url(*ip));
    }
    if let Some(hostname) = &node.hostname {
        if !hostname.is_empty() && !hosts.iter().any(|h| h == hostname) {
            hosts.push(hostname.clone());
        }
    }
    if hosts.is_empty() {
        if let Some(ip) = node.ip_address {
            hosts.push(host_for_url(ip));
        }
    }

    let mut urls = Vec::new();
    for host in hosts {
        urls.push(format!("http://{host}:{DEFAULT_NYM_NODE_HTTP_PORT}"));
        urls.push(format!("http://{host}:8000"));
        urls.push(format!("https://{host}"));
        urls.push(format!("http://{host}"));
    }
    urls
}

fn host_for_url(ip: IpAddr) -> String {
    match ip {
        IpAddr::V4(v4) => v4.to_string(),
        IpAddr::V6(v6) => format!("[{v6}]"),
    }
}

fn with_family(config: &ClientConfig, ipv4: bool) -> Option<ClientConfig> {
    let mut filtered = config.clone();
    let addresses = match &mut filtered {
        ClientConfig::QuicPlain(opts) => &mut opts.addresses,
        ClientConfig::TlsPlain(opts) => &mut opts.addresses,
        ClientConfig::SshPlain(opts) => &mut opts.addresses,
    };
    addresses.retain(|addr| addr.is_ipv4() == ipv4);
    if addresses.is_empty() {
        None
    } else {
        Some(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const QUIC: &str = r#"{"transport_type":"quic_plain","args":{"addresses":["1.2.3.4:4443","[2001:db8::1]:4443"],"host":"h","id_pubkey":"pin"}}"#;
    const TLS: &str = r#"{"transport_type":"tls_plain","args":{"addresses":["1.2.3.4:4443"],"host":"h","id_pubkey":"pin"}}"#;
    const FUTURE: &str = r#"{"transport_type":"future_plain","args":{"anything":1}}"#;

    fn parse(body: &str) -> PersistedClientConfig {
        serde_json::from_str(body).unwrap_or_else(|err| panic!("descriptor: {err}"))
    }

    fn node_with_ip(ip: IpAddr) -> TestedNodeDetails {
        // Identity is unused by contact_urls; any valid ed25519 key works.
        let identity =
            nym_sdk::mixnet::NodeIdentity::from_bytes(&[1u8; 32]).expect("fixture identity");
        TestedNodeDetails {
            identity,
            exit_router_address: None,
            network_requester_address: None,
            authenticator_address: None,
            authenticator_version: nym_authenticator_requests::AuthenticatorVersion::V6,
            ip_address: Some(ip),
            host_ips: vec![ip],
            hostname: None,
            lp_data: None,
        }
    }

    fn dummy_tunnel() -> WgTunnelConfig {
        WgTunnelConfig::new(
            "10.1.0.2",
            "fc01::2",
            "aa".repeat(32),
            "bb".repeat(32),
            "127.0.0.1:51822",
        )
    }

    #[test]
    fn contact_urls_use_default_ladder() {
        let node = node_with_ip("1.2.3.4".parse().unwrap());
        let urls = contact_urls(&node);
        assert_eq!(
            urls,
            vec![
                "http://1.2.3.4:8080".to_string(),
                "http://1.2.3.4:8000".to_string(),
                "https://1.2.3.4".to_string(),
                "http://1.2.3.4".to_string(),
            ]
        );
    }

    #[test]
    fn contact_urls_bracket_ipv6() {
        let node = node_with_ip("2001:db8::1".parse().unwrap());
        assert!(contact_urls(&node)[0].starts_with("http://[2001:db8::1]:8080"));
    }

    #[test]
    fn family_filter_drops_the_other_family() {
        let cfg = parse(&format!(r#"{{"version":"0","transports":[{QUIC}]}}"#));
        let transport = &cfg.advertised()[0];

        let v4 = with_family(transport, true).expect("v4 address");
        let v6 = with_family(transport, false).expect("v6 address");
        assert!(v4.addresses().iter().all(|addr| addr.is_ipv4()));
        assert!(v6.addresses().iter().all(|addr| addr.is_ipv6()));

        let v4_only = parse(
            r#"{"version":"0","transports":[{"transport_type":"quic_plain","args":{"addresses":["1.2.3.4:4443"],"host":"h","id_pubkey":"pin"}}]}"#,
        );
        assert!(with_family(&v4_only.advertised()[0], false).is_none());
    }

    #[test]
    fn advertised_drops_unknown_types() {
        let cfg = parse(&format!(
            r#"{{"version":"0","transports":[{QUIC}],"transports_v2":[{QUIC},{TLS},{FUTURE}]}}"#
        ));
        assert_eq!(cfg.advertised().len(), 2);
    }

    #[test]
    fn format_transport_error_keeps_io_source() {
        let io = std::io::Error::new(std::io::ErrorKind::NetworkUnreachable, "no route to host");
        let err = TransportError::SocketIo(io);
        let formatted = format_transport_error(&err);
        assert!(formatted.contains("no route to host"), "{formatted}");
    }

    fn endpoint_for(config: &ClientConfig, ipv4: bool) -> Option<String> {
        config
            .addresses()
            .iter()
            .find(|addr| addr.is_ipv4() == ipv4)
            .map(|addr| addr.to_string())
    }

    #[test]
    fn endpoint_for_picks_matching_family() {
        let cfg = parse(&format!(r#"{{"version":"0","transports":[{QUIC}]}}"#));
        let transport = &cfg.advertised()[0];
        assert_eq!(
            endpoint_for(transport, true).as_deref(),
            Some("1.2.3.4:4443")
        );
        assert_eq!(
            endpoint_for(transport, false).as_deref(),
            Some("[2001:db8::1]:4443")
        );
    }

    #[test]
    fn fold_family_keeps_connect_when_forward_fails() {
        let mut errors = Vec::new();
        let fields = fold_family(
            &mut errors,
            "ipv4",
            Some(FamilyProbe::Forwarded(BridgeTunnelOutcome {
                can_handshake: false,
                ping_ips_performance: 0.0,
            })),
        );
        assert_eq!(fields.can_connect, Some(true));
        assert_eq!(fields.can_handshake, Some(false));
        assert_eq!(
            errors,
            vec!["ipv4: wg handshake through bridge failed".to_string()]
        );

        errors.clear();
        let fields = fold_family(
            &mut errors,
            "ipv4",
            Some(FamilyProbe::DialFailed("timed out".into())),
        );
        assert_eq!(fields.can_connect, Some(false));
        assert_eq!(fields.can_handshake, None);
        assert_eq!(errors, vec!["ipv4: timed out".to_string()]);

        errors.clear();
        let fields = fold_family(
            &mut errors,
            "ipv4",
            Some(FamilyProbe::ForwardFailed("forwarder setup failed".into())),
        );
        assert_eq!(fields.can_connect, Some(true));
        assert_eq!(fields.can_handshake, Some(false));
        assert_eq!(fields.ping, None);
        assert_eq!(errors, vec!["ipv4: forwarder setup failed".to_string()]);
    }

    #[tokio::test]
    async fn unusable_transport_is_recorded_without_a_dial() {
        let cfg = parse(
            r#"{"version":"0","transports":[{"transport_type":"quic_plain","args":{"addresses":[],"host":null,"id_pubkey":""}}]}"#,
        );
        let args = NetstackArgs::default();
        let tunnel = dummy_tunnel();

        let forward = BridgeForward {
            tunnel: Some(&tunnel),
            netstack: &args,
            awg: "",
        };
        let reported = probe_transport(cfg.advertised()[0].clone(), &forward).await;
        assert_eq!(reported.transport, "quic_plain");
        assert_eq!(reported.error.as_deref(), Some("not usable"));
        assert!(reported.can_connect_v4.is_none());
        assert!(reported.can_handshake_v4.is_none());
    }

    #[tokio::test]
    async fn ipv6_only_advertisement_is_not_dialled() {
        let cfg = parse(
            r#"{"version":"0","transports":[{"transport_type":"quic_plain","args":{"addresses":["[2001:db8::1]:4443"],"host":"h","id_pubkey":"pin"}}]}"#,
        );
        let args = NetstackArgs::default();
        let forward = BridgeForward {
            tunnel: None,
            netstack: &args,
            awg: "",
        };
        let reported = probe_transport(cfg.advertised()[0].clone(), &forward).await;

        assert_eq!(
            reported.error.as_deref(),
            Some("no ipv4 endpoint advertised")
        );
        assert!(reported.can_connect_v4.is_none());
        assert!(reported.can_handshake_v4.is_none());
    }

    #[tokio::test]
    async fn unavailable_peer_still_records_registration_error() {
        let args = NetstackArgs::default();
        // IPv6-only → no dial, but the registration error is still surfaced.
        let body_v6 = r#"{"version":"0","transports":[{"transport_type":"quic_plain","args":{"addresses":["[2001:db8::1]:4443"],"host":"h","id_pubkey":"pin"}}]}"#;
        let reported = probe_descriptor(
            body_v6,
            BridgePeer::Unavailable {
                reason: "timeout waiting for authenticator".into(),
            },
            &args,
            "",
        )
        .await;

        assert_eq!(
            reported.error.as_deref(),
            Some("timeout waiting for authenticator")
        );
        assert_eq!(reported.transports.len(), 1);
        assert!(reported.transports[0].can_handshake_v4.is_none());
    }
}
