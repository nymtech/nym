#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use rocket_contrib::serve::StaticFiles;
use tokio::sync::broadcast;

mod jobs;
mod utils;
mod websockets;

// this specifies number of messages that can be held by the channel, not number of the clients.
const BROADCAST_CAPACITY: usize = 10;

#[get("/")]
fn index() -> &'static str {
    "Later we will chop this up into multiple routes, but for now we'll just use StaticFiles. Leaving this here as a pointer for the future."
}

#[tokio::main]
async fn main() {
    tokio::spawn(async move {
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
        websockets::listen(8080, sender_clone).await;
    });

    jobs::start().await;
}
