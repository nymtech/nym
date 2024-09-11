// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::helpers::ConfigArgs;
use crate::node::helpers::load_ed25519_identity_keypair;
use nym_bin_common::output_format::OutputFormat;
use nym_crypto::asymmetric::identity;
use nym_node::config::upgrade_helpers::try_load_current_config;
use nym_node::error::NymNodeError;
use nym_types::helpers::ConsoleSigningOutput;

// I don't think it makes sense to expose 'text' and 'contract-msg' as env variables
#[derive(Debug, clap::Args)]
#[clap(group = clap::ArgGroup::new("message").required(true))]
pub struct Args {
    #[clap(flatten)]
    config: ConfigArgs,

    /// Signs an arbitrary piece of text with your identity key
    #[clap(long, group = "message")]
    text: Option<String>,

    /// Signs a transaction-specific payload, that is going to be sent to the smart contract, with your identity key
    #[clap(long, group = "message")]
    contract_msg: Option<String>,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

fn print_signed_text(private_key: &identity::PrivateKey, text: &str, output: OutputFormat) {
    eprintln!("Signing the text {text:?} using your node's Ed25519 identity key...",);

    let signature = private_key.sign_text(text);
    let sign_output = ConsoleSigningOutput::new(text, signature);
    output.to_stdout(&sign_output);
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

    // SAFETY:
    // if this is a valid json, it MUST be a valid string
    #[allow(clippy::unwrap_used)]
    let decoded_string = String::from_utf8(decoded.clone()).unwrap();
    let signature = private_key.sign(&decoded).to_base58_string();

    let sign_output = ConsoleSigningOutput::new(decoded_string, signature);
    println!("{}", output.format(&sign_output));
}

pub async fn execute(args: Args) -> Result<(), NymNodeError> {
    let config = try_load_current_config(args.config.config_path()).await?;
    let identity_keypair =
        load_ed25519_identity_keypair(config.storage_paths.keys.ed25519_identity_storage_paths())?;

    // note: due to clap's ArgGroup, one (and only one) of those branches will be called
    if let Some(text) = args.text {
        print_signed_text(identity_keypair.private_key(), &text, args.output);
        Ok(())
    } else if let Some(contract_msg) = args.contract_msg {
        print_signed_contract_msg(identity_keypair.private_key(), &contract_msg, args.output);
        Ok(())
    } else {
        unreachable!()
    }
}
