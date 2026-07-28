// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! `smoldvpn-grpc` — issue a `tonic` gRPC request through the tunnel
//! (OpenSpec task 5.4).
//!
//! Brings up a single-hop tunnel, builds a `tonic` channel over the tunnel's
//! connector (so the gRPC/HTTP2 traffic flows inside the tunnel), and performs a
//! gRPC Health `Check` against `--target host:port` using `tonic-health`'s
//! ready-made client (no custom proto/build.rs). The target must be a gRPC
//! service exposing the standard Health service, reachable from the exit gateway.
//!
//! Usage:
//!   MNEMONIC="<funded mnemonic>" \
//!   cargo run -p nym-smoldvpn --example smoldvpn-grpc -- \
//!     --gateway <spec> --target <host:port> [--service <name>]

use std::process::ExitCode;

use nym_crypto::asymmetric::{ed25519, x25519};
use nym_network_defaults::NymNetworkDetails;
use nym_sdk_session::{GatewaySpec, HopConfig, Session, SessionConfig, WgRole};
use nym_smoldvpn::{PeerConfig, TunnelBuilder};
use tokio_util::sync::CancellationToken;
use tonic::transport::Endpoint;
use tonic_health::pb::{health_client::HealthClient, HealthCheckRequest};
use tracing::{error, info, warn};

/// Establishment bound: healthy bring-up is ~100ms; 15s allows several
/// WireGuard handshake retransmissions before declaring the hop dead.
const ESTABLISH_BOUND: std::time::Duration = std::time::Duration::from_secs(15);

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

fn parse_gateway(spec: &str) -> Result<GatewaySpec, String> {
    if spec.eq_ignore_ascii_case("random") {
        Ok(GatewaySpec::Random)
    } else if spec.len() == 2 && spec.chars().all(|c| c.is_ascii_alphabetic()) {
        Ok(GatewaySpec::Country(spec.to_uppercase()))
    } else {
        ed25519::PublicKey::from_base58_string(spec)
            .map(GatewaySpec::Identity)
            .map_err(|e| format!("invalid gateway spec '{spec}': {e}"))
    }
}

fn peer_from_hop(hop: &HopConfig) -> PeerConfig {
    PeerConfig {
        gateway_public_key: hop.wg_config.public_key,
        // `x25519::PrivateKey` is `!Clone`; reconstruct from bytes so the
        // registration outlives tunnel construction (needed for the
        // invalidate-and-re-register fallback below).
        client_private_key: x25519::PrivateKey::from_secret(hop.client_private_key.to_bytes()),
        preshared_key: hop.wg_config.psk.as_ref().map(|p| *p.as_bytes()),
        endpoint: hop.wg_config.endpoint,
        assigned_ipv4: hop.wg_config.private_ipv4,
        assigned_ipv6: Some(hop.wg_config.private_ipv6),
    }
}

async fn run() -> Result<(), String> {
    init_logging();
    let mut gateway_spec = None;
    let mut target = None;
    let mut service = String::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--gateway" => gateway_spec = args.next(),
            "--target" => target = args.next(),
            "--service" => service = args.next().unwrap_or_default(),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    let gateway = parse_gateway(&gateway_spec.ok_or("missing --gateway <spec>")?)?;
    let target = target.ok_or("missing --target <host:port>")?;

    let mnemonic = std::env::var("MNEMONIC")
        .map_err(|_| "set MNEMONIC to a funded mnemonic".to_string())?
        .parse()
        .map_err(|e| format!("invalid mnemonic: {e}"))?;

    // Provision + register a single-hop tunnel.
    let network = NymNetworkDetails::new_from_env();
    // Per-example, per-network data dir: `data/smoldvpn-grpc/<network>`.
    let data_dir = format!("data/smoldvpn-grpc/{}", network.network_name);
    std::fs::create_dir_all(&data_dir).map_err(|e| format!("create data dir: {e}"))?;
    info!("credential store + data directory: {data_dir}");
    let session = Session::new(
        SessionConfig {
            mnemonic,
            network,
            credential_store_path: Some(format!("{data_dir}/creds.db").into()),
            data_path: data_dir.clone().into(),
            dvpn_directory_url: None,
            automatic_topups: None,
            bandwidth_provider: None,
            reuse_registrations: true,
        },
        CancellationToken::new(),
    )
    .await
    .map_err(|e| format!("session init: {e}"))?;
    let result = async {
        session
            .ensure_ticketbooks(false)
            .await
            .map_err(|e| format!("ticketbook issuance: {e}"))?;
        let registration = session
            .register_single_hop(&gateway)
            .await
            .map_err(|e| format!("registration: {e}"))?;

        let tunnel = TunnelBuilder::single_hop(peer_from_hop(&registration.entry))
            .connect()
            .await
            .map_err(|e| format!("tunnel connect: {e}"))?;

        // Gate on WireGuard establishment. A failure here is the signature of a
        // stale cached registration: invalidate it, register fresh (spending a
        // ticket), and rebuild once.
        let tunnel = match tunnel.await_established(ESTABLISH_BOUND).await {
            Ok(()) => tunnel,
            Err(status) => {
                warn!(
                    "cached registration failed to establish within \
                     {ESTABLISH_BOUND:?} ({status}); re-registering"
                );
                tunnel.shutdown().await;
                session
                    .invalidate_registration(&registration.entry.gateway_identity, WgRole::Entry);
                let registration = session
                    .register_single_hop(&gateway)
                    .await
                    .map_err(|e| format!("re-registration: {e}"))?;
                let tunnel = TunnelBuilder::single_hop(peer_from_hop(&registration.entry))
                    .connect()
                    .await
                    .map_err(|e| format!("tunnel reconnect: {e}"))?;
                tunnel
                    .await_established(ESTABLISH_BOUND)
                    .await
                    .map_err(|s| {
                        format!("tunnel failed to establish after fresh registration: {s}")
                    })?;
                tunnel
            }
        };

        // Build a tonic channel whose transport dials through the tunnel.
        let uri = format!("http://{target}");
        let channel = Endpoint::from_shared(uri)
            .map_err(|e| format!("bad target uri: {e}"))?
            .connect_with_connector(tunnel.connector())
            .await
            .map_err(|e| format!("grpc connect through tunnel: {e}"))?;

        // Issue a real gRPC Health.Check request through the tunnel.
        let mut client = HealthClient::new(channel);
        let response = client
            .check(HealthCheckRequest { service })
            .await
            .map_err(|e| format!("grpc health check: {e}"))?;
        info!(
            "gRPC Health.Check through the tunnel returned status: {:?}",
            response.into_inner().status()
        );

        tunnel.shutdown().await;
        Ok(())
    }
    .await;

    // Close the session's credential store cleanly (checkpoints the sqlite WAL;
    // stored tickets are retained) whether the flow succeeded or failed.
    session.shutdown().await;
    result
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(e) = run().await {
        error!("{e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
