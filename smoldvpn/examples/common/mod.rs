// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Shared helpers for the `smoldvpn` example programs.
//!
//! Not an example itself (no `main`); included via
//! `#[path = "common/mod.rs"] mod common;`. Provides session/tunnel setup, the
//! hop → `PeerConfig` mapping, gateway-detail printing, an HTTPS-over-any-stream
//! fetcher used to query `ipinfo.io` (both directly and through the tunnel), and
//! a generic TLS-wrapping `tower` connector so `tonic` can speak gRPC-over-TLS
//! through the tunnel.

#![allow(dead_code)] // each example uses a subset of these helpers.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Once};
use std::task::{Context, Poll};

use http::Uri;
use hyper_util::rt::TokioIo;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_network_defaults::NymNetworkDetails;
use nym_sdk_session::{
    GatewayInfo, GatewaySpec, HopConfig, QuicBridge, Registration, Session, SessionConfig, WgRole,
};
use nym_smoldvpn::{BridgeParams, PeerConfig, Tunnel, TunnelBuilder};
use rustls::pki_types::ServerName;
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_rustls::TlsConnector;
use tower::Service;
use tracing::{info, warn};

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

static INSTALL_PROVIDER: Once = Once::new();

/// Install the rustls ring crypto provider once (needed for tokio-rustls).
pub fn init_crypto() {
    INSTALL_PROVIDER.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Install a `tracing` subscriber so example narration and the crate's
/// datapath/handshake logs are visible. Honours `RUST_LOG`
/// (e.g. `RUST_LOG=nym_smoldvpn=debug`); when unset it defaults to the running
/// example plus `smoldvpn` and `boringtun` at `info`. Idempotent — the
/// `try_` initialiser makes a second call a no-op.
pub fn init_logging() {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // `module_path!()` here is `<example_crate>::common`; the crate
                // root is the example's own log target, so its `info!` shows.
                let example = module_path!().split("::").next().unwrap_or("");
                tracing_subscriber::EnvFilter::new(format!(
                    "{example}=info,nym_smoldvpn=info,boringtun=info"
                ))
            }),
        )
        .try_init();
}

// --- session / tunnel setup -------------------------------------------------

/// The funded mnemonic from `MNEMONIC` (or `NYX_ACCOUNT_MNEMONIC`).
pub fn mnemonic() -> bip39::Mnemonic {
    std::env::var("MNEMONIC")
        .or_else(|_| std::env::var("NYX_ACCOUNT_MNEMONIC"))
        .expect("set MNEMONIC or NYX_ACCOUNT_MNEMONIC")
        .parse()
        .expect("valid bip39 mnemonic")
}

/// Default sandbox dVPN gateway-directory endpoint (provides gateway monikers +
/// QUIC bridge params). NOTE: the sandbox secrets file ships this URL with a
/// typo (`node-s/nbtatus-api`); this is the corrected value. Override with
/// `DVPN_DIRECTORY_URL`.
pub const DEFAULT_DVPN_DIRECTORY: &str =
    "https://sandbox-node-status-api.nymte.ch/dvpn/v1/directory/gateways";

/// The dVPN directory URL to use (`DVPN_DIRECTORY_URL` env, else the sandbox default).
pub fn dvpn_directory_url() -> String {
    std::env::var("DVPN_DIRECTORY_URL").unwrap_or_else(|_| DEFAULT_DVPN_DIRECTORY.to_string())
}

/// Per-example credential/data directory, laid out as `data/<example>/<network>`
/// (e.g. `data/zcash-sync/sandbox`). Created if missing, so the credential store
/// and fetcher recovery DBs land in a predictable, per-network location that is
/// reused across runs. Falls back to `unknown` when the network is unnamed.
pub fn example_data_dir(example: &str, network_name: &str) -> String {
    let network = if network_name.is_empty() {
        "unknown"
    } else {
        network_name
    };
    let dir = format!("data/{example}/{network}");
    std::fs::create_dir_all(&dir).expect("create example data dir");
    info!("credential store + data directory: {dir}");
    dir
}

/// Build a session against the network selected by the environment, storing
/// credentials under `data/<example>/<network>` (see [`example_data_dir`]). The
/// dVPN directory is configured so gateway monikers are populated and QUIC-bridge
/// entry selection is available.
pub async fn new_session(example: &str) -> Session {
    let network = NymNetworkDetails::new_from_env();
    let data_dir = example_data_dir(example, &network.network_name);
    Session::new(
        SessionConfig {
            mnemonic: mnemonic(),
            network,
            credential_store_path: Some(format!("{data_dir}/creds.db").into()),
            data_path: data_dir.into(),
            dvpn_directory_url: Some(dvpn_directory_url()),
            automatic_topups: None,
            bandwidth_provider: None,
            reuse_registrations: true,
        },
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .expect("session init")
}

/// Map a session [`QuicBridge`] into the datapath's [`BridgeParams`].
pub fn bridge_params(qb: &QuicBridge) -> BridgeParams {
    BridgeParams {
        addresses: qb.addresses.clone(),
        sni_host: qb.sni_host.clone(),
        id_pubkey_base64: qb.id_pubkey_base64.clone(),
    }
}

/// Bring up a two-hop tunnel from a [`Registration`]. When `use_quic` is set,
/// the entry leg is fronted by the QUIC bridge carried in `reg.entry.bridge`
/// (as produced by `Session::register_two_hop_quic`).
pub async fn build_two_hop_tunnel(reg: &Registration, use_quic: bool) -> Result<Tunnel, BoxError> {
    let entry = peer_from_hop(&reg.entry);
    let exit = peer_from_hop(
        reg.exit
            .as_ref()
            .ok_or("two-hop registration has no exit hop")?,
    );
    let mut builder = TunnelBuilder::two_hop(entry, exit);
    if use_quic {
        let qb = reg
            .entry
            .bridge
            .as_ref()
            .ok_or("QUIC requested but the entry hop carries no bridge params")?;
        builder = builder.quic_bridge(bridge_params(qb));
    }
    Ok(builder.connect().await?)
}

/// Bring up the tunnel described by `reg` (single- or two-hop, QUIC entry when
/// `use_quic`), dispatching on whether an exit hop is present.
pub async fn build_tunnel(reg: &Registration, use_quic: bool) -> Result<Tunnel, BoxError> {
    if reg.exit.is_some() {
        build_two_hop_tunnel(reg, use_quic).await
    } else {
        let entry = peer_from_hop(&reg.entry);
        Ok(TunnelBuilder::single_hop(entry).connect().await?)
    }
}

/// Bring up a tunnel with gateway-side bandwidth top-up wired in — the recommended default for a
/// long-lived, session-built tunnel. Spends already-stored tickets (obtained via the session's
/// bandwidth provider) against the in-tunnel `metadata` endpoint before bandwidth runs out, and
/// exposes [`nym_smoldvpn::BandwidthEvent`]s via `tunnel.bandwidth_events()`.
///
/// The bandwidth top-up meters at the exit gateway (the sole hop for one-hop), so it spends
/// `WgRole::Exit` tickets there.
pub async fn build_tunnel_with_topup(
    reg: &Registration,
    session: &Session,
    metadata_url: String,
    use_quic: bool,
) -> Result<Tunnel, BoxError> {
    use nym_credentials_interface::TicketType;
    use nym_smoldvpn::{ProviderCredentialSource, TopupConfig};

    // The metering gateway is the exit (or the sole gateway for one-hop).
    let (metering, ticket_type) = match reg.exit.as_ref() {
        Some(exit) => (exit, TicketType::V1WireguardExit),
        None => (&reg.entry, TicketType::V1WireguardEntry),
    };

    let source = std::sync::Arc::new(ProviderCredentialSource::new(
        session.bandwidth_provider(),
        metering.gateway_identity,
        ticket_type,
    ));

    let mut builder = if let Some(exit) = reg.exit.as_ref() {
        let entry = peer_from_hop(&reg.entry);
        let exit = peer_from_hop(exit);
        let mut b = TunnelBuilder::two_hop(entry, exit);
        if use_quic {
            let qb = reg
                .entry
                .bridge
                .as_ref()
                .ok_or("QUIC requested but the entry hop carries no bridge params")?;
            b = b.quic_bridge(bridge_params(qb));
        }
        b
    } else {
        TunnelBuilder::single_hop(peer_from_hop(&reg.entry))
    };
    builder = builder.bandwidth_topup(TopupConfig::new(metadata_url), source);
    Ok(builder.connect().await?)
}

// --- connect: register (cache-served when possible) + establish-gated bring-up

/// Bound for tunnel establishment: healthy bring-up is ~100ms; WireGuard
/// retransmits handshakes ~5s apart, so 15s allows several attempts before a
/// hop is declared dead (e.g. a cached registration whose gateway-side peer is
/// gone).
pub const ESTABLISH_BOUND: std::time::Duration = std::time::Duration::from_secs(15);

/// Register the tunnel described by `cli` (served from the session's
/// registration cache when possible — zero tickets), bring it up, and gate on
/// [`Tunnel::await_established`]. If establishment fails within
/// [`ESTABLISH_BOUND`] — the signature of a stale cached registration — the
/// failed hop(s) are invalidated and registered fresh (spending only those
/// hops' tickets), and the tunnel is rebuilt once.
pub async fn connect(session: &Session, cli: &Cli) -> Result<(Registration, Tunnel), BoxError> {
    let reg = register(session, cli).await?;
    let tunnel = build_tunnel(&reg, cli.quic).await?;
    let status = match tunnel.await_established(ESTABLISH_BOUND).await {
        Ok(()) => return Ok((reg, tunnel)),
        Err(status) => status,
    };

    warn!(
        "cached registration failed to establish within {ESTABLISH_BOUND:?} ({status}); \
         re-registering"
    );
    tunnel.shutdown().await;
    if !status.entry {
        session.invalidate_registration(&reg.entry.gateway_identity, WgRole::Entry);
    }
    if let (Some(hop), Some(false)) = (reg.exit.as_ref(), status.exit) {
        session.invalidate_registration(&hop.gateway_identity, WgRole::Exit);
    }

    let reg = register(session, cli).await?;
    let tunnel = build_tunnel(&reg, cli.quic).await?;
    tunnel
        .await_established(ESTABLISH_BOUND)
        .await
        .map_err(|s| format!("tunnel failed to establish after fresh registration: {s}"))?;
    Ok((reg, tunnel))
}

/// Query ipinfo.io through the tunnel with a few quick retries. The tunnel is
/// already establishment-gated by [`connect`], so this is a display probe, not
/// a warmup loop — the first packets can still race the exit's NAT setup.
pub async fn ipinfo_display(tunnel: &Tunnel) -> Result<Value, BoxError> {
    let mut last_err: Option<BoxError> = None;
    for attempt in 1..=3 {
        match ipinfo_via_tunnel(tunnel).await {
            Ok(v) => return Ok(v),
            Err(e) => {
                warn!("ipinfo probe attempt {attempt} failed ({e}); retrying");
                last_err = Some(e);
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }
    Err(last_err.unwrap_or_else(|| "ipinfo probe failed".into()))
}

// --- CLI --------------------------------------------------------------------

/// Usage string shared by the configurable examples.
pub const USAGE: &str = "\
options:
  --two-hop            entry + exit gateways (default)
  --one-hop            a single gateway (entry == exit); cannot be combined with --quic
  --entry <SPEC>       entry gateway selector (default: random)
  --exit  <SPEC>       exit gateway selector  (default: random)
  --gateway <SPEC>     set both entry and exit (handy for --one-hop)
  --quic               require a QUIC-bridge-capable entry gateway (two-hop only)
  --blocks <N>         number of blocks to sync (zcash-sync only; default 10000)
  -h, --help           print this help

<SPEC> is one of:
  random               any WireGuard-capable gateway (default)
  <CC>                 a two-letter ISO country code, e.g. DE, CH
  <identity>           an exact gateway ed25519 identity (base58)";

/// Parsed command-line options for the configurable examples.
pub struct Cli {
    /// Two-hop (entry + exit) when true; single-hop when false.
    pub two_hop: bool,
    /// Entry (or sole) gateway selector.
    pub entry: GatewaySpec,
    /// Exit gateway selector (ignored for single-hop).
    pub exit: GatewaySpec,
    /// Require a QUIC-bridge entry gateway (two-hop only).
    pub quic: bool,
    /// Number of blocks to sync (used by `zcash-sync` only; `None` = its default).
    pub blocks: Option<u64>,
}

/// Parse a gateway `<SPEC>`: `random`, a two-letter country code, or a base58
/// ed25519 identity key.
fn parse_spec(s: &str) -> Result<GatewaySpec, BoxError> {
    if s.eq_ignore_ascii_case("random") {
        Ok(GatewaySpec::Random)
    } else if s.len() == 2 && s.chars().all(|c| c.is_ascii_alphabetic()) {
        Ok(GatewaySpec::Country(s.to_ascii_uppercase()))
    } else {
        let key = ed25519::PublicKey::from_base58_string(s)
            .map_err(|e| format!("invalid gateway spec {s:?}: {e}"))?;
        Ok(GatewaySpec::Identity(key))
    }
}

/// Parse the process args into a [`Cli`] (prints usage and exits on `-h`).
pub fn parse_cli() -> Result<Cli, BoxError> {
    let mut two_hop = true;
    let (mut entry, mut exit) = (GatewaySpec::Random, GatewaySpec::Random);
    let mut quic = false;
    let mut blocks = None;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    let next = |i: &mut usize, flag: &str| -> Result<String, BoxError> {
        *i += 1;
        args.get(*i)
            .cloned()
            .ok_or_else(|| format!("{flag} requires a value").into())
    };
    while i < args.len() {
        match args[i].as_str() {
            "--quic" => quic = true,
            "--one-hop" | "--single-hop" => two_hop = false,
            "--two-hop" => two_hop = true,
            "--entry" => entry = parse_spec(&next(&mut i, "--entry")?)?,
            "--exit" => exit = parse_spec(&next(&mut i, "--exit")?)?,
            "--gateway" => {
                let s = parse_spec(&next(&mut i, "--gateway")?)?;
                entry = s.clone();
                exit = s;
            }
            "--blocks" => {
                let v = next(&mut i, "--blocks")?;
                blocks = Some(
                    v.parse()
                        .map_err(|_| format!("--blocks expects a number, got {v:?}"))?,
                );
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument {other:?}\n\n{USAGE}").into()),
        }
        i += 1;
    }

    if quic && !two_hop {
        return Err("--quic requires two-hop mode (QUIC fronts the entry leg only)".into());
    }
    Ok(Cli {
        two_hop,
        entry,
        exit,
        quic,
        blocks,
    })
}

/// Issue the required ticketbooks and register the gateway(s) for `cli`.
pub async fn register(session: &Session, cli: &Cli) -> Result<Registration, BoxError> {
    session.ensure_ticketbooks(cli.two_hop).await?;
    let reg = if !cli.two_hop {
        session.register_single_hop(&cli.entry).await?
    } else if cli.quic {
        session.register_two_hop_quic(&cli.entry, &cli.exit).await?
    } else {
        session.register_two_hop(&cli.entry, &cli.exit).await?
    };
    Ok(reg)
}

/// One-line description of the tunnel a [`Cli`] will bring up.
pub fn describe(cli: &Cli) -> String {
    let mode = if cli.two_hop { "two-hop" } else { "single-hop" };
    let quic = if cli.quic { " (QUIC entry)" } else { "" };
    format!("{mode}{quic}")
}

/// Map a session hop into the datapath's transport-agnostic peer config.
pub fn peer_from_hop(hop: &HopConfig) -> PeerConfig {
    PeerConfig {
        gateway_public_key: hop.wg_config.public_key,
        // `x25519::PrivateKey` is deliberately `!Clone`; reconstruct from bytes so
        // the registration (and its key) outlives tunnel construction — `connect()`
        // needs it for per-hop invalidation when establishment fails.
        client_private_key: x25519::PrivateKey::from_secret(hop.client_private_key.to_bytes()),
        preshared_key: hop.wg_config.psk.as_ref().map(|p| *p.as_bytes()),
        endpoint: hop.wg_config.endpoint,
        assigned_ipv4: hop.wg_config.private_ipv4,
        assigned_ipv6: Some(hop.wg_config.private_ipv6),
    }
}

/// Print a hop's gateway directory details. Nym nodes carry no free-text
/// moniker, so the node id is the human-facing identifier.
pub fn print_gateway(label: &str, gw: &GatewayInfo) {
    info!(
        identity = %gw.identity.to_base58_string(),
        moniker = gw.name.as_deref().unwrap_or("(none — Nym nodes have no moniker)"),
        node_id = %gw.node_id,
        country = gw.country.as_deref().unwrap_or("unknown"),
        ip = %gw.ip,
        "{label} gateway",
    );
}

// --- HTTPS (ipinfo.io) ------------------------------------------------------

const IPINFO_HOST: &str = "ipinfo.io";

/// Build a rustls client config trusting the webpki roots, advertising `alpn`.
pub fn tls_config(alpn: &[&[u8]]) -> Arc<rustls::ClientConfig> {
    init_crypto();
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let mut cfg = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    cfg.alpn_protocols = alpn.iter().map(|a| a.to_vec()).collect();
    Arc::new(cfg)
}

/// TLS-handshake `stream`, issue `GET {path}` to `host`, and return the JSON
/// body parsed from the response (robust to chunked framing: extracts the
/// `{ ... }` object). `stream` may be a direct socket or a tunnel `TcpStream`.
pub async fn https_get_json<S>(stream: S, host: &str, path: &str) -> Result<Value, BoxError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let connector = TlsConnector::from(tls_config(&[&b"http/1.1"[..]]));
    let sni = ServerName::try_from(host.to_string())?;
    let mut tls = connector.connect(sni, stream).await?;

    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: smoldvpn\r\n\
         Accept: application/json\r\nConnection: close\r\n\r\n"
    );
    tls.write_all(req.as_bytes()).await?;

    // Read to EOF, tolerating servers that close a `Connection: close` response
    // without a TLS close_notify (rustls surfaces that as `UnexpectedEof`).
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match tls.read(&mut chunk).await {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }
    }
    let text = String::from_utf8_lossy(&buf);
    let start = text.find('{').ok_or("no JSON body in response")?;
    let end = text.rfind('}').ok_or("truncated JSON body")?;
    Ok(serde_json::from_str(&text[start..=end])?)
}

/// Fetch `ipinfo.io/json` over a directly-dialed socket (your real IP).
pub async fn ipinfo_direct() -> Result<Value, BoxError> {
    let tcp = tokio::net::TcpStream::connect((IPINFO_HOST, 443)).await?;
    https_get_json(tcp, IPINFO_HOST, "/json").await
}

/// Fetch `ipinfo.io/json` through the tunnel (should report the exit gateway).
pub async fn ipinfo_via_tunnel(tunnel: &Tunnel) -> Result<Value, BoxError> {
    let stream = tunnel.tcp_connect_host(IPINFO_HOST, 443).await?;
    https_get_json(stream, IPINFO_HOST, "/json").await
}

/// One-line summary of an ipinfo response: `ip (city, country) — org`.
pub fn fmt_ipinfo(v: &Value) -> String {
    let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("?").to_string();
    format!(
        "{} ({}, {}) — {}",
        s("ip"),
        s("city"),
        s("country"),
        s("org")
    )
}

// --- TLS-wrapping tower connector (for gRPC-over-TLS via any transport) ------

/// A `tower` connector that dials a direct `tokio` TCP socket to the URI's
/// authority, yielding a hyper-compatible IO handle.
#[derive(Clone, Default)]
pub struct DirectConnector;

impl Service<Uri> for DirectConnector {
    type Response = TokioIo<tokio::net::TcpStream>;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, BoxError>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, uri: Uri) -> Self::Future {
        Box::pin(async move {
            let host = uri.host().ok_or("uri missing host")?.to_string();
            let port = uri.port_u16().unwrap_or(443);
            let tcp = tokio::net::TcpStream::connect((host.as_str(), port)).await?;
            Ok(TokioIo::new(tcp))
        })
    }
}

/// Wraps an inner connector (yielding `TokioIo<S>`) and layers rustls TLS on
/// top, yielding `TokioIo<TlsStream<S>>` so `tonic` speaks h2 over TLS. SNI is
/// taken from the requested URI's host.
#[derive(Clone)]
pub struct TlsWrap<C> {
    inner: C,
    config: Arc<rustls::ClientConfig>,
}

impl<C> TlsWrap<C> {
    /// Wrap `inner`, advertising ALPN `h2` for gRPC.
    pub fn h2(inner: C) -> Self {
        Self {
            inner,
            config: tls_config(&[&b"h2"[..]]),
        }
    }
}

impl<C, S> Service<Uri> for TlsWrap<C>
where
    C: Service<Uri, Response = TokioIo<S>> + Clone + Send + 'static,
    C::Future: Send + 'static,
    C::Error: Into<BoxError>,
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    type Response = TokioIo<tokio_rustls::client::TlsStream<S>>;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, BoxError>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, uri: Uri) -> Self::Future {
        let mut inner = self.inner.clone();
        let config = self.config.clone();
        Box::pin(async move {
            let host = uri.host().ok_or("uri missing host")?.to_string();
            let io = inner.call(uri).await.map_err(Into::into)?;
            let stream = io.into_inner();
            let sni = ServerName::try_from(host)?;
            let tls = TlsConnector::from(config).connect(sni, stream).await?;
            Ok(TokioIo::new(tls))
        })
    }
}
