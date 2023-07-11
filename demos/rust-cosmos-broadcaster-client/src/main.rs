use std::path::PathBuf;
use clap::{ Parser, Subcommand, Args};
use nym_sdk::mixnet::{Recipient, MixnetClientBuilder, StoragePaths};
use nym_validator_client::nyxd::AccountId;
use nym_bin_common::logging::setup_logging;
mod commands;

#[derive(Debug, Parser)]
#[clap(name = "nym cosmos tx signer ")]
#[clap(about = "demo binary with which users can perform offline signing and transmission of signed tx to broadcaster via the mixnet ")]
struct Cli {
    // TODO make this import from file & remove from cli args
    // #[clap(long, global = true)]
    // #[clap(
    //     help = "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC."
    // )]
    // mnemonic: Option<bip39::Mnemonic>,

    // TODO add SP address

    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
   /// sign a transaction offline
    OfflineSignTx(OfflineSignTx),
    /// send signed tx to SP for broadcast
    SendTx(SendTx)
}

#[derive(Debug, Clone, Args)]
struct OfflineSignTx {
    /// mnemonic of signing + sending account (you!) - TODO this will be removed and replaced with file
    mnemonic: bip39::Mnemonic,
    /// recipient nyx chain address for token transfer
    nyx_token_receipient: AccountId
}

#[derive(Debug, Args)]
struct SendTx {
    /// the base58 encoded signed payload created in OfflineSign()
    base58_payload: String,
    /// the nym address of the broadcaster service provider
    sp_address: Recipient
}

#[tokio::main]
async fn main() {
    // setup_logging();
    let cli = Cli::parse();
    // TODO look @ arg env setup from NR main.rs
    // TODO take from args
    let sp_address = Recipient::try_from_base58_string("HfbesQm2pRYCN4BAdYXhkqXBbV1Pp929mtKsESVeWXh8.8AgoUPUQbXNBCPaqAaWd3vnxhc9484qwfgrrQwBngQk2@Ck8zpXTSXMtS9YZ7k7a5BiaoLZfffWuqGWLndujh4Lw4").unwrap();
    let config_dir = PathBuf::from("/tmp/cosmos-broadcaster-mixnet-client-2");
    let storage_paths = StoragePaths::new_from_dir(&config_dir).unwrap();
    let client = MixnetClientBuilder::new_with_default_storage(storage_paths)
        .await
        .unwrap()
        .build()
        .await
        .unwrap();
    let mut client = client.connect_to_mixnet().await.unwrap();
    let our_address = client.nym_address();
    println!("\nOur client nym address is: {our_address}");

    match &cli.command {
        Some(Commands::OfflineSignTx(OfflineSignTx { mnemonic, nyx_token_receipient} )) => {
            println!("sending offline sign info"); 
            let base58_tx_bytes = commands::commands::offline_sign(mnemonic.clone(), nyx_token_receipient.clone(), &mut client, sp_address.clone()).await;

            println!("base58 encoded signed tx payload: \n\n{}\n\n", &base58_tx_bytes);
            println!("do you wish to send the tx? y/n");

            let mut input = String::new();
            let stdin = std::io::stdin();
            let n = stdin.read_line(&mut input).unwrap();

            if input.chars().next().unwrap() == 'y' { // TODO add proper parsing for getting y/n
                println!("\nsending tx thru the mixnet to broadcaster service");
                let tx_hash = commands::commands::send_tx(base58_tx_bytes, sp_address, &mut client).await;
                println!("the response from the broadcaster: {:#?}", tx_hash);
            } else if input.chars().next().unwrap() == 'n' {
                println!("\nok, you can send the signed tx at a later date by passing the base58 string above as the argument for send-tx")
            } else { //TODO make a loop & return to the question if input is not y/n?
                println!("\nunrecognised user input");
            }
        }
        Some(Commands::SendTx(SendTx { base58_payload, sp_address} )) => {
            let tx_hash = commands::commands::send_tx(base58_payload.clone(), sp_address.clone(), &mut client).await;
            println!("the response from the broadcaster: {:#?}", tx_hash);
        }
        None => {println!("no command specified - nothing to do")}
    }
    println!("\nend ~(0.o)~ ")
}
