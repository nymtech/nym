// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

//! Shared helpers for the `nym-smol-dvpn` example programs.
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
use nym_network_defaults::NymNetworkDetails;
use nym_sdk_session::{GatewayInfo, HopConfig, QuicBridge, Registration, Session, SessionConfig};
use nym_smol_dvpn::{BridgeParams, PeerConfig, Tunnel, TunnelBuilder};
use rustls::pki_types::ServerName;
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_rustls::TlsConnector;
use tower::Service;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

static INSTALL_PROVIDER: Once = Once::new();

/// Install the rustls ring crypto provider once (needed for tokio-rustls).
pub fn init_crypto() {
    INSTALL_PROVIDER.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
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

/// Build a session against the sandbox network (from env), storing credentials
/// under `data_dir`. The dVPN directory is configured so gateway monikers are
/// populated and QUIC-bridge entry selection is available.
pub async fn new_session(data_dir: &str) -> Session {
    Session::new(
        SessionConfig {
            mnemonic: mnemonic(),
            network: NymNetworkDetails::new_from_env(),
            credential_store_path: Some(format!("{data_dir}/creds.db").into()),
            data_path: data_dir.into(),
            dvpn_directory_url: Some(dvpn_directory_url()),
        },
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .expect("session init")
}

/// Map a session [`QuicBridge`] into the datapath's [`BridgeParams`].
pub fn bridge_params(qb: &QuicBridge) -> Result<BridgeParams, BoxError> {
    Ok(BridgeParams {
        addresses: qb.addresses.clone(),
        sni_host: qb.sni_host.clone(),
        id_pubkey: BridgeParams::id_pubkey_from_base64(&qb.id_pubkey_base64)?,
    })
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
        builder = builder.quic_bridge(bridge_params(qb)?);
    }
    Ok(builder.connect().await?)
}

/// Map a session hop into the datapath's transport-agnostic peer config.
pub fn peer_from_hop(hop: &HopConfig) -> PeerConfig {
    PeerConfig {
        gateway_public_key: hop.wg_config.public_key.to_bytes(),
        client_private_key: hop.client_private_key.to_bytes(),
        preshared_key: hop.wg_config.psk.as_ref().map(|p| *p.as_bytes()),
        endpoint: hop.wg_config.endpoint,
        assigned_ipv4: hop.wg_config.private_ipv4,
        assigned_ipv6: Some(hop.wg_config.private_ipv6),
    }
}

/// Print a hop's gateway directory details. Nym nodes carry no free-text
/// moniker, so the node id is the human-facing identifier.
pub fn print_gateway(label: &str, gw: &GatewayInfo) {
    println!("  {label} gateway:");
    println!("    identity : {}", gw.identity.to_base58_string());
    println!(
        "    moniker  : {}",
        gw.name
            .as_deref()
            .unwrap_or("(none — Nym nodes have no moniker)")
    );
    println!("    node id  : {}", gw.node_id);
    println!(
        "    country  : {}",
        gw.country.as_deref().unwrap_or("unknown")
    );
    println!("    ip       : {}", gw.ip);
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
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: nym-smol-dvpn\r\n\
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
