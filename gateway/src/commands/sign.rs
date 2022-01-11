// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::*;
use crate::config::{persistence::pathfinder::GatewayPathfinder, Config};
use clap::{App, Arg, ArgMatches};
use colored::Colorize;
use config::NymConfig;
use crypto::asymmetric::identity;
use log::error;

const SIGN_TEXT_ARG_NAME: &str = "text";
const SIGN_ADDRESS_ARG_NAME: &str = "address";

pub fn command_args<'a, 'b>() -> App<'a, 'b> {
    let cmd = App::new("sign")
        .about("Sign text to prove ownership of this mixnode")
        .arg(
            Arg::with_name(ID_ARG_NAME)
                .long(ID_ARG_NAME)
                .help("The id of the mixnode you want to sign with")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(SIGN_TEXT_ARG_NAME)
                .long(SIGN_TEXT_ARG_NAME)
                .help("Signs an arbitrary piece of text with your identity key")
                .takes_value(true)
                .conflicts_with(SIGN_ADDRESS_ARG_NAME),
        );

    let mut address_sign_cmd = Arg::with_name(SIGN_ADDRESS_ARG_NAME)
        .long(SIGN_ADDRESS_ARG_NAME)
        .help("Signs your blockchain address with your identity key")
        .takes_value(true)
        .conflicts_with(SIGN_TEXT_ARG_NAME);

    if cfg!(feature = "coconut") {
        // without coconut feature, we shall just take our mnemonic
        // and derive address from it
        address_sign_cmd = address_sign_cmd.takes_value(true);
    }

    cmd.arg(address_sign_cmd)
}

pub fn load_identity_keys(pathfinder: &GatewayPathfinder) -> identity::KeyPair {
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

pub fn execute(matches: &ArgMatches) {
    let id = matches.value_of(ID_ARG_NAME).unwrap();

    let config = match Config::load_from_file(Some(id)) {
        Ok(cfg) => cfg,
        Err(err) => {
            error!("Failed to load config for {}. Are you sure you have run `init` before? (Error was: {})", id, err);
            return;
        }
    };

    if !version_check(&config) {
        error!("failed the local version check");
        return;
    }

    let pathfinder = GatewayPathfinder::new_from_config(&config);
    let identity_keypair = load_identity_keys(&pathfinder);

    if let Some(text) = matches.value_of(SIGN_TEXT_ARG_NAME) {
        print_signed_text(identity_keypair.private_key(), text)
    } else if let Some(address) = matches.value_of(SIGN_ADDRESS_ARG_NAME) {
        print_signed_address(identity_keypair.private_key(), address);
    } else {
        let error_message = format!(
            "You must specify either '--{}' or '--{}' argument!",
            SIGN_TEXT_ARG_NAME, SIGN_ADDRESS_ARG_NAME
        )
        .red();
        println!("{}", error_message);
    }
}
