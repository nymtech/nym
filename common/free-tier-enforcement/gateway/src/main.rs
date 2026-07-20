// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Host-side driver for the local free-tier gateway harness.
//!
//! Registers a WireGuard dVPN peer against the running (unbonded, `--standalone`)
//! `nym-node` container over the Lewes Protocol, bypassing topology selection: the
//! node's parameters are discovered directly from its HTTP API. Validates the
//! zk-nym route first (mock ecash), then the free-tier credential arm.
//!
//! Increment 1: discover the node's identity, WireGuard, and LP parameters over
//! HTTP and assemble the [`NymNodeLPInformation`] a registration needs.

use anyhow::{Context, Result};
use clap::Parser;
use nym_bandwidth_controller::mock::MockBandwidthController;
use nym_bin_common::logging::setup_tracing_logger;
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_kkt_ciphersuite::{Ciphersuite, HashFunction, KEM, KEMKeyDigests};
use nym_lp::peer::{DHKeyPair, LpRemotePeer};
use nym_node_requests::api::client::NymNodeApiClientExt;
use nym_node_requests::api::v1::lewes_protocol::models::{LPHashFunction, LPKEM};
use nym_registration_client::LpRegistrationClient;
use nym_registration_common::{NymNodeLPInformation, WireguardConfiguration};
use nym_smol_dvpn::{PeerConfig, Tunnel, TunnelBuilder};
use rand09::SeedableRng;
use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::info;

#[derive(Parser, Debug)]
#[command(about = "Free-tier gateway harness driver (test-only)")]
struct Args {
    /// Base URL of the running gateway's HTTP API (used for discovery).
    #[arg(long, env = "FT_GATEWAY_HTTP", default_value = "http://localhost:8080")]
    gateway_http: String,

    /// Address the gateway's LP + WireGuard ports are reachable at (the published
    /// container ports on the host). Distinct from the node's announced IP.
    #[arg(long, env = "FT_GATEWAY_IP", default_value_t = IpAddr::V4(Ipv4Addr::LOCALHOST))]
    gateway_ip: IpAddr,

    /// Present a free-tier credential instead of the zk-nym (mock ecash) route.
    #[arg(long, env = "FT_FREE_TIER", default_value_t = false)]
    free_tier: bool,

    /// Optional override for the in-tunnel probe target. By default it is derived
    /// from the assigned client IP (the gateway's tunnel IP is the /16 network
    /// base + 1) plus the discovered metadata port; the gateway serves its
    /// private-metadata HTTP endpoint there, so reaching it proves the datapath.
    #[arg(long, env = "FT_PROBE")]
    probe: Option<SocketAddr>,

    /// Additional in-tunnel targets to reach-test (repeatable, `host:port`). Each is
    /// tried with an HTTP GET through the tunnel; reachable ones print the response
    /// status, unreachable ones (e.g. dropped by the walled garden) print BLOCKED.
    #[arg(long)]
    reach: Vec<SocketAddr>,
}

/// The node's registration-relevant parameters, discovered over HTTP.
#[derive(Debug)]
struct NodeInfo {
    /// ed25519 identity - the `gateway_identity` argument to `register_dvpn`.
    identity: ed25519::PublicKey,
    /// WireGuard public key (also returned by registration; kept for cross-check).
    wg_public_key: x25519::PublicKey,
    /// WireGuard tunnel UDP port.
    wg_port: u16,
    /// WireGuard private-metadata HTTP port (served at the gateway's tunnel IP).
    metadata_port: u16,
    /// LP handshake + registration parameters.
    lp: NymNodeLPInformation,
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_tracing_logger();

    let args = Args::parse();

    let info = discover_node(&args.gateway_http, args.gateway_ip)
        .await
        .context("failed to discover node parameters over HTTP")?;

    println!(
        "discovered node at {} (LP {}):",
        args.gateway_http, info.lp.address
    );
    println!("  identity      : {}", info.identity.to_base58_string());
    println!(
        "  wireguard     : {} (udp {})",
        info.wg_public_key.to_base58_string(),
        info.wg_port
    );
    println!(
        "  lp version    : {} / ciphersuite {:?}",
        info.lp.lp_protocol_version, info.lp.ciphersuite
    );
    println!(
        "  lp kem digests: {} KEM(s)",
        info.lp.expected_kem_key_hashes.len()
    );

    let metadata_port = info.metadata_port;
    let (cfg, wg) = register(info, args.free_tier)
        .await
        .context("registration failed")?;

    println!("registered - wireguard configuration:");
    println!("  gateway wg key: {}", cfg.public_key.to_base58_string());
    println!("  endpoint      : {}", cfg.endpoint);
    println!("  assigned ipv4 : {}", cfg.private_ipv4);
    println!("  assigned ipv6 : {}", cfg.private_ipv6);
    println!(
        "  preshared key : {}",
        if cfg.psk.is_some() { "set" } else { "none" }
    );

    // Bring up the userspace tunnel and prove the datapath by reaching the gateway's
    // in-tunnel private-metadata endpoint - the only in-tunnel target on a standalone
    // entry gateway (there is no exit NAT to reach the internet through).
    let probe = args
        .probe
        .unwrap_or_else(|| gateway_tunnel_addr(cfg.private_ipv4, metadata_port));

    let peer = PeerConfig {
        gateway_public_key: cfg.public_key.to_bytes(),
        client_private_key: wg.private_key().to_bytes(),
        preshared_key: cfg.psk.map(Into::into),
        endpoint: cfg.endpoint,
        assigned_ipv4: cfg.private_ipv4,
        assigned_ipv6: Some(cfg.private_ipv6),
    };

    info!("bringing up the userspace dVPN tunnel...");
    let tunnel = TunnelBuilder::single_hop(peer)
        .connect()
        .await
        .context("failed to bring up the dVPN tunnel")?;

    println!("in-tunnel reachability:");
    // the gateway's own metadata endpoint (proves the datapath itself)
    reach_test(&tunnel, probe).await;
    // any extra targets - internet egress and/or walled-garden allow/deny checks
    for target in &args.reach {
        reach_test(&tunnel, *target).await;
    }

    tunnel.shutdown().await;
    std::process::exit(0);
}

/// Register a single WireGuard hop against the discovered node over LP, mirroring
/// `register_hop` (`sdk/rust/nym-sdk-session/src/session.rs`). `free_tier = false`
/// takes the zk-nym route (accepted by the gateway's mock-ecash manager);
/// `free_tier = true` needs a token-bearing provider (a later increment).
async fn register(
    info: NodeInfo,
    free_tier: bool,
) -> Result<(WireguardConfiguration, x25519::KeyPair)> {
    let NodeInfo { identity, lp, .. } = info;
    let NymNodeLPInformation {
        address,
        expected_kem_key_hashes,
        x25519,
        ciphersuite,
        lp_protocol_version,
    } = lp;

    let dh = Arc::new(DHKeyPair::new(&mut rand09::rng()));
    let peer = LpRemotePeer::new(x25519).with_key_digests(expected_kem_key_hashes);
    let mut client = LpRegistrationClient::<TcpStream>::new_with_default_config(
        dh,
        peer,
        address,
        ciphersuite,
        lp_protocol_version,
    );

    client
        .perform_handshake()
        .await
        .context("LP handshake failed")?;

    info!("LP handshake successful, registering...");

    let mut rng = rand09::rngs::StdRng::from_os_rng();
    let wg = x25519::KeyPair::new(&mut rand::thread_rng());
    // zk-nym route: the mock provider ships a canned credential the gateway's
    // MockEcashManager accepts without verification (run with --lp-use-mock-ecash)
    let provider = MockBandwidthController::default();

    let cfg = client
        .register_dvpn(
            &mut rng,
            &wg,
            &identity,
            &provider,
            TicketType::V1WireguardEntry,
            free_tier,
        )
        .await
        .context("register_dvpn failed")?;

    Ok((cfg, wg))
}

/// Derive the gateway's tunnel IP from the assigned client address. nym assigns
/// clients from the gateway's /16 tunnel network with the gateway at the network
/// base + 1 (e.g. 10.1.0.1 for a 10.1.x.x client), so a non-default subnet base
/// still resolves. Pair it with the gateway's private-metadata port.
fn gateway_tunnel_addr(client_ipv4: Ipv4Addr, metadata_port: u16) -> SocketAddr {
    let o = client_ipv4.octets();
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(o[0], o[1], 0, 1)), metadata_port)
}

/// Reach-test one in-tunnel target and print the outcome. Never errors: a blocked
/// or unreachable target (e.g. dropped by the walled garden) is an expected result,
/// so it is reported, not propagated.
async fn reach_test(tunnel: &Tunnel, target: SocketAddr) {
    match fetch_through_tunnel(tunnel, target).await {
        Ok(status) => println!("  {target}: OK - {status}"),
        Err(e) => println!("  {target}: BLOCKED/unreachable - {e:#}"),
    }
}

/// Send a minimal HTTP GET to `target` through the tunnel and return the response
/// status line - exercises TCP through the tunnel + a real response end to end.
async fn fetch_through_tunnel(tunnel: &Tunnel, target: SocketAddr) -> Result<String> {
    let mut stream = timeout(Duration::from_secs(8), tunnel.tcp_connect(target))
        .await
        .context("connect timed out")?
        .context("connect failed")?;

    let request = format!("GET / HTTP/1.1\r\nHost: {target}\r\nConnection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .await
        .context("write failed")?;

    let mut buf = [0u8; 256];
    let n = timeout(Duration::from_secs(8), stream.read(&mut buf))
        .await
        .context("read timed out")?
        .context("read failed")?;

    let response = String::from_utf8_lossy(&buf[..n]);
    Ok(response
        .lines()
        .next()
        .unwrap_or("<no response>")
        .to_string())
}

/// Fetch the node's identity, WireGuard, and LP parameters from its HTTP API and
/// assemble them into a [`NodeInfo`]. Mirrors the SDK's `build_lp`
/// (`sdk/rust/nym-sdk-session/src/gateway.rs`) but sourced from HTTP, not topology.
async fn discover_node(base: &str, gateway_ip: IpAddr) -> Result<NodeInfo> {
    let client = nym_node_requests::api::Client::new(base.parse()?, None);

    // 1. identity
    let host_info = client.get_host_information().await?;
    let identity = host_info.keys.ed25519_identity;

    // 2. wireguard
    let wg = client.get_wireguard().await?;
    let wg_public_key: x25519::PublicKey = wg.public_key.parse()?;
    let wg_port = wg.tunnel_port;
    let metadata_port = wg.metadata_port;

    // 3. lewes protocol (typed, for the KEM/x25519 material)
    let lp = client.get_lewes_protocol().await?;

    let lp = NymNodeLPInformation {
        // connect to the reachable (published) address, not the node's announced IP
        address: SocketAddr::new(gateway_ip, lp.control_port),
        expected_kem_key_hashes: decode_kem_key_hashes(&lp.kem_keys)?,
        x25519: lp.x25519,
        ciphersuite: Ciphersuite::default(),
        lp_protocol_version: nym_lp_data::packet::version::CURRENT,
    };

    Ok(NodeInfo {
        identity,
        wg_public_key,
        wg_port,
        metadata_port,
        lp,
    })
}

/// Convert the HTTP `LewesProtocol.kem_keys` (hex-encoded digests keyed by the
/// node-request enums) into the `nym-kkt-ciphersuite` map an `LpRemotePeer` wants.
fn decode_kem_key_hashes(
    kem_keys: &BTreeMap<LPKEM, BTreeMap<LPHashFunction, String>>,
) -> Result<BTreeMap<KEM, KEMKeyDigests>> {
    let mut out = BTreeMap::new();
    for (kem, digests) in kem_keys {
        let mut inner: KEMKeyDigests = BTreeMap::new();
        for (hash_fn, hex_digest) in digests {
            let digest = hex::decode(hex_digest).context("malformed hex KEM digest")?;
            inner.insert(HashFunction::from(*hash_fn), digest);
        }
        out.insert(KEM::from(*kem), inner);
    }
    Ok(out)
}
