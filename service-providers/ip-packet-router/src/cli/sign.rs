use crate::cli::{try_load_current_config, version_check};
use crate::error::IpPacketRouterError;
use clap::Args;
use nym_bin_common::output_format::OutputFormat;
use nym_client_core::client::key_manager::persistence::OnDiskKeys;
use nym_client_core::error::ClientCoreError;
use nym_crypto::asymmetric::identity;
use nym_types::helpers::ConsoleSigningOutput;

#[derive(Args, Clone)]
pub(crate) struct Sign {
    /// The id of the mixnode you want to sign with
    #[arg(long)]
    id: String,

    /// Signs a transaction-specific payload, that is going to be sent to the smart contract, with your identity key
    #[arg(long)]
    contract_msg: String,

    #[arg(short, long, default_value_t = OutputFormat::default())]
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

pub(crate) async fn execute(args: &Sign) -> Result<(), IpPacketRouterError> {
    let config = try_load_current_config(&args.id).await?;

    if !version_check(&config) {
        log::error!("Failed the local version check");
        return Err(IpPacketRouterError::FailedLocalVersionCheck);
    }

    let key_store = OnDiskKeys::new(config.storage_paths.common_paths.keys);
    let identity_keypair = key_store.load_identity_keypair().map_err(|source| {
        IpPacketRouterError::ClientCoreError(ClientCoreError::KeyStoreError {
            source: Box::new(source),
        })
    })?;

    print_signed_contract_msg(
        identity_keypair.private_key(),
        &args.contract_msg,
        args.output,
    );

    Ok(())
}
