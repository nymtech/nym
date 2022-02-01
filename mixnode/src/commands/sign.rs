// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::convert::TryFrom;

use crate::commands::validate_bech32_address_or_exit;
use crate::config::{persistence::pathfinder::MixNodePathfinder, Config};
use crate::node::MixNode;
use anyhow::{anyhow, Result};
use clap::{ArgGroup, Args};
use config::NymConfig;
use crypto::asymmetric::identity;
use log::error;

use super::version_check;

#[derive(Args, Clone)]
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

enum SignedTarget {
    Text(String),
    Address(String),
}

impl TryFrom<Sign> for SignedTarget {
    type Error = anyhow::Error;

    fn try_from(args: Sign) -> Result<Self, Self::Error> {
        if let Some(text) = args.text {
            Ok(SignedTarget::Text(text))
        } else if let Some(address) = args.address {
            Ok(SignedTarget::Address(address))
        } else {
            // This is unreachable, and hopefully clap will support it explicitly by outputting an
            // enum from the ArgGroup in the future.
            // See: https://github.com/clap-rs/clap/issues/2621
            Err(anyhow!("Error: missing signed target flag"))
        }
    }
}

fn print_signed_address(private_key: &identity::PrivateKey, raw_address: &str) {
    let trimmed = raw_address.trim();
    validate_bech32_address_or_exit(trimmed);
    let signature = private_key.sign_text(trimmed);

    println!(
        "The base58-encoded signature on '{}' is: {}",
        trimmed, signature
    );
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
    );
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
        error!("Failed the local version check");
        return;
    }

    let signed_target = match SignedTarget::try_from(args.clone()) {
        Ok(s) => s,
        Err(err) => {
            error!("{err}");
            return;
        }
    };
    let pathfinder = MixNodePathfinder::new_from_config(&config);
    let identity_keypair = MixNode::load_identity_keys(&pathfinder);

    match signed_target {
        SignedTarget::Text(text) => print_signed_text(identity_keypair.private_key(), &text),
        SignedTarget::Address(addr) => print_signed_address(identity_keypair.private_key(), &addr),
    }
}
