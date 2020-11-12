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

fn parse_args<'a>() -> ArgMatches<'a> {
    App::new("Nym Explorer")
        .author("Nymtech")
        .arg(
            Arg::with_name(VALIDATOR_ARG)
                .help("REST endpoint of the explorer will use to periodically grab topology and node status.")
                .takes_value(true)
                .required(true),
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
    let validator_base_url = matches.value_of(VALIDATOR_ARG).unwrap();

    tokio::task::spawn_blocking(|| {
        rocket::ignite()
            .mount("/", StaticFiles::from("public"))
            .launch()
    });

    let (sender, _) = broadcast::channel(BROADCAST_CAPACITY);
    // the only reason for cloning the sender is that because more receivers can only be created
    // out of senders
    let sender_clone = sender.clone();

    tokio::spawn(async move {
        websockets::subscribe("wss://qa-metrics.nymtech.net/ws", sender).await;
    });

    tokio::spawn(async move {
        websockets::listen(1648, sender_clone).await;
    });

    jobs::start(validator_base_url).await;
}
