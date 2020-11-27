#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use clap::{App, Arg, ArgMatches};
use rocket_contrib::serve::StaticFiles;
use tokio::sync::broadcast;

mod jobs;
mod utils;
mod websockets;

// this specifies number of messages that can be held by the channel, not number of the clients.
const BROADCAST_CAPACITY: usize = 10;
const VALIDATOR_ARG: &str = "validator";
const METRICS_ARG: &str = "metrics";

fn parse_args<'a>() -> ArgMatches<'a> {
    App::new("Nym Explorer")
        .author("Nymtech")
        .arg(
            Arg::with_name(VALIDATOR_ARG)
                .long(VALIDATOR_ARG)
                .help("REST endpoint of the validator that explorer will use to periodically grab topology and node status.")
                .takes_value(true)
        )
        .arg(
            Arg::with_name(METRICS_ARG)
                .long(METRICS_ARG)
                .help("websocket endpoint of the metrics server explorer will subscribe to and broadcast to its clients")
                .takes_value(true)
        )
        .get_matches()
}

#[get("/")]
fn index() -> &'static str {
    "Later we will chop this up into multiple routes, but for now we'll just use StaticFiles. Leaving this here as a pointer for the future."
}

#[tokio::main]
async fn main() {
    let matches = parse_args();
    let validator_base_url = matches
        .value_of(VALIDATOR_ARG)
        .unwrap_or_else(|| "http://testnet-validator1.nymtech.net:8081");
    let metrics_websocket_url = matches
        .value_of(METRICS_ARG)
        .unwrap_or_else(|| "wss://testnet-metrics.nymtech.net/ws")
        .to_owned();

    let public_path = std::env::current_exe()
        .expect("Failed to evaluate current exe path")
        .parent()
        .expect("the binary itself has no parent path?!")
        .join("public");

    std::thread::spawn(|| {
        rocket::ignite()
            .mount("/", StaticFiles::from(public_path))
            .launch()
    });

    let (sender, _) = broadcast::channel(BROADCAST_CAPACITY);
    // the only reason for cloning the sender is that because more receivers can only be created
    // out of senders
    let sender_clone = sender.clone();

    tokio::spawn(async move {
        websockets::subscribe(&*metrics_websocket_url, sender).await;
    });

    tokio::spawn(async move {
        websockets::listen(1648, sender_clone).await;
    });

    jobs::start(validator_base_url).await;
}
