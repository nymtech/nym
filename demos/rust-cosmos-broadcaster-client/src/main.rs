use clap::{CommandFactory, Parser, Subcommand, Args};
use nym_validator_client::nyxd::CosmWasmClient;
use nym_validator_client::signing::direct_wallet::DirectSecp256k1HdWallet;
use nym_validator_client::signing::tx_signer::TxSigner;
use nym_validator_client::signing::SignerData;
use cosmrs::bank::MsgSend;
use cosmrs::rpc::{self, HttpClient};
use cosmrs::tx::Msg;
use cosmrs::{tx, AccountId, Coin, Denom};


#[derive(Debug, Parser)]
#[clap(name = "cosmos tx broadcaster ")]
#[clap(about = "binary which accepts pre-signed txs from the mixnet and broadcasts them to a cosmos sdk chain ")]
struct Cli {
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Reverses a string
    Reverse(Reverse),
    /// Inspects a string
    Inspect(Inspect),
}

#[derive(Debug, Args)]
struct Reverse {
    /// The string to reverse
    string: Option<String>,
}

#[derive(Debug, Args)]
struct Inspect {
    /// The string to inspect
    string: Option<String>,
    #[arg(short = 'd', long = "digits")]
    only_digits: bool,
}


fn main() {
    println!("Hello, world!");
}
