// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::env::vars::*;
use nym_crypto::asymmetric::x25519;
use tracing::info;

/// Arguments for the `keygen` subcommand.
#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    /// Specifies the path to the noise key file used for establishing tunnel with the node being tested
    #[arg(long, env = NYM_NETWORK_MONITOR_AGENT_NOISE_KEY_PATH_ARG)]
    noise_key_path: String,
}

/// Generates a fresh x25519 Noise private key and writes it to the path specified in `args`.
pub(crate) fn execute(args: Args) -> anyhow::Result<()> {
    let mut rng = rand::thread_rng();
    let noise_key = x25519::PrivateKey::new(&mut rng);

    nym_pemstore::store_key(&noise_key, &args.noise_key_path)?;
    info!("noise key written to '{}'", args.noise_key_path);
    Ok(())
}
