// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;

#[derive(Debug, Parser)]
pub(crate) struct Args {
    #[clap(short, long)]
    pub key: String,
}

pub(crate) fn decode_mixnode_key(args: Args) {
    let b64_decoded = base64::decode(args.key).expect("failed to decode base64 string");
    let b58_encoded = bs58::encode(&b64_decoded).into_string();

    println!("{}", b58_encoded)
}
