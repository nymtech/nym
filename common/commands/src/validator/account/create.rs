// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use nym_validator_client::signing::direct_wallet::DirectSecp256k1HdWallet;
use nym_validator_client::signing::signer::OfflineSigner;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    // allowed values are 12, 15, 18, 21 or 24
    pub word_count: Option<usize>,
}

pub fn create_account(args: Args, prefix: &str) {
    // bip39 3.x takes a `WordCount` rather than a raw count
    let word_count = match args.word_count.unwrap_or(24) {
        12 => bip39::WordCount::Words12,
        15 => bip39::WordCount::Words15,
        18 => bip39::WordCount::Words18,
        21 => bip39::WordCount::Words21,
        24 => bip39::WordCount::Words24,
        other => panic!("invalid word count {other}! allowed values are 12, 15, 18, 21 or 24"),
    };
    let mnemonic = bip39::Mnemonic::generate(word_count).expect("failed to generate mnemonic!");

    let wallet = DirectSecp256k1HdWallet::checked_from_mnemonic(prefix, mnemonic)
        .expect("failed to derive accounts!");

    // Output address and mnemonics into separate lines for easier parsing
    println!("{}", wallet.mnemonic_string().as_str());
    println!("{}", wallet.signer_addresses()[0]);
}
