use clap::Args;
use nym_bin_common::output_format::OutputFormat;
use nym_client_core::config::persistence::key_pathfinder::ClientKeyPathfinder;
use nym_config::NymConfig;
use nym_crypto::asymmetric::identity;
use nym_types::helpers::ConsoleSigningOutput;

use crate::{config::Config, error::NetworkRequesterError};

use super::version_check;

#[derive(Args, Clone)]
pub(crate) struct Sign {
    /// The id of the mixnode you want to sign with
    #[clap(long)]
    id: String,

    /// Signs a transaction-specific payload, that is going to be sent to the smart contract, with your identity key
    #[clap(long)]
    contract_msg: String,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
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

pub(crate) async fn execute(args: &Sign) -> Result<(), NetworkRequesterError> {
    let id = &args.id;

    let mut config = match Config::load_from_file(id) {
        Ok(cfg) => cfg,
        Err(err) => {
            log::error!(
                "Failed to load config for {}. Are you sure you have run `init` before? (Error was: {err})",
                id,
            );
            return Err(NetworkRequesterError::FailedToLoadConfig(id.to_string()));
        }
    };

    if !config.validate() {
        return Err(NetworkRequesterError::ConfigValidationFailure);
    }

    if config.get_base_mut().set_empty_fields_to_defaults() {
        log::warn!(
            "Some of the core config options were left unset. \
            The default values are going to get used instead."
        );
    }

    if !version_check(&config) {
        log::error!("Failed the local version check");
        return Err(NetworkRequesterError::FailedLocalVersionCheck);
    }

    let pathfinder = ClientKeyPathfinder::new_from_config(config.get_base());
    let identity_keypair = nym_client_core::init::load_identity_keys(&pathfinder)?;

    print_signed_contract_msg(
        identity_keypair.private_key(),
        &args.contract_msg,
        args.output,
    );

    Ok(())
}
