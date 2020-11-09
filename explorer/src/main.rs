#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use rocket_contrib::serve::StaticFiles;
use std::thread;

mod jobs;
mod utils;
mod websocket;

#[get("/")]
fn index() -> &'static str {
    "Later we will chop this up into multiple routes, but for now we'll just use StaticFiles. Leaving this here as a pointer for the future."
}

#[tokio::main]
async fn main() {
    thread::spawn(|| {
        rocket::ignite()
            .mount("/", StaticFiles::from("public"))
            .launch()
    });

    tokio::spawn(async move {
        websocket::start().await;
    });

    jobs::start().await;
}
