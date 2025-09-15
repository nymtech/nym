// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use clap::Parser;
use cosmrs::crypto::PublicKey;
use log::{error, info};
use serde_json::json;

use nym_validator_client::nyxd::AccountId;

use crate::context::QueryClient;
use crate::validator::signature::helpers::secp256k1_verify_with_public_key;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(value_parser)]
    #[clap(
        help = "The public key of the account, or the account id to query for a public key (NOTE: the account must have signed a message stored on the chain for the public key record to exist)"
    )]
    pub public_key_or_address: String,

    #[clap(value_parser)]
    #[clap(help = "The signature to verify as hex")]
    pub signature_as_hex: String,

    #[clap(value_parser)]
    #[clap(help = "The message to verify as a string")]
    pub message: String,
}

pub async fn verify(args: Args, client: &QueryClient) {
    if args.public_key_or_address.trim().is_empty() {
        error!("Please ensure the public key or address is not empty or whitespace");
        return;
    }

    let public_key = match AccountId::from_str(&args.public_key_or_address) {
        Ok(address) => {
            info!("Found account address instead of public key, so looking up public key for {address} from chain");
            match client.get_account_public_key(&address).await.ok() {
                Some(public_key) => {
                    if let Some(k) = public_key {
                        info!("Found public key {}", json!(k));
                    }
                    public_key
                }
                None => {
                    error!(
                        "Address {address} does not have a public key recorded on the chain. This is probably because the account has never signed a transaction."
                    );
                    None
                }
            }
        }
        Err(_) => match PublicKey::from_json(&args.public_key_or_address) {
            Ok(parsed) => Some(parsed),
            Err(e) => {
                error!("Public key should be JSON. Unable to parse: {e}");
                None
            }
        },
    };

    match public_key {
        Some(public_key) => {
            if public_key.type_url() != PublicKey::SECP256K1_TYPE_URL {
                error!("Sorry, we only support secp256k1 public keys at the moment");
                return;
            }

            match secp256k1_verify_with_public_key(
                &public_key.to_bytes(),
                args.signature_as_hex,
                args.message,
            ) {
                Ok(()) => println!("SUCCESS ✅ signature verified"),
                Err(e) => {
                    error!("FAILURE ❌ Signature verification failed: {e}");
                }
            }
        }
        None => {
            error!("Unable to verify, as unable to get the public key");
        }
    }
}
