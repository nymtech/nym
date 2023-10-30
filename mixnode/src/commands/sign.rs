// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::{try_load_current_config, validate_bech32_address_or_exit};
use crate::node::helpers::load_identity_keys;
use anyhow::{bail, Result};
use clap::{ArgGroup, Args};
use log::error;
use nym_bin_common::output_format::OutputFormat;
use nym_crypto::asymmetric::identity;
use nym_types::helpers::ConsoleSigningOutput;
use nym_validator_client::nyxd;
use std::convert::TryFrom;

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

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
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

fn print_signed_address(
    private_key: &identity::PrivateKey,
    wallet_address: nyxd::AccountId,
    output: OutputFormat,
) {
    // perform extra validation to ensure we have correct prefix
    validate_bech32_address_or_exit(wallet_address.as_ref());

    print_signed_text(private_key, wallet_address.as_ref(), output)
}

fn print_signed_text(private_key: &identity::PrivateKey, text: &str, output: OutputFormat) {
    eprintln!("Signing the text {text:?} using your mixnode's Ed25519 identity key...");

    let signature = private_key.sign_text(text);
    let sign_output = ConsoleSigningOutput::new(text, signature);
    println!("{}", output.format(&sign_output));
}

fn print_signed_contract_msg(
    private_key: &identity::PrivateKey,
    raw_msg: &str,
    output: OutputFormat,
) {
    let trimmed = raw_msg.trim();
    eprintln!(">>> attempting to sign {trimmed}");

    let Ok(decoded) = bs58::decode(trimmed).into_vec() else {
        println!("it seems you have incorrectly copied the message to sign. Make sure you didn't accidentally skip any characters");
        return;
    };

    eprintln!(">>> decoding the message...");

    // we don't really care about what particular information is embedded inside of it,
    // we just want to know if user correctly copied the string, i.e. whether it's a valid bs58 encoded json
    if serde_json::from_slice::<serde_json::Value>(&decoded).is_err() {
        println!("it seems you have incorrectly copied the message to sign. Make sure you didn't accidentally skip any characters");
        return;
    };

    // if this is a valid json, it MUST be a valid string
    let decoded_string = String::from_utf8(decoded.clone()).unwrap();
    let signature = private_key.sign(&decoded).to_base58_string();

    let sign_output = ConsoleSigningOutput::new(decoded_string, signature);
    println!("{}", output.format(&sign_output));
}

pub(crate) fn execute(args: &Sign) -> anyhow::Result<()> {
    let config = try_load_current_config(&args.id)?;

    if !version_check(&config) {
        error!("failed the local version check");
        bail!("failed the local version check")
    }

    let signed_target = match SignedTarget::try_from(args.clone()) {
        Ok(s) => s,
        Err(err) => {
            error!("{err}");
            bail!(err);
        }
    };
    let identity_keypair = load_identity_keys(&config)?;

    match signed_target {
        SignedTarget::Text(text) => {
            print_signed_text(identity_keypair.private_key(), &text, args.output)
        }
        SignedTarget::Address(addr) => {
            print_signed_address(identity_keypair.private_key(), addr, args.output)
        }
        SignedTarget::ContractMsg(raw_msg) => {
            print_signed_contract_msg(identity_keypair.private_key(), &raw_msg, args.output)
        }
    }
    Ok(())
}
