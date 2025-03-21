// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::{try_load_current_config, ConfigOverridableArgs};
use crate::error::NymRewarderError;
use nym_crypto::asymmetric::ed25519;
use rand::rngs::OsRng;
use std::path::PathBuf;
use tracing::warn;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[command(flatten)]
    config_override: ConfigOverridableArgs,

    /// ☣️ Specifies whether the existing keypair should get overwritten
    #[clap(long)]
    unsafe_overwrite: bool,

    /// Specifies custom location for the configuration file of nym validators rewarder.
    #[clap(long, env = "NYM_VALIDATOR_REWARDER_PROCESS_BLOCK_CONFIG_PATH")]
    custom_config_path: Option<PathBuf>,
}

pub(crate) async fn execute(args: Args) -> Result<(), NymRewarderError> {
    let config =
        try_load_current_config(&args.custom_config_path)?.with_override(args.config_override);

    let mut rng = OsRng;

    let keypair = ed25519::KeyPair::new(&mut rng);

    let public_key_path = config
        .storage_paths
        .public_ed25519_identity_key_file
        .clone();
    let private_key_path = config
        .storage_paths
        .private_ed25519_identity_key_file
        .clone();

    if public_key_path.exists() || private_key_path.exists() {
        if !args.unsafe_overwrite {
            return Err(NymRewarderError::AttemptedEd25519KeyOverwrite);
        }

        warn!("☣️☣️☣️ OVERWRITING ed25519 IDENTITY KEYS!!")
    }

    nym_pemstore::store_keypair(
        &keypair,
        &nym_pemstore::KeyPairPath::new(&private_key_path, &public_key_path),
    )
    .map_err(|source| NymRewarderError::Ed25519KeyStoreFailure {
        public_key_path,
        private_key_path,
        source,
    })?;

    Ok(())
}
