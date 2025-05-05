use clap::{Args, Parser, Subcommand};
use nym_bin_common::output_format::OutputFormat;
use nym_crypto::asymmetric::ed25519;
use nym_types::helpers::ConsoleSigningOutput;
use nym_validator_client::nyxd::error::NyxdError;
use std::path::PathBuf;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct MixnetOperatorsIdentityKey {
    #[clap(subcommand)]
    pub command: MixnetOperatorsIdentityKeyCommands,
}

#[derive(Debug, Subcommand)]
pub enum MixnetOperatorsIdentityKeyCommands {
    /// Register a name alias for a nym address
    Sign(SignArgs),
}

#[derive(Debug, Parser)]
pub struct SignArgs {
    /// Path to private identity key (example: private_identity_key.pem)
    #[clap(long)]
    private_key: PathBuf,

    /// Base58 encoded message to sign
    #[clap(long)]
    base58_msg: String,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub async fn sign(args: SignArgs) -> Result<(), NyxdError> {
    eprintln!(">>> loading: {}", args.private_key.display());
    let private_identity_key: ed25519::PrivateKey =
        nym_pemstore::load_key(args.private_key).expect("failed to load key");

    print_signed_msg(&private_identity_key, &args.base58_msg, args.output);
    Ok(())
}

fn print_signed_msg(private_key: &ed25519::PrivateKey, raw_msg: &str, output: OutputFormat) {
    let trimmed = raw_msg.trim();
    eprintln!(">>> attempting to sign: {trimmed}");

    let Ok(decoded) = bs58::decode(trimmed).into_vec() else {
        println!("failed to base58 decode the message, did you copy it correctly?");
        return;
    };

    eprintln!(">>> decoding the message...");

    // we don't really care about what particular information is embedded inside of it,
    // we just want to know if user correctly copied the string, i.e. whether it's a valid bs58 encoded json
    if serde_json::from_slice::<serde_json::Value>(&decoded).is_err() {
        println!("failed to parse the message after decoding, did you copy it correctly?");
        return;
    };

    // if this is a valid json, it MUST be a valid string
    let decoded_string = String::from_utf8(decoded.clone()).unwrap();
    let signature = private_key.sign(&decoded).to_base58_string();

    let sign_output = ConsoleSigningOutput::new(decoded_string, signature);
    println!("{}", output.format(&sign_output));
}
