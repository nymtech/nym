use clap::{Args, Parser, Subcommand};
use nym_sdk::mixnet::Recipient;
use nym_validator_client::nyxd::AccountId;
use rust_cosmos_broadcaster::{
    client::{offline_sign, send_tx},
    create_client,
};

#[derive(Debug, Parser)]
#[clap(name = "nym cosmos tx signer ")]
#[clap(
    about = "demo binary with which users can perform offline signing and transmission of signed tx to broadcaster via the mixnet "
)]
struct Cli {
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// sign a transaction offline
    OfflineSignTx(OfflineSignTx),
    /// send signed tx to SP for broadcast
    SendTx(SendTx),
}

#[derive(Debug, Clone, Args)]
struct OfflineSignTx {
    /// mnemonic of signing + sending account (you!) - TODO this will be removed and replaced with file
    mnemonic: bip39::Mnemonic,
    /// recipient nyx chain address for token transfer
    nyx_token_receipient: AccountId,
}

#[derive(Debug, Args)]
struct SendTx {
    /// the base58 encoded signed payload created in OfflineSign()
    base58_payload: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // setup_logging();
    let cli = Cli::parse();
    let mut client = create_client("/tmp/cosmos-broadcaster-mixnet-client-5".into()).await;
    let our_address = client.nym_address();
    println!("\nclient's nym address: {our_address}\n");

    let sp_address = Recipient::try_from_base58_string("DP84PUbje5nMuz4HYSqpdPYHrb5WPjitTyufGubc6MNy.8YhkurLGEeSuRLxhKG5uY7Kz6M4YjytKpUNdZhmo8z56@HcH4JQ4oZ8M4mMXDj5UDAb4WpuhpTKGHBEsZ112mkPkm").unwrap();

    match &cli.command {
        Some(Commands::OfflineSignTx(OfflineSignTx {
            mnemonic,
            nyx_token_receipient,
        })) => {
            println!("sending offline sign info to broadcaster via the mixnet: getting signing account sequence and chain ID");
            let base58_tx_bytes = offline_sign(
                mnemonic.clone(),
                nyx_token_receipient.clone(),
                &mut client,
                sp_address,
            )
            .await;

            println!(
                "Encoded response (signed tx data) as base58 for tx broadcast: \n\n{:?}\n",
                &base58_tx_bytes
            );
            println!("do you wish to send the tx? y/n");

            let mut input = String::new();
            let stdin = std::io::stdin();
            let _n = stdin.read_line(&mut input).unwrap();

            if input.starts_with('y') {
                println!("\nsending tx thru the mixnet to broadcaster service");
                let Ok((tx_hash, success)) = send_tx(base58_tx_bytes.unwrap(), sp_address, &mut client).await else { todo!() };
                println!(
                    "tx hash returned from the broadcaster: {}\ntx was successful: {}",
                    tx_hash, success
                );
            } else if input.starts_with('n') {
                println!("\nok, you can send the signed tx at a later date by passing the base58 string above as the argument for send-tx");
            } else {
                println!("\nunrecognised user input");
            }
        }
        Some(Commands::SendTx(SendTx { base58_payload })) => {
            let tx_hash = send_tx(base58_payload.clone(), sp_address, &mut client).await;
            println!("response from the broadcaster (tx hash) {:#?}", tx_hash);
        }
        None => {
            println!("\nno command specified - nothing to do")
        }
    }
    println!("\ndisconnecting client");
    println!("end");
    Ok(())
}
