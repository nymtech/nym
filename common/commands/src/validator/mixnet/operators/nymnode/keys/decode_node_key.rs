// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use base64::Engine;
use clap::Parser;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(short, long)]
    pub key: String,
}

pub fn decode_node_key(args: Args) {
    let b64_decoded = base64::prelude::BASE64_STANDARD
        .decode(args.key)
        .expect("failed to decode base64 string");
    let b58_encoded = bs58::encode(&b64_decoded).into_string();

    println!("{b58_encoded}")
}
