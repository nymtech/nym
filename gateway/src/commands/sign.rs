// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::{ensure_correct_bech32_prefix, OverrideConfig};
use crate::error::GatewayError;
use crate::support::config::build_config;
use crate::{
    commands::ensure_config_version_compatibility,
    config::persistence::pathfinder::GatewayPathfinder,
};
use anyhow::{anyhow, Result};
use clap::{ArgGroup, Args};
use nym_crypto::asymmetric::identity;
use std::error::Error;
use validator_client::nyxd;

#[derive(Args, Clone)]
#[clap(group(ArgGroup::new("sign").required(true).args(&["wallet_address", "text"])))]
pub struct Sign {
    /// The id of the mixnode you want to sign with
    #[clap(long)]
    id: String,

    /// Signs your blockchain address with your identity key
    #[clap(long)]
    wallet_address: Option<nyxd::AccountId>,

    /// Signs an arbitrary piece of text with your identity key
    #[clap(long)]
    text: Option<String>,
}

enum SignedTarget {
    Text(String),
    Address(nyxd::AccountId),
}

impl TryFrom<Sign> for SignedTarget {
    type Error = anyhow::Error;

    fn try_from(args: Sign) -> Result<Self, Self::Error> {
        if let Some(text) = args.text {
            Ok(SignedTarget::Text(text))
        } else if let Some(address) = args.wallet_address {
            Ok(SignedTarget::Address(address))
        } else {
            // This is unreachable, and hopefully clap will support it explicitly by outputting an
            // enum from the ArgGroup in the future.
            // See: https://github.com/clap-rs/clap/issues/2621
            Err(anyhow!("Error: missing signed target flag"))
        }
    }
}

pub fn load_identity_keys(pathfinder: &GatewayPathfinder) -> identity::KeyPair {
    let identity_keypair: identity::KeyPair = pemstore::load_keypair(&pemstore::KeyPairPath::new(
        pathfinder.private_identity_key().to_owned(),
        pathfinder.public_identity_key().to_owned(),
    ))
    .expect("Failed to read stored identity key files");
    identity_keypair
}

fn print_signed_address(
    private_key: &identity::PrivateKey,
    wallet_address: nyxd::AccountId,
) -> Result<(), GatewayError> {
    // perform extra validation to ensure we have correct prefix
    ensure_correct_bech32_prefix(&wallet_address)?;

    let signature = private_key.sign_text(wallet_address.as_ref());
    eprintln!("The base58-encoded signature on '{wallet_address}' is: {signature}");
    Ok(())
}

fn print_signed_text(private_key: &identity::PrivateKey, text: &str) {
    eprintln!(
        "Signing the text {:?} using your mixnode's Ed25519 identity key...",
        text
    );

    let signature = private_key.sign_text(text);

    eprintln!(
        "The base58-encoded signature on '{}' is: {}",
        text, signature
    );
}

pub fn execute(args: Sign) -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = build_config(args.id.clone(), OverrideConfig::default())?;
    ensure_config_version_compatibility(&config)?;

    let signed_target = SignedTarget::try_from(args)?;
    let pathfinder = GatewayPathfinder::new_from_config(&config);
    let identity_keypair = load_identity_keys(&pathfinder);

    match signed_target {
        SignedTarget::Text(text) => print_signed_text(identity_keypair.private_key(), &text),
        SignedTarget::Address(addr) => print_signed_address(identity_keypair.private_key(), addr)?,
    }

    Ok(())
}
