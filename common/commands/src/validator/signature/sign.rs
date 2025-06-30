// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use cosmrs::crypto::PublicKey;
use log::error;
use nym_validator_client::signing::direct_wallet::DirectSecp256k1HdWallet;
use nym_validator_client::signing::signer::OfflineSigner;
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Serialize)]
pub struct SignatureOutputJson {
    pub account_id: String,
    pub public_key: PublicKey,
    pub signature_as_hex: String,
}

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(value_parser)]
    #[clap(help = "The message to sign")]
    pub message: String,
}

pub fn sign(args: Args, prefix: &str, mnemonic: Option<bip39::Mnemonic>) {
    if args.message.trim().is_empty() {
        error!("Message is empty or contains only whitespace");
        return;
    }

    if mnemonic.is_none() {
        error!(
            "Please provide the mnemonic as an argument or using the MNEMONIC environment variable"
        );
        return;
    }

    let wallet =
        DirectSecp256k1HdWallet::from_mnemonic(prefix, mnemonic.expect("mnemonic not set"));
    match wallet.try_derive_accounts() {
        Ok(accounts) => match accounts.first() {
            Some(account) => {
                let msg = args.message.into_bytes();
                match wallet.sign_raw_with_account(account, msg) {
                    Ok(signature) => {
                        let output = SignatureOutputJson {
                            account_id: account.address().to_string(),
                            public_key: account.public_key(),
                            signature_as_hex: signature.to_string(),
                        };
                        println!("{}", json!(output));
                    }
                    Err(e) => {
                        error!("Failed to sign message. {e}");
                    }
                }
            }
            None => {
                error!("Could not derive an account key from the mnemonic",)
            }
        },
        Err(e) => {
            error!("Failed to derive accounts. {e}");
        }
    }
}
