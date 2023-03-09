// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use validator_client::nyxd::wallet::DirectSecp256k1HdWallet;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    // allowed values are 12, 18 or 24
    pub word_count: Option<usize>,
}

pub fn create_account(args: Args, prefix: &str) {
    let word_count = args.word_count.unwrap_or(24);
    let mnemonic = bip39::Mnemonic::generate(word_count).expect("failed to generate mnemonic!");

    let wallet = DirectSecp256k1HdWallet::from_mnemonic(prefix, mnemonic);

    // Output address and mnemonics into separate lines for easier parsing
    println!("{}", wallet.mnemonic());
    println!("{}", wallet.try_derive_accounts().unwrap()[0].address());
}
