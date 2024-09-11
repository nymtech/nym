// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(short, long)]
    pub key: String,
}

pub fn decode_mixnode_key(args: Args) {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    let b64_decoded = STANDARD
        .decode(args.key)
        .expect("failed to decode base64 string");
    let b58_encoded = bs58::encode(&b64_decoded).into_string();

    println!("{b58_encoded}")
}
