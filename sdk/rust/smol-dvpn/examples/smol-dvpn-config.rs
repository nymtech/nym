// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! `smol-dvpn-config` — single-hop LP registration → plain WireGuard config
//! export (OpenSpec task 7.1).
//!
//! Registers one gateway with a funded mnemonic and prints a `[Interface]` /
//! `[Peer]` config usable with stock `wg`/`wg-quick`. The exported config works
//! only until the registered zk-nym bandwidth is exhausted (stock WireGuard
//! performs no top-up — use `smol-dvpn-topup` for that).
//!
//! Usage:
//!   MNEMONIC="<funded mnemonic>" \
//!   cargo run -p nym-smol-dvpn --example smol-dvpn-config -- --gateway <spec>
//!
//! `<spec>` is `random`, a two-letter country code (e.g. `CH`), or a gateway
//! ed25519 identity (base58). The network defaults to sandbox; override with
//! `--network mainnet`.

use std::process::ExitCode;

use nym_crypto::asymmetric::ed25519;
use nym_network_defaults::NymNetworkDetails;
use nym_sdk_session::{GatewaySpec, Session, SessionConfig};
use tokio_util::sync::CancellationToken;

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

/// Standard-alphabet base64 encoder (avoids an extra dep for key export).
fn base64_encode(input: &[u8]) -> String {
    const ALPHA: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        out.push(ALPHA[(b[0] >> 2) as usize] as char);
        out.push(ALPHA[(((b[0] & 0x03) << 4) | (b[1] >> 4)) as usize] as char);
        out.push(if chunk.len() > 1 {
            ALPHA[(((b[1] & 0x0f) << 2) | (b[2] >> 6)) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHA[(b[2] & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}

async fn run() -> Result<(), String> {
    // --- args ---
    let mut gateway_spec = None;
    let mut network = "sandbox".to_string();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--gateway" => gateway_spec = args.next(),
            "--network" => network = args.next().unwrap_or(network),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    let gateway = parse_gateway(&gateway_spec.ok_or("missing --gateway <spec>")?)?;

    let mnemonic = std::env::var("MNEMONIC")
        .map_err(|_| "set MNEMONIC to a funded mnemonic".to_string())?
        .parse()
        .map_err(|e| format!("invalid mnemonic: {e}"))?;

    let network = match network.as_str() {
        "mainnet" => NymNetworkDetails::new_mainnet(),
        _ => NymNetworkDetails::new_from_env(),
    };

    // --- register a single hop ---
    let cancel = CancellationToken::new();
    let session = Session::new(
        SessionConfig {
            mnemonic,
            network,
            credential_store_path: Some("smol-dvpn-config-creds.db".into()),
            data_path: "smol-dvpn-config-data".into(),
            dvpn_directory_url: None,
        },
        cancel,
    )
    .await
    .map_err(|e| format!("session init failed: {e}"))?;

    session
        .ensure_ticketbooks(false)
        .await
        .map_err(|e| format!("ticketbook issuance failed: {e}"))?;

    let registration = session
        .register_single_hop(&gateway)
        .await
        .map_err(|e| format!("registration failed: {e}"))?;

    // --- emit a stock WireGuard config ---
    let hop = &registration.entry;
    let wg = &hop.wg_config;
    let private = base64_encode(&hop.client_private_key.to_bytes());
    let peer_public = base64_encode(&wg.public_key.to_bytes());

    println!("[Interface]");
    println!("PrivateKey = {private}");
    println!("Address = {}/32, {}/128", wg.private_ipv4, wg.private_ipv6);
    println!();
    println!("[Peer]");
    println!("PublicKey = {peer_public}");
    if let Some(psk) = &wg.psk {
        println!("PresharedKey = {}", base64_encode(psk.as_bytes()));
    }
    println!("Endpoint = {}", wg.endpoint);
    println!("AllowedIPs = 0.0.0.0/0, ::/0");

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
