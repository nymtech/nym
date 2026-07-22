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

use anyhow::{Context, Result, anyhow, bail};
use async_trait::async_trait;
use clap::Parser;
use nym_bandwidth_controller::error::BandwidthControllerError;
use nym_bandwidth_controller::mock::MockBandwidthController;
use nym_bandwidth_controller::{
    BandwidthTicketProvider, PreparedCredential, PreparedCredentialMetadata,
};
use nym_bin_common::logging::setup_tracing_logger;
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_free_tier_check::{CREDENTIAL_PROXY_JWT_ISSUER, FreeTierPurpose, generate_free_tier_jwt};
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
use std::time::{Duration, Instant};
use time::OffsetDateTime;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
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

    /// Base58 ed25519 PRIVATE key of the free-tier signer, used to mint the demo's
    /// capability token. Defaults to the committed TEST-ONLY harness signer key (matches
    /// `.env` `NYM_FREE_TIER_SIGNER_PRIVKEY`); only meaningful with `--free-tier`.
    #[arg(
        long,
        env = "NYM_FREE_TIER_SIGNER_PRIVKEY",
        default_value = "8SFEk5QPzFKLF2YoYVhZcQPhTyjbKmB9shqhvx9VUUPa"
    )]
    free_tier_signer_privkey: String,

    /// Bulk-download a URL through the tunnel, timed (repeatable). Reports bytes, elapsed,
    /// and throughput - use it to show the rate-limit throttle (free vs paid) and to
    /// exhaust the free allowance (download the big file) then confirm the walled garden
    /// (a non-whitelisted URL becomes BLOCKED while a whitelisted one still succeeds). The
    /// host is DNS-resolved on this machine and the resolved IP is printed (whitelist it).
    #[arg(long)]
    download: Vec<String>,

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
    let (cfg, wg) = register(info, args.free_tier, &args.free_tier_signer_privkey)
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

    // bulk downloads: throttle demo (free vs paid) + allowance exhaustion -> walled garden
    if !args.download.is_empty() {
        println!("bulk downloads (through the tunnel):");
        for url in &args.download {
            println!("  {url}");
            match download_through_tunnel(&tunnel, url).await {
                Ok(report) => println!("    -> {report}"),
                Err(e) => println!("    -> BLOCKED/failed - {e:#}"),
            }
        }
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
    signer_privkey: &str,
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

    // free-tier: present a minted NewUser capability token; otherwise the zk-nym route,
    // where the mock provider ships a canned credential the gateway's MockEcashManager
    // accepts without verification (run with --lp-use-mock-ecash).
    let provider: Box<dyn BandwidthTicketProvider> = if free_tier {
        let token = mint_free_tier_jwt(signer_privkey).context("failed to mint free-tier token")?;
        info!("minted free-tier NewUser capability token");
        Box::new(FreeTrialProvider { token })
    } else {
        Box::new(MockBandwidthController::default())
    };

    let cfg = client
        .register_dvpn(
            &mut rng,
            &wg,
            &identity,
            provider.as_ref(),
            TicketType::V1WireguardEntry,
            free_tier,
        )
        .await
        .context("register_dvpn failed")?;

    Ok((cfg, wg))
}

/// A [`BandwidthTicketProvider`] presenting a single pre-minted free-tier capability token.
/// Only `get_free_trial_token` is exercised on the free-tier registration path; the other
/// methods are unused stubs (the free tier never presents an ecash ticket).
struct FreeTrialProvider {
    token: String,
}

#[async_trait]
impl BandwidthTicketProvider for FreeTrialProvider {
    async fn get_ecash_ticket(
        &self,
        _ticket_type: TicketType,
        _gateway_id: ed25519::PublicKey,
        _tickets_to_spend: u32,
        _spend_time: OffsetDateTime,
    ) -> Result<Option<PreparedCredential>, BandwidthControllerError> {
        Ok(None)
    }

    async fn get_upgrade_mode_token(&self) -> Result<Option<String>, BandwidthControllerError> {
        Ok(None)
    }

    async fn get_free_trial_token(&self) -> Result<Option<String>, BandwidthControllerError> {
        Ok(Some(self.token.clone()))
    }

    async fn attempt_revert_spending(
        &self,
        _metadata: PreparedCredentialMetadata,
    ) -> Result<bool, BandwidthControllerError> {
        Ok(true)
    }

    async fn close(&self) {}
}

/// Mint a NewUser free-tier capability JWT signed by the harness signer key (base58 ed25519
/// private key). Mirrors what the credential proxy issues; the gateway verifies it offline.
fn mint_free_tier_jwt(signer_privkey_b58: &str) -> Result<String> {
    let private = ed25519::PrivateKey::from_base58_string(signer_privkey_b58)
        .context("invalid free-tier signer private key")?;
    let keypair = ed25519::KeyPair::from(private);
    Ok(generate_free_tier_jwt(
        Duration::from_secs(3600),
        &keypair,
        Some(CREDENTIAL_PROXY_JWT_ISSUER),
        FreeTierPurpose::NewUser,
    ))
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

/// Bulk-download `url` through the tunnel, timed, and return a human report. The host is
/// DNS-resolved on THIS machine (the resolved IP is included so it can be whitelisted), the
/// resolved IP is connected through the tunnel, TLS is used for `https`, and up to a few
/// redirects are followed. A mid-transfer stall (e.g. the walled garden dropping the peer
/// once the allowance is exhausted) surfaces as a read timeout = a BLOCKED result.
async fn download_through_tunnel(tunnel: &Tunnel, url_str: &str) -> Result<String> {
    let mut url = reqwest::Url::parse(url_str).context("invalid URL")?;
    let start = Instant::now();

    for _hop in 0..5 {
        let host = url
            .host_str()
            .ok_or_else(|| anyhow!("URL has no host"))?
            .to_string();
        let https = match url.scheme() {
            "https" => true,
            "http" => false,
            other => bail!("unsupported URL scheme: {other}"),
        };
        let port = url
            .port_or_known_default()
            .unwrap_or(if https { 443 } else { 80 });
        let mut path = url.path().to_string();
        if path.is_empty() {
            path.push('/');
        }
        if let Some(query) = url.query() {
            path.push('?');
            path.push_str(query);
        }

        // resolve on this machine; connect the resolved IP through the tunnel.
        // Force IPv4: the tunnel stack only carries a v4 default route (smol-core
        // dual-stack is deferred), and the local resolver may return AAAA (v6) first.
        let ip = timeout(
            Duration::from_secs(5),
            tokio::net::lookup_host((host.as_str(), port)),
        )
        .await
        .context("DNS resolution timed out")?
        .context("DNS resolution failed")?
        .map(|addr| addr.ip())
        .find(IpAddr::is_ipv4)
        .ok_or_else(|| anyhow!("no IPv4 address resolved for {host} (tunnel is v4-only)"))?;

        let stream = timeout(
            Duration::from_secs(15),
            tunnel.tcp_connect(SocketAddr::new(ip, port)),
        )
        .await
        .context("connect timed out")?
        .with_context(|| format!("connect to {ip}:{port} failed"))?;

        let (status, location, bytes) = if https {
            let tls = tls_wrap(stream, &host).await?;
            http_get_drain(tls, &host, &path).await?
        } else {
            http_get_drain(stream, &host, &path).await?
        };

        if (300..400).contains(&status) {
            let location = location.ok_or_else(|| anyhow!("HTTP {status} without a Location"))?;
            url = url.join(&location).context("invalid redirect Location")?;
            continue;
        }
        if status != 200 {
            bail!("HTTP {status}");
        }

        let elapsed = start.elapsed().as_secs_f64();
        let mb = bytes as f64 / 1_000_000.0;
        let mbps = mb / elapsed.max(0.001);
        return Ok(format!(
            "{mb:.1} MB in {elapsed:.1}s ({mbps:.2} MB/s) via {ip}"
        ));
    }

    bail!("too many redirects")
}

/// Wrap a tunnel TCP stream in a TLS client session (webpki roots, SNI = `host`).
async fn tls_wrap<S: AsyncRead + AsyncWrite + Unpin>(
    stream: S,
    host: &str,
) -> Result<tokio_rustls::client::TlsStream<S>> {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder_with_provider(Arc::new(
        tokio_rustls::rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .context("tls protocol versions")?
    .with_root_certificates(roots)
    .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));
    let server_name = ServerName::try_from(host.to_string()).context("invalid TLS server name")?;
    connector
        .connect(server_name, stream)
        .await
        .context("TLS handshake failed")
}

/// Issue an HTTP/1.1 GET over `stream` and drain the full response body, counting its bytes.
/// Returns `(status_code, Location header, body_bytes)`. Each read has a timeout so a stalled
/// transfer (dropped by the walled garden) fails fast rather than hanging. Prints a progress
/// line (cumulative bytes + current speed) roughly once a second while the body streams.
async fn http_get_drain<S: AsyncRead + AsyncWrite + Unpin>(
    mut stream: S,
    host: &str,
    path: &str,
) -> Result<(u16, Option<String>, u64)> {
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: nym-ft-demo\r\nAccept: */*\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .await
        .context("write request")?;
    stream.flush().await.ok();

    let mut header = Vec::new();
    let mut header_done = false;
    let mut body_bytes: u64 = 0;
    let mut buf = vec![0u8; 64 * 1024];
    // progress reporting: cumulative bytes + speed over the last interval
    let mut last_report = Instant::now();
    let mut last_bytes: u64 = 0;
    loop {
        let n = timeout(Duration::from_secs(15), stream.read(&mut buf))
            .await
            .context("read stalled (blocked?)")?
            .context("read failed")?;
        if n == 0 {
            break;
        }
        if header_done {
            body_bytes += n as u64;
        } else {
            header.extend_from_slice(&buf[..n]);
            if let Some(end) = find_subslice(&header, b"\r\n\r\n") {
                let body_start = end + 4;
                body_bytes += (header.len() - body_start) as u64;
                header.truncate(body_start);
                header_done = true;
            }
        }
        let interval = last_report.elapsed();
        if header_done && interval >= Duration::from_secs(1) {
            let mbps = (body_bytes - last_bytes) as f64 / 1_000_000.0 / interval.as_secs_f64();
            println!(
                "     {:.1} MB downloaded ({mbps:.2} MB/s)",
                body_bytes as f64 / 1_000_000.0
            );
            last_report = Instant::now();
            last_bytes = body_bytes;
        }
    }

    Ok((
        parse_status(&header)?,
        parse_header_value(&header, "location"),
        body_bytes,
    ))
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn parse_status(header: &[u8]) -> Result<u16> {
    let text = String::from_utf8_lossy(header);
    let status_line = text
        .lines()
        .next()
        .ok_or_else(|| anyhow!("empty response"))?;
    status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| anyhow!("malformed status line: {status_line}"))?
        .parse::<u16>()
        .context("bad status code")
}

fn parse_header_value(header: &[u8], name: &str) -> Option<String> {
    let text = String::from_utf8_lossy(header);
    text.lines().skip(1).find_map(|line| {
        line.split_once(':').and_then(|(k, v)| {
            k.trim()
                .eq_ignore_ascii_case(name)
                .then(|| v.trim().to_string())
        })
    })
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
