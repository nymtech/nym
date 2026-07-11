// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! `smol-dvpn-topup` — spend a stored ticket to extend a registration's
//! bandwidth (OpenSpec task 7.2).
//!
//! Spends one stored WireGuard ticket against a gateway's `metadata` endpoint
//! (`topup_bandwidth`) and prints the updated available bandwidth.
//!
//! Usage:
//!   MNEMONIC="<funded mnemonic>" \
//!   cargo run -p nym-smol-dvpn --example smol-dvpn-topup -- \
//!     --gateway-id <ed25519 base58> --metadata-url <https://gateway:port/>
//!
//! `--metadata-url` is the gateway's metadata HTTP endpoint; `--gateway-id` is
//! the gateway identity the credential is bound to. Network defaults to sandbox.

use std::process::ExitCode;

use nym_crypto::asymmetric::ed25519;
use nym_network_defaults::NymNetworkDetails;
use nym_sdk_session::{Session, SessionConfig, WgRole};
use tokio_util::sync::CancellationToken;

async fn run() -> Result<(), String> {
    let mut gateway_id = None;
    let mut metadata_url = None;
    let mut network = "sandbox".to_string();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--gateway-id" => gateway_id = args.next(),
            "--metadata-url" => metadata_url = args.next(),
            "--network" => network = args.next().unwrap_or(network),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    let gateway_id = ed25519::PublicKey::from_base58_string(
        &gateway_id.ok_or("missing --gateway-id <ed25519 base58>")?,
    )
    .map_err(|e| format!("invalid --gateway-id: {e}"))?;
    let metadata_url = metadata_url.ok_or("missing --metadata-url <url>")?;

    let mnemonic = std::env::var("MNEMONIC")
        .map_err(|_| "set MNEMONIC to a funded mnemonic".to_string())?
        .parse()
        .map_err(|e| format!("invalid mnemonic: {e}"))?;
    let network = match network.as_str() {
        "mainnet" => NymNetworkDetails::new_mainnet(),
        _ => NymNetworkDetails::new_from_env(),
    };

    let session = Session::new(
        SessionConfig {
            mnemonic,
            network,
            credential_store_path: Some("smol-dvpn-topup-creds.db".into()),
            data_path: "smol-dvpn-topup-data".into(),
            dvpn_directory_url: None,
        },
        CancellationToken::new(),
    )
    .await
    .map_err(|e| format!("session init failed: {e}"))?;

    // Make sure there is a ticketbook to spend from.
    session
        .ensure_ticketbooks(false)
        .await
        .map_err(|e| format!("ticketbook issuance failed: {e}"))?;

    let before = nym_smol_dvpn::query_available_bandwidth(&metadata_url)
        .await
        .map_err(|e| format!("available-bandwidth query failed: {e}"))?;
    println!("available bandwidth before top-up: {before} bytes");

    // Spend one stored ticket, bound to the gateway identity.
    let credential = session
        .obtain_wireguard_credential(gateway_id, WgRole::Entry)
        .await
        .map_err(|e| format!("could not obtain credential: {e}"))?;

    let after = nym_smol_dvpn::topup_bandwidth(&metadata_url, credential)
        .await
        .map_err(|e| format!("top-up failed: {e}"))?;
    println!("available bandwidth after top-up:  {after} bytes");

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
