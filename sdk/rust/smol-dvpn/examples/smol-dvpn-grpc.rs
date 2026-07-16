// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! `smol-dvpn-grpc` — issue a `tonic` gRPC request through the tunnel
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
//!   cargo run -p nym-smol-dvpn --example smol-dvpn-grpc -- \
//!     --gateway <spec> --target <host:port> [--service <name>]

use std::process::ExitCode;

use nym_crypto::asymmetric::ed25519;
use nym_network_defaults::NymNetworkDetails;
use nym_sdk_session::{GatewaySpec, HopConfig, Session, SessionConfig};
use nym_smol_dvpn::{PeerConfig, TunnelBuilder};
use tokio_util::sync::CancellationToken;
use tonic::transport::Endpoint;
use tonic_health::pb::{health_client::HealthClient, HealthCheckRequest};

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
        gateway_public_key: hop.wg_config.public_key.to_bytes(),
        client_private_key: hop.client_private_key.to_bytes(),
        preshared_key: hop.wg_config.psk.as_ref().map(|p| *p.as_bytes()),
        endpoint: hop.wg_config.endpoint,
        assigned_ipv4: hop.wg_config.private_ipv4,
        assigned_ipv6: Some(hop.wg_config.private_ipv6),
    }
}

async fn run() -> Result<(), String> {
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
    let session = Session::new(
        SessionConfig {
            mnemonic,
            network: NymNetworkDetails::new_from_env(),
            credential_store_path: Some("smol-dvpn-grpc-creds.db".into()),
            data_path: "smol-dvpn-grpc-data".into(),
            dvpn_directory_url: None,
        },
        CancellationToken::new(),
    )
    .await
    .map_err(|e| format!("session init: {e}"))?;
    session
        .ensure_ticketbooks(false)
        .await
        .map_err(|e| format!("ticketbook issuance: {e}"))?;
    let registration = session
        .register_single_hop(&gateway, false)
        .await
        .map_err(|e| format!("registration: {e}"))?;

    let tunnel = TunnelBuilder::single_hop(peer_from_hop(&registration.entry))
        .connect()
        .await
        .map_err(|e| format!("tunnel connect: {e}"))?;

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
    println!(
        "gRPC Health.Check through the tunnel returned status: {:?}",
        response.into_inner().status()
    );

    tunnel.shutdown().await;
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(e) = run().await {
        eprintln!("error: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
