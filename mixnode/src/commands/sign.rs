use crate::commands::*;
use crate::config::{persistence::pathfinder::MixNodePathfinder, Config};
use clap::{App, Arg, ArgMatches};
use colored::*;
use config::NymConfig;
use crypto::asymmetric::identity;
use log::error;

const SIGN_TEXT_ARG_NAME: &str = "text";

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
            Arg::with_name(SIGN_TEXT_ARG_NAME)
                .long(SIGN_TEXT_ARG_NAME)
                .help("The text to sign")
                .takes_value(true)
                .required(true),
        )
}

fn load_identity_keys(pathfinder: &MixNodePathfinder) -> identity::KeyPair {
    let identity_keypair: identity::KeyPair = pemstore::load_keypair(&pemstore::KeyPairPath::new(
        pathfinder.private_identity_key().to_owned(),
        pathfinder.public_identity_key().to_owned(),
    ))
    .expect("Failed to read stored identity key files");
    identity_keypair
}

pub fn execute(matches: &ArgMatches) {
    let id = matches.value_of(ID_ARG_NAME).unwrap();
    let text = matches.value_of(SIGN_TEXT_ARG_NAME).unwrap();

    let config = match Config::load_from_file(Some(id)) {
        Ok(cfg) => cfg,
        Err(err) => {
            error!("Failed to load config for {}. Are you sure you have run `init` before? (Error was: {})", id, err);
            return;
        }
    };
    let pathfinder = MixNodePathfinder::new_from_config(&config);
    let identity_keypair = load_identity_keys(&pathfinder);
    let signature_bytes = identity_keypair
        .private_key()
        .sign(text.as_ref())
        .to_bytes();

    let signature = bs58::encode(signature_bytes).into_string();
    let identity = identity_keypair.public_key().to_base58_string();

    let channel_name = "@nymchan_help_chat".bright_cyan();

    println!(
        "Signing the text {:?} using your mixnode's Ed25519 identity key...",
        text
    );
    println!();
    println!("Signature is: {}", signature);
    println!();
    println!("You can claim your mixnode in Telegram by talking to our bot. To do so:");
    println!();
    println!("* go to the '{}' channel", channel_name);
    println!("* copy the following line of text, and paste it into the channel");
    println!();
    println!("/claim {} {}", identity, signature);
    println!();
}
