// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use log::trace;
use nym_credentials_interface::{generate_keypair_user, generate_keypair_user_from_seed, Base58};
use serde::{Deserialize, Serialize};
use std::io::stdout;

#[derive(Serialize, Deserialize)]
pub struct Bs58EncodedKeys {
    pub secret_key: String,
    pub public_key: String,
}

#[derive(Debug, Parser)]
pub struct Args {
    /// Secret value that's used for deriving underlying ecash keypair
    #[clap(long)]
    pub(crate) bs58_encoded_client_secret: Option<String>,
}

pub fn generate_ecash_keypair(args: Args) -> anyhow::Result<()> {
    trace!("args: {args:?}");

    let keypair = if let Some(secret) = args.bs58_encoded_client_secret {
        let seed = bs58::decode(&secret).into_vec()?;
        generate_keypair_user_from_seed(&seed)
    } else {
        generate_keypair_user()
    };

    let encoded = Bs58EncodedKeys {
        secret_key: keypair.secret_key().to_bs58(),
        public_key: keypair.public_key().to_bs58(),
    };

    serde_json::to_writer_pretty(stdout(), &encoded)?;

    Ok(())
}
