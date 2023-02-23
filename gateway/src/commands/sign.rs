// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::{ensure_correct_bech32_prefix, OverrideConfig};
use crate::error::GatewayError;
use crate::support::config::build_config;
use crate::{
    commands::ensure_config_version_compatibility,
    config::persistence::pathfinder::GatewayPathfinder,
};
use anyhow::{bail, Result};
use clap::{ArgGroup, Args};
use nym_crypto::asymmetric::identity;
use std::error::Error;
use validator_client::nyxd;

#[derive(Args, Clone)]
#[clap(group(ArgGroup::new("sign").required(true).args(&["wallet_address", "text", "contract_msg"])))]
pub(crate) struct Sign {
    /// The id of the mixnode you want to sign with
    #[clap(long)]
    id: String,

    /// Signs your blockchain address with your identity key
    // the alias here is included for backwards compatibility (1.1.4 and before)
    #[clap(long, alias = "address")]
    wallet_address: Option<nyxd::AccountId>,

    /// Signs an arbitrary piece of text with your identity key
    #[clap(long)]
    text: Option<String>,

    /// Signs a transaction-specific payload, that is going to be sent to the smart contract, with your identity key
    #[clap(long)]
    contract_msg: Option<String>,
}

enum SignedTarget {
    Text(String),
    Address(nyxd::AccountId),
    ContractMsg(String),
}

impl TryFrom<Sign> for SignedTarget {
    type Error = anyhow::Error;

    fn try_from(args: Sign) -> Result<Self, Self::Error> {
        if let Some(text) = args.text {
            Ok(SignedTarget::Text(text))
        } else if let Some(address) = args.wallet_address {
            Ok(SignedTarget::Address(address))
        } else if let Some(msg) = args.contract_msg {
            Ok(SignedTarget::ContractMsg(msg))
        } else {
            // This is unreachable, and hopefully clap will support it explicitly by outputting an
            // enum from the ArgGroup in the future.
            // See: https://github.com/clap-rs/clap/issues/2621
            bail!("Error: missing signed target flag")
        }
    }
}

pub fn load_identity_keys(pathfinder: &GatewayPathfinder) -> identity::KeyPair {
    let identity_keypair: identity::KeyPair =
        nym_pemstore::load_keypair(&nym_pemstore::KeyPairPath::new(
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

fn print_signed_contract_msg(private_key: &identity::PrivateKey, raw_msg: &str) {
    // we don't really care about what particular information is embedded inside of it,
    // we just want to know if user correctly copied the string, i.e. whether it's a valid json
    if serde_json::from_str::<serde_json::Value>(raw_msg).is_err() {
        println!("it seems you have incorrectly copied the message to sign. Make sure you didn't accidentally skip any characters")
    } else {
        let msg = raw_msg.trim();
        let signature = private_key.sign(msg.trim().as_bytes()).to_base58_string();
        println!("The base58-encoded signature on\n{msg}\nis:\n{signature}");
    }
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
        SignedTarget::ContractMsg(raw_msg) => {
            print_signed_contract_msg(identity_keypair.private_key(), &raw_msg)
        }
    }

    Ok(())
}
