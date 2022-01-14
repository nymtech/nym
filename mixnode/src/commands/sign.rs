// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::*;
use crate::config::{persistence::pathfinder::MixNodePathfinder, Config};
use clap::ArgGroup;
use colored::Colorize;
use config::NymConfig;
use crypto::asymmetric::identity;
use log::error;

const SIGN_TEXT_ARG_NAME: &str = "text";
const SIGN_ADDRESS_ARG_NAME: &str = "address";

#[derive(Args)]
#[clap(group(ArgGroup::new("sign").required(true).args(&["address", "text"])))]
pub(crate) struct Sign {
    /// The id of the mixnode you want to sign with
    #[clap(long)]
    id: String,

    /// Signs your blockchain address with your identity key
    #[clap(long)]
    address: Option<String>,

    /// Signs an arbitrary piece of text with your identity key
    #[clap(long)]
    text: Option<String>,
}

pub fn load_identity_keys(pathfinder: &MixNodePathfinder) -> identity::KeyPair {
    let identity_keypair: identity::KeyPair = pemstore::load_keypair(&pemstore::KeyPairPath::new(
        pathfinder.private_identity_key().to_owned(),
        pathfinder.public_identity_key().to_owned(),
    ))
    .expect("Failed to read stored identity key files");
    identity_keypair
}

fn print_signed_address(private_key: &identity::PrivateKey, raw_address: &str) -> String {
    let trimmed = raw_address.trim();
    validate_bech32_address_or_exit(trimmed);
    let signature = private_key.sign_text(trimmed);

    println!(
        "The base58-encoded signature on '{}' is: {}",
        trimmed, signature
    );
    signature
}

fn print_signed_text(private_key: &identity::PrivateKey, text: &str) {
    println!(
        "Signing the text {:?} using your mixnode's Ed25519 identity key...",
        text
    );

    let signature = private_key.sign_text(text);

    println!(
        "The base58-encoded signature on '{}' is: {}",
        text, signature
    )
}

pub(crate) fn execute(args: &Sign) {
    let config = match Config::load_from_file(Some(&args.id)) {
        Ok(cfg) => cfg,
        Err(err) => {
            error!(
                "Failed to load config for {}. Are you sure you have run `init` before? (Error was: {})",
                args.id,
                err,
            );
            return;
        }
    };

    if !version_check(&config) {
        error!("failed the local version check");
        return;
    }

    let pathfinder = MixNodePathfinder::new_from_config(&config);
    let identity_keypair = load_identity_keys(&pathfinder);

    if let Some(text) = args.text.as_deref() {
        print_signed_text(identity_keypair.private_key(), &text)
    } else if let Some(address) = args.address.as_deref() {
        print_signed_address(identity_keypair.private_key(), &address);
    } else {
        let error_message = format!(
            "You must specify either '--{}' or '--{}' argument!",
            SIGN_TEXT_ARG_NAME, SIGN_ADDRESS_ARG_NAME
        )
        .red();
        println!("{}", error_message);
    }
}
