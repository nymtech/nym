// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::*;
use crate::config::{persistence::pathfinder::GatewayPathfinder, Config};
use clap::{App, Arg, ArgMatches};
use colored::Colorize;
use config::defaults::BECH32_PREFIX;
use config::NymConfig;
use crypto::asymmetric::identity;
use log::error;
use std::process;
use subtle_encoding::bech32;

const SIGN_TEXT_ARG_NAME: &str = "text";
const SIGN_ADDRESS_ARG_NAME: &str = "address";

pub fn command_args<'a, 'b>() -> App<'a, 'b> {
    App::new("sign")
        .about("Sign text to prove ownership of this mixnode")
        .arg(
            Arg::with_name(ID_ARG_NAME)
                .long(ID_ARG_NAME)
                .help("The id of the mixnode you want to sign with")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(SIGN_ADDRESS_ARG_NAME)
                .long(SIGN_ADDRESS_ARG_NAME)
                .help("Signs your blockchain address with your identity key")
                .takes_value(true)
                .conflicts_with(SIGN_TEXT_ARG_NAME),
        )
        .arg(
            Arg::with_name(SIGN_TEXT_ARG_NAME)
                .long(SIGN_TEXT_ARG_NAME)
                .help("Signs an arbitrary piece of text with your identity key")
                .takes_value(true)
                .conflicts_with(SIGN_ADDRESS_ARG_NAME),
        )
}

fn load_identity_keys(pathfinder: &GatewayPathfinder) -> identity::KeyPair {
    let identity_keypair: identity::KeyPair = pemstore::load_keypair(&pemstore::KeyPairPath::new(
        pathfinder.private_identity_key().to_owned(),
        pathfinder.public_identity_key().to_owned(),
    ))
    .expect("Failed to read stored identity key files");
    identity_keypair
}

// we do tiny bit of sanity check validation
fn sign_address(private_key: &identity::PrivateKey, raw_address: &str) {
    let trimmed = raw_address.trim();

    // try to decode the address (to make sure it's a valid bech32 encoding)
    let (prefix, _) = match bech32::decode(trimmed) {
        Ok(decoded) => decoded,
        Err(err) => {
            let error_message =
                format!("Your wallet address failed to get decoded! Are you sure you copied it correctly?  The error was: {}", err).red();
            println!("{}", error_message);
            process::exit(1);
        }
    };

    if prefix != BECH32_PREFIX {
        let error_message =
            format!("Your wallet address must start with a '{}'", BECH32_PREFIX).red();
        println!("{}", error_message);
        process::exit(1);
    }

    let signature_bytes = private_key.sign(trimmed.as_ref()).to_bytes();
    let signature = bs58::encode(signature_bytes).into_string();

    println!(
        "The base58-encoded signature on '{}' is: {}",
        trimmed, signature
    )
}

// we just sign whatever the user has provided
fn sign_text(private_key: &identity::PrivateKey, text: &str) {
    println!(
        "Signing the text {:?} using your mixnode's Ed25519 identity key...",
        text
    );

    let signature_bytes = private_key.sign(text.as_ref()).to_bytes();
    let signature = bs58::encode(signature_bytes).into_string();

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
    let pathfinder = GatewayPathfinder::new_from_config(&config);
    let identity_keypair = load_identity_keys(&pathfinder);

    if let Some(text) = matches.value_of(SIGN_TEXT_ARG_NAME) {
        sign_text(identity_keypair.private_key(), text)
    } else if let Some(address) = matches.value_of(SIGN_ADDRESS_ARG_NAME) {
        sign_address(identity_keypair.private_key(), address)
    } else {
        let error_message = format!(
            "You must specify either '--{}' or '--{}' argument!",
            SIGN_TEXT_ARG_NAME, SIGN_ADDRESS_ARG_NAME
        )
        .red();
        println!("{}", error_message);
    }
}
