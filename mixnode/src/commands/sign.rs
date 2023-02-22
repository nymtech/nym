// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::convert::TryFrom;

use crate::commands::validate_bech32_address_or_exit;
use crate::config::{persistence::pathfinder::MixNodePathfinder, Config};
use crate::node::MixNode;
use anyhow::{bail, Result};
use clap::{ArgGroup, Args};
use log::error;
use nym_config::NymConfig;
use nym_crypto::asymmetric::identity;
use validator_client::nyxd;

use super::version_check;

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

fn print_signed_address(private_key: &identity::PrivateKey, wallet_address: nyxd::AccountId) {
    // perform extra validation to ensure we have correct prefix
    validate_bech32_address_or_exit(wallet_address.as_ref());

    let signature = private_key.sign_text(wallet_address.as_ref());
    println!("The base58-encoded signature on '{wallet_address}' is: {signature}",);
}

fn print_signed_text(private_key: &identity::PrivateKey, text: &str) {
    println!("Signing the text {text:?} using your mixnode's Ed25519 identity key...");

    let signature = private_key.sign_text(text);

    println!("The base58-encoded signature on '{text}' is: {signature}");
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

pub(crate) fn execute(args: &Sign) {
    let config = match Config::load_from_file(&args.id) {
        Ok(cfg) => cfg,
        Err(err) => {
            error!(
                "Failed to load config for {}. Are you sure you have run `init` before? (Error was: {err})",
                args.id,
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
        SignedTarget::Address(addr) => print_signed_address(identity_keypair.private_key(), addr),
        SignedTarget::ContractMsg(raw_msg) => {
            print_signed_contract_msg(identity_keypair.private_key(), &raw_msg)
        }
    }
}
