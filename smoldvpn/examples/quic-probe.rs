// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! `quic-probe` — test QUIC-bridge connectivity to sandbox gateways, isolating
//! infrastructure reachability from our client's cert-pinning/framing.
//!
//! For each target it runs two checks against the bridge's IPv4 `:4443`:
//!   1. **raw**   — a bare `quinn` handshake (ALPN `hq-29`, cert verification
//!      DISABLED). Success ⇒ the bridge server is reachable and speaks our
//!      QUIC/ALPN; a timeout ⇒ infra (UDP unreachable).
//!   2. **pinned** — our real `nym_smoldvpn::probe_bridge` (ed25519 cert
//!      pinning + `open_bi`). A failure here while raw succeeds ⇒ our bug.
//!
//! Defaults to the two known sandbox QUIC gateways; override with
//! `--addr <ip:port> --sni <host> --id <base64-ed25519>`.
//!
//! Usage (no mnemonic/network needed — this only touches the bridge):
//!   cargo run --release -p nym-smoldvpn --example quic-probe

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use nym_smoldvpn::{probe_bridge, BridgeParams};
use quinn::crypto::rustls::QuicClientConfig;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

struct Target {
    name: &'static str,
    addr: &'static str,
    sni: &'static str,
    id_b64: &'static str,
}

/// Known sandbox QUIC gateways (from the dVPN directory).
const DEFAULTS: &[Target] = &[
    Target {
        name: "road trust program (CH, ExitGateway)",
        addr: "172.232.192.169:4443",
        sni: "netdna.bootstrapcdn.com",
        id_b64: "W6YTPX10G0CsLS4ur0r6jBiem3QCrstQV7ZcRte/oP0=",
    },
    Target {
        name: "decorate notice hedgehog (CH, EntryGateway)",
        addr: "18.171.210.241:4443",
        sni: "netdna.bootstrapcdn.com",
        id_b64: "c1dhOYrb2rjCLUDQt6nWxQ8/fV+cu7rhKoyi1xZCLqk=",
    },
];

/// Accept-any cert verifier for the raw reachability check (ISOLATION ONLY —
/// never used for real traffic).
#[derive(Debug)]
struct AcceptAll;

impl ServerCertVerifier for AcceptAll {
    fn verify_server_cert(
        &self,
        _e: &CertificateDer<'_>,
        _i: &[CertificateDer<'_>],
        _n: &ServerName<'_>,
        _o: &[u8],
        _t: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        _m: &[u8],
        _c: &CertificateDer<'_>,
        _d: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }
    fn verify_tls13_signature(
        &self,
        _m: &[u8],
        _c: &CertificateDer<'_>,
        _d: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::ED25519,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PSS_SHA256,
        ]
    }
}

/// A bare `quinn` handshake with the given ALPN and cert verification disabled.
async fn raw_handshake(addr: SocketAddr, sni: &str, alpn: &[u8]) -> Result<(), BoxError> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(AcceptAll))
        .with_no_client_auth();
    crypto.alpn_protocols = vec![alpn.to_vec()];
    let qcc = QuicClientConfig::try_from(crypto)?;
    let mut ep = quinn::Endpoint::client("0.0.0.0:0".parse()?)?;
    ep.set_default_client_config(quinn::ClientConfig::new(Arc::new(qcc)));
    let connecting = ep.connect(addr, sni)?;
    let conn = tokio::time::timeout(Duration::from_secs(10), connecting).await??;
    let alpn_neg = conn
        .handshake_data()
        .and_then(|d| d.downcast::<quinn::crypto::rustls::HandshakeData>().ok())
        .and_then(|d| d.protocol)
        .map(|p| String::from_utf8_lossy(&p).into_owned())
        .unwrap_or_else(|| "<none>".into());
    info!("raw: OK — handshake completed, ALPN={alpn_neg}");
    ep.wait_idle().await;
    Ok(())
}

async fn probe_target(name: &str, addr: &str, sni: &str, id_b64: &str) {
    info!("== {name} ==");
    info!("addr={addr} sni={sni}");
    let sock: SocketAddr = match addr.parse() {
        Ok(s) => s,
        Err(e) => {
            warn!("invalid addr: {e}");
            return;
        }
    };

    // 1. Raw reachability (hq-29).
    match raw_handshake(sock, sni, b"hq-29").await {
        Ok(()) => {}
        Err(e) => warn!("raw: FAIL — {e}"),
    }

    // 2. Our real pinned client (the id_pubkey is validated by the client at
    // connect time).
    let params = BridgeParams {
        addresses: vec![sock],
        sni_host: Some(sni.to_string()),
        id_pubkey_base64: id_b64.to_string(),
    };
    match probe_bridge(&params, &CancellationToken::new()).await {
        Ok(()) => info!("pinned: OK — bridge connect + open_bi succeeded"),
        Err(e) => warn!("pinned: FAIL — {e}"),
    }
}

/// Install a `tracing` subscriber so example narration and the crate's
/// datapath/handshake logs are visible. Honours `RUST_LOG`
/// (e.g. `RUST_LOG=nym_smoldvpn=debug`); when unset it defaults to this example
/// plus `smoldvpn` and `boringtun` at `info`.
fn init_logging() {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // `module_path!()` is this example's crate — its own log target.
                let example = module_path!().split("::").next().unwrap_or("");
                tracing_subscriber::EnvFilter::new(format!(
                    "{example}=info,nym_smoldvpn=info,boringtun=info"
                ))
            }),
        )
        .try_init();
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    init_logging();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };

    if let (Some(addr), Some(sni), Some(id)) = (arg("--addr"), arg("--sni"), arg("--id")) {
        probe_target("custom", &addr, &sni, &id).await;
    } else {
        for t in DEFAULTS {
            probe_target(t.name, t.addr, t.sni, t.id_b64).await;
        }
    }
    info!("done.");
    std::process::exit(0);
}
